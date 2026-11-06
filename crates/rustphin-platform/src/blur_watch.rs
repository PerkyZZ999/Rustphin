//! An owned, cancellable connection for capability events. Never polls a
//! borrowed toolkit display after its handle's lifetime.

use std::{
    io,
    net::Shutdown,
    os::{fd::AsFd, unix::net::UnixStream},
    thread::{self, JoinHandle},
};

use raw_window_handle::{HasDisplayHandle, RawDisplayHandle};
use rustix::{
    event::{PollFd, PollFlags, poll},
    net::{
        AddressFamily, SocketAddrUnix, SocketFlags, SocketType, UCred, connect, getpeername,
        socket_with, sockopt::socket_peercred,
    },
};
use wayland_backend::client::{Backend, WaylandError};
use wayland_client::{
    Connection, Dispatch, Proxy, QueueHandle,
    protocol::{wl_callback, wl_registry},
};
use wayland_protocols::ext::background_effect::v1::client::ext_background_effect_manager_v1::{
    self as manager, ExtBackgroundEffectManagerV1,
};

use super::BlurError;

/// A live compositor capability subscription and its worker's lifetime guard.
///
/// This owns a separate connection to the toolkit display's verified Unix
/// peer. Unnamed connections cannot be monitored and fail closed. It never
/// reads the toolkit display or uses its surface from the worker thread.
pub struct BlurWatch {
    changes: flume::Receiver<Result<bool, BlurError>>,
    stop: UnixStream,
    worker: Option<JoinHandle<()>>,
}

impl BlurWatch {
    /// Capability changes, starting with the compositor's initial state.
    ///
    /// `true` means blur is advertised, not that the caller has attached an
    /// effect. An error or channel closure requires opaque rendering.
    pub fn changes(&self) -> flume::Receiver<Result<bool, BlurError>> {
        self.changes.clone()
    }

    fn spawn(endpoint: Endpoint) -> Result<Self, BlurError> {
        let (stop, cancelled) = UnixStream::pair().map_err(connection_error)?;
        let (sender, changes) = flume::unbounded();
        let worker = thread::Builder::new()
            .name("rustphin-blur".into())
            .spawn(move || {
                if let Err(error) = monitor(endpoint, &cancelled, sender.clone()) {
                    // Receiver closure means the window has already gone away.
                    let _ = sender.send(Err(error));
                }
            })
            .map_err(connection_error)?;
        Ok(Self {
            changes,
            stop,
            worker: Some(worker),
        })
    }
}

impl Drop for BlurWatch {
    fn drop(&mut self) {
        if let Err(error) = self.stop.shutdown(Shutdown::Both)
            && error.kind() != io::ErrorKind::NotConnected
        {
            eprintln!("could not stop the background-blur monitor: {error}");
        }
        if let Some(worker) = self.worker.take()
            && worker.join().is_err()
        {
            eprintln!("the background-blur monitor panicked");
        }
    }
}

struct Endpoint {
    address: SocketAddrUnix,
    credentials: UCred,
}

impl Endpoint {
    fn from_socket(socket: impl AsFd) -> Result<Self, BlurError> {
        let address: SocketAddrUnix = getpeername(&socket)
            .map_err(connection_error)?
            .ok_or(BlurError::Unmonitorable)?
            .try_into()
            .map_err(|_| BlurError::Unmonitorable)?;
        // The address decoder represents an unnamed peer as an empty abstract name.
        if address.path_bytes().is_none()
            && address.abstract_name().is_none_or(|name| name.is_empty())
        {
            return Err(BlurError::Unmonitorable);
        }
        let credentials = socket_peercred(&socket).map_err(connection_error)?;
        Ok(Self {
            address,
            credentials,
        })
    }
}

pub(super) fn start(display: &dyn HasDisplayHandle) -> Result<BlurWatch, BlurError> {
    let handle = display.display_handle().map_err(connection_error)?;
    let RawDisplayHandle::Wayland(display) = handle.as_raw() else {
        return Err(BlurError::NotWayland);
    };
    // SAFETY: the live wl_display is borrowed only for this call. The foreign
    // backend does not close it. Only owned socket metadata escapes the borrow.
    let backend = unsafe { Backend::from_foreign_display(display.display.as_ptr().cast()) };
    let endpoint = Endpoint::from_socket(backend.poll_fd())?;
    BlurWatch::spawn(endpoint)
}

fn connection_error(error: impl std::fmt::Display) -> BlurError {
    BlurError::Connection(error.to_string())
}

fn would_block(error: &WaylandError) -> bool {
    matches!(error, WaylandError::Io(error) if error.kind() == io::ErrorKind::WouldBlock)
}

/// `None` means cancellation, including during connection or initial registry discovery.
fn wait(
    socket: impl AsFd,
    cancelled: &UnixStream,
    interests: PollFlags,
) -> Result<Option<PollFlags>, BlurError> {
    let mut fds = [
        PollFd::new(&socket, interests),
        PollFd::new(cancelled, PollFlags::IN),
    ];
    loop {
        match poll(&mut fds, None) {
            Err(rustix::io::Errno::INTR) => continue,
            result => result.map_err(connection_error)?,
        };
        return if fds[1].revents().is_empty() {
            Ok(Some(fds[0].revents()))
        } else {
            Ok(None)
        };
    }
}

fn monitor(
    endpoint: Endpoint,
    cancelled: &UnixStream,
    sender: flume::Sender<Result<bool, BlurError>>,
) -> Result<(), BlurError> {
    let socket = socket_with(
        AddressFamily::UNIX,
        SocketType::STREAM,
        SocketFlags::CLOEXEC | SocketFlags::NONBLOCK,
        None,
    )
    .map_err(connection_error)?;
    match connect(&socket, &endpoint.address) {
        Ok(()) => {}
        Err(rustix::io::Errno::INPROGRESS) => {
            if wait(&socket, cancelled, PollFlags::OUT)?.is_none() {
                return Ok(());
            }
            rustix::net::sockopt::socket_error(&socket)
                .map_err(connection_error)?
                .map_err(connection_error)?;
        }
        Err(error) => return Err(connection_error(error)),
    }
    if socket_peercred(&socket).map_err(connection_error)? != endpoint.credentials {
        return Err(BlurError::Unmonitorable);
    }
    monitor_connection(
        Connection::from_socket(UnixStream::from(socket)).map_err(connection_error)?,
        cancelled,
        sender,
    )
}

fn monitor_connection(
    connection: Connection,
    cancelled: &UnixStream,
    sender: flume::Sender<Result<bool, BlurError>>,
) -> Result<(), BlurError> {
    let mut queue = connection.new_event_queue();
    let handle = queue.handle();
    let display = connection.display();
    display.get_registry(&handle, ());
    display.sync(&handle, ());
    let mut state = WatchState {
        manager: None,
        last: None,
        sender,
    };
    loop {
        queue
            .dispatch_pending(&mut state)
            .map_err(connection_error)?;
        let interests = match connection.flush() {
            Ok(()) => PollFlags::IN,
            Err(error) if would_block(&error) => PollFlags::IN | PollFlags::OUT,
            Err(error) => return Err(connection_error(error)),
        };
        let Some(read) = queue.prepare_read() else {
            continue;
        };
        let Some(ready) = wait(read.connection_fd(), cancelled, interests)? else {
            return Ok(());
        };
        if ready.intersects(PollFlags::ERR | PollFlags::HUP | PollFlags::NVAL) {
            return Err(connection_error("compositor disconnected"));
        }
        if ready.contains(PollFlags::IN) {
            match read.read() {
                Ok(_) => {}
                Err(error) if would_block(&error) => {}
                Err(error) => return Err(connection_error(error)),
            }
        }
    }
}

struct WatchState {
    manager: Option<(u32, ExtBackgroundEffectManagerV1)>,
    last: Option<bool>,
    sender: flume::Sender<Result<bool, BlurError>>,
}

impl WatchState {
    fn publish(&mut self, available: bool) {
        if self.last != Some(available) {
            self.last = Some(available);
            // A closed receiver means the window no longer needs these events.
            let _ = self.sender.send(Ok(available));
        }
    }
}

impl Dispatch<wl_registry::WlRegistry, ()> for WatchState {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        handle: &QueueHandle<Self>,
    ) {
        match event {
            wl_registry::Event::Global {
                name,
                interface,
                version,
            } if interface == ExtBackgroundEffectManagerV1::interface().name
                && version >= 1
                && state.manager.is_none() =>
            {
                state.manager = Some((name, registry.bind(name, 1, handle, ())));
            }
            wl_registry::Event::GlobalRemove { name }
                if state.manager.as_ref().is_some_and(|(id, _)| *id == name) =>
            {
                if let Some((_, manager)) = state.manager.take() {
                    manager.destroy();
                }
                state.publish(false);
            }
            _ => {}
        }
    }
}

impl Dispatch<wl_callback::WlCallback, ()> for WatchState {
    fn event(
        state: &mut Self,
        _: &wl_callback::WlCallback,
        _: wl_callback::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if state.manager.is_none() {
            state.publish(false);
        }
    }
}

impl Dispatch<ExtBackgroundEffectManagerV1, ()> for WatchState {
    fn event(
        state: &mut Self,
        manager: &ExtBackgroundEffectManagerV1,
        event: manager::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if !state
            .manager
            .as_ref()
            .is_some_and(|(_, current)| current == manager)
        {
            return;
        }
        if let manager::Event::Capabilities { flags } = event {
            state.publish(u32::from(flags) & manager::Capability::Blur.bits() != 0);
        }
    }
}

#[cfg(test)]
#[path = "blur_watch_tests.rs"]
mod tests;

//! Protocol contracts against an in-process compositor; no desktop or socket path.

#![allow(clippy::unwrap_used)]

use std::{
    io::Write as _,
    os::unix::net::UnixStream,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    thread::{self, JoinHandle},
};

use rustix::event::{PollFd, PollFlags, poll};
use wayland_client::{Connection, globals::registry_queue_init};
use wayland_protocols::ext::background_effect::v1::server::{
    ext_background_effect_manager_v1::{self as manager, ExtBackgroundEffectManagerV1},
    ext_background_effect_surface_v1::{self as effect, ExtBackgroundEffectSurfaceV1},
};
use wayland_server::{
    Client, DataInit, Dispatch, Display, DisplayHandle, GlobalDispatch, New,
    protocol::{
        wl_compositor::{self, WlCompositor},
        wl_region, wl_surface,
    },
};

use super::{BlurError, State, enable_on};

wayland_client::delegate_noop!(State: ignore wayland_client::protocol::wl_surface::WlSurface);

struct Compositor {
    flags: u32,
    blur_requests: Arc<AtomicUsize>,
    managers: Vec<ExtBackgroundEffectManagerV1>,
}

impl GlobalDispatch<WlCompositor, ()> for Compositor {
    fn bind(
        _: &mut Self,
        _: &DisplayHandle,
        _: &Client,
        resource: New<WlCompositor>,
        _: &(),
        init: &mut DataInit<'_, Self>,
    ) {
        init.init(resource, ());
    }
}

impl Dispatch<WlCompositor, ()> for Compositor {
    fn request(
        _: &mut Self,
        _: &Client,
        _: &WlCompositor,
        request: wl_compositor::Request,
        _: &(),
        _: &DisplayHandle,
        init: &mut DataInit<'_, Self>,
    ) {
        match request {
            wl_compositor::Request::CreateSurface { id } => {
                init.init(id, ());
            }
            wl_compositor::Request::CreateRegion { id } => {
                init.init(id, ());
            }
            _ => panic!("unexpected compositor request"),
        }
    }
}

impl GlobalDispatch<ExtBackgroundEffectManagerV1, ()> for Compositor {
    fn bind(
        state: &mut Self,
        _: &DisplayHandle,
        _: &Client,
        resource: New<ExtBackgroundEffectManagerV1>,
        _: &(),
        init: &mut DataInit<'_, Self>,
    ) {
        let manager = init.init(resource, ());
        manager.capabilities(manager::Capability::from_bits_retain(state.flags));
        state.managers.push(manager);
    }
}

impl Dispatch<ExtBackgroundEffectManagerV1, ()> for Compositor {
    fn request(
        _: &mut Self,
        _: &Client,
        _: &ExtBackgroundEffectManagerV1,
        request: manager::Request,
        _: &(),
        _: &DisplayHandle,
        init: &mut DataInit<'_, Self>,
    ) {
        match request {
            manager::Request::GetBackgroundEffect { id, .. } => {
                init.init(id, ());
            }
            manager::Request::Destroy => {}
            _ => panic!("unexpected background-effect manager request"),
        }
    }
}

impl Dispatch<ExtBackgroundEffectSurfaceV1, ()> for Compositor {
    fn request(
        state: &mut Self,
        _: &Client,
        _: &ExtBackgroundEffectSurfaceV1,
        request: effect::Request,
        _: &(),
        _: &DisplayHandle,
        _: &mut DataInit<'_, Self>,
    ) {
        match request {
            effect::Request::SetBlurRegion { region } => {
                assert!(region.is_some());
                state.blur_requests.fetch_add(1, Ordering::SeqCst);
            }
            effect::Request::Destroy => {}
            _ => panic!("unexpected background-effect request"),
        }
    }
}

impl Dispatch<wl_surface::WlSurface, ()> for Compositor {
    fn request(
        _: &mut Self,
        _: &Client,
        _: &wl_surface::WlSurface,
        request: wl_surface::Request,
        _: &(),
        _: &DisplayHandle,
        _: &mut DataInit<'_, Self>,
    ) {
        assert!(matches!(request, wl_surface::Request::Destroy));
    }
}

impl Dispatch<wl_region::WlRegion, ()> for Compositor {
    fn request(
        _: &mut Self,
        _: &Client,
        _: &wl_region::WlRegion,
        request: wl_region::Request,
        _: &(),
        _: &DisplayHandle,
        _: &mut DataInit<'_, Self>,
    ) {
        match request {
            wl_region::Request::Add { .. } | wl_region::Request::Destroy => {}
            _ => panic!("unexpected region request"),
        }
    }
}

pub(super) struct Server {
    commands: flume::Sender<Command>,
    wake: UnixStream,
    thread: Option<JoinHandle<()>>,
}

enum Command {
    Stop,
    Capabilities(u32),
    Remove,
    Add(u32),
}

impl Server {
    pub(super) fn start(
        socket: UnixStream,
        flags: Option<u32>,
        blur_requests: Arc<AtomicUsize>,
    ) -> Self {
        let (wake, stop) = UnixStream::pair().unwrap();
        let (commands, incoming) = flume::unbounded();
        let thread = thread::spawn(move || {
            let mut display = Display::<Compositor>::new().unwrap();
            let mut handle = display.handle();
            handle.create_global::<Compositor, WlCompositor, _>(4, ());
            let mut global = flags.map(|_| {
                handle.create_global::<Compositor, ExtBackgroundEffectManagerV1, _>(1, ())
            });
            handle.insert_client(socket, Arc::new(())).unwrap();
            let mut state = Compositor {
                flags: flags.unwrap_or_default(),
                blur_requests,
                managers: Vec::new(),
            };
            loop {
                let mut fds = [
                    PollFd::new(&display, PollFlags::IN),
                    PollFd::new(&stop, PollFlags::IN),
                ];
                poll(&mut fds, None).unwrap();
                if fds[1].revents().intersects(PollFlags::IN | PollFlags::HUP) {
                    use std::io::Read as _;
                    (&stop).read_exact(&mut [0]).unwrap();
                    match incoming.recv().unwrap() {
                        Command::Stop => break,
                        Command::Capabilities(flags) => {
                            state.flags = flags;
                            for manager in &state.managers {
                                manager.capabilities(manager::Capability::from_bits_retain(flags));
                            }
                        }
                        Command::Remove => {
                            handle.remove_global::<Compositor>(global.take().unwrap());
                            state.managers.clear();
                        }
                        Command::Add(flags) => {
                            assert!(global.is_none());
                            state.flags = flags;
                            global = Some(
                                handle
                                    .create_global::<Compositor, ExtBackgroundEffectManagerV1, _>(
                                        1,
                                        (),
                                    ),
                            );
                        }
                    }
                }
                display.dispatch_clients(&mut state).unwrap();
                display.flush_clients().unwrap();
            }
        });
        Self {
            commands,
            wake,
            thread: Some(thread),
        }
    }
    fn send(&self, command: Command) {
        self.commands.send(command).unwrap();
        (&self.wake).write_all(&[1]).unwrap();
    }

    pub(super) fn capabilities(&self, flags: u32) {
        self.send(Command::Capabilities(flags));
    }

    pub(super) fn remove(&self) {
        self.send(Command::Remove);
    }

    pub(super) fn add(&self, flags: u32) {
        self.send(Command::Add(flags));
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.send(Command::Stop);
        if let Some(thread) = self.thread.take() {
            thread.join().unwrap();
        }
    }
}

fn request_blur(flags: Option<u32>) -> (Result<(), BlurError>, usize) {
    let (client, server) = UnixStream::pair().unwrap();
    let requests = Arc::new(AtomicUsize::new(0));
    let _server = Server::start(server, flags, requests.clone());
    let connection = Connection::from_socket(client).unwrap();
    let (globals, mut queue) = registry_queue_init::<State>(&connection).unwrap();
    let handle = queue.handle();
    let compositor: wayland_client::protocol::wl_compositor::WlCompositor =
        globals.bind(&handle, 1..=4, ()).unwrap();
    let surface = compositor.create_surface(&handle, ());
    queue.roundtrip(&mut State::default()).unwrap();
    let result = enable_on(connection, surface);
    (result, requests.load(Ordering::SeqCst))
}

#[test]
fn a_manager_without_blur_capability_does_not_enable_transparency() {
    assert_eq!(request_blur(Some(0)), (Err(BlurError::Unsupported), 0));
}

#[test]
fn an_advertised_blur_capability_requests_the_effect() {
    assert_eq!(request_blur(Some(1)), (Ok(()), 1));
}

#[test]
fn an_unknown_capability_alone_does_not_enable_transparency() {
    assert_eq!(request_blur(Some(2)), (Err(BlurError::Unsupported), 0));
}

#[test]
fn a_future_capability_does_not_hide_advertised_blur_support() {
    assert_eq!(request_blur(Some(3)), (Ok(()), 1));
}

#[test]
fn a_missing_manager_does_not_enable_transparency() {
    assert_eq!(request_blur(None), (Err(BlurError::Unsupported), 0));
}

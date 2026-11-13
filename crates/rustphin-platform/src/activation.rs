//! Window focus for windows a running Rustphin opens on request (W-110d).
//!
//! On Wayland a window takes the focus only with the activation token of
//! the user action that asked for it. A launch passes its
//! `XDG_ACTIVATION_TOKEN` to the running Rustphin, and this module hands
//! that token to the compositor for the new window through
//! `xdg_activation_v1`, as the toolkit does itself only for a process's
//! first window. The toolkit's connection is borrowed as for blur (D-010).

use raw_window_handle::{HasDisplayHandle, HasWindowHandle, RawDisplayHandle, RawWindowHandle};
use wayland_backend::client::{Backend, ObjectId};
use wayland_client::{
    Connection, Proxy as _, QueueHandle,
    globals::{GlobalListContents, registry_queue_init},
    protocol::{wl_registry, wl_surface::WlSurface},
};
use wayland_protocols::xdg::activation::v1::client::xdg_activation_v1::XdgActivationV1;

/// Why a window could not be given its token.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ActivationError {
    /// The window is not a Wayland window.
    #[error("not a Wayland window")]
    NotWayland,
    /// The compositor offers no `xdg_activation_v1`.
    #[error("the compositor does not offer window activation")]
    Unsupported,
    /// Talking to the compositor failed.
    #[error("Wayland connection error: {0}")]
    Connection(String),
}

#[derive(Default)]
struct State;

impl wayland_client::Dispatch<wl_registry::WlRegistry, GlobalListContents> for State {
    fn event(
        _: &mut Self,
        _: &wl_registry::WlRegistry,
        _: wl_registry::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}
wayland_client::delegate_noop!(State: ignore XdgActivationV1);

/// Asks the compositor to activate `window` with `token`.
pub fn activate(
    window: &dyn HasWindowHandle,
    display: &dyn HasDisplayHandle,
    token: &str,
) -> Result<(), ActivationError> {
    let connection_error =
        |error: &dyn std::fmt::Display| ActivationError::Connection(error.to_string());
    let display = display.display_handle().map_err(|e| connection_error(&e))?;
    let RawDisplayHandle::Wayland(display) = display.as_raw() else {
        return Err(ActivationError::NotWayland);
    };
    let window = window.window_handle().map_err(|e| connection_error(&e))?;
    let RawWindowHandle::Wayland(window) = window.as_raw() else {
        return Err(ActivationError::NotWayland);
    };
    // SAFETY: the pointer is the toolkit's live `wl_display`, borrowed for the
    // duration of this call through `display`. libwayland supports several
    // event queues on one display, and a backend made from a foreign display
    // never closes it.
    let backend = unsafe { Backend::from_foreign_display(display.display.as_ptr().cast()) };
    let connection = Connection::from_backend(backend);
    // SAFETY: the pointer is the live `wl_surface` proxy of `window`, borrowed
    // for the duration of this call, and it belongs to the display above.
    let surface_id =
        unsafe { ObjectId::from_ptr(WlSurface::interface(), window.surface.as_ptr().cast()) }
            .map_err(|e| connection_error(&e))?;
    let surface = WlSurface::from_id(&connection, surface_id).map_err(|e| connection_error(&e))?;
    activate_on(connection, &surface, token)
}

/// Sends `activate(token, surface)` and lets go of the global again.
fn activate_on(
    connection: Connection,
    surface: &WlSurface,
    token: &str,
) -> Result<(), ActivationError> {
    let connection_error =
        |error: &dyn std::fmt::Display| ActivationError::Connection(error.to_string());
    let (globals, mut queue) =
        registry_queue_init::<State>(&connection).map_err(|e| connection_error(&e))?;
    let handle = queue.handle();
    let activation: XdgActivationV1 = globals
        .bind(&handle, 1..=1, ())
        .map_err(|_| ActivationError::Unsupported)?;
    activation.activate(token.to_owned(), surface);
    activation.destroy();
    queue
        .roundtrip(&mut State)
        .map_err(|e| connection_error(&e))?;
    // These wrappers refer to a display the toolkit owns and may close
    // first, so they are leaked rather than dropped, as for blur.
    std::mem::forget((queue, connection));
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::{
        os::unix::net::UnixStream,
        sync::{Arc, Mutex},
        thread,
    };

    use wayland_protocols::xdg::activation::v1::server::xdg_activation_v1::{
        self as server_activation, XdgActivationV1 as ServerActivation,
    };
    use wayland_server::{
        Client, DataInit, Dispatch, Display, DisplayHandle, GlobalDispatch, New,
        protocol::{wl_compositor, wl_surface},
    };

    use super::*;

    wayland_client::delegate_noop!(State: ignore wayland_client::protocol::wl_compositor::WlCompositor);
    wayland_client::delegate_noop!(State: ignore WlSurface);

    /// A compositor that records the tokens it is given.
    struct Compositor {
        tokens: Arc<Mutex<Vec<String>>>,
    }

    impl GlobalDispatch<wl_compositor::WlCompositor, ()> for Compositor {
        fn bind(
            _: &mut Self,
            _: &DisplayHandle,
            _: &Client,
            resource: New<wl_compositor::WlCompositor>,
            _: &(),
            init: &mut DataInit<'_, Self>,
        ) {
            init.init(resource, ());
        }
    }
    impl Dispatch<wl_compositor::WlCompositor, ()> for Compositor {
        fn request(
            _: &mut Self,
            _: &Client,
            _: &wl_compositor::WlCompositor,
            request: wl_compositor::Request,
            _: &(),
            _: &DisplayHandle,
            init: &mut DataInit<'_, Self>,
        ) {
            if let wl_compositor::Request::CreateSurface { id } = request {
                init.init(id, ());
            }
        }
    }
    impl Dispatch<wl_surface::WlSurface, ()> for Compositor {
        fn request(
            _: &mut Self,
            _: &Client,
            _: &wl_surface::WlSurface,
            _: wl_surface::Request,
            _: &(),
            _: &DisplayHandle,
            _: &mut DataInit<'_, Self>,
        ) {
        }
    }
    impl GlobalDispatch<ServerActivation, ()> for Compositor {
        fn bind(
            _: &mut Self,
            _: &DisplayHandle,
            _: &Client,
            resource: New<ServerActivation>,
            _: &(),
            init: &mut DataInit<'_, Self>,
        ) {
            init.init(resource, ());
        }
    }
    impl Dispatch<ServerActivation, ()> for Compositor {
        fn request(
            state: &mut Self,
            _: &Client,
            _: &ServerActivation,
            request: server_activation::Request,
            _: &(),
            _: &DisplayHandle,
            _: &mut DataInit<'_, Self>,
        ) {
            if let server_activation::Request::Activate { token, .. } = request {
                state.tokens.lock().unwrap().push(token);
            }
        }
    }

    /// Runs `activate_on` against a compositor with or without activation;
    /// returns the result and the tokens the compositor received.
    fn activate_with(offered: bool) -> (Result<(), ActivationError>, Vec<String>) {
        let (client, server) = UnixStream::pair().unwrap();
        let tokens = Arc::new(Mutex::new(Vec::new()));
        let recorded = tokens.clone();
        let (done, finished) = std::sync::mpsc::channel::<()>();
        let compositor = thread::spawn(move || {
            let mut display = Display::<Compositor>::new().unwrap();
            let mut handle = display.handle();
            handle.create_global::<Compositor, wl_compositor::WlCompositor, _>(4, ());
            if offered {
                handle.create_global::<Compositor, ServerActivation, _>(1, ());
            }
            handle.insert_client(server, Arc::new(())).unwrap();
            let mut state = Compositor { tokens: recorded };
            while finished.try_recv().is_err() {
                display.dispatch_clients(&mut state).unwrap();
                display.flush_clients().unwrap();
                thread::yield_now();
            }
        });
        let connection = Connection::from_socket(client).unwrap();
        let (globals, mut queue) = registry_queue_init::<State>(&connection).unwrap();
        let handle = queue.handle();
        let compositor_proxy: wayland_client::protocol::wl_compositor::WlCompositor =
            globals.bind(&handle, 1..=4, ()).unwrap();
        let surface = compositor_proxy.create_surface(&handle, ());
        queue.roundtrip(&mut State).unwrap();
        let result = activate_on(connection, &surface, "launch-token-7");
        done.send(()).unwrap();
        compositor.join().unwrap();
        let received = tokens.lock().unwrap().clone();
        (result, received)
    }

    #[test]
    fn the_launch_token_reaches_the_compositor_for_the_window() {
        assert_eq!(
            activate_with(true),
            (Ok(()), vec!["launch-token-7".to_owned()])
        );
    }

    #[test]
    fn a_compositor_without_activation_is_reported() {
        assert_eq!(
            activate_with(false),
            (Err(ActivationError::Unsupported), vec![])
        );
    }
}

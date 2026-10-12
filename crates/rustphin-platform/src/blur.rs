//! Compositor blur behind a translucent window.
//!
//! GPUI only speaks the legacy `org_kde_kwin_blur` protocol, which current KWin
//! no longer advertises. [`WaylandBlur`] asks for blur through
//! `ext-background-effect-v1` instead, on the toolkit's own Wayland connection.

use raw_window_handle::{HasDisplayHandle, HasWindowHandle, RawDisplayHandle, RawWindowHandle};
use wayland_backend::client::{Backend, ObjectId};
use wayland_client::{
    Connection, Dispatch, Proxy, QueueHandle, delegate_noop,
    globals::{GlobalListContents, registry_queue_init},
    protocol::{
        wl_compositor::WlCompositor, wl_region::WlRegion, wl_registry, wl_surface::WlSurface,
    },
};
use wayland_protocols::ext::background_effect::v1::client::{
    ext_background_effect_manager_v1::ExtBackgroundEffectManagerV1,
    ext_background_effect_surface_v1::ExtBackgroundEffectSurfaceV1,
};

/// Why a window has no background blur.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum BlurError {
    /// Blur was turned off by configuration.
    #[error("background blur is disabled")]
    Disabled,
    /// The window is not a Wayland surface.
    #[error("background blur needs a Wayland window")]
    NotWayland,
    /// The compositor does not offer the blur protocol.
    #[error("the compositor does not support background blur")]
    Unsupported,
    /// The compositor connection failed while asking for blur.
    #[error("asking the compositor for background blur failed: {0}")]
    Connection(String),
}

/// Requests blur of whatever is behind a window.
pub trait BackgroundBlur {
    /// Asks the compositor to blur the area behind the whole of `window`.
    ///
    /// On success the effect stays for the lifetime of the window's surface. On
    /// failure the caller should draw an opaque background instead.
    fn enable(
        &self,
        window: &dyn HasWindowHandle,
        display: &dyn HasDisplayHandle,
    ) -> Result<(), BlurError>;
}

/// The fallback: never blurs, so callers use their opaque appearance.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoBlur;

impl BackgroundBlur for NoBlur {
    fn enable(&self, _: &dyn HasWindowHandle, _: &dyn HasDisplayHandle) -> Result<(), BlurError> {
        Err(BlurError::Disabled)
    }
}

/// Blur through the `ext-background-effect-v1` Wayland protocol.
#[derive(Debug, Clone, Copy, Default)]
pub struct WaylandBlur;

struct State;

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for State {
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
delegate_noop!(State: ignore ExtBackgroundEffectManagerV1);
delegate_noop!(State: ignore ExtBackgroundEffectSurfaceV1);
delegate_noop!(State: ignore WlCompositor);
delegate_noop!(State: ignore WlRegion);

impl BackgroundBlur for WaylandBlur {
    fn enable(
        &self,
        window: &dyn HasWindowHandle,
        display: &dyn HasDisplayHandle,
    ) -> Result<(), BlurError> {
        let connection_error =
            |error: &dyn std::fmt::Display| BlurError::Connection(error.to_string());
        let display = display.display_handle().map_err(|e| connection_error(&e))?;
        let RawDisplayHandle::Wayland(display) = display.as_raw() else {
            return Err(BlurError::NotWayland);
        };
        let window = window.window_handle().map_err(|e| connection_error(&e))?;
        let RawWindowHandle::Wayland(window) = window.as_raw() else {
            return Err(BlurError::NotWayland);
        };

        // SAFETY: the pointer is the toolkit's live `wl_display`, borrowed for the
        // duration of this call through `display`. libwayland supports several
        // event queues on one display, and a backend made from a foreign display
        // never closes it.
        let backend = unsafe { Backend::from_foreign_display(display.display.as_ptr().cast()) };
        let connection = Connection::from_backend(backend);
        let (globals, mut queue) =
            registry_queue_init::<State>(&connection).map_err(|e| connection_error(&e))?;
        let handle = queue.handle();

        let manager: ExtBackgroundEffectManagerV1 = globals
            .bind(&handle, 1..=1, ())
            .map_err(|_| BlurError::Unsupported)?;
        let compositor: WlCompositor = globals
            .bind(&handle, 1..=4, ())
            .map_err(|e| connection_error(&e))?;

        // SAFETY: the pointer is the live `wl_surface` proxy of `window`, borrowed
        // for the duration of this call, and it belongs to the display above.
        let surface_id =
            unsafe { ObjectId::from_ptr(WlSurface::interface(), window.surface.as_ptr().cast()) }
                .map_err(|e| connection_error(&e))?;
        let surface =
            WlSurface::from_id(&connection, surface_id).map_err(|e| connection_error(&e))?;

        let effect = manager.get_background_effect(&surface, &handle, ());
        let region = compositor.create_region(&handle, ());
        // The compositor clips the region to the surface, so this covers it at any size.
        region.add(0, 0, i32::MAX, i32::MAX);
        effect.set_blur_region(Some(&region));
        region.destroy();
        queue
            .roundtrip(&mut State)
            .map_err(|e| connection_error(&e))?;

        // The effect lasts while its protocol object exists. These wrappers refer
        // to a display the toolkit owns and may close first, so they are leaked
        // rather than dropped: a few small objects per window, never touched again.
        std::mem::forget((effect, manager, compositor, queue, connection));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use raw_window_handle::{
        DisplayHandle, HandleError, WindowHandle, XcbDisplayHandle, XcbWindowHandle,
    };

    use super::*;

    struct X11Window;

    impl HasWindowHandle for X11Window {
        fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
            let raw = XcbWindowHandle::new(std::num::NonZeroU32::MIN);
            // SAFETY: the handle is never dereferenced; the test only inspects its variant.
            Ok(unsafe { WindowHandle::borrow_raw(raw.into()) })
        }
    }

    impl HasDisplayHandle for X11Window {
        fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
            let raw = XcbDisplayHandle::new(None, 0);
            // SAFETY: the handle is never dereferenced; the test only inspects its variant.
            Ok(unsafe { DisplayHandle::borrow_raw(raw.into()) })
        }
    }

    #[test]
    fn wayland_blur_refuses_windows_that_are_not_wayland() {
        assert_eq!(
            WaylandBlur.enable(&X11Window, &X11Window),
            Err(BlurError::NotWayland)
        );
    }

    #[test]
    fn fallback_reports_blur_as_disabled() {
        assert_eq!(
            NoBlur.enable(&X11Window, &X11Window),
            Err(BlurError::Disabled)
        );
    }
}

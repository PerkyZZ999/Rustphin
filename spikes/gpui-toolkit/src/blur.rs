//! DISPOSABLE SPIKE — asks KWin for blur through `ext-background-effect-v1`.
//!
//! GPUI only speaks the legacy `org_kde_kwin_blur` protocol, which Plasma 6.7 no
//! longer advertises. This borrows GPUI's `wl_display` and `wl_surface` through
//! raw-window-handle and issues the request on a private event queue.

use raw_window_handle::{HasDisplayHandle, HasWindowHandle, RawDisplayHandle, RawWindowHandle};
use wayland_backend::client::{Backend, ObjectId};
use wayland_client::{
    Connection, Dispatch, Proxy, QueueHandle, delegate_noop,
    globals::{GlobalListContents, registry_queue_init},
    protocol::{wl_compositor::WlCompositor, wl_region::WlRegion, wl_registry, wl_surface::WlSurface},
};
use wayland_protocols::ext::background_effect::v1::client::{
    ext_background_effect_manager_v1::ExtBackgroundEffectManagerV1,
    ext_background_effect_surface_v1::ExtBackgroundEffectSurfaceV1,
};

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

pub fn enable(window: &(impl HasWindowHandle + HasDisplayHandle)) -> Result<(), String> {
    let RawDisplayHandle::Wayland(display) =
        window.display_handle().map_err(|e| e.to_string())?.as_raw()
    else {
        return Err("not a Wayland display".into());
    };
    let RawWindowHandle::Wayland(surface) =
        window.window_handle().map_err(|e| e.to_string())?.as_raw()
    else {
        return Err("not a Wayland window".into());
    };

    // SAFETY: both pointers come from GPUI's live Wayland connection and window,
    // which outlive this call; libwayland supports several queues per display.
    let backend = unsafe { Backend::from_foreign_display(display.display.as_ptr().cast()) };
    let conn = Connection::from_backend(backend);
    let (globals, mut queue) = registry_queue_init::<State>(&conn).map_err(|e| e.to_string())?;
    let qh = queue.handle();

    let manager: ExtBackgroundEffectManagerV1 = globals
        .bind(&qh, 1..=1, ())
        .map_err(|e| format!("ext_background_effect_manager_v1: {e}"))?;
    let compositor: WlCompositor = globals.bind(&qh, 1..=4, ()).map_err(|e| e.to_string())?;

    let surface_id =
        unsafe { ObjectId::from_ptr(WlSurface::interface(), surface.surface.as_ptr().cast()) }
            .map_err(|e| e.to_string())?;
    let surface = WlSurface::from_id(&conn, surface_id).map_err(|e| e.to_string())?;

    let effect = manager.get_background_effect(&surface, &qh, ());
    let region = compositor.create_region(&qh, ());
    region.add(0, 0, i32::MAX, i32::MAX);
    effect.set_blur_region(Some(&region));
    region.destroy();
    queue.roundtrip(&mut State).map_err(|e| e.to_string())?;

    // The effect lasts as long as these objects; keep them for the process lifetime.
    std::mem::forget((effect, manager, compositor, queue, conn));
    Ok(())
}

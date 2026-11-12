//! Window appearance follows the compositor without competing with operation notices.

use gpui_kit::WindowBackgroundAppearance;
use rustphin_platform::blur::{BackgroundBlur as _, BlurError, BlurWatch, WaylandBlur};

use super::*;

#[derive(Default)]
pub(super) struct Appearance {
    watch: Option<BlurWatch>,
    effect_attached: bool,
    translucent: bool,
    unavailable: bool,
    /// The desktop offers blur now.
    available: bool,
    /// Settings, Appearance chose the opaque look (W-106d).
    opaque_chosen: bool,
}

impl Workspace {
    pub(crate) fn follow_blur(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.appearance = Some(Appearance::default());
        let watch = match WaylandBlur.watch(window) {
            Ok(watch) => watch,
            Err(error) => {
                self.blur_changed(Err(error), window, cx);
                return;
            }
        };
        let changes = watch.changes();
        if let Some(appearance) = self.appearance.as_mut() {
            appearance.watch = Some(watch);
        }
        cx.spawn_in(window, async move |this, cx| {
            while let Ok(change) = changes.recv_async().await {
                if this
                    .update_in(cx, |this, window, cx| this.blur_changed(change, window, cx))
                    .is_err()
                {
                    return;
                }
            }
            // A stopped or failed monitor can no longer justify transparent surfaces.
            let _ = this.update_in(cx, |this, window, cx| {
                this.blur_changed(
                    Err(BlurError::Connection("capability monitor closed".into())),
                    window,
                    cx,
                );
            });
        })
        .detach();
    }

    fn blur_changed(
        &mut self,
        change: Result<bool, BlurError>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(appearance) = self.appearance.as_mut() else {
            return;
        };
        let available = change.and_then(|available| {
            if available && !appearance.effect_attached {
                WaylandBlur.enable(window, window)?;
                // The effect belongs to this surface for its lifetime. Attaching
                // it twice is a protocol error, even after a capability loss.
                appearance.effect_attached = true;
            }
            Ok(available)
        });
        let available = match available {
            Ok(available) => available,
            Err(reason) => {
                tracing::info!(%reason, "using the opaque theme");
                false
            }
        };
        appearance.available = available;
        appearance.unavailable = !available;
        self.show_look(window, cx);
    }

    /// Chooses the opaque look, or glass where the desktop blurs
    /// (Settings, Appearance). The blur effect stays attached either way:
    /// attaching it again later would be a protocol error.
    pub(crate) fn choose_opaque(
        &mut self,
        opaque: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(appearance) = self.appearance.as_mut()
            && appearance.opaque_chosen != opaque
        {
            appearance.opaque_chosen = opaque;
            self.show_look(window, cx);
        }
    }

    /// Draws glass when the desktop blurs and the opaque look is not chosen.
    fn show_look(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(appearance) = self.appearance.as_mut() else {
            return;
        };
        let translucent = appearance.available && !appearance.opaque_chosen;
        if appearance.translucent != translucent {
            appearance.translucent = translucent;
            window.set_background_appearance(if translucent {
                WindowBackgroundAppearance::Transparent
            } else {
                WindowBackgroundAppearance::Opaque
            });
            let theme = Theme::obsidian(translucent);
            theme.apply_to_components(cx);
            cx.set_global(theme);
            // Cached child views also need the changed surface tokens.
            window.refresh();
        }
        cx.notify();
    }

    pub(super) fn blur_status(&self, theme: Theme) -> Option<impl IntoElement + use<>> {
        self.appearance
            .as_ref()
            // Not when the opaque look was chosen: nothing is missing then.
            .filter(|state| state.unavailable && !state.opaque_chosen)
            .map(|_| {
                let label = "Background blur unavailable; using the opaque theme.";
                div()
                    .id("blur-status")
                    .role(Role::Status)
                    .aria_label(label)
                    .a11y_synthetic_children(|builder| {
                        builder
                            .parent_node()
                            .set_live(gpui_kit::accesskit::Live::Polite);
                    })
                    .flex()
                    .flex_none()
                    .items_center()
                    .gap(px(space::SM))
                    .min_h(px(size::BLUR_STATUS))
                    .px(px(space::MD))
                    .py(px(space::XS))
                    .border_t_1()
                    .border_color(theme.border)
                    .bg(theme.chrome)
                    .text_size(text::scaled(text::BODY))
                    .text_color(theme.text_muted)
                    .child(
                        gpui_kit::component::Icon::new(gpui_kit::component::IconName::Info)
                            .size(px(size::ICON))
                            .flex_none(),
                    )
                    .child(label)
            })
    }
}

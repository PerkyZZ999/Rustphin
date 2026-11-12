//! The Confirmations page of Settings (W-106e, mockup 36f): which actions
//! ask before they act. Deleting for good always asks (D-011).

use std::rc::Rc;

use gpui_kit::{
    AnyElement, Context, IntoElement, ParentElement as _, Styled as _, Window,
    component::{Disableable as _, checkbox::Checkbox},
    div, px,
};

use crate::{
    preferences::preferences,
    settings::{Confirmations, Programs},
    settings_controls::{self as controls, Pick},
    settings_dialog::{SettingsDialog, group, setting_row},
    theme::{Theme, space, text},
};

/// Changes one of the page's choices, keeping the others.
fn choose(cx: &mut Context<SettingsDialog>, change: impl FnOnce(&mut Confirmations)) {
    preferences(cx).update(cx, |store, cx| {
        store.update(
            |settings| {
                let mut confirmations = settings.confirmations();
                change(&mut confirmations);
                settings.set_confirmations(confirmations);
            },
            cx,
        );
    });
}

impl SettingsDialog {
    /// The Confirmations page's groups.
    pub(crate) fn confirmations(
        &mut self,
        theme: Theme,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let current = preferences(cx).read(cx).settings().confirmations();
        let ask = |id: &'static str,
                   label: &'static str,
                   checked: bool,
                   set: fn(&mut Confirmations, bool),
                   cx: &mut Context<Self>| {
            let pick: Pick<bool> = Rc::new(move |_, on, _, cx| {
                choose(cx, |confirmations| set(confirmations, on));
            });
            controls::checkbox(id, label, checked, pick, cx)
        };
        let ask_before = vec![
            ask(
                "settings-ask-trash",
                "Moving to the trash",
                current.trash,
                |c, on| c.trash = on,
                cx,
            ),
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap(px(space::SM))
                .child(
                    Checkbox::new("settings-ask-delete")
                        .text_size(text::scaled(text::BODY))
                        .label("Deleting for good")
                        .accessibility_label("Deleting for good: always asked")
                        .checked(true)
                        .disabled(true),
                )
                .child(controls::hint("Always asked: it cannot be undone.", theme))
                .into_any_element(),
            ask(
                "settings-ask-empty",
                "Emptying the trash",
                current.empty_trash,
                |c, on| c.empty_trash = on,
                cx,
            ),
            ask(
                "settings-ask-close",
                "Closing a window with several tabs",
                current.close_tabs,
                |c, on| c.close_tabs = on,
                cx,
            ),
        ];
        let pick_programs: Pick<Programs> = Rc::new(|_, programs, _, cx| {
            choose(cx, |confirmations| confirmations.programs = programs);
        });
        let executables = vec![setting_row(
            "Opening one",
            div()
                .flex()
                .flex_col()
                .flex_1()
                .min_w_0()
                .gap(px(space::XS))
                .child(controls::select(
                    "settings-programs",
                    "Opening an executable file",
                    &[
                        (Programs::Ask, "Ask each time"),
                        (Programs::Open, "Open it"),
                        (Programs::Run, "Run it"),
                    ],
                    current.programs,
                    &self.selects,
                    theme,
                    pick_programs,
                    cx,
                ))
                .child(controls::hint(
                    "Open hands it to the application for its type. A shortcut still \
                     asks unless Run it.",
                    theme,
                ))
                .into_any_element(),
        )];
        vec![
            group(
                "Ask before",
                vec![
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(space::SM))
                        .children(ask_before)
                        .into_any_element(),
                ],
                theme,
            ),
            group("Executable files", executables, theme),
        ]
    }
}

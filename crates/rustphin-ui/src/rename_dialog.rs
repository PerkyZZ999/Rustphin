//! The batch-rename dialog: one rule, a live preview, and no change until Rename.

use std::{
    ffi::{OsStr, OsString},
    os::unix::ffi::OsStrExt,
    path::PathBuf,
};

use gpui_kit::{
    App, AppContext as _, Context, Entity, Focusable as _, InteractiveElement as _, IntoElement,
    ParentElement as _, Rgba, Role, SharedString, StatefulInteractiveElement as _, Styled as _,
    Subscription, Window,
    component::{
        Disableable as _, Sizable as _,
        button::{Button, ButtonVariants as _},
        checkbox::Checkbox,
        input::{Input, InputEvent, InputState},
    },
    div,
    prelude::FluentBuilder as _,
    px,
};
use rustphin_core::{CaseChange, Preview, RenameRules, display_name, ops::Request};

use crate::{
    operations::hub,
    theme::{Theme, size, space, text},
    workspace::Workspace,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Replace,
    Pattern,
    Case,
}

/// The open batch-rename dialog and the names it was given.
pub struct RenameDialog {
    directory: PathBuf,
    selected: Vec<OsString>,
    staying: Vec<OsString>,
    mode: Mode,
    case: CaseChange,
    regex: bool,
    find: Entity<InputState>,
    replace: Entity<InputState>,
    pattern: Entity<InputState>,
    start: Entity<InputState>,
    step: Entity<InputState>,
    _inputs: Vec<Subscription>,
}

impl RenameDialog {
    /// Opens the dialog on `selected`, in list order. `staying` is every other
    /// name in the folder.
    pub fn open(
        directory: PathBuf,
        selected: Vec<OsString>,
        staying: Vec<OsString>,
        window: &mut Window,
        cx: &mut Context<Workspace>,
    ) -> Self {
        let find = cx.new(|cx| InputState::new(window, cx).placeholder("Text to find"));
        let replace = cx.new(|cx| InputState::new(window, cx).placeholder("Replacement"));
        let pattern = cx.new(|cx| {
            let mut input = InputState::new(window, cx).placeholder("photo-###");
            input.set_value("@-#", window, cx);
            input
        });
        let start = cx.new(|cx| {
            let mut input = InputState::new(window, cx).placeholder("1");
            input.set_value("1", window, cx);
            input
        });
        let step = cx.new(|cx| {
            let mut input = InputState::new(window, cx).placeholder("1");
            input.set_value("1", window, cx);
            input
        });
        let watch = |input: &Entity<InputState>, cx: &mut Context<Workspace>| {
            cx.subscribe_in(
                input,
                window,
                |this, _, event: &InputEvent, window, cx| match event {
                    InputEvent::PressEnter { .. } => this.apply_rename(window, cx),
                    InputEvent::Change => cx.notify(),
                    InputEvent::Focus | InputEvent::Blur => {}
                },
            )
        };
        let _inputs = vec![
            watch(&find, cx),
            watch(&replace, cx),
            watch(&pattern, cx),
            watch(&start, cx),
            watch(&step, cx),
        ];
        find.read(cx).focus_handle(cx).focus(window, cx);
        Self {
            directory,
            selected,
            staying,
            mode: Mode::Replace,
            case: CaseChange::Lower,
            regex: false,
            find,
            replace,
            pattern,
            start,
            step,
            _inputs,
        }
    }

    fn rules(&self, cx: &App) -> Result<RenameRules, String> {
        match self.mode {
            Mode::Replace => Ok(RenameRules::replace(
                self.find.read(cx).value().to_string(),
                self.replace.read(cx).value().to_string(),
                self.regex,
            )),
            Mode::Pattern => {
                let start = whole_number(&self.start.read(cx).value());
                let step = whole_number(&self.step.read(cx).value());
                match (start, step) {
                    (Some(start), Some(step)) => Ok(RenameRules::pattern(
                        self.pattern.read(cx).value().to_string(),
                        start,
                        step,
                    )),
                    _ => {
                        Err("Start and step have to be whole numbers, such as 1 or -2.".to_owned())
                    }
                }
            }
            Mode::Case => Ok(RenameRules::case(self.case)),
        }
    }

    /// The request to run, when the preview has a real change and no problem.
    pub fn request(&self, cx: &App) -> Option<Request> {
        let rules = self.rules(cx).ok()?;
        let preview = rules.preview(&self.selected, &self.staying);
        let changed = preview.rows.iter().any(|row| row.from != row.to);
        if !preview.can_apply() || !changed {
            return None;
        }
        Some(Request::RenameBatch {
            directory: self.directory.clone(),
            pairs: preview
                .rows
                .iter()
                .map(|row| (row.from.clone(), row.to.clone()))
                .collect(),
        })
    }

    pub fn render(
        &self,
        theme: Theme,
        window: &Window,
        cx: &mut Context<Workspace>,
    ) -> impl IntoElement {
        let (plan_problem, preview) = match self.rules(cx) {
            Ok(rules) => {
                let preview = rules.preview(&self.selected, &self.staying);
                (preview.plan_problem.clone(), Some(preview))
            }
            Err(problem) => (Some(problem), None),
        };
        let changed = preview
            .as_ref()
            .is_some_and(|preview| preview.rows.iter().any(|row| row.from != row.to));
        let ready = preview.as_ref().is_some_and(Preview::can_apply) && changed;
        let count = self.selected.len();

        div()
            .absolute()
            .inset_0()
            .occlude()
            .flex()
            .items_center()
            .justify_center()
            .p(px(space::LG))
            .bg(theme.scrim)
            .child(
                div()
                    .id("rename-dialog")
                    .role(Role::Dialog)
                    .aria_label(format!("Rename {count} items"))
                    .aria_description(format!(
                        "{} mode. {} Review the preview before Rename; Escape cancels.",
                        match self.mode {
                            Mode::Replace => "Replace",
                            Mode::Pattern => "Pattern",
                            Mode::Case => "Case",
                        },
                        plan_problem
                            .as_deref()
                            .unwrap_or("Only the preview is shown until Rename is activated.")
                    ))
                    .a11y_synthetic_children(|builder| builder.parent_node().set_modal())
                    .key_context("Dialog")
                    .on_action(cx.listener(
                        |this, _: &crate::workspace::DialogAccept, window, cx| {
                            this.apply_rename(window, cx);
                        },
                    ))
                    .on_action(cx.listener(
                        |this, _: &crate::workspace::DialogCancel, window, cx| {
                            this.cancel_rename(window, cx);
                        },
                    ))
                    .flex()
                    .flex_col()
                    .gap(px(space::MD))
                    .w_full()
                    .max_w(px(size::RENAME_DIALOG))
                    .max_h_full()
                    .p(px(space::LG))
                    .rounded(px(size::RADIUS))
                    .bg(theme.raised)
                    .border_1()
                    .border_color(theme.border_accent)
                    .shadow(vec![theme.floating_shadow()])
                    .text_color(theme.text)
                    .font_family(text::FAMILY_BODY)
                    .text_size(px(text::BODY))
                    .line_height(px(text::LINE))
                    .child(
                        div()
                            .font_family(text::FAMILY_HEADING)
                            .text_size(px(text::HEADING))
                            .child(format!("Rename {count} items")),
                    )
                    .child(self.modes(cx))
                    .child(self.fields(theme, window, cx))
                    .when_some(plan_problem, |dialog, problem| {
                        dialog.child(note(theme.danger, problem))
                    })
                    .when(
                        !changed && preview.as_ref().is_some_and(Preview::can_apply),
                        |dialog| {
                            dialog.child(note(
                                theme.text_muted,
                                "Nothing would change. Edit the rule to give the files new names."
                                    .to_owned(),
                            ))
                        },
                    )
                    .child(self.preview(preview.as_ref(), theme))
                    .child(
                        div()
                            .flex()
                            .justify_end()
                            .gap(px(space::SM))
                            .child(
                                Button::new("rename-cancel")
                                    .outline()
                                    .label("Cancel (Esc)")
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.cancel_rename(window, cx)
                                    })),
                            )
                            .child(
                                Button::new("rename-apply")
                                    .primary()
                                    .disabled(!ready)
                                    .label("Rename (Enter)")
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.apply_rename(window, cx)
                                    })),
                            ),
                    ),
            )
    }

    fn modes(&self, cx: &mut Context<Workspace>) -> impl IntoElement {
        div()
            .flex()
            .gap(px(space::SM))
            .child(self.mode_button("mode-replace", "Replace", Mode::Replace, cx))
            .child(self.mode_button("mode-pattern", "Pattern", Mode::Pattern, cx))
            .child(self.mode_button("mode-case", "Case", Mode::Case, cx))
    }

    fn mode_button(
        &self,
        id: &'static str,
        label: &'static str,
        mode: Mode,
        cx: &mut Context<Workspace>,
    ) -> impl IntoElement {
        let button = Button::new(id).label(label);
        let button = if self.mode == mode {
            button.primary()
        } else {
            button.outline()
        };
        button.on_click(cx.listener(move |this, _, _, cx| {
            if let Some(dialog) = &mut this.rename {
                dialog.mode = mode;
            }
            cx.notify();
        }))
    }

    fn fields(
        &self,
        theme: Theme,
        window: &Window,
        cx: &mut Context<Workspace>,
    ) -> impl IntoElement {
        match self.mode {
            Mode::Replace => div()
                .flex()
                .flex_col()
                .gap(px(space::SM))
                .child(field(theme, "Find", &self.find, window, cx))
                .child(field(theme, "Replace with", &self.replace, window, cx))
                .child(
                    Checkbox::new("rename-regex")
                        .label("Regular expression")
                        .checked(self.regex)
                        .on_click(cx.listener(|this, checked: &bool, _, cx| {
                            if let Some(dialog) = &mut this.rename {
                                dialog.regex = *checked;
                            }
                            cx.notify();
                        })),
                ),
            Mode::Pattern => div()
                .flex()
                .flex_col()
                .gap(px(space::SM))
                .child(field(theme, "Pattern", &self.pattern, window, cx))
                .child(note(
                    theme.text_muted,
                    "# is the counter. The number of # signs is how many digits it has. @ is the file's current name.".to_owned(),
                ))
                .child(
                    div()
                        .flex()
                        .gap(px(space::SM))
                        .child(field(theme, "Start", &self.start, window, cx))
                        .child(field(theme, "Step", &self.step, window, cx)),
                ),
            Mode::Case => div().flex().gap(px(space::SM)).child(self.case_button(
                "case-lower",
                "Lower case",
                CaseChange::Lower,
                cx,
            )).child(self.case_button(
                "case-upper",
                "Upper case",
                CaseChange::Upper,
                cx,
            )).child(self.case_button(
                "case-words",
                "Capitalise words",
                CaseChange::Words,
                cx,
            )),
        }
    }

    fn case_button(
        &self,
        id: &'static str,
        label: &'static str,
        case: CaseChange,
        cx: &mut Context<Workspace>,
    ) -> impl IntoElement {
        let button = Button::new(id).label(label);
        let button = if self.case == case {
            button.primary()
        } else {
            button.outline()
        };
        button.on_click(cx.listener(move |this, _, _, cx| {
            if let Some(dialog) = &mut this.rename {
                dialog.case = case;
                dialog.mode = Mode::Case;
            }
            cx.notify();
        }))
    }

    fn preview(&self, preview: Option<&Preview>, theme: Theme) -> impl IntoElement {
        let rows = preview.map(|preview| {
            preview
                .rows
                .iter()
                .enumerate()
                .map(|(index, row)| {
                    let problems: Vec<_> = row
                        .problems
                        .iter()
                        .map(|problem| problem.message())
                        .collect();
                    div()
                        .id(("rename-preview-row", index))
                        .role(Role::ListItem)
                        .aria_label(format!(
                            "{} becomes {}",
                            display_name(row.from.as_bytes()),
                            display_name(row.to.as_bytes()),
                        ))
                        .aria_description(problems.join(" "))
                        .aria_position_in_set(index)
                        .flex()
                        .flex_col()
                        .py(px(space::XS))
                        .child(
                            div()
                                .flex()
                                .gap(px(space::SM))
                                .child(name_cell(&row.from, theme.text_muted))
                                .child(div().text_color(theme.text_muted).child("→"))
                                .child(name_cell(
                                    &row.to,
                                    if problems.is_empty() {
                                        theme.text
                                    } else {
                                        theme.danger
                                    },
                                )),
                        )
                        .children(
                            problems
                                .into_iter()
                                .map(|message| note(theme.danger, message)),
                        )
                })
                .collect::<Vec<_>>()
        });
        div()
            .flex()
            .flex_col()
            .gap(px(space::XS))
            .id("rename-preview")
            .role(Role::List)
            .aria_label("Rename preview")
            .aria_size_of_set(self.selected.len())
            .max_h(px(size::RENAME_PREVIEW))
            .overflow_y_scroll()
            .child(
                div()
                    .flex()
                    .gap(px(space::SM))
                    .text_color(theme.text_muted)
                    .text_size(px(text::SMALL))
                    .child(div().flex_1().child("Current name"))
                    .child(div().child(" "))
                    .child(div().flex_1().child("New name")),
            )
            .children(rows.into_iter().flatten())
    }
}

fn field(
    theme: Theme,
    label: &'static str,
    state: &Entity<InputState>,
    window: &Window,
    cx: &App,
) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .flex_1()
        .gap(px(space::XS))
        .child(
            div()
                .text_size(px(text::SMALL))
                .text_color(theme.text_muted)
                .child(label),
        )
        .child(crate::input::named(
            Input::new(state).small(),
            state,
            label,
            window,
            cx,
        ))
}

fn note(color: Rgba, message: String) -> impl IntoElement {
    div()
        .id(SharedString::from(message.clone()))
        .role(Role::Label)
        .aria_value(message.clone())
        .text_size(px(text::SMALL))
        .text_color(color)
        .child(message)
}

fn name_cell(name: &OsStr, color: Rgba) -> impl IntoElement {
    let shown: SharedString = display_name(name.as_bytes()).into_owned().into();
    div()
        .flex_1()
        .min_w_0()
        .overflow_hidden()
        .text_color(color)
        .child(shown)
}

fn whole_number(value: &str) -> Option<i64> {
    value.trim().parse().ok()
}

impl Workspace {
    pub(crate) fn apply_rename(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(request) = self.rename.as_ref().and_then(|dialog| dialog.request(cx)) else {
            return;
        };
        self.rename = None;
        hub(cx).update(cx, |hub, _| hub.start(request));
        self.focus_active(window, cx);
        cx.notify();
    }

    pub(crate) fn cancel_rename(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.rename.is_none() {
            return;
        }
        self.rename = None;
        self.focus_active(window, cx);
        cx.notify();
    }
}

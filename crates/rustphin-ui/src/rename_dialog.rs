//! The batch-rename dialog: one rule, a live preview, and no change until Rename.

use std::{
    ffi::{OsStr, OsString},
    os::unix::ffi::OsStrExt,
    path::PathBuf,
};

use gpui_kit::{
    App, AppContext as _, Context, Entity, FocusHandle, Focusable as _, InteractiveElement as _,
    IntoElement, ParentElement as _, Rgba, Role, ScrollHandle, SharedString,
    StatefulInteractiveElement as _, Styled as _, Subscription, Window,
    component::{
        Disableable as _,
        button::ButtonVariants as _,
        checkbox::Checkbox,
        input::{Input, InputEvent, InputState},
        scroll::{Scrollbar, ScrollbarMode},
    },
    div,
    prelude::FluentBuilder as _,
    px,
};
use rustphin_core::{CaseChange, Preview, RenameRules, display_name, ops::Request};

use crate::{
    dialog,
    layout::scroll_offset,
    operations::hub,
    theme::{Theme, size, space, text},
    workspace::Workspace,
};

// Rule rows follow the modes row in the scrollable body.
const FIRST_FIELD_ROW: usize = 1;
const SECOND_FIELD_ROW: usize = 2;
const COUNTER_FIELDS_ROW: usize = 3;

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
    scroll: ScrollHandle,
    focus: FocusHandle,
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
        let watch = |input: &Entity<InputState>, row: usize, cx: &mut Context<Workspace>| {
            cx.subscribe_in(
                input,
                window,
                move |this, _, event: &InputEvent, window, cx| match event {
                    InputEvent::PressEnter { .. } => this.apply_rename(window, cx),
                    InputEvent::Change => cx.notify(),
                    InputEvent::Focus => {
                        if let Some(dialog) = &this.rename {
                            dialog.scroll.scroll_to_item(row);
                            cx.notify();
                        }
                    }
                    InputEvent::Blur => {}
                },
            )
        };
        let _inputs = vec![
            watch(&find, FIRST_FIELD_ROW, cx),
            watch(&replace, SECOND_FIELD_ROW, cx),
            watch(&pattern, FIRST_FIELD_ROW, cx),
            watch(&start, COUNTER_FIELDS_ROW, cx),
            watch(&step, COUNTER_FIELDS_ROW, cx),
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
            scroll: ScrollHandle::new(),
            focus: cx.focus_handle(),
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

        let panel = dialog::panel(
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
                .track_focus(&self.focus)
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
                .on_key_down(cx.listener(|this, event: &gpui_kit::KeyDownEvent, _, cx| {
                    let modifiers = event.keystroke.modifiers;
                    if !modifiers.control || modifiers.alt || modifiers.platform || modifiers.shift
                    {
                        return;
                    }
                    if !matches!(event.keystroke.key.as_str(), "pageup" | "pagedown") {
                        return;
                    }
                    let Some(dialog) = &this.rename else {
                        return;
                    };
                    let mut offset = dialog.scroll.offset();
                    if let Some(y) = scroll_offset(
                        &event.keystroke.key,
                        f32::from(offset.y),
                        f32::from(dialog.scroll.max_offset().y),
                        f32::from(dialog.scroll.bounds().size.height),
                    ) {
                        offset.y = px(y);
                        dialog.scroll.set_offset(offset);
                        cx.stop_propagation();
                        cx.notify();
                    }
                })),
            size::RENAME_DIALOG,
            theme,
            window,
        )
        .child(dialog::title_row(
            gpui_kit::assets::IconName::Pencil,
            format!("Rename {count} items"),
            "Cancel (Esc)",
            cx.listener(|this, _, window, cx| this.cancel_rename(window, cx)),
            theme,
        ))
        .child(
            dialog::body().child(
                div()
                    .relative()
                    .flex()
                    .flex_col()
                    .min_h_0()
                    .flex_shrink_1()
                    .child(
                        div()
                            .id("rename-content")
                            .flex()
                            .flex_col()
                            .gap(px(space::LG))
                            .min_h_0()
                            .overflow_y_scroll()
                            .pr(px(space::LG))
                            .track_scroll(&self.scroll)
                            .child(self.modes(theme, cx))
                            .children(self.fields(theme, window, cx))
                            .when_some(plan_problem, |dialog, problem| {
                                dialog.child(problem_note(problem, theme))
                            })
                            .when(
                                !changed && preview.as_ref().is_some_and(Preview::can_apply),
                                |dialog| {
                                    dialog.child(neutral_note(
                                        "Nothing would change. Edit the rule to give the files new names.",
                                        theme,
                                    ))
                                },
                            )
                            .child(self.preview(preview.as_ref(), theme)),
                    )
                    .child(
                        Scrollbar::vertical(&self.scroll)
                            .mode(ScrollbarMode::Always)
                            .styles(|styles| {
                                styles
                                    .track(|style| style.bg(gpui_kit::transparent_black()))
                                    .thumb(|style| {
                                        style.bg(theme.text_muted).radius(px(size::RADIUS))
                                    })
                                    .thumb_hover(|style| style.bg(theme.accent))
                                    .thumb_active(|style| style.bg(theme.accent))
                            }),
                    ),
            ),
        )
        .child(
            dialog::actions(theme)
                .child(
                    dialog::button("rename-cancel", "Cancel (Esc)")
                        .outline()
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.cancel_rename(window, cx)
                        })),
                )
                .child(
                    dialog::button("rename-apply", "Rename (Enter)")
                        .primary()
                        .disabled(!ready)
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.apply_rename(window, cx)
                        })),
                ),
        );
        dialog::scrim(panel, theme)
    }

    /// The three modes as one segmented row, the chosen one filled.
    fn modes(&self, theme: Theme, cx: &mut Context<Workspace>) -> impl IntoElement {
        div()
            .flex_none()
            .flex()
            .w_auto()
            .self_start()
            .rounded(px(size::RADIUS))
            .border_1()
            .border_color(theme.border_strong)
            .overflow_hidden()
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
        let button = dialog::button(id, label);
        let button = if self.mode == mode {
            button.primary()
        } else {
            button.ghost()
        };
        button.on_click(cx.listener(move |this, _, window, cx| {
            if let Some(dialog) = &mut this.rename {
                dialog.mode = mode;
                dialog.scroll.set_offset(gpui_kit::point(px(0.), px(0.)));
                let focus = match mode {
                    Mode::Replace => dialog.find.read(cx).focus_handle(cx),
                    Mode::Pattern => dialog.pattern.read(cx).focus_handle(cx),
                    Mode::Case => dialog.focus.clone(),
                };
                focus.focus(window, cx);
            }
            cx.notify();
        }))
    }

    fn fields(
        &self,
        theme: Theme,
        window: &Window,
        cx: &mut Context<Workspace>,
    ) -> Vec<gpui_kit::AnyElement> {
        match self.mode {
            Mode::Replace => vec![
                field(theme, "Find", &self.find, window, cx).into_any_element(),
                field(theme, "Replace with", &self.replace, window, cx).into_any_element(),
                div().flex_none().child(Checkbox::new("rename-regex")
                    .label("Regular expression")
                    .checked(self.regex)
                    .on_click(cx.listener(|this, checked: &bool, _, cx| {
                        if let Some(dialog) = &mut this.rename {
                            dialog.regex = *checked;
                        }
                        cx.notify();
                    })))
                    .into_any_element(),
            ],
            Mode::Pattern => vec![
                field(theme, "Pattern", &self.pattern, window, cx).into_any_element(),
                note(
                    theme.text_muted,
                    "# is the counter. The number of # signs is how many digits it has. @ is the file's current name.".to_owned(),
                ).into_any_element(),
                div()
                    .flex_none()
                    .flex()
                    .gap(px(space::SM))
                    .child(div().flex_1().min_w_0().child(field(theme, "Start", &self.start, window, cx)))
                    .child(div().flex_1().min_w_0().child(field(theme, "Step", &self.step, window, cx)))
                    .into_any_element(),
            ],
            Mode::Case => vec![div()
                .flex_none()
                .flex()
                .flex_wrap()
                .gap(px(space::SM))
                .child(self.case_button("case-lower", "Lower case", CaseChange::Lower, cx))
                .child(self.case_button("case-upper", "Upper case", CaseChange::Upper, cx))
                .child(self.case_button("case-words", "Capitalise words", CaseChange::Words, cx))
                .into_any_element()],
        }
    }

    fn case_button(
        &self,
        id: &'static str,
        label: &'static str,
        case: CaseChange,
        cx: &mut Context<Workspace>,
    ) -> impl IntoElement {
        let button = dialog::button(id, label);
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

    /// Old and new names side by side, an arrow between, unusable new
    /// names in the error colour with the reason under them.
    fn preview(&self, preview: Option<&Preview>, theme: Theme) -> impl IntoElement {
        let line = || {
            div()
                .flex()
                .items_center()
                .gap(px(space::SM))
                .min_h(px(size::MENU_ROW))
                .px(px(space::MD))
        };
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
                    let bad = !problems.is_empty();
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
                        .border_t_1()
                        .border_color(theme.border)
                        .font_family(text::FAMILY_DATA)
                        .text_size(px(text::DATA))
                        .child(
                            line()
                                .child(name_cell(
                                    &row.from,
                                    if bad { theme.danger } else { theme.text },
                                ))
                                .child(
                                    div()
                                        .w(px(size::ICON + space::SM))
                                        .flex_none()
                                        .text_center()
                                        .text_color(theme.text_muted)
                                        .child("\u{2192}"),
                                )
                                .child(name_cell(
                                    &row.to,
                                    if bad { theme.danger } else { theme.text },
                                )),
                        )
                        .children(problems.into_iter().map(|message| {
                            div()
                                .px(px(space::MD))
                                .pb(px(space::XS))
                                .font_family(text::FAMILY_BODY)
                                .text_size(px(text::BODY))
                                .text_color(theme.danger)
                                .child(message)
                        }))
                })
                .collect::<Vec<_>>()
        });
        div()
            .flex()
            .flex_col()
            .flex_none()
            .id("rename-preview")
            .role(Role::List)
            .aria_label("Rename preview")
            .aria_size_of_set(self.selected.len())
            .rounded(px(size::RADIUS))
            .border_1()
            .border_color(theme.border)
            .child(
                line()
                    .text_color(theme.text_muted)
                    .child(div().flex_1().child("Current name"))
                    .child(div().w(px(size::ICON + space::SM)).flex_none())
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
        .flex_none()
        .min_w_0()
        .gap(px(space::XS))
        .child(div().text_color(theme.text_muted).child(label))
        .child(crate::input::named(
            Input::new(state).h(px(size::CONTROL)),
            state,
            label,
            window,
            cx,
        ))
}

fn note(color: Rgba, message: String) -> impl IntoElement {
    div()
        .flex_none()
        .id(SharedString::from(message.clone()))
        .role(Role::Label)
        .aria_value(message.clone())
        .text_color(color)
        .child(message)
}

/// A problem with the rule: error colours, an alert icon and an outline.
fn problem_note(message: String, theme: Theme) -> impl IntoElement {
    boxed_note(gpui_kit::assets::IconName::TriangleAlert, message)
        .bg(theme.danger_surface)
        .border_color(theme.danger_border)
        .text_color(theme.danger)
}

/// Something worth knowing that is not wrong, such as nothing changing.
fn neutral_note(message: &'static str, theme: Theme) -> impl IntoElement {
    boxed_note(gpui_kit::assets::IconName::Info, message.to_owned())
        .border_color(theme.border)
        .text_color(theme.text_muted)
}

fn boxed_note(
    icon: gpui_kit::assets::IconName,
    message: String,
) -> gpui_kit::Stateful<gpui_kit::Div> {
    div()
        .id(SharedString::from(message.clone()))
        .role(Role::Label)
        .aria_value(message.clone())
        .flex()
        .flex_none()
        .items_start()
        .gap(px(space::SM))
        .px(px(space::MD))
        .py(px(space::SM))
        .rounded(px(size::RADIUS))
        .border_1()
        .child(
            gpui_kit::component::Icon::new(icon)
                .size(px(size::ICON))
                .flex_none()
                .mt(px(space::XS / 2.)),
        )
        .child(div().flex_1().min_w_0().child(message))
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

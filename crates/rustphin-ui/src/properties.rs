//! The properties dialog: facts about one entry, and its permissions.

use std::{
    path::PathBuf,
    sync::{Arc, atomic::AtomicBool},
};

use gpui_kit::{
    AnyElement, FocusHandle, InteractiveElement as _, IntoElement, ParentElement as _, Role,
    ScrollHandle, StatefulInteractiveElement as _, Styled as _, Window,
    component::{
        Disableable as _, Sizable as _,
        button::{Button, ButtonVariants as _},
        checkbox::Checkbox,
        scroll::{Scrollbar, ScrollbarMode},
    },
    div,
    prelude::FluentBuilder as _,
    px,
};
use jiff::tz::TimeZone;
use rustphin_core::{
    combine_mode, display_name, execute_bit, has_bit, has_special, octal_mode, read_bit,
    symbolic_mode, with_bit, write_bit,
};
use rustphin_platform::properties::{Facts, Kind};

use crate::{
    format,
    layout::scroll_offset,
    theme::{Theme, size, space, text},
    workspace::{DialogAccept, DialogCancel, Workspace},
};

const PERMISSION_CLASSES: [&str; 3] = ["Owner", "Group", "Others"];

/// How far a folder's size has got.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SizeLine {
    /// Not known yet.
    Reading,
    /// A file's size.
    Known(u64),
    /// A folder is being measured.
    Calculating,
    /// A folder's measured size.
    Folder {
        /// Bytes that could be added up.
        bytes: u64,
        /// Entries that could not be read.
        unreadable: u64,
    },
    /// The measurement failed.
    Failed(String),
}

/// The open properties dialog.
pub struct PropertiesDialog {
    /// The entry.
    pub path: PathBuf,
    /// The name in the title.
    pub title: String,
    /// A readable type, such as "Plain text document".
    pub type_label: String,
    /// The facts, once read.
    pub facts: Option<Facts>,
    /// Why the facts could not be read.
    pub error: Option<String>,
    /// The size line.
    pub size: SizeLine,
    /// Permission bits, including edits not yet written.
    pub mode: u32,
    /// The bits on disk when the dialog opened.
    pub original: u32,
    /// Whether the checkboxes change this entry.
    pub editable: bool,
    /// Matches the request that filled this dialog, so a late result is ignored.
    pub token: u64,
    /// Set when the dialog closes, so a folder walk stops.
    pub cancel: Arc<AtomicBool>,
    scroll: ScrollHandle,
}

impl PropertiesDialog {
    /// A dialog that has not read anything yet.
    pub fn loading(path: PathBuf, type_label: String, token: u64) -> Self {
        let title = path
            .file_name()
            .map(|name| display_name(name.as_encoded_bytes()).into_owned())
            .unwrap_or_else(|| "Properties".to_owned());
        Self {
            path,
            title,
            type_label,
            facts: None,
            error: None,
            size: SizeLine::Reading,
            mode: 0,
            original: 0,
            editable: false,
            token,
            cancel: Arc::new(AtomicBool::new(false)),
            scroll: ScrollHandle::new(),
        }
    }

    /// Stores a successful read. A directory then waits for its size.
    pub fn set_facts(&mut self, facts: Facts) {
        self.editable = facts.kind != Kind::Symlink;
        self.mode = facts.mode;
        self.original = facts.mode;
        self.size = match (facts.kind, facts.size) {
            (Kind::Directory, _) => SizeLine::Calculating,
            (_, Some(bytes)) => SizeLine::Known(bytes),
            (_, None) => SizeLine::Reading,
        };
        self.facts = Some(facts);
    }

    /// The dialog, over a dimmed window.
    pub fn render(
        &self,
        focus: &FocusHandle,
        zone: &TimeZone,
        theme: Theme,
        window: &Window,
        cx: &mut gpui_kit::Context<Workspace>,
    ) -> AnyElement {
        let dirty = self.editable && self.mode != self.original;
        let body = self.body(zone, theme, cx);
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
                    .id("properties-dialog")
                    .role(Role::Dialog)
                    .aria_label(format!("Properties: {}", self.title))
                    .aria_description("File properties and permissions. Apply saves permission changes; Close or Escape discards unapplied changes.")
                    .a11y_synthetic_children(|builder| builder.parent_node().set_modal())
                    .key_context("Dialog")
                    .track_focus(focus)
                    .on_action(cx.listener(|this, _: &DialogAccept, _, cx| {
                        this.apply_properties(cx);
                    }))
                    .on_action(cx.listener(|this, _: &DialogCancel, _, cx| {
                        this.close_properties(cx);
                    }))
                    .on_key_down(cx.listener(|this, event: &gpui_kit::KeyDownEvent, _, cx| {
                        let modifiers = event.keystroke.modifiers;
                        if modifiers.control
                            || modifiers.alt
                            || modifiers.platform
                            || modifiers.shift
                        {
                            return;
                        }
                        let Some(dialog) = &this.properties else {
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
                    }))
                    .flex()
                    .flex_col()
                    .gap(px(space::MD))
                    .w_full()
                    .max_w(px(size::DIALOG))
                    .max_h(px(f32::from(window.viewport_size().height) - space::LG * 2.))
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
                            .flex_none()
                            .font_family(text::FAMILY_HEADING)
                            .text_size(px(text::HEADING))
                            .truncate()
                            .child(self.title.clone()),
                    )
                    .child(
                        div()
                            .relative()
                            .flex()
                            .flex_col()
                            .min_h_0()
                            .flex_shrink_1()
                            .child(
                                div()
                                    .id("properties-content")
                                    .min_h_0()
                                    .overflow_y_scroll()
                                    .track_scroll(&self.scroll)
                                    .child(body),
                            )
                            .child(
                                Scrollbar::vertical(&self.scroll)
                                    .mode(ScrollbarMode::Always)
                                    .styles(|styles| {
                                        styles
                                            .track(|style| style.bg(gpui_kit::transparent_black()))
                                            .thumb(|style| style.bg(theme.text_muted).radius(px(size::RADIUS)))
                                            .thumb_hover(|style| style.bg(theme.accent))
                                            .thumb_active(|style| style.bg(theme.accent))
                                    }),
                            ),
                    )
                    .child(self.buttons(dirty, theme, cx)),
            )
            .into_any_element()
    }

    fn body(
        &self,
        zone: &TimeZone,
        theme: Theme,
        cx: &mut gpui_kit::Context<Workspace>,
    ) -> gpui_kit::AnyElement {
        if let Some(error) = &self.error {
            return div()
                .id("properties-error")
                .role(Role::Status)
                .aria_label(format!("{error} Close and try again."))
                .child(format!("{error} Close and try again."))
                .into_any_element();
        }
        let Some(facts) = &self.facts else {
            return div()
                .id("properties-loading")
                .role(Role::Status)
                .aria_label("Reading properties…")
                .child("Reading properties…")
                .into_any_element();
        };
        let type_label = if facts.kind == Kind::Symlink {
            "Symbolic link".to_owned()
        } else {
            self.type_label.clone()
        };
        div()
            .flex()
            .flex_col()
            .gap(px(space::SM))
            .child(fact("Type", type_label, theme))
            .child(fact("Size", self.size_text(), theme))
            .child(fact("Modified", time_text(facts.modified, zone), theme))
            .child(fact("Accessed", time_text(facts.accessed, zone), theme))
            .when_some(facts.created, |column, created| {
                column.child(fact("Created", time_text(Some(created), zone), theme))
            })
            .when_some(facts.target.clone(), |column, target| {
                column.child(fact(
                    "Points to",
                    display_name(target.as_os_str().as_encoded_bytes()).into_owned(),
                    theme,
                ))
            })
            .child(self.permissions(theme, cx))
            .into_any_element()
    }

    fn size_text(&self) -> String {
        match &self.size {
            SizeLine::Reading => "Reading…".to_owned(),
            SizeLine::Known(bytes) => format::size(*bytes),
            SizeLine::Calculating => "Calculating size…".to_owned(),
            SizeLine::Folder { bytes, unreadable } => {
                let size = format::size(*bytes);
                if *unreadable == 0 {
                    size
                } else {
                    format!(
                        "{size}. {} {} could not be read.",
                        format::count(*unreadable as usize),
                        format::plural(*unreadable as usize, "item", "items")
                    )
                }
            }
            SizeLine::Failed(message) => message.clone(),
        }
    }

    fn permissions(&self, theme: Theme, cx: &mut gpui_kit::Context<Workspace>) -> impl IntoElement {
        let mode = combine_mode(self.mode, self.original);
        let editable = self.editable;
        div()
            .id("permissions")
            .role(Role::Group)
            .aria_label("Permissions")
            .aria_description(format!("{}  {}", symbolic_mode(mode), octal_mode(mode)))
            .flex()
            .flex_col()
            .gap(px(space::SM))
            .pt(px(space::SM))
            .child("Permissions")
            .child(
                div()
                    .font_family(text::FAMILY_DATA)
                    .text_size(px(text::DATA))
                    .text_color(theme.text)
                    .child(format!("{}  {}", symbolic_mode(mode), octal_mode(mode))),
            )
            .when(has_special(self.original), |column| {
                column.child(
                    div()
                        .id("special-permissions")
                        .role(Role::Label)
                        .aria_value("Setuid, setgid or the sticky bit is set, and is kept.")
                        .text_color(theme.text_muted)
                        .text_size(px(text::SMALL))
                        .child("Setuid, setgid or the sticky bit is set, and is kept."),
                )
            })
            .when(!editable && self.facts.is_some(), |column| {
                column.child(
                    div()
                        .id("shortcut-permissions")
                        .role(Role::Label)
                        .aria_value("Permissions of a shortcut are not changed here.")
                        .text_color(theme.text_muted)
                        .text_size(px(text::SMALL))
                        .child("Permissions of a shortcut are not changed here."),
                )
            })
            .child(self.class_row(0, editable, cx))
            .child(self.class_row(1, editable, cx))
            .child(self.class_row(2, editable, cx))
    }

    fn class_row(
        &self,
        class: u8,
        editable: bool,
        cx: &mut gpui_kit::Context<Workspace>,
    ) -> impl IntoElement {
        let name = PERMISSION_CLASSES[class as usize];
        div()
            .id(("permission-class", class as usize))
            .role(Role::Group)
            .aria_label(name)
            .flex()
            .items_center()
            .gap(px(space::SM))
            .child(div().w(px(size::COLUMN_SIZE)).flex_none().child(name))
            .child(self.bit(class, "Read", read_bit(class), editable, cx))
            .child(self.bit(class, "Write", write_bit(class), editable, cx))
            .child(self.bit(class, "Execute", execute_bit(class), editable, cx))
    }

    fn bit(
        &self,
        class: u8,
        label: &str,
        bit: u32,
        editable: bool,
        cx: &mut gpui_kit::Context<Workspace>,
    ) -> impl IntoElement {
        let checked = has_bit(self.mode, bit);
        Checkbox::new(format!("perm-{class}-{label}"))
            .label(label)
            .accessibility_label(format!("{} {label}", PERMISSION_CLASSES[class as usize]))
            .checked(checked)
            .disabled(!editable)
            .on_click(cx.listener(move |this, checked: &bool, _, cx| {
                if let Some(dialog) = &mut this.properties
                    && dialog.editable
                {
                    dialog.mode = with_bit(dialog.mode, bit, *checked);
                }
                cx.notify();
            }))
    }

    fn buttons(
        &self,
        dirty: bool,
        _theme: Theme,
        cx: &mut gpui_kit::Context<Workspace>,
    ) -> impl IntoElement {
        div()
            .flex_none()
            .flex()
            .justify_end()
            .gap(px(space::SM))
            .pt(px(space::SM))
            .child(
                Button::new("properties-close")
                    .small()
                    .primary()
                    .label("Close (Esc)")
                    .on_click(cx.listener(|this, _, _, cx| this.close_properties(cx))),
            )
            .when(self.error.is_none(), |row| {
                row.child(
                    Button::new("properties-apply")
                        .small()
                        .outline()
                        .disabled(!dirty)
                        .label("Apply (Enter)")
                        .on_click(cx.listener(|this, _, _, cx| this.apply_properties(cx))),
                )
            })
    }
}

fn fact(label: &str, value: String, theme: Theme) -> impl IntoElement {
    div()
        .id(gpui_kit::SharedString::from(format!("property-{label}")))
        .role(Role::Label)
        .aria_value(format!("{label}: {value}"))
        .flex()
        .items_baseline()
        .gap(px(space::MD))
        .child(
            div()
                .w(px(size::COLUMN_SIZE))
                .flex_none()
                .text_color(theme.text_muted)
                .child(label.to_owned()),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .font_family(text::FAMILY_DATA)
                .text_size(px(text::DATA))
                .text_color(theme.text)
                .child(value),
        )
}

fn time_text(seconds: Option<i64>, zone: &TimeZone) -> String {
    seconds
        .map(|seconds| format::timestamp(seconds, zone))
        .filter(|text| !text.is_empty())
        .unwrap_or_else(|| "Unknown".to_owned())
}

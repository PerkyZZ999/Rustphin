//! The properties dialog: facts about one entry, and its permissions.

use crate::controls::Outlined as _;
use std::{
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
};

use gpui_kit::{
    AnyElement, FocusHandle, InteractiveElement as _, IntoElement, ParentElement as _, Role,
    ScrollHandle, StatefulInteractiveElement as _, Styled as _, Window,
    component::{
        Disableable as _,
        button::ButtonVariants as _,
        checkbox::Checkbox,
        scroll::{Scrollbar, ScrollbarMode},
    },
    div, img,
    prelude::FluentBuilder as _,
    px, relative,
};
use jiff::tz::TimeZone;
use rustphin_core::{
    combine_mode, display_name, execute_bit, has_bit, has_special, octal_mode, read_bit,
    symbolic_mode, with_bit, write_bit,
};
use rustphin_platform::properties::{Facts, Kind};

use crate::{
    dialog, format,
    layout::scroll_offset,
    theme::{Theme, size, space, text},
    workspace::{DialogAccept, DialogCancel, Workspace},
};

const PERMISSION_CLASSES: [&str; 3] = ["Owner", "Group", "Others"];

/// What the dialog's header shows for an entry before and besides its facts.
#[derive(Debug, Clone, Default)]
pub struct Look {
    /// A folder, drawn with the folder icon when the theme has none.
    pub folder: bool,
    /// The type's icon from the system theme.
    pub icon: Option<Arc<Path>>,
    /// A thumbnail can be made of it.
    pub picture: bool,
}

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
        /// Entries inside it, folders included.
        items: u64,
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
    /// Entries a folder walk has found so far.
    pub seen: Arc<AtomicU64>,
    /// The header's picture.
    look: Look,
    scroll: ScrollHandle,
}

impl PropertiesDialog {
    /// A dialog that has not read anything yet.
    pub fn loading(path: PathBuf, type_label: String, look: Look, token: u64) -> Self {
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
            seen: Arc::new(AtomicU64::new(0)),
            look,
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
        use gpui_kit::assets::IconName as I;
        let dirty = self.editable && self.mode != self.original;
        let body = self.body(zone, theme, cx);
        let icon = if self.look.picture {
            I::Image
        } else if self.look.folder {
            I::Folder
        } else {
            I::File
        };
        let panel = dialog::panel(
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
                    if modifiers.control || modifiers.alt || modifiers.platform || modifiers.shift {
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
                })),
            size::DIALOG,
            theme,
            window,
        )
        .child(dialog::title_row(
            icon,
            format!("{} \u{2014} Properties", self.title),
            "Close (Esc)",
            cx.listener(|this, _, _, cx| this.close_properties(cx)),
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
                    // The scrollbar lies in the body's right padding, so the
                    // content keeps the full width.
                    .mr(px(-space::LG))
                    .child(
                        div()
                            .id("properties-content")
                            .min_h_0()
                            .overflow_y_scroll()
                            .track_scroll(&self.scroll)
                            .child(div().pr(px(space::LG)).child(body)),
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
            ),
        )
        .child(self.buttons(dirty, theme, cx));
        dialog::scrim(panel, theme)
    }

    /// The header: a thumbnail or the type icon, the name and the type.
    fn header(
        &self,
        type_label: String,
        theme: Theme,
        cx: &mut gpui_kit::Context<Workspace>,
    ) -> impl IntoElement {
        use gpui_kit::StyledImage as _;
        let thumbnail = self.look.picture.then(|| {
            let size = rustphin_platform::thumbnails::ThumbSize::for_edge(
                size::PROPERTIES_THUMB_WIDTH as u32,
            );
            crate::thumbs::thumbs(cx).update(cx, |thumbs, _| thumbs.get(&self.path, size))
        });
        let picture = match (thumbnail.flatten(), &self.look.icon) {
            (Some(thumb), _) => img(thumb)
                .w(px(size::PROPERTIES_THUMB_WIDTH))
                .h(px(size::PROPERTIES_THUMB_HEIGHT))
                .object_fit(gpui_kit::ObjectFit::Contain)
                .into_any_element(),
            (None, Some(icon)) => img(icon.clone())
                .size(px(size::ICON_HEADER))
                .into_any_element(),
            (None, None) => gpui_kit::component::Icon::new(if self.look.folder {
                gpui_kit::assets::IconName::Folder
            } else {
                gpui_kit::assets::IconName::File
            })
            .size(px(size::ICON_HEADER))
            .text_color(theme.text_muted)
            .into_any_element(),
        };
        div()
            .flex()
            .items_center()
            .gap(px(space::LG))
            .child(div().flex_none().child(picture))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .min_w_0()
                    .child(
                        div()
                            .truncate()
                            .font_family(text::FAMILY_HEADING)
                            .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                            .text_size(px(text::HEADING))
                            .line_height(px(text::HEADING_LINE))
                            .child(self.title.clone()),
                    )
                    .child(
                        div()
                            .truncate()
                            .font_family(text::FAMILY_DATA)
                            .text_size(px(text::DATA))
                            .text_color(theme.text_muted)
                            .child(type_label),
                    ),
            )
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
        let location = self
            .path
            .parent()
            .map(|folder| display_name(folder.as_os_str().as_encoded_bytes()).into_owned())
            .unwrap_or_default();
        div()
            .flex()
            .flex_col()
            .gap(px(space::LG))
            .child(self.header(type_label, theme, cx))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(space::XS))
                    .child(fact("Location", location, theme))
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
                    }),
            )
            .child(div().h(px(size::HAIRLINE)).bg(theme.border))
            .child(self.permissions(facts, theme, cx))
            .into_any_element()
    }

    fn size_text(&self) -> String {
        match &self.size {
            SizeLine::Reading => "Reading…".to_owned(),
            SizeLine::Known(bytes) => format::size_exact(*bytes),
            SizeLine::Calculating => match self.seen.load(Ordering::Relaxed) {
                0 => "Calculating size…".to_owned(),
                seen => format!(
                    "Calculating… {} {} so far",
                    format::count(seen as usize),
                    format::plural(seen as usize, "item", "items")
                ),
            },
            SizeLine::Folder {
                bytes,
                unreadable,
                items,
            } => {
                let size = format!(
                    "{}, {} {}",
                    format::size_exact(*bytes),
                    format::count(*items as usize),
                    format::plural(*items as usize, "item", "items")
                );
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

    fn permissions(
        &self,
        facts: &Facts,
        theme: Theme,
        cx: &mut gpui_kit::Context<Workspace>,
    ) -> impl IntoElement {
        let mode = combine_mode(self.mode, self.original);
        let editable = self.editable;
        let words = format!("{} ({})", symbolic_mode(mode), octal_mode(mode));
        let names = [
            format!("OWNER \u{b7} {}", facts.owner),
            format!("GROUP \u{b7} {}", facts.group),
            "OTHERS".to_owned(),
        ];
        div()
            .id("permissions")
            .role(Role::Group)
            .aria_label("Permissions")
            .aria_description(words.clone())
            .flex()
            .flex_col()
            .gap(px(space::MD))
            .child(fact("Permissions", words, theme))
            .when(has_special(self.original), |column| {
                column.child(
                    div()
                        .id("special-permissions")
                        .role(Role::Label)
                        .aria_value("Setuid, setgid or the sticky bit is set, and is kept.")
                        .text_color(theme.text_muted)
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
                        .child("Permissions of a shortcut are not changed here."),
                )
            })
            .child(div().flex().children({
                let mut columns = Vec::new();
                for (class, name) in names.into_iter().enumerate() {
                    columns.push(self.class_column(class as u8, name, editable, theme, cx));
                }
                columns
            }))
    }

    /// One class's checkboxes under its name, hairlines between the columns.
    fn class_column(
        &self,
        class: u8,
        heading: String,
        editable: bool,
        theme: Theme,
        cx: &mut gpui_kit::Context<Workspace>,
    ) -> AnyElement {
        let name = PERMISSION_CLASSES[class as usize];
        div()
            .id(("permission-class", class as usize))
            .role(Role::Group)
            .aria_label(heading.clone())
            .flex()
            .flex_col()
            // Three equal columns, whatever their padding.
            .flex_none()
            .w(relative(1. / 3.))
            .min_w_0()
            .gap(px(space::XS))
            .when(class == 0, |column| column.pr(px(space::LG)))
            .when(class > 0, |column| {
                column
                    .px(px(space::LG))
                    .border_l_1()
                    .border_color(theme.border)
            })
            .child(
                div()
                    .truncate()
                    .font_family(text::FAMILY_DATA)
                    .font_weight(gpui_kit::FontWeight::MEDIUM)
                    .text_size(px(text::LABEL))
                    .text_color(theme.text_muted)
                    // `label-sm` is set in capitals; the group's name keeps
                    // the original spelling for assistive technology.
                    .child(heading.to_uppercase()),
            )
            .child(self.bit(class, "Read", read_bit(class), editable, cx))
            .child(self.bit(class, "Write", write_bit(class), editable, cx))
            .child(self.bit(class, "Execute", execute_bit(class), editable, cx))
            .aria_description(format!("{name} permissions"))
            .into_any_element()
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
        // 28-pixel rows, as the design's checkbox rows.
        div().flex().items_center().h(px(size::CHECK_ROW)).child(
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
                })),
        )
    }

    fn buttons(
        &self,
        dirty: bool,
        theme: Theme,
        cx: &mut gpui_kit::Context<Workspace>,
    ) -> impl IntoElement {
        dialog::actions(theme)
            .child(
                dialog::button("properties-close", "Close (Esc)")
                    .outlined()
                    .on_click(cx.listener(|this, _, _, cx| this.close_properties(cx))),
            )
            .when(self.error.is_none(), |row| {
                row.child(
                    dialog::button("properties-apply", "Apply (Enter)")
                        .primary()
                        .disabled(!dirty)
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
        .items_start()
        .gap(px(space::SM))
        .child(
            div()
                .w(px(size::PROPERTIES_LABEL))
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
        .map(|seconds| format::timestamp_precise(seconds, zone))
        .filter(|text| !text.is_empty())
        .unwrap_or_else(|| "Unknown".to_owned())
}

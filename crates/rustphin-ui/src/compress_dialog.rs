//! Compress to… (W-111d, D-032, mockup 37): the new archive's name, format
//! and compression, and exactly what will be made. Nothing is ever
//! replaced: a taken name disables Compress and says so.

use std::{collections::HashSet, path::PathBuf};

use gpui_kit::{
    AppContext as _, Context, Entity, FocusHandle, InteractiveElement as _, IntoElement,
    ParentElement as _, Role, StatefulInteractiveElement as _, Styled as _, Subscription, Window,
    component::{
        Disableable as _, Sizable as _,
        button::{Button, ButtonRounded, ButtonVariants},
        input::{Input, InputEvent, InputState},
    },
    div,
    prelude::FluentBuilder as _,
    px,
};
use rustphin_core::{
    archive::{Format, Level},
    ops::Request,
};

use crate::{
    controls::Outlined as _,
    dialog,
    operations::{folder_name, hub},
    theme::{Theme, size, space, text},
    workspace::Workspace,
};

/// The formats offered, in the menu's order: ZIP first (D-032).
const FORMATS: [(Format, &str); 5] = [
    (Format::Zip, "ZIP"),
    (Format::TarGz, "TAR.GZ"),
    (Format::TarXz, "TAR.XZ"),
    (Format::TarZst, "TAR.ZST"),
    (Format::SevenZ, "7Z"),
];

const LEVELS: [(Level, &str); 3] = [
    (Level::Fast, "Fast"),
    (Level::Normal, "Normal"),
    (Level::Smallest, "Smallest"),
];

/// The width the panel grows to.
const WIDTH: f32 = 560.;
/// The label column.
const LABEL: f32 = 96.;

/// The file name `name` and `format` give, or what is wrong with it.
pub fn archive_name(
    name: &str,
    format: Format,
    taken: &HashSet<Vec<u8>>,
) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("Type a name for the archive.".into());
    }
    if name.contains('/') || name.contains('\0') {
        return Err("A name cannot contain \u{201c}/\u{201d}.".into());
    }
    let file = format!("{name}.{}", format.extension());
    if file.len() > 255 {
        return Err("The name is too long.".into());
    }
    if taken.contains(file.as_bytes()) {
        return Err(format!(
            "\u{201c}{file}\u{201d} already exists here. Choose another name."
        ));
    }
    Ok(file)
}

/// What each format is good for, said under the choices.
fn format_hint(format: Format) -> &'static str {
    match format {
        Format::Zip => "ZIP opens everywhere, including Windows and phones.",
        Format::TarGz => "TAR.GZ is the usual choice on Linux.",
        Format::TarXz => "TAR.XZ is smaller and slower to make.",
        Format::TarZst => "TAR.ZST is fast and small; older systems may not open it.",
        Format::SevenZ => "7Z is small; links are not kept.",
        Format::Tar | Format::TarBz2 => "",
    }
}

/// The open Compress dialog.
pub struct CompressDialog {
    sources: Vec<PathBuf>,
    folder: PathBuf,
    taken: HashSet<Vec<u8>>,
    format: Format,
    level: Level,
    name: Entity<InputState>,
    focus: FocusHandle,
    _input: Subscription,
}

impl CompressDialog {
    /// Opens the dialog for `sources` in `folder`, offering `base`.
    pub fn open(
        sources: Vec<PathBuf>,
        folder: PathBuf,
        base: String,
        taken: Vec<Vec<u8>>,
        window: &mut Window,
        cx: &mut Context<Workspace>,
    ) -> Self {
        let name = cx.new(|cx| {
            let mut input = InputState::new(window, cx).placeholder("Archive name");
            input.set_value(base, window, cx);
            input
        });
        let _input =
            cx.subscribe_in(
                &name,
                window,
                |this, _, event: &InputEvent, window, cx| match event {
                    InputEvent::PressEnter { .. } => this.apply_compress(window, cx),
                    InputEvent::Change => cx.notify(),
                    InputEvent::Focus | InputEvent::Blur => {}
                },
            );
        gpui_kit::Focusable::focus_handle(name.read(cx), cx).focus(window, cx);
        Self {
            sources,
            folder,
            taken: taken.into_iter().collect(),
            format: Format::Zip,
            level: Level::Normal,
            name,
            focus: cx.focus_handle(),
            _input,
        }
    }

    fn file_name(&self, cx: &gpui_kit::App) -> Result<String, String> {
        archive_name(&self.name.read(cx).value(), self.format, &self.taken)
    }

    /// The request the dialog makes now, if the name is usable.
    fn request(&self, cx: &gpui_kit::App) -> Option<Request> {
        let file = self.file_name(cx).ok()?;
        Some(Request::Compress {
            sources: self.sources.clone(),
            archive: self.folder.join(file),
            format: self.format,
            level: self.level,
        })
    }

    pub fn render(
        &self,
        theme: Theme,
        window: &Window,
        cx: &mut Context<Workspace>,
    ) -> impl IntoElement {
        let count = self.sources.len();
        let title = if count == 1 {
            format!("Compress {}", folder_name(&self.sources[0]))
        } else {
            format!("Compress {count} items")
        };
        let checked = self.file_name(cx);
        let note = match &checked {
            Ok(file) => format!(
                "Makes \u{201c}{file}\u{201d} in {}. {}",
                folder_name(&self.folder),
                format_hint(self.format)
            ),
            Err(problem) => problem.clone(),
        };
        let ready = checked.is_ok();
        let row = |label: &'static str, control: gpui_kit::AnyElement| {
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap(px(space::SM))
                .child(
                    div()
                        .w(px(LABEL))
                        .flex_none()
                        .text_color(theme.text_muted)
                        .child(label),
                )
                .child(control)
        };
        let formats = segmented(
            "compress-format",
            "Format",
            &FORMATS,
            self.format,
            theme,
            |this, format| this.format = format,
            cx,
        );
        let levels = segmented(
            "compress-level",
            "Compression",
            &LEVELS,
            self.level,
            theme,
            |this, level| this.level = level,
            cx,
        );
        let panel = dialog::panel(
            div()
                .id("compress-dialog")
                .role(Role::Dialog)
                .aria_label(title.clone())
                .aria_description(note.clone())
                .a11y_synthetic_children(|builder| builder.parent_node().set_modal())
                .key_context("Dialog")
                .track_focus(&self.focus)
                .on_action(
                    cx.listener(|this, _: &crate::workspace::DialogAccept, window, cx| {
                        this.apply_compress(window, cx);
                    }),
                )
                .on_action(
                    cx.listener(|this, _: &crate::workspace::DialogCancel, window, cx| {
                        this.cancel_compress(window, cx);
                    }),
                ),
            WIDTH,
            theme,
            window,
        )
        .child(dialog::title_row(
            gpui_kit::assets::IconName::Archive,
            title,
            "Cancel (Esc)",
            cx.listener(|this, _, window, cx| this.cancel_compress(window, cx)),
            theme,
        ))
        .child(
            dialog::body()
                .gap(px(space::LG))
                .child(row(
                    "Name",
                    div()
                        .flex_1()
                        .min_w_0()
                        .child(crate::input::named(
                            Input::new(&self.name).min_h(px(size::BUTTON)),
                            &self.name,
                            "Archive name",
                            window,
                            cx,
                        ))
                        .into_any_element(),
                ))
                .child(row("Format", formats))
                .child(row("Compression", levels))
                .child(
                    div()
                        .text_size(text::scaled(text::SMALL))
                        .text_color(if ready {
                            theme.text_muted
                        } else {
                            theme.danger
                        })
                        .child(note),
                ),
        )
        .child(
            dialog::actions(theme)
                .child(
                    dialog::button("compress-cancel", "Cancel (Esc)")
                        .outlined()
                        .on_click(
                            cx.listener(|this, _, window, cx| this.cancel_compress(window, cx)),
                        ),
                )
                .child(
                    dialog::button("compress-apply", "Compress (Enter)")
                        .primary()
                        .disabled(!ready)
                        .on_click(
                            cx.listener(|this, _, window, cx| this.apply_compress(window, cx)),
                        ),
                ),
        );
        dialog::scrim(panel, theme)
    }
}

/// A row of segments choosing one of `options`; `set` applies a choice.
fn segmented<T: Copy + PartialEq + 'static>(
    id: &'static str,
    label: &'static str,
    options: &[(T, &'static str)],
    current: T,
    theme: Theme,
    set: fn(&mut CompressDialog, T),
    cx: &mut Context<Workspace>,
) -> gpui_kit::AnyElement {
    div()
        .id(id)
        .role(Role::RadioGroup)
        .aria_label(label)
        .flex()
        .flex_wrap()
        .rounded(px(size::RADIUS))
        .border_1()
        .border_color(theme.border_strong)
        .overflow_hidden()
        .children(options.iter().enumerate().map(|(index, &(value, words))| {
            Button::new((id, index))
                .with_size(px(space::LG * 5.))
                .h(px(size::BUTTON))
                .rounded(ButtonRounded::None)
                .when(value == current, ButtonVariants::primary)
                .when(value != current, ButtonVariants::ghost)
                // Said as well as shown, so screen readers know the choice.
                .toggled(value == current)
                .accessibility_label(words)
                .child(div().text_size(text::scaled(text::BODY)).child(words))
                .when(index > 0, |button| {
                    button.border_l_1().border_color(theme.border_strong)
                })
                .on_click(cx.listener(move |this, _, _, cx| {
                    if let Some(dialog) = this.compress.as_mut() {
                        set(dialog, value);
                        cx.notify();
                    }
                }))
        }))
        .into_any_element()
}

impl Workspace {
    /// Opens Compress to… for `sources` in `folder`.
    pub(crate) fn open_compress(
        &mut self,
        sources: Vec<PathBuf>,
        folder: PathBuf,
        base: String,
        taken: Vec<Vec<u8>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.compress.is_some() || !hub(cx).read(cx).questions.is_empty() {
            return;
        }
        self.compress = Some(CompressDialog::open(
            sources, folder, base, taken, window, cx,
        ));
        cx.notify();
    }

    /// Starts the compression the dialog describes, if its name is usable.
    pub(crate) fn apply_compress(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(request) = self.compress.as_ref().and_then(|dialog| dialog.request(cx)) else {
            return;
        };
        self.compress = None;
        hub(cx).update(cx, |hub, _| hub.start(request));
        self.focus_active(window, cx);
        cx.notify();
    }

    /// Closes the dialog without compressing.
    pub(crate) fn cancel_compress(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.compress.take().is_some() {
            self.focus_active(window, cx);
            cx.notify();
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn the_name_gets_the_formats_ending_and_a_taken_one_is_refused() {
        let taken: HashSet<Vec<u8>> = [b"report.zip".to_vec()].into_iter().collect();
        assert_eq!(
            archive_name("  photos ", Format::TarXz, &taken),
            Ok("photos.tar.xz".into())
        );
        assert!(
            archive_name("report", Format::Zip, &taken)
                .unwrap_err()
                .contains("already exists")
        );
        assert_eq!(
            archive_name("report", Format::SevenZ, &taken),
            Ok("report.7z".into())
        );
        assert!(archive_name("   ", Format::Zip, &taken).is_err());
        assert!(archive_name("a/b", Format::Zip, &taken).is_err());
        assert!(archive_name(&"x".repeat(260), Format::Zip, &taken).is_err());
    }
}

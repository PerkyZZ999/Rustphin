//! DISPOSABLE SPIKE — Discovery phase toolkit check. Not production code.
//!
//! Exercises, on Plasma Wayland: a 500k-row virtualised list, Breeze SVG icons,
//! dragging files out to other apps, receiving dropped files, file clipboard
//! read/write, and compositor blur behind a translucent window.

mod blur;
mod clipboard;

use std::path::PathBuf;

use gpui::{
    App, Bounds, Context, ExternalDragPayload, ExternalPaths,
    FileDragPaths, SharedString, Window, WindowBackgroundAppearance, WindowBounds, WindowOptions,
    div, img, prelude::*, px, rgb, rgba, size, uniform_list,
};
use gpui_platform::application;

const ROWS: usize = 500_000;
const ICON_DIR: &str = "/usr/share/icons/breeze-dark";

#[derive(Clone)]
struct DraggedFile(PathBuf);

struct DragPreview(SharedString);

impl Render for DragPreview {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .px_2()
            .py_1()
            .rounded_md()
            .bg(rgb(0x2a4fd6))
            .text_color(rgb(0xffffff))
            .child(self.0.clone())
    }
}

struct Spike {
    files: Vec<PathBuf>,
    status: SharedString,
}

impl Spike {
    fn button(
        label: &'static str,
        cx: &mut Context<Self>,
        on_click: impl Fn(&mut Self, &mut Context<Self>) + 'static,
    ) -> impl IntoElement {
        div()
            .id(label)
            .px_3()
            .py_1()
            .rounded_md()
            .bg(rgb(0x2a4fd6))
            .cursor_pointer()
            .child(label)
            .on_click(cx.listener(move |this, _, _, cx| {
                on_click(this, cx);
                cx.notify();
            }))
    }
}

impl Render for Spike {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let files = self.files.clone();
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(rgba(0x0b0d1299))
            .text_color(rgb(0xe6e8ef))
            .text_sm()
            .on_drop(cx.listener(|this, paths: &ExternalPaths, _, cx| {
                this.status = format!("DROPPED IN: {:?}", paths.paths()).into();
                println!("{}", this.status);
                cx.notify();
            }))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .p_3()
                    .child(img(PathBuf::from(format!("{ICON_DIR}/places/64/folder.svg"))).size_16())
                    .child(
                        img(PathBuf::from(format!("{ICON_DIR}/mimetypes/64/text-plain.svg")))
                            .size_16(),
                    )
                    .child(Self::button("Copy 2 files", cx, |this, _| {
                        let paths: Vec<PathBuf> = this.files.iter().take(2).cloned().collect();
                        this.status = format!("COPY: {:?}", clipboard::write_files(&paths, false)).into();
                        println!("{}", this.status);
                    }))
                    .child(Self::button("Cut last file", cx, |this, _| {
                        let paths: Vec<PathBuf> = this.files.iter().rev().take(1).cloned().collect();
                        this.status = format!("CUT: {:?}", clipboard::write_files(&paths, true)).into();
                        println!("{}", this.status);
                    }))
                    .child(Self::button("Read clipboard", cx, |this, cx| {
                        this.status = format!(
                            "FILES: {:?} | GPUI: {:?}",
                            clipboard::read_files(),
                            cx.read_from_clipboard().map(|item| item.entries().len())
                        )
                        .into();
                        println!("{}", this.status);
                    })),
            )
            .child(div().px_3().pb_2().child(self.status.clone()))
            .child(
                uniform_list("entries", ROWS, move |range, _window, _cx| {
                    range
                        .map(|ix| {
                            let path = files[ix % files.len()].clone();
                            let name: SharedString = format!(
                                "{ix:06}  {}",
                                path.file_name().unwrap().to_string_lossy()
                            )
                            .into();
                            let preview = name.clone();
                            div()
                                .id(ix)
                                .h(px(28.))
                                .px_3()
                                .flex()
                                .items_center()
                                .gap_2()
                                .hover(|style| style.bg(rgba(0x2a4fd640)))
                                .child(
                                    img(PathBuf::from(format!(
                                        "{ICON_DIR}/mimetypes/16/text-plain.svg"
                                    )))
                                    .size_4(),
                                )
                                .child(name)
                                .on_drag(DraggedFile(path), move |_, _, _, cx| {
                                    cx.new(|_| DragPreview(preview.clone()))
                                })
                                .external_drag_payload(|dragged: &DraggedFile, _, _| {
                                    println!("EXTERNAL DRAG requested for {:?}", dragged.0);
                                    Some(ExternalDragPayload::Files(FileDragPaths::new([(
                                        dragged.0.clone(),
                                        false,
                                    )])))
                                })
                        })
                        .collect::<Vec<_>>()
                })
                .flex_1(),
            )
    }
}

fn main() {
    let dir = std::env::temp_dir().join("rustphin-spike-files");
    std::fs::create_dir_all(&dir).unwrap();
    let files: Vec<PathBuf> = (0..20)
        .map(|i| {
            let path = dir.join(format!("spike-file-{i:02}.txt"));
            std::fs::write(&path, format!("rustphin spike file {i}\n")).unwrap();
            path
        })
        .collect();

    application().run(move |cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(720.), px(560.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_background: WindowBackgroundAppearance::Blurred,
                app_id: Some("rustphin-spike".into()),
                ..Default::default()
            },
            |window, cx| {
                match blur::enable(window) {
                    Ok(()) => println!("BLUR requested via ext-background-effect-v1"),
                    Err(err) => println!("BLUR unavailable: {err}"),
                }
                window.refresh();
                cx.new(|_| Spike {
                    files,
                    status: "ready".into(),
                })
            },
        )
        .unwrap();
        cx.activate(true);
    });
}

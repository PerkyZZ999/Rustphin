//! The Views page of Settings (W-106c2, mockup 36c): how folders are shown
//! when they open. The default view's changes reach every open pane at
//! once; the rest redraw where they show.

use std::rc::Rc;

use gpui_kit::{
    AnyElement, Context, IntoElement, ParentElement as _, Styled as _, Window,
    component::slider::Slider, div, px,
};
use rustphin_core::{SortKey, ViewOptions};

use crate::{
    format::{self, Dates, Units},
    layout::ViewMode,
    preferences::preferences,
    settings::{DetailColumns, Previews, Settings},
    settings_controls::{self as controls, Pick},
    settings_dialog::{SettingsDialog, group, setting_row},
    theme::{Theme, space, text},
};

/// The preview limits offered, in megabytes.
const LIMITS_MB: [u32; 4] = [5, 10, 50, 100];

/// Changes the default view: every open pane takes it on.
fn default_view(
    cx: &mut Context<SettingsDialog>,
    change: impl FnOnce(&mut ViewMode, &mut usize, &mut ViewOptions),
) {
    preferences(cx).update(cx, |store, cx| {
        store.update_default_view(
            |settings| {
                let (mut mode, mut zoom, mut options) =
                    (settings.mode(), settings.zoom(), settings.options());
                change(&mut mode, &mut zoom, &mut options);
                settings.set_view(mode, zoom, options);
            },
            cx,
        );
    });
}

/// Changes another choice of the page.
fn choose(cx: &mut Context<SettingsDialog>, change: impl FnOnce(&mut Settings)) {
    preferences(cx).update(cx, |store, cx| store.update(change, cx));
}

impl SettingsDialog {
    /// The Views page's groups.
    pub(crate) fn views(
        &mut self,
        theme: Theme,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let settings = preferences(cx).read(cx).settings().clone();
        let (mode, zoom, options) = (settings.mode(), settings.zoom(), settings.options());
        // The slider follows the default, whichever way it was changed.
        if self.zoom_slider.read(cx).value().start() != zoom as f32 {
            self.zoom_slider
                .update(cx, |slider, cx| slider.set_value(zoom as f32, window, cx));
        }

        let pick_mode: Pick<ViewMode> = Rc::new(|_, mode, _, cx| {
            default_view(cx, |current, _, _| *current = mode);
        });
        let pick_per_folder: Pick<bool> = Rc::new(|_, on, _, cx| {
            choose(cx, |settings| settings.set_per_folder(on));
        });
        let default = vec![
            setting_row(
                "View",
                controls::segmented(
                    "settings-view-mode",
                    "View",
                    &[
                        (ViewMode::Icons, "Icons"),
                        (ViewMode::Compact, "Compact"),
                        (ViewMode::Details, "Details"),
                    ],
                    Some(mode),
                    theme,
                    pick_mode,
                    cx,
                ),
            ),
            setting_row(
                "Size",
                div()
                    .flex()
                    .items_center()
                    .gap(px(space::MD))
                    .child(
                        div().w(px(crate::theme::size::SETTING_SELECT)).child(
                            Slider::new(&self.zoom_slider)
                                .horizontal()
                                .bg(theme.accent)
                                .text_color(theme.text),
                        ),
                    )
                    .child(
                        div()
                            .font_family(text::FAMILY_DATA)
                            .text_size(text::scaled(text::SMALL))
                            .child(format!("{} px", mode.layout(zoom).icon as u32)),
                    )
                    .into_any_element(),
            ),
            setting_row(
                "Each folder",
                controls::checkbox(
                    "settings-per-folder",
                    "Remember the view for each folder",
                    settings.per_folder(),
                    pick_per_folder,
                    cx,
                ),
            ),
        ];

        let pick_sort: Pick<SortKey> = Rc::new(|_, key, _, cx| {
            default_view(cx, |_, _, options| options.sort = key);
        });
        let pick_direction: Pick<bool> = Rc::new(|_, descending, _, cx| {
            default_view(cx, |_, _, options| options.descending = descending);
        });
        let pick_folders_first: Pick<bool> = Rc::new(|_, on, _, cx| {
            default_view(cx, |_, _, options| options.folders_first = on);
        });
        let pick_hidden: Pick<bool> = Rc::new(|_, on, _, cx| {
            default_view(cx, |_, _, options| options.show_hidden = on);
        });
        let order = vec![
            setting_row(
                "Sort by",
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap(px(space::SM))
                    .child(controls::short_select(
                        "settings-sort",
                        "Sort by",
                        &[
                            (SortKey::Name, "Name"),
                            (SortKey::Size, "Size"),
                            (SortKey::Modified, "Modified"),
                            (SortKey::Type, "Type"),
                        ],
                        options.sort,
                        &self.selects,
                        theme,
                        pick_sort,
                        cx,
                    ))
                    .child(controls::segmented(
                        "settings-direction",
                        "Direction",
                        &[(false, "A to Z"), (true, "Z to A")],
                        Some(options.descending),
                        theme,
                        pick_direction,
                        cx,
                    ))
                    .into_any_element(),
            ),
            setting_row(
                "",
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap(px(space::LG))
                    .child(controls::checkbox(
                        "settings-folders-first",
                        "Folders first",
                        options.folders_first,
                        pick_folders_first,
                        cx,
                    ))
                    .child(controls::checkbox(
                        "settings-hidden",
                        "Show hidden files",
                        options.show_hidden,
                        pick_hidden,
                        cx,
                    ))
                    .into_any_element(),
            ),
        ];

        let columns = settings.detail_columns();
        let column = |id: &'static str,
                      label: &'static str,
                      checked: bool,
                      set: fn(&mut DetailColumns, bool),
                      cx: &mut Context<Self>| {
            let pick: Pick<bool> = Rc::new(move |_, on, _, cx| {
                choose(cx, |settings| {
                    let mut columns = settings.detail_columns();
                    set(&mut columns, on);
                    settings.set_detail_columns(columns);
                });
            });
            controls::checkbox(id, label, checked, pick, cx)
        };
        let (dates, units) = settings.formats();
        let now = jiff::Timestamp::now();
        let zone = jiff::tz::TimeZone::system();
        let iso = format::timestamp_in(now.as_second(), &zone, Dates::Iso, now);
        let relative = format::timestamp_in(now.as_second(), &zone, Dates::Relative, now);
        let pick_dates: Pick<Dates> = Rc::new(|_, dates, _, cx| {
            choose(cx, |settings| {
                let units = settings.formats().1;
                settings.set_formats(dates, units);
            });
        });
        let pick_units: Pick<Units> = Rc::new(|_, units, _, cx| {
            choose(cx, |settings| {
                let dates = settings.formats().0;
                settings.set_formats(dates, units);
            });
        });
        let details = vec![
            setting_row(
                "Columns",
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap(px(space::LG))
                    .child(column(
                        "settings-column-size",
                        "Size",
                        columns.size,
                        |columns, on| columns.size = on,
                        cx,
                    ))
                    .child(column(
                        "settings-column-modified",
                        "Modified",
                        columns.modified,
                        |columns, on| columns.modified = on,
                        cx,
                    ))
                    .child(column(
                        "settings-column-type",
                        "Type",
                        columns.kind,
                        |columns, on| columns.kind = on,
                        cx,
                    ))
                    .into_any_element(),
            ),
            setting_row(
                "Dates",
                controls::select(
                    "settings-dates",
                    "Dates",
                    &[
                        (Dates::Iso, iso.as_str()),
                        (Dates::Relative, relative.as_str()),
                    ],
                    dates,
                    &self.selects,
                    theme,
                    pick_dates,
                    cx,
                ),
            ),
            setting_row(
                "Sizes",
                controls::segmented(
                    "settings-units",
                    "Sizes",
                    &[
                        (Units::Binary, "Binary (KiB, MiB)"),
                        (Units::Decimal, "Decimal (kB, MB)"),
                    ],
                    Some(units),
                    theme,
                    pick_units,
                    cx,
                ),
            ),
        ];

        let previews = settings.previews();
        let pick_shown: Pick<bool> = Rc::new(|_, shown, _, cx| {
            choose(cx, |settings| {
                let previews = settings.previews();
                settings.set_previews(Previews { shown, ..previews });
            });
        });
        let pick_limit: Pick<u32> = Rc::new(|_, limit_mb, _, cx| {
            choose(cx, |settings| {
                let previews = settings.previews();
                settings.set_previews(Previews {
                    limit_mb,
                    ..previews
                });
            });
        });
        let mut limits: Vec<u32> = LIMITS_MB.to_vec();
        if !limits.contains(&previews.limit_mb) {
            limits.push(previews.limit_mb);
        }
        let limit_words: Vec<(u32, String)> = limits
            .into_iter()
            .map(|mb| (mb, format!("up to {mb} MB a file")))
            .collect();
        let limits: Vec<(u32, &str)> = limit_words
            .iter()
            .map(|(mb, words)| (*mb, words.as_str()))
            .collect();
        let thumbnails = vec![setting_row(
            "Thumbnails",
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap(px(space::LG))
                .child(controls::checkbox(
                    "settings-thumbnails",
                    "Show pictures\u{2019} thumbnails",
                    previews.shown,
                    pick_shown,
                    cx,
                ))
                .child(controls::select(
                    "settings-thumbnail-limit",
                    "Largest picture previewed",
                    &limits,
                    previews.limit_mb,
                    &self.selects,
                    theme,
                    pick_limit,
                    cx,
                ))
                .into_any_element(),
        )];

        vec![
            group("Default view", default, theme),
            group("Order", order, theme),
            group("Details view", details, theme),
            group("Previews", thumbnails, theme),
        ]
    }
}

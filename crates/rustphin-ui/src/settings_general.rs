//! The General page of Settings (W-106c, mockup 36a): how Rustphin starts
//! and how it works with the rest of the desktop.

use std::rc::Rc;

use gpui_kit::{
    AnyElement, AppContext as _, Context, Focusable as _, IntoElement, ParentElement as _,
    SharedString, Styled as _, Window,
    component::{Disableable as _, input::Input},
    div,
    prelude::FluentBuilder as _,
    px,
};
use rustphin_platform::{apps::Associations, kdeglobals, terminal, xdg};

use crate::{
    location::Location,
    preferences::preferences,
    settings::{Click, ShownIn, Start},
    settings_controls::{self as controls, Pick},
    settings_dialog::{SettingsDialog, group, setting_row},
    theme::{Theme, size, text},
};

/// Rustphin's desktop-file id, as installed.
pub const DESKTOP_ID: &str = "rustphin.desktop";

/// The type every file manager opens.
const FOLDER_TYPE: &str = "inode/directory";

/// What the page shows about the desktop, read off the UI thread.
#[derive(Debug, Clone)]
pub struct DesktopFacts {
    /// KDE's terminal, as words.
    kde_terminal: Vec<String>,
    /// The name of the application that opens folders now, if any.
    file_manager: Option<String>,
    /// That application is Rustphin.
    is_default: bool,
    /// Rustphin's desktop file is installed, so it can be made the default.
    installed: bool,
    /// KDE opens files with one click.
    kde_single_click: bool,
}

impl DesktopFacts {
    /// Reads KDE's settings and the application associations. Blocking.
    pub fn read() -> Self {
        let config_home = xdg::config_home();
        let kde = kdeglobals::read(config_home.as_deref());
        let apps = Associations::load(&xdg::config_dirs(), &xdg::data_dirs());
        let default = apps.default_for(FOLDER_TYPE);
        Self {
            kde_terminal: terminal::configured(config_home.as_deref()),
            file_manager: default.map(|app| app.name.clone()),
            is_default: default.is_some_and(|app| app.id == DESKTOP_ID),
            installed: apps.get(DESKTOP_ID).is_some(),
            kde_single_click: kde.is_some_and(|text| kdeglobals::single_click(&text)),
        }
    }
}

/// Where "Make Rustphin the default" stands.
#[derive(Debug, Clone, Default)]
pub enum MakeDefault {
    /// Not asked.
    #[default]
    Idle,
    /// Writing the association.
    Working,
    /// It did not take; why, in words.
    Failed(String),
}

impl SettingsDialog {
    /// The General page's groups.
    pub(crate) fn general(
        &mut self,
        theme: Theme,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let settings = preferences(cx).read(cx).settings().clone();
        let (start, start_folder) = settings.start();
        let start_folder = start_folder.map(str::to_owned);

        let pick_start: Pick<Start> = Rc::new(|this, start, _, cx| {
            let folder = (start == Start::Folder)
                .then(|| this.shown_folder_saved())
                .flatten();
            preferences(cx).update(cx, |store, cx| {
                store.update(
                    |settings| {
                        // The first choice of a folder takes the one shown.
                        let keep = settings.start().1.is_some();
                        settings.set_start(start, if keep { None } else { folder });
                    },
                    cx,
                );
            });
        });
        let mut starting = vec![setting_row(
            "When Rustphin starts",
            controls::select(
                "settings-start",
                "When Rustphin starts",
                &[
                    (Start::Restore, "Restore the last tabs"),
                    (Start::Home, "Open the home folder"),
                    (Start::Folder, "Open a chosen folder"),
                ],
                start,
                &self.selects,
                theme,
                pick_start,
                cx,
            ),
        )];
        if start == Start::Folder {
            let shown = self.shown_folder_saved();
            let chosen: SharedString = match start_folder.as_deref().and_then(Location::from_saved)
            {
                Some(Location::Folder(path)) => path.display().to_string().into(),
                Some(Location::Remote(address)) => address.to_string().into(),
                None => "No folder chosen: Rustphin opens the home folder.".into(),
            };
            let same = shown.is_some() && shown == start_folder;
            starting.push(setting_row(
                "",
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap(px(crate::theme::space::SM))
                    .child(
                        div()
                            .min_w_0()
                            .font_family(text::FAMILY_DATA)
                            .text_size(px(text::SMALL))
                            .child(chosen),
                    )
                    .child(
                        controls::button("settings-start-shown", "Use the folder shown")
                            .disabled(shown.is_none() || same)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                let folder = this.shown_folder_saved();
                                preferences(cx).update(cx, |store, cx| {
                                    store.update(
                                        |settings| settings.set_start(Start::Folder, folder),
                                        cx,
                                    );
                                });
                            })),
                    )
                    .into_any_element(),
            ));
        }
        let pick_shown: Pick<ShownIn> = Rc::new(|_, shown, _, cx| {
            preferences(cx).update(cx, |store, cx| {
                store.update(|settings| settings.set_shown_folders(shown), cx);
            });
        });
        starting.push(setting_row(
            "Folders opened by other applications",
            controls::segmented(
                "settings-shown-in",
                "Folders opened by other applications",
                &[
                    (ShownIn::Tab, "In a new tab"),
                    (ShownIn::Window, "In a new window"),
                ],
                Some(settings.shown_folders()),
                theme,
                pick_shown,
                cx,
            ),
        ));

        let mut desktop = vec![setting_row(
            "Default file manager",
            self.file_manager_row(theme, cx),
        )];
        desktop.extend(self.terminal_rows(settings.terminal(), theme, window, cx));
        let kde_single = self
            .desktop
            .as_ref()
            .map_or(preferences(cx).read(cx).kde_single_click(), |facts| {
                facts.kde_single_click
            });
        let pick_click: Pick<Click> = Rc::new(|_, click, _, cx| {
            preferences(cx).update(cx, |store, cx| {
                store.update(|settings| settings.set_click(Some(click)), cx);
            });
        });
        let current = settings.click().unwrap_or(if kde_single {
            Click::Single
        } else {
            Click::Double
        });
        desktop.push(setting_row(
            "Open files and folders with",
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap(px(crate::theme::space::SM))
                .child(controls::segmented(
                    "settings-click",
                    "Open files and folders with",
                    &[
                        (Click::Double, "A double click"),
                        (Click::Single, "A single click"),
                    ],
                    Some(current),
                    theme,
                    pick_click,
                    cx,
                ))
                .when(settings.click().is_none(), |row| {
                    row.child(controls::hint("As KDE is set", theme))
                })
                .into_any_element(),
        ));

        vec![
            group("Starting", starting, theme),
            group("Desktop", desktop, theme),
        ]
    }

    /// The folder the window shows, as the settings save a location.
    fn shown_folder_saved(&self) -> Option<String> {
        self.shown
            .clone()
            .map(|path| Location::Folder(path).to_saved())
    }

    fn file_manager_row(&self, theme: Theme, cx: &mut Context<Self>) -> AnyElement {
        let Some(facts) = &self.desktop else {
            return controls::hint("Checking which application opens folders\u{2026}", theme);
        };
        if facts.is_default {
            return controls::status_ok("Rustphin opens folders for this desktop.", theme);
        }
        let now: SharedString = match &facts.file_manager {
            Some(name) => format!("{name} opens folders now.").into(),
            None => "No application is set to open folders.".into(),
        };
        let working = matches!(self.make_default, MakeDefault::Working);
        let failed = match &self.make_default {
            MakeDefault::Failed(reason) => Some(reason.clone()),
            _ => None,
        };
        div()
            .flex()
            .flex_col()
            .gap(px(crate::theme::space::XS))
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap(px(crate::theme::space::SM))
                    .child(now)
                    .child(
                        controls::button(
                            "settings-make-default",
                            if working {
                                "Making it the default\u{2026}"
                            } else {
                                "Make Rustphin the default"
                            },
                        )
                        .disabled(working || !facts.installed)
                        .on_click(cx.listener(|this, _, _, cx| this.make_default(cx))),
                    ),
            )
            .when(!facts.installed, |column| {
                column.child(controls::hint(
                    "Rustphin\u{2019}s desktop entry (rustphin.desktop) is not installed, so \
                     the desktop cannot choose it.",
                    theme,
                ))
            })
            .when_some(failed, |column, reason| {
                column.child(
                    div()
                        .text_size(px(text::SMALL))
                        .text_color(theme.danger)
                        .child(reason),
                )
            })
            .into_any_element()
    }

    /// Writes Rustphin as the folder handler, then reads back what the
    /// desktop will use: another list can still win.
    fn make_default(&mut self, cx: &mut Context<Self>) {
        self.make_default = MakeDefault::Working;
        cx.notify();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move {
                    let home = xdg::config_home()
                        .ok_or_else(|| "There is no configuration folder.".to_owned())?;
                    rustphin_platform::apps::set_default(&home, FOLDER_TYPE, DESKTOP_ID)
                        .map_err(|error| format!("Could not save the choice: {error}."))?;
                    Ok::<_, String>(DesktopFacts::read())
                })
                .await;
            this.update(cx, |this, cx| {
                match result {
                    Ok(facts) => {
                        this.make_default = if facts.is_default {
                            MakeDefault::Idle
                        } else {
                            MakeDefault::Failed(format!(
                                "Saved, but {} still opens folders. Choose Rustphin in System \
                                 Settings, Default Applications, File manager.",
                                facts
                                    .file_manager
                                    .as_deref()
                                    .unwrap_or("another application")
                            ))
                        };
                        this.desktop = Some(facts);
                    }
                    Err(reason) => this.make_default = MakeDefault::Failed(reason),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn terminal_rows(
        &mut self,
        chosen: Option<&str>,
        theme: Theme,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let editing = self.editing_terminal || chosen.is_some();
        let (name, source): (SharedString, &str) = match chosen {
            Some(command) => (
                terminal::words(command)
                    .map(|words| terminal::display_name(&words))
                    .unwrap_or_else(|| command.to_owned())
                    .into(),
                "chosen here",
            ),
            None => match &self.desktop {
                Some(facts) => (
                    terminal::display_name(&facts.kde_terminal).into(),
                    "from KDE\u{2019}s settings",
                ),
                None => ("\u{2026}".into(), "from KDE\u{2019}s settings"),
            },
        };
        let mut rows = vec![setting_row(
            "Terminal",
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap(px(crate::theme::space::SM))
                .child(name)
                .child(controls::hint(source, theme))
                .when(!editing, |row| {
                    row.child(
                        controls::button("settings-terminal-other", "Use another\u{2026}")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.editing_terminal = true;
                                let focus = this.terminal_input.read(cx).focus_handle(cx);
                                focus.focus(window, cx);
                                cx.notify();
                            })),
                    )
                })
                .into_any_element(),
        )];
        if editing {
            let invalid = chosen.is_some_and(|command| terminal::words(command).is_none());
            rows.push(setting_row(
                "",
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .gap(px(crate::theme::space::XS))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(crate::theme::space::SM))
                            .child(
                                div().flex_1().max_w(px(size::SETTING_FIELD)).child(
                                    crate::input::named(
                                        Input::new(&self.terminal_input)
                                            .min_h(px(size::SETTING_CONTROL)),
                                        &self.terminal_input,
                                        "Terminal command",
                                        window,
                                        cx,
                                    ),
                                ),
                            )
                            .when(chosen.is_some(), |row| {
                                row.child(
                                    controls::button("settings-terminal-kde", "Use KDE\u{2019}s")
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            this.editing_terminal = false;
                                            this.terminal_input.update(cx, |input, cx| {
                                                input.set_value("", window, cx);
                                            });
                                            preferences(cx).update(cx, |store, cx| {
                                                store.update(|s| s.set_terminal(""), cx);
                                            });
                                        })),
                                )
                            }),
                    )
                    .child(if invalid {
                        div()
                            .text_size(px(text::SMALL))
                            .text_color(theme.danger)
                            .child("A quote is not closed: the command cannot be run.")
                            .into_any_element()
                    } else {
                        controls::hint(
                            "The program and its options; Rustphin adds the folder.",
                            theme,
                        )
                    })
                    .into_any_element(),
            ));
        }
        rows
    }
}

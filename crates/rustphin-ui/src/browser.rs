//! The browser view: toolbar, entry list and status line for one location.

use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

use gpui_kit::{
    App, AppContext as _, ClickEvent, Context, Entity, FocusHandle, InteractiveElement as _,
    IntoElement, KeyBinding, KeyDownEvent, Modifiers, ParentElement as _, Render, ScrollStrategy,
    SharedString, StatefulInteractiveElement as _, Styled as _, Subscription,
    UniformListScrollHandle, Window, actions,
    component::{
        Disableable as _, Icon, IconName, Selectable as _, Sizable as _,
        button::{Button, ButtonVariants as _},
        input::{Input, InputEvent, InputState},
    },
    div, img,
    prelude::FluentBuilder as _,
    px, uniform_list,
};
use jiff::tz::TimeZone;
use rustphin_core::{
    DirectoryModel, LoadError, ModelEvent, RealFs, Selection, Snapshot, SortKey, ViewOptions,
    display_name,
};

use crate::{
    filetypes::{Database, TypeInfo},
    format::{self, count, plural},
    theme::{Theme, size, space, text},
};

const KEY_CONTEXT: &str = "Browser";
/// The browser while no text field inside it has focus.
const LIST_CONTEXT: &str = "Browser && !Input";
/// Ancestors shown in the location bar before the rest collapse into an ellipsis.
const MAX_CRUMBS: usize = 4;
/// A pause longer than this starts a new type-ahead search.
const TYPE_AHEAD_PAUSE: Duration = Duration::from_millis(1000);
/// Rows moved by Page Up and Page Down.
const PAGE_ROWS: isize = 15;

actions!(
    browser,
    [
        SelectPrevious,
        SelectNext,
        SelectFirst,
        SelectLast,
        PageUp,
        PageDown,
        ExtendPrevious,
        ExtendNext,
        ExtendFirst,
        ExtendLast,
        ExtendPageUp,
        ExtendPageDown,
        SelectAll,
        InvertSelection,
        Activate,
        GoUp,
        GoBack,
        GoForward,
        Reload,
        ToggleHidden,
        EditLocation,
        CancelEdit,
    ]
);

/// Registers the browser's keyboard shortcuts. They match Dolphin's defaults.
///
/// Keys that also edit text (arrows, Enter, Backspace, Home, End) only act on
/// the list while no text field has focus; otherwise typing a path would move
/// the selection or navigate.
pub fn bind_keys(cx: &mut App) {
    let list = Some(LIST_CONTEXT);
    let view = Some(KEY_CONTEXT);
    cx.bind_keys([
        KeyBinding::new("up", SelectPrevious, list),
        KeyBinding::new("down", SelectNext, list),
        KeyBinding::new("home", SelectFirst, list),
        KeyBinding::new("end", SelectLast, list),
        KeyBinding::new("pageup", PageUp, list),
        KeyBinding::new("pagedown", PageDown, list),
        KeyBinding::new("shift-up", ExtendPrevious, list),
        KeyBinding::new("shift-down", ExtendNext, list),
        KeyBinding::new("shift-home", ExtendFirst, list),
        KeyBinding::new("shift-end", ExtendLast, list),
        KeyBinding::new("shift-pageup", ExtendPageUp, list),
        KeyBinding::new("shift-pagedown", ExtendPageDown, list),
        KeyBinding::new("ctrl-a", SelectAll, list),
        KeyBinding::new("ctrl-shift-a", InvertSelection, list),
        KeyBinding::new("enter", Activate, list),
        KeyBinding::new("backspace", GoUp, list),
        KeyBinding::new("alt-up", GoUp, view),
        KeyBinding::new("alt-left", GoBack, view),
        KeyBinding::new("alt-right", GoForward, view),
        KeyBinding::new("f5", Reload, view),
        KeyBinding::new("ctrl-h", ToggleHidden, view),
        KeyBinding::new("alt-.", ToggleHidden, view),
        KeyBinding::new("ctrl-l", EditLocation, view),
        KeyBinding::new("f6", EditLocation, view),
        KeyBinding::new("escape", CancelEdit, view),
    ]);
}

/// A failed attempt to open a location, shown inline until dismissed.
struct Failure {
    path: PathBuf,
    error: LoadError,
}

/// One browsing view of the filesystem.
pub struct Browser {
    /// `None` when the model's worker pool could not be created.
    model: Option<DirectoryModel>,
    snapshot: Option<Arc<Snapshot>>,
    /// The newest request; older snapshots are ignored.
    generation: u64,
    selection: Selection,
    /// Row of the selection's focus in the current snapshot, looked up once
    /// per snapshot instead of on every frame.
    focus_row: Option<usize>,
    /// Characters typed so far in a type-ahead search, and when the last arrived.
    typed: String,
    typed_at: Option<Instant>,
    /// Entry to select once the directory being opened has been listed.
    select_on_arrival: Option<Vec<u8>>,
    failure: Option<Failure>,
    editing_location: bool,
    location_input: Entity<InputState>,
    focus: FocusHandle,
    /// The system's file-type databases, once loaded in the background.
    types: Option<Arc<Database>>,
    /// Icon and description per MIME type, filled in as rows need them.
    type_info: HashMap<String, TypeInfo>,
    /// Types whose lookup is running, so each is requested once.
    types_pending: HashSet<String>,
    /// The zone dates are shown in.
    zone: TimeZone,
    scroll: UniformListScrollHandle,
    _subscriptions: Vec<Subscription>,
}

impl Browser {
    /// Creates the view and starts listing `start`.
    pub fn new(start: PathBuf, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let location_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("Enter a folder path"));
        let subscriptions = vec![cx.subscribe_in(&location_input, window, Self::on_location_input)];
        let focus = cx.focus_handle();
        focus.focus(window, cx);

        let mut this = Self {
            model: None,
            snapshot: None,
            generation: 0,
            selection: Selection::default(),
            focus_row: None,
            typed: String::new(),
            typed_at: None,
            select_on_arrival: None,
            failure: None,
            editing_location: false,
            location_input,
            focus,
            types: None,
            type_info: HashMap::new(),
            types_pending: HashSet::new(),
            zone: TimeZone::system(),
            scroll: UniformListScrollHandle::new(),
            _subscriptions: subscriptions,
        };
        match DirectoryModel::new(Arc::new(RealFs)) {
            Ok((mut model, events)) => {
                // The details view shows sizes and dates, so always load them.
                model.set_options(ViewOptions {
                    details: true,
                    ..ViewOptions::default()
                });
                this.model = Some(model);
                this.load_file_types(cx);
                cx.spawn(async move |view, cx| {
                    while let Ok(event) = events.recv_async().await {
                        if view
                            .update(cx, |view, cx| view.on_model_event(event, cx))
                            .is_err()
                        {
                            break;
                        }
                    }
                })
                .detach();
                this.open(start);
            }
            Err(error) => {
                tracing::error!(%error, "could not start the directory model");
                this.failure = Some(Failure {
                    path: start,
                    error: error.into(),
                });
            }
        }
        this
    }

    fn on_model_event(&mut self, event: ModelEvent, cx: &mut Context<Self>) {
        match event {
            ModelEvent::Snapshot(snapshot) if snapshot.generation() == self.generation => {
                // Follows the selection through a re-sort and empties it when
                // the directory was read again.
                self.selection.sync(&snapshot);
                let arrival = self.select_on_arrival.take_if(|_| !snapshot.is_loading());
                if let Some(row) = arrival.and_then(|name| snapshot.row_of(&name)) {
                    self.selection.select_only(&snapshot, row);
                }
                self.focus_row = self.selection.focus_row(&snapshot);
                self.snapshot = Some(snapshot);
                self.reveal_focus();
            }
            ModelEvent::Failed {
                generation,
                path,
                error,
            } if generation == self.generation => {
                self.select_on_arrival = None;
                self.failure = Some(Failure { path, error });
            }
            // Results of a request that has since been superseded.
            ModelEvent::Snapshot(_) | ModelEvent::Failed { .. } => return,
        }
        cx.notify();
    }

    fn open(&mut self, path: PathBuf) {
        if let Some(model) = &mut self.model {
            self.generation = model.open(path);
            self.failure = None;
        }
    }

    /// Changes the selection against the current snapshot and redraws.
    fn change_selection(
        &mut self,
        change: impl FnOnce(&mut Selection, &Snapshot),
        cx: &mut Context<Self>,
    ) {
        let Some(snapshot) = self.snapshot.clone() else {
            return;
        };
        change(&mut self.selection, &snapshot);
        self.focus_row = self.selection.focus_row(&snapshot);
        self.reveal_focus();
        cx.notify();
    }

    /// A click on a row: plain selects it alone, Ctrl toggles it, Shift
    /// selects the range from the anchor.
    fn click_row(&mut self, row: usize, modifiers: Modifiers, cx: &mut Context<Self>) {
        self.change_selection(
            |selection, snapshot| {
                if modifiers.shift {
                    selection.extend_to(snapshot, row);
                } else if modifiers.control {
                    selection.toggle(snapshot, row);
                } else {
                    selection.select_only(snapshot, row);
                }
            },
            cx,
        );
    }

    fn reveal_focus(&self) {
        if let Some(row) = self.focus_row {
            self.scroll.scroll_to_item(row, ScrollStrategy::Nearest);
        }
    }

    /// Moves the focus by keyboard; with `extend`, selects the range to it.
    fn move_selection(&mut self, motion: Motion, extend: bool, cx: &mut Context<Self>) {
        let len = self.snapshot.as_ref().map_or(0, |snapshot| snapshot.len());
        let Some(row) = target_row(self.focus_row, len, motion) else {
            return;
        };
        self.change_selection(
            |selection, snapshot| {
                if extend {
                    selection.extend_to(snapshot, row);
                } else {
                    selection.select_only(snapshot, row);
                }
            },
            cx,
        );
    }

    /// Jumps to the next entry whose name starts with what has been typed.
    fn type_ahead(&mut self, typed: &str, cx: &mut Context<Self>) {
        let now = Instant::now();
        if self
            .typed_at
            .is_none_or(|at| now.duration_since(at) > TYPE_AHEAD_PAUSE)
        {
            self.typed.clear();
        }
        self.typed.push_str(typed);
        self.typed_at = Some(now);

        let (prefix, advance) = type_ahead_query(&self.typed);
        let from = self.focus_row.map_or(0, |row| row + usize::from(advance));
        let found = self
            .snapshot
            .as_ref()
            .and_then(|snapshot| snapshot.find_prefix(prefix.as_bytes(), from));
        if let Some(row) = found {
            self.change_selection(
                |selection, snapshot| selection.select_only(snapshot, row),
                cx,
            );
        }
    }

    fn on_key_down(&mut self, event: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        let modifiers = event.keystroke.modifiers;
        if self.editing_location || modifiers.control || modifiers.alt || modifiers.platform {
            return;
        }
        if let Some(typed) = event
            .keystroke
            .key_char
            .as_deref()
            .filter(|text| !text.is_empty() && !text.chars().any(char::is_control))
        {
            self.type_ahead(typed, cx);
        }
    }

    fn activate_row(&mut self, row: usize, cx: &mut Context<Self>) {
        let Some(snapshot) = &self.snapshot else {
            return;
        };
        if row < snapshot.len() && snapshot.is_container(row) {
            self.open(snapshot.entry_path(row));
            cx.notify();
        }
    }

    fn go_up(&mut self, cx: &mut Context<Self>) {
        let Some(model) = &mut self.model else { return };
        let came_from = model.location().and_then(|path| {
            path.file_name()
                .map(|name| name.as_encoded_bytes().to_vec())
        });
        if let Some(generation) = model.up() {
            self.generation = generation;
            self.failure = None;
            self.select_on_arrival = came_from;
            cx.notify();
        }
    }

    fn navigate(
        &mut self,
        step: impl FnOnce(&mut DirectoryModel) -> Option<u64>,
        cx: &mut Context<Self>,
    ) {
        if let Some(generation) = self.model.as_mut().and_then(step) {
            self.generation = generation;
            self.failure = None;
            cx.notify();
        }
    }

    /// Reads the file-type databases on a background thread.
    fn load_file_types(&mut self, cx: &mut Context<Self>) {
        let loading = cx
            .background_executor()
            .spawn(async { Arc::new(Database::load()) });
        cx.spawn(async move |view, cx| {
            let database = loading.await;
            // The view may be gone by the time loading finishes.
            let _ = view.update(cx, |view, cx| {
                view.types = Some(database);
                cx.notify();
            });
        })
        .detach();
    }

    /// The icon and description for an entry, if already known. An unknown
    /// type is looked up in the background and the view redrawn when it arrives.
    fn type_info_for(
        &mut self,
        name: &[u8],
        is_dir: bool,
        cx: &mut Context<Self>,
    ) -> Option<TypeInfo> {
        let database = self.types.as_ref()?;
        let mime = database.mime_type(name, is_dir);
        if let Some(info) = self.type_info.get(mime) {
            return Some(info.clone());
        }
        if self.types_pending.insert(mime.to_owned()) {
            let mime = mime.to_owned();
            let database = Arc::clone(database);
            let lookup = cx.background_executor().spawn({
                let mime = mime.clone();
                async move { database.resolve(&mime, size::ICON_LOOKUP) }
            });
            cx.spawn(async move |view, cx| {
                let info = lookup.await;
                let _ = view.update(cx, |view, cx| {
                    view.types_pending.remove(&mime);
                    view.type_info.insert(mime, info);
                    cx.notify();
                });
            })
            .detach();
        }
        None
    }

    /// Orders by `key`; choosing the current key again reverses the direction.
    fn sort_by(&mut self, key: SortKey, cx: &mut Context<Self>) {
        let Some(model) = &mut self.model else { return };
        let options = model.options();
        let options = ViewOptions {
            sort: key,
            descending: options.sort == key && !options.descending,
            ..options
        };
        if let Some(generation) = model.set_options(options) {
            self.generation = generation;
            cx.notify();
        }
    }

    fn toggle_hidden(&mut self, cx: &mut Context<Self>) {
        let Some(model) = &mut self.model else { return };
        let options = model.options();
        let options = ViewOptions {
            show_hidden: !options.show_hidden,
            ..options
        };
        if let Some(generation) = model.set_options(options) {
            self.generation = generation;
            cx.notify();
        }
    }

    fn show_hidden(&self) -> bool {
        self.model
            .as_ref()
            .is_some_and(|model| model.options().show_hidden)
    }

    fn location(&self) -> Option<PathBuf> {
        self.model.as_ref().and_then(DirectoryModel::location)
    }

    fn edit_location(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let current = self
            .location()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default();
        self.editing_location = true;
        self.location_input.update(cx, |input, cx| {
            input.set_value(current, window, cx);
            input.focus(window, cx);
        });
        cx.notify();
    }

    fn stop_editing(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.editing_location {
            self.editing_location = false;
            self.focus.focus(window, cx);
            cx.notify();
        }
    }

    fn on_location_input(
        &mut self,
        input: &Entity<InputState>,
        event: &InputEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            InputEvent::PressEnter { .. } => {
                let typed = input.read(cx).value();
                if let Some(path) = expand_path(typed.trim(), self.location().as_deref()) {
                    self.open(path);
                }
                self.stop_editing(window, cx);
            }
            InputEvent::Blur => self.stop_editing(window, cx),
            InputEvent::Change | InputEvent::Focus => {}
        }
    }

    fn toolbar(&self, theme: Theme, cx: &mut Context<Self>) -> impl IntoElement {
        let can_back = self.model.as_ref().is_some_and(DirectoryModel::can_go_back);
        let can_forward = self
            .model
            .as_ref()
            .is_some_and(DirectoryModel::can_go_forward);
        let location = self.location();
        let can_up = location.as_deref().and_then(Path::parent).is_some();
        let show_hidden = self.show_hidden();

        div()
            .flex()
            .flex_none()
            .items_center()
            .gap(px(space::XS))
            .h(px(size::TOOLBAR))
            .px(px(space::SM))
            .bg(theme.chrome)
            .border_b_1()
            .border_color(theme.border)
            .child(
                Button::new("back")
                    .ghost()
                    .small()
                    .icon(IconName::ArrowLeft)
                    .tooltip("Back (Alt+Left)")
                    .disabled(!can_back)
                    .on_click(
                        cx.listener(|this, _, _, cx| this.navigate(DirectoryModel::back, cx)),
                    ),
            )
            .child(
                Button::new("forward")
                    .ghost()
                    .small()
                    .icon(IconName::ArrowRight)
                    .tooltip("Forward (Alt+Right)")
                    .disabled(!can_forward)
                    .on_click(
                        cx.listener(|this, _, _, cx| this.navigate(DirectoryModel::forward, cx)),
                    ),
            )
            .child(
                Button::new("up")
                    .ghost()
                    .small()
                    .icon(IconName::ArrowUp)
                    .tooltip("Up one folder (Alt+Up)")
                    .disabled(!can_up)
                    .on_click(cx.listener(|this, _, _, cx| this.go_up(cx))),
            )
            .child(div().flex_1().min_w_0().px(px(space::XS)).map(|bar| {
                if self.editing_location {
                    bar.child(Input::new(&self.location_input).small())
                } else {
                    bar.child(self.breadcrumbs(location.as_deref(), theme, cx))
                }
            }))
            .child(
                Button::new("hidden")
                    .ghost()
                    .small()
                    .icon(if show_hidden {
                        IconName::Eye
                    } else {
                        IconName::EyeOff
                    })
                    .selected(show_hidden)
                    .tooltip(if show_hidden {
                        "Hide hidden files (Ctrl+H)"
                    } else {
                        "Show hidden files (Ctrl+H)"
                    })
                    .on_click(cx.listener(|this, _, _, cx| this.toggle_hidden(cx))),
            )
    }

    /// The current location as clickable ancestors. Clicking the free space
    /// after them switches to the editable path field.
    fn breadcrumbs(
        &self,
        location: Option<&Path>,
        theme: Theme,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let ancestors: Vec<PathBuf> = location
            .map(|path| {
                let mut list: Vec<_> = path.ancestors().map(Path::to_path_buf).collect();
                list.reverse();
                list
            })
            .unwrap_or_default();
        let last = ancestors.len().saturating_sub(1);
        let first_shown = ancestors.len().saturating_sub(MAX_CRUMBS);

        div()
            .id("breadcrumbs")
            .flex()
            .items_center()
            .h(px(size::ROW))
            .overflow_hidden()
            .rounded(px(size::RADIUS))
            .cursor_text()
            .on_click(cx.listener(|this, _, window, cx| this.edit_location(window, cx)))
            .when(first_shown > 0, |crumbs| {
                crumbs.child(
                    div()
                        .px(px(space::XS))
                        .text_color(theme.text_muted)
                        .child("\u{2026}"),
                )
            })
            .children(
                ancestors
                    .into_iter()
                    .enumerate()
                    .skip(first_shown)
                    .map(|(index, path)| {
                        let label: SharedString = match path.file_name() {
                            Some(name) => display_name(name.as_encoded_bytes()).into_owned().into(),
                            None => "/".into(),
                        };
                        let current = index == last;
                        div()
                            .flex()
                            .flex_none()
                            .items_center()
                            .when(index > 1 || (index == 1 && first_shown > 0), |crumb| {
                                crumb.child(div().text_color(theme.text_muted).child("/"))
                            })
                            .child(
                                div()
                                    .id(("crumb", index))
                                    .px(px(space::XS))
                                    .rounded(px(size::RADIUS))
                                    .cursor_pointer()
                                    .text_color(if current {
                                        theme.text
                                    } else {
                                        theme.text_muted
                                    })
                                    .hover(|crumb| crumb.bg(theme.hover).text_color(theme.text))
                                    .child(label)
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        cx.stop_propagation();
                                        if !current {
                                            this.open(path.clone());
                                            cx.notify();
                                        }
                                    })),
                            )
                    }),
            )
    }

    fn failure_banner(
        &self,
        failure: &Failure,
        theme: Theme,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div()
            .flex()
            .flex_none()
            .items_center()
            .gap(px(space::SM))
            .mx(px(space::SM))
            .mt(px(space::SM))
            .px(px(space::MD))
            .py(px(space::SM))
            .rounded(px(size::RADIUS))
            .bg(theme.danger_surface)
            .border_1()
            .border_color(theme.danger)
            .text_color(theme.danger)
            .child(
                Icon::new(IconName::TriangleAlert)
                    .size(px(size::ICON))
                    .flex_none(),
            )
            .child(div().flex_1().min_w_0().child(format!(
                "Could not open {}: {}.",
                display_name(failure.path.as_os_str().as_encoded_bytes()),
                failure.error
            )))
            .child(
                Button::new("dismiss-failure")
                    .ghost()
                    .xsmall()
                    .label("Dismiss")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.failure = None;
                        cx.notify();
                    })),
            )
    }

    /// One clickable column heading. The active one shows its direction with
    /// an arrow as well as brighter text.
    fn heading(
        &self,
        label: &'static str,
        key: SortKey,
        options: ViewOptions,
        theme: Theme,
        cx: &mut Context<Self>,
    ) -> gpui_kit::Stateful<gpui_kit::Div> {
        let active = options.sort == key;
        div()
            .id(label)
            .flex()
            .items_center()
            .gap(px(space::XS))
            .h_full()
            .cursor_pointer()
            .text_color(if active { theme.text } else { theme.text_muted })
            .hover(|heading| heading.text_color(theme.text))
            .child(label)
            .when(active, |heading| {
                heading.child(
                    Icon::new(if options.descending {
                        IconName::ChevronDown
                    } else {
                        IconName::ChevronUp
                    })
                    .size(px(size::ICON_SMALL)),
                )
            })
            .on_click(cx.listener(move |this, _, _, cx| this.sort_by(key, cx)))
    }

    /// The details view: column headings over a virtualised list of entries.
    fn list(
        &self,
        snapshot: &Arc<Snapshot>,
        columns: Columns,
        theme: Theme,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let rows = Arc::clone(snapshot);
        let options = snapshot.options();
        let zone = self.zone.clone();
        // Headings line up with the row text: same gutter, accent bar and padding.
        let gutter = space::SM * 2. + size::SELECTION_BAR;

        let headings = div()
            .flex()
            .flex_none()
            .items_center()
            .gap(px(space::MD))
            .h(px(size::ROW))
            .px(px(gutter))
            .border_b_1()
            .border_color(theme.border)
            .text_size(px(text::SMALL))
            .child(
                self.heading("Name", SortKey::Name, options, theme, cx)
                    .flex_1()
                    .min_w_0(),
            )
            .child(
                self.heading("Size", SortKey::Size, options, theme, cx)
                    .w(px(size::COLUMN_SIZE))
                    .justify_end(),
            )
            .when(columns.modified, |headings| {
                headings.child(
                    self.heading("Modified", SortKey::Modified, options, theme, cx)
                        .w(px(size::COLUMN_MODIFIED)),
                )
            })
            .when(columns.kind, |headings| {
                headings.child(
                    self.heading("Type", SortKey::Type, options, theme, cx)
                        .w(px(size::COLUMN_TYPE)),
                )
            });

        let list = uniform_list(
            "entries",
            snapshot.len(),
            cx.processor(move |this, range: std::ops::Range<usize>, _, cx| {
                range
                    .map(|row| {
                        let is_selected = this.selection.contains(&rows, row);
                        let is_focus = this.focus_row == Some(row);
                        let container = rows.is_container(row);
                        let name: SharedString = display_name(rows.name(row)).into_owned().into();
                        let info = this.type_info_for(rows.name(row), container, cx);
                        let metadata = rows.metadata(row);
                        let secondary = if is_selected {
                            theme.on_selection
                        } else {
                            theme.text_muted
                        };
                        // Folders show no size: a folder's own size says nothing useful.
                        let size_text = metadata
                            .filter(|_| !container)
                            .map(|metadata| format::size(metadata.size))
                            .unwrap_or_default();
                        let modified_text = metadata
                            .map(|metadata| format::timestamp(metadata.modified, &zone))
                            .unwrap_or_default();
                        let description = info
                            .as_ref()
                            .map(|info| info.description.clone())
                            .unwrap_or_default();
                        let icon = match info.and_then(|info| info.icon) {
                            Some(path) => img(path)
                                .size(px(size::ICON_ROW))
                                .flex_none()
                                .into_any_element(),
                            // Until the theme icon is known, or if the theme has none.
                            None => Icon::new(if container {
                                IconName::Folder
                            } else {
                                IconName::File
                            })
                            .size(px(size::ICON_ROW))
                            .flex_none()
                            .text_color(if container { theme.accent } else { secondary })
                            .into_any_element(),
                        };
                        let entry = div()
                            .flex()
                            .items_center()
                            .gap(px(space::MD))
                            .size_full()
                            .px(px(space::SM))
                            .rounded(px(size::RADIUS))
                            .border_l(px(size::SELECTION_BAR))
                            .border_color(gpui_kit::transparent_black())
                            .text_color(theme.text)
                            .when(is_selected, |entry| {
                                // Selection is a fill plus an accent bar, never colour alone.
                                entry
                                    .bg(theme.selection)
                                    .border_color(theme.accent)
                                    .text_color(theme.on_selection)
                            })
                            .when(!is_selected, |entry| {
                                // The keyboard focus without selection keeps the bar only.
                                entry
                                    .when(is_focus, |entry| entry.border_color(theme.accent))
                                    .hover(|entry| entry.bg(theme.hover))
                            })
                            .child(
                                div()
                                    .flex()
                                    .flex_1()
                                    .min_w_0()
                                    .items_center()
                                    .gap(px(space::SM))
                                    .child(icon)
                                    .child(div().flex_1().min_w_0().truncate().child(name)),
                            )
                            .child(
                                div()
                                    .w(px(size::COLUMN_SIZE))
                                    .flex_none()
                                    .text_right()
                                    .text_color(secondary)
                                    .child(size_text),
                            )
                            .when(columns.modified, |entry| {
                                entry.child(
                                    div()
                                        .w(px(size::COLUMN_MODIFIED))
                                        .flex_none()
                                        .text_color(secondary)
                                        .child(modified_text),
                                )
                            })
                            .when(columns.kind, |entry| {
                                entry.child(
                                    div()
                                        .w(px(size::COLUMN_TYPE))
                                        .flex_none()
                                        .truncate()
                                        .text_color(secondary)
                                        .child(description),
                                )
                            });
                        div()
                            .id(row)
                            .w_full()
                            .h(px(size::ROW))
                            .px(px(space::SM))
                            .child(entry)
                            .on_click(cx.listener(move |this, event: &ClickEvent, _, cx| {
                                let modifiers = event.modifiers();
                                this.click_row(row, modifiers, cx);
                                let plain = !modifiers.shift && !modifiers.control;
                                if plain && event.click_count() >= 2 {
                                    this.activate_row(row, cx);
                                }
                            }))
                    })
                    .collect::<Vec<_>>()
            }),
        )
        .track_scroll(&self.scroll)
        .flex_1()
        .py(px(space::XS));

        div()
            .flex()
            .flex_1()
            .flex_col()
            .min_h_0()
            .child(headings)
            .child(list)
    }

    /// Shown instead of the list when it has no rows: says why and what to do.
    fn empty_state(
        &self,
        snapshot: &Snapshot,
        theme: Theme,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let hidden = snapshot.total();
        let (icon, heading) = if snapshot.is_loading() {
            (IconName::Loader, "Loading\u{2026}")
        } else {
            (
                IconName::FolderOpen,
                if hidden > 0 {
                    "This folder has only hidden items"
                } else {
                    "This folder is empty"
                },
            )
        };
        div()
            .flex()
            .flex_1()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(space::MD))
            .text_color(theme.text_muted)
            .child(Icon::new(icon).size(px(size::ICON_LARGE)))
            .child(
                div()
                    .text_size(px(text::HEADING))
                    .text_color(theme.text)
                    .child(heading),
            )
            .when(hidden > 0 && !snapshot.is_loading(), |state| {
                state
                    .child(format!(
                        "{} hidden {} not shown.",
                        count(hidden),
                        plural(hidden, "item is", "items are")
                    ))
                    .child(
                        Button::new("show-hidden")
                            .primary()
                            .small()
                            .label("Show hidden files")
                            .on_click(cx.listener(|this, _, _, cx| this.toggle_hidden(cx))),
                    )
            })
    }

    fn status(&self, theme: Theme) -> impl IntoElement {
        let message = match &self.snapshot {
            None => "Loading\u{2026}".to_owned(),
            Some(snapshot) => {
                let shown = snapshot.len();
                let hidden = snapshot.total() - shown;
                let selected = self.selection.len();
                let mut message = if selected > 0 {
                    format!(
                        "{} of {} {} selected",
                        count(selected),
                        count(shown),
                        plural(shown, "item", "items")
                    )
                } else {
                    format!("{} {}", count(shown), plural(shown, "item", "items"))
                };
                if hidden > 0 {
                    message.push_str(&format!(", {} hidden", count(hidden)));
                }
                if snapshot.is_loading() {
                    message.push_str(" \u{2014} loading\u{2026}");
                }
                message
            }
        };
        div()
            .flex()
            .flex_none()
            .items_center()
            .h(px(size::STATUS))
            .px(px(space::LG))
            .bg(theme.chrome)
            .border_t_1()
            .border_color(theme.border)
            .text_size(px(text::SMALL))
            .text_color(theme.text_muted)
            .child(message)
    }
}

impl Render for Browser {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *cx.global::<Theme>();
        let snapshot = self.snapshot.clone();
        let columns = Columns::for_width(f32::from(window.viewport_size().width));

        div()
            .key_context(KEY_CONTEXT)
            .track_focus(&self.focus)
            .on_action(cx.listener(|this, _: &SelectPrevious, _, cx| {
                this.move_selection(Motion::By(-1), false, cx);
            }))
            .on_action(cx.listener(|this, _: &SelectNext, _, cx| {
                this.move_selection(Motion::By(1), false, cx);
            }))
            .on_action(cx.listener(|this, _: &SelectFirst, _, cx| {
                this.move_selection(Motion::First, false, cx);
            }))
            .on_action(cx.listener(|this, _: &SelectLast, _, cx| {
                this.move_selection(Motion::Last, false, cx);
            }))
            .on_action(cx.listener(|this, _: &PageUp, _, cx| {
                this.move_selection(Motion::By(-PAGE_ROWS), false, cx);
            }))
            .on_action(cx.listener(|this, _: &PageDown, _, cx| {
                this.move_selection(Motion::By(PAGE_ROWS), false, cx);
            }))
            .on_action(cx.listener(|this, _: &ExtendPrevious, _, cx| {
                this.move_selection(Motion::By(-1), true, cx);
            }))
            .on_action(cx.listener(|this, _: &ExtendNext, _, cx| {
                this.move_selection(Motion::By(1), true, cx);
            }))
            .on_action(cx.listener(|this, _: &ExtendFirst, _, cx| {
                this.move_selection(Motion::First, true, cx);
            }))
            .on_action(cx.listener(|this, _: &ExtendLast, _, cx| {
                this.move_selection(Motion::Last, true, cx);
            }))
            .on_action(cx.listener(|this, _: &ExtendPageUp, _, cx| {
                this.move_selection(Motion::By(-PAGE_ROWS), true, cx);
            }))
            .on_action(cx.listener(|this, _: &ExtendPageDown, _, cx| {
                this.move_selection(Motion::By(PAGE_ROWS), true, cx);
            }))
            .on_action(cx.listener(|this, _: &SelectAll, _, cx| {
                this.change_selection(Selection::select_all, cx);
            }))
            .on_action(cx.listener(|this, _: &InvertSelection, _, cx| {
                this.change_selection(Selection::invert, cx);
            }))
            .on_action(cx.listener(|this, _: &Activate, _, cx| {
                if let Some(row) = this.focus_row {
                    this.activate_row(row, cx);
                }
            }))
            .on_key_down(cx.listener(Self::on_key_down))
            .on_action(cx.listener(|this, _: &GoUp, _, cx| this.go_up(cx)))
            .on_action(
                cx.listener(|this, _: &GoBack, _, cx| this.navigate(DirectoryModel::back, cx)),
            )
            .on_action(
                cx.listener(|this, _: &GoForward, _, cx| {
                    this.navigate(DirectoryModel::forward, cx)
                }),
            )
            .on_action(
                cx.listener(|this, _: &Reload, _, cx| this.navigate(DirectoryModel::reload, cx)),
            )
            .on_action(cx.listener(|this, _: &ToggleHidden, _, cx| this.toggle_hidden(cx)))
            .on_action(
                cx.listener(|this, _: &EditLocation, window, cx| this.edit_location(window, cx)),
            )
            .on_action(cx.listener(|this, _: &CancelEdit, window, cx| {
                if this.editing_location {
                    this.stop_editing(window, cx);
                } else {
                    this.change_selection(|selection, _| selection.clear(), cx);
                }
            }))
            .flex()
            .flex_col()
            .size_full()
            .bg(theme.window)
            .text_color(theme.text)
            .text_size(px(text::BODY))
            .line_height(px(text::LINE))
            .child(self.toolbar(theme, cx))
            .when_some(self.failure.as_ref(), |view, failure| {
                view.child(self.failure_banner(failure, theme, cx))
            })
            .child(
                // The file list sits on a more solid surface than the glass
                // toolbar and status line, so columns of text stay readable.
                div()
                    .flex()
                    .flex_1()
                    .flex_col()
                    .min_h_0()
                    .bg(theme.content)
                    .map(|content| match &snapshot {
                        Some(snapshot) if !snapshot.is_empty() => {
                            content.child(self.list(snapshot, columns, theme, cx))
                        }
                        Some(snapshot) => content.child(self.empty_state(snapshot, theme, cx)),
                        None => content,
                    }),
            )
            .child(self.status(theme))
    }
}

/// What a type-ahead buffer searches for: the prefix, and whether to start
/// after the focused row. Pressing one letter repeatedly cycles through the
/// names starting with it; anything else narrows the search in place.
fn type_ahead_query(typed: &str) -> (&str, bool) {
    let mut chars = typed.chars();
    match chars.next() {
        Some(first) if chars.all(|c| c == first) => (&typed[..first.len_utf8()], true),
        _ => (typed, false),
    }
}

/// A keyboard movement of the selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Motion {
    /// Up (negative) or down (positive) by a number of rows.
    By(isize),
    First,
    Last,
}

/// The row a motion lands on in a list of `len` rows, or `None` for an empty
/// list. With nothing selected, moving down starts at the top and moving up at
/// the bottom.
fn target_row(current: Option<usize>, len: usize, motion: Motion) -> Option<usize> {
    let last = len.checked_sub(1)?;
    Some(match (motion, current) {
        (Motion::First, _) => 0,
        (Motion::Last, _) => last,
        (Motion::By(delta), Some(row)) => row.saturating_add_signed(delta).min(last),
        (Motion::By(delta), None) if delta >= 0 => 0,
        (Motion::By(_), None) => last,
    })
}

/// Turns typed text into an absolute path: expands a leading `~` and resolves
/// relative paths against `base`. Returns `None` for empty input.
fn expand_path(typed: &str, base: Option<&Path>) -> Option<PathBuf> {
    if typed.is_empty() {
        return None;
    }
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let path = match (typed.strip_prefix('~'), home) {
        (Some(""), Some(home)) => home,
        (Some(rest), Some(home)) if rest.starts_with('/') => {
            home.join(rest.trim_start_matches('/'))
        }
        _ => PathBuf::from(typed),
    };
    Some(match base {
        Some(base) if path.is_relative() => base.join(path),
        _ => path,
    })
}

/// Which optional columns fit the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Columns {
    modified: bool,
    kind: bool,
}

impl Columns {
    /// Narrow windows drop the type column first, then the date, so the name
    /// always keeps a usable width.
    fn for_width(width: f32) -> Self {
        Self {
            modified: width >= size::WIDTH_FOR_MODIFIED,
            kind: width >= size::WIDTH_FOR_TYPE,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeating_one_letter_cycles_and_a_longer_prefix_narrows() {
        assert_eq!(type_ahead_query("f"), ("f", true));
        assert_eq!(type_ahead_query("fff"), ("f", true));
        assert_eq!(type_ahead_query("fi"), ("fi", false));
        assert_eq!(type_ahead_query("\u{e9}\u{e9}"), ("\u{e9}", true));
        assert_eq!(type_ahead_query(""), ("", false));
    }

    #[test]
    fn end_selects_the_last_row_even_with_nothing_selected() {
        assert_eq!(target_row(None, 500_000, Motion::Last), Some(499_999));
        assert_eq!(target_row(Some(3), 10, Motion::Last), Some(9));
    }

    #[test]
    fn home_selects_the_first_row() {
        assert_eq!(target_row(Some(7), 10, Motion::First), Some(0));
        assert_eq!(target_row(None, 10, Motion::First), Some(0));
    }

    #[test]
    fn stepping_is_clamped_to_the_list() {
        assert_eq!(target_row(Some(0), 10, Motion::By(-1)), Some(0));
        assert_eq!(target_row(Some(9), 10, Motion::By(1)), Some(9));
        assert_eq!(target_row(Some(4), 10, Motion::By(15)), Some(9));
        assert_eq!(target_row(Some(4), 10, Motion::By(-15)), Some(0));
        assert_eq!(target_row(Some(4), 10, Motion::By(2)), Some(6));
    }

    #[test]
    fn first_step_without_a_selection_enters_from_the_matching_edge() {
        assert_eq!(target_row(None, 10, Motion::By(1)), Some(0));
        assert_eq!(target_row(None, 10, Motion::By(-1)), Some(9));
    }

    #[test]
    fn an_empty_list_has_no_target() {
        assert_eq!(target_row(None, 0, Motion::Last), None);
        assert_eq!(target_row(None, 0, Motion::By(1)), None);
    }

    #[test]
    fn narrow_windows_drop_the_type_column_then_the_date() {
        let wide = Columns::for_width(1200.);
        assert!(wide.modified && wide.kind);
        let medium = Columns::for_width(size::WIDTH_FOR_TYPE - 1.);
        assert!(medium.modified && !medium.kind);
        let narrow = Columns::for_width(size::WIDTH_FOR_MODIFIED - 1.);
        assert!(!narrow.modified && !narrow.kind);
    }

    #[test]
    fn typed_paths_are_made_absolute() {
        let base = Path::new("/srv/data");
        assert_eq!(expand_path("", Some(base)), None);
        assert_eq!(expand_path("/etc", Some(base)), Some(PathBuf::from("/etc")));
        assert_eq!(
            expand_path("sub/dir", Some(base)),
            Some(PathBuf::from("/srv/data/sub/dir"))
        );
        assert_eq!(
            expand_path("~weird", Some(base)),
            Some(PathBuf::from("/srv/data/~weird"))
        );
    }
}

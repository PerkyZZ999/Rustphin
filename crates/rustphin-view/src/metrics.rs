//! The numeric design tokens of docs/DESIGN.md: spacing, sizes and the type
//! scale, in pixels. Colours stay with each toolkit's theme; these numbers
//! are shared so layout logic is computed the same way in every interface.

/// Spacing scale in pixels (4-pixel grid).
pub mod space {
    /// Hairline gaps inside a control.
    pub const XS: f32 = 4.;
    /// Gap between an icon and its label; padding inside rows.
    pub const SM: f32 = 8.;
    /// Padding inside bars and banners.
    pub const MD: f32 = 12.;
    /// Padding around the content area.
    pub const LG: f32 = 16.;
}

/// Fixed sizes in pixels.
pub mod size {
    /// Height of the column headings. Entry rows take their height from the
    /// view layout.
    pub const ROW: f32 = 28.;
    /// The gap between grid cells, split as half a gap around each cell.
    pub const CELL_GAP: f32 = 8.;
    /// Height of the rename field inside a row.
    pub const RENAME_FIELD: f32 = 28.;
    /// The drag chip.
    pub const DRAG_CHIP: f32 = 28.;
    /// Height of the toolbar across the window (`toolbar`).
    pub const TOOLBAR: f32 = 44.;
    /// Toolbar buttons, the location field and Actions: 36 pixels square or
    /// tall.
    pub const CONTROL: f32 = 36.;
    /// Height of the status line.
    pub const STATUS: f32 = 28.;
    /// Narrowest pane whose status line still shows the free space.
    pub const STATUS_FREE_SPACE_MIN: f32 = 600.;
    /// Below this pane width, a status line with a selection shows only it.
    pub const STATUS_SELECTION_ONLY_BELOW: f32 = 440.;
    /// Height of the filter bar above the status line.
    pub const FILTER_BAR: f32 = 40.;
    /// Height of the field in the filter bar, leaving room for its ring.
    pub const FILTER_FIELD: f32 = 32.;
    /// Widest the filter field grows.
    pub const FILTER_FIELD_MAX: f32 = 360.;
    /// Height of the tab strip.
    pub const TABS: f32 = 36.;
    /// Width of a tab.
    pub const TAB: f32 = 160.;
    /// The narrowest a tab gets when many share the strip.
    pub const TAB_MIN: f32 = 96.;
    /// Below this width only the active tab shows its close button.
    pub const TAB_CLOSE_SHOWN: f32 = 128.;
    /// A tab's close button.
    pub const TAB_CLOSE: f32 = 24.;
    /// Width of the new-tab button after the last tab.
    pub const TAB_ADD: f32 = 36.;
    /// Icons in toolbars and banners.
    pub const ICON: f32 = 16.;
    /// Sort arrows and other inline indicators.
    pub const ICON_SMALL: f32 = 12.;
    /// Width of the accent bar marking a selected row.
    pub const SELECTION_BAR: f32 = 2.;
    /// Width of the size column.
    pub const COLUMN_SIZE: f32 = 96.;
    /// Minimum pane width that leaves room for the size column beside names.
    pub const WIDTH_FOR_SIZE: f32 = 360.;
    /// Width of the modified column.
    pub const COLUMN_MODIFIED: f32 = 168.;
    /// Width of the type column.
    pub const COLUMN_TYPE: f32 = 160.;
    /// Narrowest pane that still shows the modified column.
    pub const WIDTH_FOR_MODIFIED: f32 = 600.;
    /// Narrowest pane that still shows the type column.
    pub const WIDTH_FOR_TYPE: f32 = 820.;
    /// Icons in empty and error states.
    pub const ICON_LARGE: f32 = 32.;
    /// Height of a drive card's row in the places panel: the card and the
    /// 4 px gap below it (W-148).
    pub const DEVICE_ROW: f32 = 60.;
    /// Height of a drive card (`device-card`): 8 px padding, the name line
    /// (16), 4, the usage bar (4), the last line (16), 8 px padding.
    pub const DEVICE_CARD: f32 = 56.;
    /// Height of a place row and of a section heading in the places panel.
    pub const PLACE_ROW: f32 = 28.;
    /// Height of a separator row in the places panel (W-152): a hairline
    /// in the middle of 12 px, tall enough to point at and drag.
    pub const PLACE_SEPARATOR: f32 = 12.;
    /// Place and device icons in the places panel.
    pub const PLACE_ICON: f32 = 16.;
    /// Small icon buttons: New group, close in a bar.
    pub const SMALL_BUTTON: f32 = 28.;
    /// Widest line of text under a state's heading.
    pub const STATE_TEXT_MAX: f32 = 440.;
    /// A checkbox and its label, as one row (`checkbox-row`).
    pub const CHECK_ROW: f32 = 28.;
    /// Height of a usage bar under a volume's name.
    pub const USAGE: f32 = 4.;
    /// Diameter of the ring of used and free space in a drive's Properties
    /// (W-153).
    pub const DRIVE_RING: f32 = 112.;
    /// Width of that ring's stroke.
    pub const DRIVE_RING_STROKE: f32 = 12.;
    /// Side of a colour swatch in the ring's legend.
    pub const SWATCH: f32 = 12.;
    /// Width of the places panel, by default.
    pub const SIDEBAR: f32 = 240.;
    /// The narrowest the location field is drawn before toolbar items
    /// leave (W-106f).
    pub const TOOLBAR_LOCATION_MIN: f32 = 280.;
    /// The narrowest the places panel can be made (Settings, Places).
    pub const SIDEBAR_MIN: f32 = 200.;
    /// The widest the places panel can be made.
    pub const SIDEBAR_MAX: f32 = 360.;
    /// The strip at the places panel's right edge that drags its width.
    pub const SIDEBAR_GRIP: f32 = 4.;
    /// Width of a context menu and of its application list.
    pub const MENU: f32 = 240.;
    /// Properties' thumbnail: a 3:2 picture fitted in 96 × 64 pixels.
    pub const PROPERTIES_THUMB_WIDTH: f32 = 96.;
    /// Height of Properties' thumbnail.
    pub const PROPERTIES_THUMB_HEIGHT: f32 = 64.;
    /// The type icon in Properties' header when there is no thumbnail.
    pub const ICON_HEADER: f32 = 48.;
    /// Width of the label column of Properties' facts.
    pub const PROPERTIES_LABEL: f32 = 128.;
    /// Width of the label column beside a dialog fact.
    pub const FACT_LABEL: f32 = 72.;
    /// A hairline divider, such as a menu separator.
    pub const HAIRLINE: f32 = 1.;
    /// Height of a menu item.
    pub const MENU_ROW: f32 = 32.;
    /// Tallest a context menu grows before it scrolls: twelve rows.
    pub const MENU_VISIBLE: f32 = 384.;
    /// Width of a dialog.
    pub const DIALOG: f32 = 480.;
    /// Width of the Compress to… dialog.
    pub const COMPRESS_DIALOG: f32 = 560.;
    /// Width of the batch-rename dialog, which has a two-column preview.
    pub const RENAME_DIALOG: f32 = 640.;
    /// Maximum width of the command palette.
    pub const PALETTE: f32 = 640.;
    /// Height of a command result, including its second line.
    pub const PALETTE_ROW: f32 = 52.;
    /// Height of the palette's search field.
    pub const PALETTE_FIELD: f32 = 40.;
    /// Height of a progress bar.
    pub const PROGRESS: f32 = 6.;
    /// Narrowest a progress bar gets in the activity line.
    pub const PROGRESS_MIN: f32 = 80.;
    /// Height of a transfer card's speed graph.
    pub const SPEED_GRAPH: f32 = 48.;
    /// Height of an activity or result line under the panes.
    pub const LINE: f32 = 44.;
    /// Height of a text button in lines and dialogs.
    pub const BUTTON: f32 = 32.;
    /// Height of a dialog's title row.
    pub const DIALOG_TITLE: f32 = 48.;
    /// The Settings dialog's width at its largest (W-106, W-155).
    pub const SETTINGS_WIDTH: f32 = 800.;
    /// The Settings dialog's height at its largest.
    pub const SETTINGS_HEIGHT: f32 = 576.;
    /// Its page list on the left.
    pub const SETTINGS_NAV: f32 = 216.;
    /// The search field in its title row.
    pub const SETTINGS_SEARCH: f32 = 240.;
    /// Below this window width the page list becomes a row of tabs.
    pub const SETTINGS_NARROW: f32 = 720.;
    /// A row of the page list, and of the lists on the Toolbar page.
    pub const SETTING_ROW: f32 = 32.;
    /// The least height of a setting in a section's panel.
    pub const SETTING_LINE: f32 = 48.;
    /// A control in a setting's row: segments, drop-downs, chips, small
    /// buttons.
    pub const SETTING_CONTROL: f32 = 28.;
    /// The width of a drop-down in Settings.
    pub const SETTING_SELECT: f32 = 216.;
    /// A key cap on the Shortcuts page.
    pub const KEY_CAP: f32 = 20.;
    /// The widest a text field in Settings is drawn.
    pub const SETTING_FIELD: f32 = 360.;
    /// A switch's track: its width.
    pub const SWITCH_WIDTH: f32 = 36.;
    /// A switch's track: its height.
    pub const SWITCH_HEIGHT: f32 = 20.;
    /// A slider's track in Settings.
    pub const SLIDER: f32 = 160.;
    /// The box beside a slider that shows its value.
    pub const SLIDER_VALUE: f32 = 64.;
    /// The drawing of a window in each look on the Appearance page.
    pub const LOOK_TILE: f32 = 64.;
    /// The height of the lists of the Toolbar page.
    pub const SETTINGS_LIST: f32 = 224.;
    /// The logo on the About page.
    pub const ABOUT_LOGO: f32 = 64.;
    /// Corner radius shared by controls, rows and panels.
    pub const RADIUS: f32 = 2.;
    /// Height of the window's own title bar (`title-bar`).
    pub const TITLE_BAR: f32 = 36.;
    /// Width of a Minimise, Maximise or Close button (`window-button`).
    pub const WINDOW_BUTTON: f32 = 44.;
    /// Thickness of the bands along the window's edges that resize it.
    pub const RESIZE_EDGE: f32 = 4.;
    /// The icon of a palette result (`palette-result`).
    pub const PALETTE_ICON: f32 = 20.;
    /// Height of the blur status line (`blur-status`).
    pub const BLUR_STATUS: f32 = 32.;
    /// Height of the hairlines between the toolbar's groups.
    pub const TOOL_SEPARATOR: f32 = 20.;
    /// The bar under the toolbar while acting as administrator.
    pub const ADMIN_BAR: f32 = 36.;
    /// A split pane's path bar (`path-bar`).
    pub const PATH_BAR: f32 = 28.;
    /// A plain toolbar button and the view-mode segments (`toolbar-button`).
    pub const TOOL_BUTTON: f32 = 32.;
    /// A toolbar button that opens a menu: its icon and a chevron.
    pub const TOOL_MENU_BUTTON: f32 = 48.;
    /// Thickness of a scrollbar's thumb.
    pub const SCROLLBAR: f32 = 8.;
    /// Gap between a scrollbar's thumb and the edge it runs along.
    pub const SCROLLBAR_INSET: f32 = 2.;
}

/// Type scale in pixels (D-026).
pub mod text {
    /// Interface and file-name text (`body-md`, `body-sm`).
    pub const FAMILY_BODY: &str = "Inter";
    /// Sizes, dates, paths and counts (`data-md`, `data-sm`).
    pub const FAMILY_DATA: &str = "JetBrains Mono";
    /// Headings in empty and error states (`headline-sm`).
    pub const FAMILY_HEADING: &str = "Space Grotesk";
    /// Data cells (`data-md`).
    pub const DATA: f32 = 11.;
    /// Secondary information: status line, captions.
    pub const SMALL: f32 = 11.;
    /// Body text and list rows.
    pub const BODY: f32 = 12.;
    /// Short capital labels such as OWNER (`label-sm`).
    pub const LABEL: f32 = 10.;
    /// File and place names (`body-lg`).
    pub const NAME: f32 = 12.;
    /// Headings of empty, loading and error states (`headline-md`).
    pub const TITLE: f32 = 15.;
    /// Panel and dialog titles (`headline-sm`).
    pub const HEADING: f32 = 15.;
    /// Line height of `headline-sm`.
    pub const HEADING_LINE: f32 = 20.;
    /// Line height of `headline-md`.
    pub const TITLE_LINE: f32 = 20.;
    /// Line height for body text.
    pub const LINE: f32 = 16.;
}

/// The faintest hidden entries may be drawn when chosen in Settings,
/// Appearance (W-106d): the lowest opacity, in whole percent, at which
/// their names still reach WCAG AA in both themes (tested below).
pub const HIDDEN_OPACITY_MIN_PERCENT: u32 = 60;

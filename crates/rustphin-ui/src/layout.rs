//! Geometry of the three view modes at each zoom level.

/// How entries are laid out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ViewMode {
    /// A grid of large icons with the name underneath.
    Icons,
    /// A dense grid of small icons with the name beside them.
    Compact,
    /// One entry per row with size, date and type columns.
    #[default]
    Details,
}

/// Number of zoom levels; valid levels are `0..ZOOM_LEVELS`.
pub const ZOOM_LEVELS: usize = 5;
/// The zoom level a new window starts at: 24-pixel icons and 36-pixel rows
/// in the list views, 96-pixel icons in the icons view.
pub const DEFAULT_ZOOM: usize = 2;

/// Icon sizes of the two list-like modes, by zoom level.
const SMALL_ICONS: [f32; ZOOM_LEVELS] = [16., 20., 24., 32., 48.];
/// Icon sizes of the icons mode, by zoom level.
const LARGE_ICONS: [f32; ZOOM_LEVELS] = [48., 64., 96., 128., 192.];
/// Column pitch of the compact mode, by zoom level: the column plus the
/// 8-pixel gap after it (240-pixel columns at the default level).
const COMPACT_WIDTHS: [f32; ZOOM_LEVELS] = [232., 240., 248., 264., 296.];
/// Width a large icon's cell adds to the icon, gap included: 128-pixel
/// cells 8 pixels apart at the 96-pixel level.
const ICON_CELL_EXTRA_WIDTH: f32 = 40.;
/// Height a large icon's cell adds for padding, two lines of name and the
/// gap: 152-pixel cells 8 pixels apart at the 96-pixel level.
const ICON_CELL_EXTRA_HEIGHT: f32 = 64.;
/// Rows are at least this tall, and otherwise the icon plus this padding:
/// 28, 32, 36, 44 and 60 pixels.
const MIN_ROW: f32 = 28.;
const ROW_PADDING: f32 = 12.;

/// Sizes, in pixels, for drawing entries in one mode at one zoom level.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Layout {
    /// Edge length of the file-type icon.
    pub icon: f32,
    /// Height of one row of the list or grid.
    pub row_height: f32,
    /// Width of one grid cell; `None` when each entry spans the full width.
    pub cell_width: Option<f32>,
}

impl ViewMode {
    /// The sizes for this mode at `zoom`, which is clamped to the valid levels.
    pub fn layout(self, zoom: usize) -> Layout {
        let zoom = zoom.min(ZOOM_LEVELS - 1);
        let small_row = (SMALL_ICONS[zoom] + ROW_PADDING).max(MIN_ROW);
        match self {
            Self::Details => Layout {
                icon: SMALL_ICONS[zoom],
                row_height: small_row,
                cell_width: None,
            },
            Self::Compact => Layout {
                icon: SMALL_ICONS[zoom],
                row_height: small_row,
                cell_width: Some(COMPACT_WIDTHS[zoom]),
            },
            Self::Icons => Layout {
                icon: LARGE_ICONS[zoom],
                row_height: LARGE_ICONS[zoom] + ICON_CELL_EXTRA_HEIGHT,
                cell_width: Some(LARGE_ICONS[zoom] + ICON_CELL_EXTRA_WIDTH),
            },
        }
    }
}

impl Layout {
    /// How many entries fit side by side in `width` pixels; at least one.
    pub fn columns(&self, width: f32) -> usize {
        match self.cell_width {
            Some(cell) => ((width / cell).floor() as usize).max(1),
            None => 1,
        }
    }

    /// How many rows fit one above the other in `height` pixels; at least one.
    pub fn rows(&self, height: f32) -> usize {
        ((height / self.row_height).floor() as usize).max(1)
    }
}

/// A rectangle in list coordinates: pixels from the left edge of the first
/// column and the top of the first row, growing right and down.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    /// Left edge.
    pub left: f32,
    /// Top edge.
    pub top: f32,
    /// Right edge.
    pub right: f32,
    /// Bottom edge.
    pub bottom: f32,
}

impl Rect {
    /// The rectangle spanning two opposite corners given in any order.
    pub fn between(a: (f32, f32), b: (f32, f32)) -> Self {
        Self {
            left: a.0.min(b.0),
            top: a.1.min(b.1),
            right: a.0.max(b.0),
            bottom: a.1.max(b.1),
        }
    }
}

/// The order in which entries fill the view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flow {
    /// Along each line from left to right, then down to the next line; the
    /// view scrolls up and down (details and icons).
    Across,
    /// Down each column from top to bottom, then on to the next column; the
    /// view scrolls sideways (compact, as in Dolphin).
    Down,
}

impl ViewMode {
    /// The order this mode fills the view in.
    pub fn flow(self) -> Flow {
        match self {
            Self::Compact => Flow::Down,
            Self::Icons | Self::Details => Flow::Across,
        }
    }
}

/// A key that moves the keyboard focus through the entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Up,
    Down,
    Left,
    Right,
    PageUp,
    PageDown,
    First,
    Last,
}

/// Lines moved by Page Up and Page Down in a view that scrolls up and down.
const PAGE_LINES: isize = 15;

/// Where `len` entries sit when drawn with `layout` in `flow` order, `lanes`
/// to a line (across) or to a column (down). Positions are in list
/// coordinates: from the top-left corner of the first entry.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Grid {
    /// Sizes of one entry.
    pub layout: Layout,
    /// The order entries fill the view in.
    pub flow: Flow,
    /// Entries per line (across) or per column (down); at least one.
    pub lanes: usize,
    /// Number of entries.
    pub len: usize,
}

impl Grid {
    /// The arrangement of `len` entries, `lanes` to a line or column.
    pub fn new(layout: Layout, flow: Flow, lanes: usize, len: usize) -> Self {
        Self {
            layout,
            flow,
            lanes: lanes.max(1),
            len,
        }
    }

    /// The width of one column: a cell, or the whole width for a list.
    fn pitch(&self) -> f32 {
        self.layout.cell_width.unwrap_or(0.)
    }

    /// Entry `index`'s row and column on screen.
    pub fn cell(&self, index: usize) -> (usize, usize) {
        match self.flow {
            Flow::Across => (index / self.lanes, index % self.lanes),
            Flow::Down => (index % self.lanes, index / self.lanes),
        }
    }

    /// The entry at a row and column on screen, if there is one; the row
    /// (across) or column (down) must be within the lanes.
    fn index_at(&self, row: usize, column: usize) -> Option<usize> {
        let index = match self.flow {
            Flow::Across => row * self.lanes + column,
            Flow::Down => column * self.lanes + row,
        };
        (index < self.len).then_some(index)
    }

    /// Lines of the scrolling direction: rows across, columns down.
    pub fn lines(&self) -> usize {
        self.len.div_ceil(self.lanes)
    }

    /// The width of all columns together in a view that fills down.
    pub fn width(&self) -> f32 {
        match self.flow {
            Flow::Across => self.lanes as f32 * self.pitch(),
            Flow::Down => self.lines() as f32 * self.pitch(),
        }
    }

    /// The bottom-left corner of entry `index`'s row or cell. A
    /// keyboard-opened context menu appears there, at the entry, as Dolphin
    /// places it.
    pub fn cell_bottom_left(&self, index: usize) -> (f32, f32) {
        let (row, column) = self.cell(index);
        (
            column as f32 * self.pitch(),
            (row + 1) as f32 * self.layout.row_height,
        )
    }

    /// The left and right edges of entry `index`'s column.
    pub fn column_span(&self, index: usize) -> (f32, f32) {
        let left = self.cell(index).1 as f32 * self.pitch();
        (left, left + self.pitch())
    }

    /// The columns of a view that fills down that show between `left` and
    /// `right`, in list coordinates.
    pub fn columns_between(&self, left: f32, right: f32) -> std::ops::Range<usize> {
        let pitch = self.pitch();
        if pitch <= 0. || right <= left {
            return 0..0;
        }
        let lines = self.lines();
        let first = ((left.max(0.) / pitch).floor() as usize).min(lines);
        let end = ((right.max(0.) / pitch).ceil() as usize).min(lines);
        first..end.max(first)
    }

    /// The entries a rubber band touches. In a full-width list every row the
    /// band's height crosses counts, whatever its horizontal position.
    pub fn rows_in(&self, band: Rect) -> Vec<usize> {
        if band.bottom < 0. || band.right < 0. || self.len == 0 {
            return Vec::new();
        }
        let cell = |edge: f32, size: f32| (edge.max(0.) / size).floor() as usize;
        let rows =
            cell(band.top, self.layout.row_height)..=cell(band.bottom, self.layout.row_height);
        let columns = match self.layout.cell_width {
            Some(width) => cell(band.left, width)..=cell(band.right, width),
            None => 0..=0,
        };
        // Bounded by the lanes and the entries that exist, so a band over
        // empty space selects nothing and costs nothing however large it is.
        let (last_row, last_column) = match self.flow {
            Flow::Across => ((self.len - 1) / self.lanes, self.lanes - 1),
            Flow::Down => (self.lanes - 1, (self.len - 1) / self.lanes),
        };
        let rows = *rows.start()..=(*rows.end()).min(last_row);
        let columns = *columns.start()..=(*columns.end()).min(last_column);
        let mut found: Vec<usize> = rows
            .flat_map(|row| {
                columns
                    .clone()
                    .filter_map(move |column| self.index_at(row, column))
            })
            .collect();
        found.sort_unstable();
        found
    }

    /// The entry a key moves the focus to from `from`, or `None` when the
    /// key does nothing here. Across, Up and Down change lines and Left and
    /// Right neighbours; a list has no neighbours to the side. Down columns,
    /// Up and Down step through the entries, Left and Right change columns,
    /// and Page Up and Page Down go to the top and bottom of the column, as
    /// in Dolphin. At an edge the focus stays put.
    pub fn step(&self, from: Option<usize>, step: Step) -> Option<usize> {
        let lanes = self.lanes as isize;
        let motion = match (self.flow, step, from) {
            (_, Step::First, _) => Motion::First,
            (_, Step::Last, _) => Motion::Last,
            (Flow::Across, Step::Left | Step::Right, _) if self.lanes == 1 => return None,
            // Nothing focused yet: forward keys start at the top, the others
            // at the bottom.
            (_, Step::Up | Step::Left | Step::PageUp, None) => Motion::By(-1),
            (_, Step::Down | Step::Right | Step::PageDown, None) => Motion::By(1),
            (Flow::Across, Step::Up, _) => Motion::By(-lanes),
            (Flow::Across, Step::Down, _) => Motion::By(lanes),
            (Flow::Across, Step::Left, _) => Motion::By(-1),
            (Flow::Across, Step::Right, _) => Motion::By(1),
            (Flow::Across, Step::PageUp, _) => Motion::By(-PAGE_LINES * lanes),
            (Flow::Across, Step::PageDown, _) => Motion::By(PAGE_LINES * lanes),
            (Flow::Down, Step::Up, _) => Motion::By(-1),
            (Flow::Down, Step::Down, _) => Motion::By(1),
            (Flow::Down, Step::Left, Some(row)) => {
                Motion::By(if row < self.lanes { 0 } else { -lanes })
            }
            (Flow::Down, Step::Right, Some(row)) => {
                let last_column = self.len.saturating_sub(1) / self.lanes;
                Motion::By(if row / self.lanes >= last_column {
                    0
                } else {
                    lanes
                })
            }
            (Flow::Down, Step::PageUp, Some(row)) => Motion::By(-((row % self.lanes) as isize)),
            (Flow::Down, Step::PageDown, Some(row)) => {
                Motion::By((self.lanes - 1 - row % self.lanes) as isize)
            }
        };
        target_row(from, self.len, motion)
    }
}

/// The scroll offset that brings the span from `start` to `end` into a
/// viewport `viewport` long, moving as little as possible. Offsets are zero
/// or negative: how far the content has moved back.
pub fn nearest_offset(offset: f32, start: f32, end: f32, viewport: f32) -> f32 {
    let shown = -offset;
    if start < shown || end - start > viewport {
        -start
    } else if end > shown + viewport {
        -(end - viewport)
    } else {
        offset
    }
}

/// A keyboard movement of the focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Motion {
    /// Back (negative) or forward (positive) by a number of entries.
    By(isize),
    First,
    Last,
}

/// The entry a motion lands on among `len` entries, or `None` when there are
/// none. With nothing focused, moving forward starts at the first entry and
/// moving back at the last.
pub(crate) fn target_row(current: Option<usize>, len: usize, motion: Motion) -> Option<usize> {
    let last = len.checked_sub(1)?;
    Some(match (motion, current) {
        (Motion::First, _) => 0,
        (Motion::Last, _) => last,
        (Motion::By(delta), Some(row)) => row.saturating_add_signed(delta).min(last),
        (Motion::By(delta), None) if delta >= 0 => 0,
        (Motion::By(_), None) => last,
    })
}

/// The zoom level after stepping `delta` levels from `zoom`, kept in range.
pub fn step_zoom(zoom: usize, delta: isize) -> usize {
    zoom.saturating_add_signed(delta).min(ZOOM_LEVELS - 1)
}

/// Scroll a content offset by one line, visible page or to an edge; ignore other keys.
pub fn scroll_offset(key: &str, offset: f32, maximum: f32, page: f32) -> Option<f32> {
    let next = match key {
        "up" => offset + crate::theme::text::LINE,
        "down" => offset - crate::theme::text::LINE,
        "pageup" => offset + page,
        "pagedown" => offset - page,
        "home" => 0.,
        "end" => -maximum,
        _ => return None,
    };
    Some(next.clamp(-maximum, 0.))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn across(layout: Layout, lanes: usize, len: usize) -> Grid {
        Grid::new(layout, Flow::Across, lanes, len)
    }

    /// The compact view at the default zoom: 36-pixel rows in 248-pixel
    /// columns, `lanes` rows to a column.
    fn down(lanes: usize, len: usize) -> Grid {
        Grid::new(
            ViewMode::Compact.layout(DEFAULT_ZOOM),
            Flow::Down,
            lanes,
            len,
        )
    }

    #[test]
    fn details_rows_span_the_width_and_grow_with_the_icon() {
        let default = ViewMode::Details.layout(DEFAULT_ZOOM);
        assert_eq!(default.cell_width, None);
        assert_eq!((default.icon, default.row_height), (24., 36.));
        let rows: Vec<f32> = (0..ZOOM_LEVELS)
            .map(|zoom| ViewMode::Details.layout(zoom).row_height)
            .collect();
        assert_eq!(rows, [28., 32., 36., 44., 60.]);
        assert_eq!(default.columns(2000.), 1);
    }

    #[test]
    fn a_cell_corner_follows_its_line_and_column() {
        let list = Layout {
            icon: 16.,
            row_height: 28.,
            cell_width: None,
        };
        assert_eq!(across(list, 1, 1000).cell_bottom_left(0), (0., 28.));
        assert_eq!(across(list, 1, 1000).cell_bottom_left(3), (0., 112.));
        let grid = Layout {
            icon: 64.,
            row_height: 100.,
            cell_width: Some(90.),
        };
        assert_eq!(across(grid, 4, 1000).cell_bottom_left(0), (0., 100.));
        assert_eq!(across(grid, 4, 1000).cell_bottom_left(6), (180., 200.));
        assert_eq!(across(grid, 0, 1000).cell_bottom_left(6), (0., 700.));
    }

    #[test]
    fn icons_mode_cells_hold_the_icon_and_two_lines_with_a_gap() {
        // 128 × 152-pixel cells, 8 pixels apart, at the 96-pixel level.
        let layout = ViewMode::Icons.layout(DEFAULT_ZOOM);
        assert_eq!(layout.icon, 96.);
        assert_eq!(layout.cell_width, Some(128. + 8.));
        assert_eq!(layout.row_height, 152. + 8.);
        // Compact columns are 240 pixels with the same gap.
        assert_eq!(
            ViewMode::Compact.layout(DEFAULT_ZOOM).cell_width,
            Some(240. + 8.)
        );
    }

    #[test]
    fn zooming_in_never_shrinks_anything() {
        for mode in [ViewMode::Icons, ViewMode::Compact, ViewMode::Details] {
            for zoom in 1..ZOOM_LEVELS {
                let (smaller, larger) = (mode.layout(zoom - 1), mode.layout(zoom));
                assert!(larger.icon > smaller.icon);
                assert!(larger.row_height >= smaller.row_height);
                assert!(larger.cell_width >= smaller.cell_width);
            }
        }
    }

    #[test]
    fn every_size_is_on_the_four_pixel_grid() {
        for mode in [ViewMode::Icons, ViewMode::Compact, ViewMode::Details] {
            for zoom in 0..ZOOM_LEVELS {
                let layout = mode.layout(zoom);
                for value in [
                    layout.icon,
                    layout.row_height,
                    layout.cell_width.unwrap_or(0.),
                ] {
                    assert_eq!(value % 4., 0., "{mode:?} at zoom {zoom}: {value}");
                }
            }
        }
    }

    #[test]
    fn columns_fill_the_width_with_whole_cells() {
        let layout = ViewMode::Icons.layout(DEFAULT_ZOOM);
        assert_eq!(layout.columns(136. * 5.), 5);
        assert_eq!(layout.columns(136. * 5. + 135.), 5);
        assert_eq!(layout.columns(136. * 6.), 6);
        // A window narrower than one cell still shows a column.
        assert_eq!(layout.columns(50.), 1);
    }

    #[test]
    fn a_band_over_a_list_selects_every_row_its_height_crosses() {
        let layout = ViewMode::Details.layout(0); // 28-pixel rows
        let band = Rect::between((400., 30.), (10., 85.));
        assert_eq!(across(layout, 1, 100).rows_in(band), [1, 2, 3]);
        // A band inside one row selects that row.
        assert_eq!(
            across(layout, 1, 100).rows_in(Rect::between((0., 2.), (5., 9.))),
            [0]
        );
    }

    #[test]
    fn a_band_over_a_grid_selects_only_the_cells_it_touches() {
        let layout = ViewMode::Icons.layout(1); // 104 × 128-pixel cells
        // Columns 1-2 of lines 0-1, five cells per line.
        let band = Rect::between((130., 10.), (300., 200.));
        assert_eq!(across(layout, 5, 100).rows_in(band), [1, 2, 6, 7]);
    }

    #[test]
    fn a_band_is_clipped_to_the_entries_that_exist() {
        let layout = ViewMode::Icons.layout(1);
        // Seven entries, five per line: the second line has only two cells.
        let band = Rect::between((0., 0.), (2000., 2000.));
        assert_eq!(across(layout, 5, 7).rows_in(band), [0, 1, 2, 3, 4, 5, 6]);
        let lower_right = Rect::between((300., 130.), (640., 250.));
        assert_eq!(
            across(layout, 5, 7).rows_in(lower_right),
            Vec::<usize>::new()
        );
        let list = ViewMode::Details.layout(DEFAULT_ZOOM);
        assert_eq!(
            across(list, 1, 3).rows_in(Rect::between((0., 0.), (10., 5000.))),
            [0, 1, 2]
        );
    }

    #[test]
    fn a_band_entirely_above_or_left_of_the_entries_selects_nothing() {
        let layout = ViewMode::Icons.layout(1);
        assert!(
            across(layout, 5, 100)
                .rows_in(Rect::between((10., -50.), (60., -5.)))
                .is_empty()
        );
        assert!(
            across(layout, 5, 100)
                .rows_in(Rect::between((-50., 10.), (-5., 60.)))
                .is_empty()
        );
        // Starting above the first row still reaches down into it.
        assert_eq!(
            across(layout, 5, 100).rows_in(Rect::between((10., -50.), (60., 5.))),
            [0]
        );
    }

    #[test]
    fn compact_entries_fill_each_column_before_the_next() {
        // Three rows to a column: 0 1 2 down the first, 3 4 5 down the second.
        let grid = down(3, 7);
        assert_eq!(grid.cell(0), (0, 0));
        assert_eq!(grid.cell(2), (2, 0));
        assert_eq!(grid.cell(3), (0, 1));
        assert_eq!(grid.cell(6), (0, 2));
        assert_eq!(grid.lines(), 3);
        assert_eq!(grid.width(), 3. * 248.);
        assert_eq!(grid.cell_bottom_left(4), (248., 72.));
        assert_eq!(grid.column_span(5), (248., 496.));
    }

    #[test]
    fn the_rows_that_fit_a_height_make_a_column() {
        let layout = ViewMode::Compact.layout(DEFAULT_ZOOM); // 36-pixel rows
        assert_eq!(layout.rows(36. * 10. + 35.), 10);
        assert_eq!(layout.rows(10.), 1);
    }

    #[test]
    fn a_band_over_compact_columns_selects_the_cells_it_touches() {
        // Ten rows to a column; rows 1-2 of the second and third columns.
        let grid = down(10, 100);
        let band = Rect::between((300., 40.), (500., 100.));
        assert_eq!(grid.rows_in(band), [11, 12, 21, 22]);
        // Below the last row of a column there is nothing.
        let below = Rect::between((0., 36. * 10. + 1.), (600., 900.));
        assert!(grid.rows_in(below).is_empty());
        // Under a column's last row there is nothing, though later entries
        // exist: they are in the next columns.
        let tall = down(4, 30);
        assert!(
            tall.rows_in(Rect::between((10., 4. * 36. + 1.), (300., 400.)))
                .is_empty()
        );
        // The last column holds only what is left: 23 entries fill two
        // columns and three rows of the third.
        let short = down(10, 23);
        let right = Rect::between((496., 0.), (2000., 2000.));
        assert_eq!(short.rows_in(right), [20, 21, 22]);
    }

    #[test]
    fn up_and_down_step_through_compact_entries_and_wrap_columns() {
        let grid = down(3, 8);
        assert_eq!(grid.step(Some(2), Step::Down), Some(3));
        assert_eq!(grid.step(Some(3), Step::Up), Some(2));
        assert_eq!(grid.step(Some(7), Step::Down), Some(7));
        assert_eq!(grid.step(Some(0), Step::Up), Some(0));
    }

    #[test]
    fn left_and_right_change_compact_columns_and_stop_at_the_edges() {
        // 0 3 6
        // 1 4 7
        // 2 5
        let grid = down(3, 8);
        assert_eq!(grid.step(Some(1), Step::Right), Some(4));
        assert_eq!(grid.step(Some(4), Step::Left), Some(1));
        // Into the short last column: the nearest entry, the last.
        assert_eq!(grid.step(Some(5), Step::Right), Some(7));
        // At the first and last columns the focus stays.
        assert_eq!(grid.step(Some(1), Step::Left), Some(1));
        assert_eq!(grid.step(Some(6), Step::Right), Some(6));
        assert_eq!(grid.step(Some(7), Step::Right), Some(7));
    }

    #[test]
    fn page_keys_reach_the_ends_of_the_compact_column() {
        let grid = down(3, 8);
        assert_eq!(grid.step(Some(4), Step::PageUp), Some(3));
        assert_eq!(grid.step(Some(4), Step::PageDown), Some(5));
        // The short last column ends at the last entry.
        assert_eq!(grid.step(Some(6), Step::PageDown), Some(7));
        assert_eq!(grid.step(Some(3), Step::PageUp), Some(3));
    }

    #[test]
    fn keys_in_lines_across_keep_their_meaning() {
        let icons = across(ViewMode::Icons.layout(DEFAULT_ZOOM), 5, 100);
        assert_eq!(icons.step(Some(7), Step::Down), Some(12));
        assert_eq!(icons.step(Some(7), Step::Up), Some(2));
        assert_eq!(icons.step(Some(7), Step::Right), Some(8));
        assert_eq!(icons.step(Some(7), Step::PageDown), Some(82));
        // A list has no neighbours to the side.
        let list = across(ViewMode::Details.layout(DEFAULT_ZOOM), 1, 100);
        assert_eq!(list.step(Some(7), Step::Right), None);
        assert_eq!(list.step(Some(7), Step::PageUp), Some(0));
    }

    #[test]
    fn keys_without_a_focus_enter_from_the_matching_end() {
        let grid = down(3, 8);
        assert_eq!(grid.step(None, Step::Down), Some(0));
        assert_eq!(grid.step(None, Step::Right), Some(0));
        assert_eq!(grid.step(None, Step::Up), Some(7));
        assert_eq!(grid.step(None, Step::PageUp), Some(7));
        assert_eq!(down(3, 0).step(None, Step::Down), None);
    }

    #[test]
    fn only_the_columns_in_view_are_drawn() {
        let grid = down(10, 1000); // 100 columns of 248 pixels
        assert_eq!(grid.columns_between(0., 800.), 0..4);
        assert_eq!(grid.columns_between(500., 1000.), 2..5);
        assert_eq!(grid.columns_between(24_700., 30_000.), 99..100);
        assert_eq!(grid.columns_between(0., 0.), 0..0);
    }

    #[test]
    fn revealing_a_span_scrolls_as_little_as_possible() {
        // Already in view: no change.
        assert_eq!(nearest_offset(-100., 200., 400., 500.), -100.);
        // Before the view: its start comes to the leading edge.
        assert_eq!(nearest_offset(-300., 200., 400., 500.), -200.);
        // After the view: its end comes to the trailing edge.
        assert_eq!(nearest_offset(0., 600., 800., 500.), -300.);
        // Longer than the view: its start shows.
        assert_eq!(nearest_offset(0., 600., 1200., 500.), -600.);
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
    fn zoom_steps_stay_within_the_levels() {
        assert_eq!(step_zoom(1, 1), 2);
        assert_eq!(step_zoom(0, -1), 0);
        assert_eq!(step_zoom(ZOOM_LEVELS - 1, 1), ZOOM_LEVELS - 1);
        assert_eq!(
            ViewMode::Details.layout(99),
            ViewMode::Details.layout(ZOOM_LEVELS - 1)
        );
    }

    #[test]
    fn scroll_keys_reach_and_respect_content_edges() {
        assert_eq!(scroll_offset("home", -100., 300., 120.), Some(0.));
        assert_eq!(scroll_offset("end", 0., 300., 120.), Some(-300.));
        assert_eq!(scroll_offset("up", 0., 300., 120.), Some(0.));
        assert_eq!(scroll_offset("down", -295., 300., 120.), Some(-300.));
    }

    #[test]
    fn scroll_keys_move_by_line_or_visible_page() {
        assert_eq!(scroll_offset("down", 0., 300., 120.), Some(-20.));
        assert_eq!(scroll_offset("up", -100., 300., 120.), Some(-80.));
        assert_eq!(scroll_offset("pagedown", 0., 300., 120.), Some(-120.));
        assert_eq!(scroll_offset("pageup", -200., 300., 120.), Some(-80.));
        assert_eq!(scroll_offset("pagedown", 0., 0., 120.), Some(0.));
        assert_eq!(scroll_offset("enter", -100., 300., 120.), None);
    }
}

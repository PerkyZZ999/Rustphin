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
    /// The bottom-left corner of entry `index`'s row or cell, in list
    /// coordinates. A keyboard-opened context menu appears there, at the
    /// entry, as Dolphin places it.
    pub fn cell_bottom_left(&self, index: usize, per_row: usize) -> (f32, f32) {
        let per_row = per_row.max(1);
        let (line, column) = (index / per_row, index % per_row);
        let left = self.cell_width.map_or(0., |width| column as f32 * width);
        (left, (line + 1) as f32 * self.row_height)
    }

    /// How many entries fit side by side in `width` pixels; at least one.
    pub fn columns(&self, width: f32) -> usize {
        match self.cell_width {
            Some(cell) => ((width / cell).floor() as usize).max(1),
            None => 1,
        }
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

impl Layout {
    /// The rows a rubber band touches in a list of `len` entries drawn
    /// `per_row` to a line. In a full-width list every row the band's height
    /// crosses counts, whatever its horizontal position.
    pub fn rows_in(&self, band: Rect, per_row: usize, len: usize) -> Vec<usize> {
        let per_row = per_row.max(1);
        if band.bottom < 0. || band.right < 0. || len == 0 {
            return Vec::new();
        }
        let cell = |edge: f32, size: f32| (edge.max(0.) / size).floor() as usize;
        let last_line = (len - 1) / per_row;
        let lines =
            cell(band.top, self.row_height)..=cell(band.bottom, self.row_height).min(last_line);
        let columns = match self.cell_width {
            Some(width) => cell(band.left, width)..=cell(band.right, width).min(per_row - 1),
            None => 0..=0,
        };
        lines
            .flat_map(|line| columns.clone().map(move |column| line * per_row + column))
            .filter(|&row| row < len)
            .collect()
    }
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
        assert_eq!(list.cell_bottom_left(0, 1), (0., 28.));
        assert_eq!(list.cell_bottom_left(3, 1), (0., 112.));
        let grid = Layout {
            icon: 64.,
            row_height: 100.,
            cell_width: Some(90.),
        };
        assert_eq!(grid.cell_bottom_left(0, 4), (0., 100.));
        assert_eq!(grid.cell_bottom_left(6, 4), (180., 200.));
        assert_eq!(grid.cell_bottom_left(6, 0), (0., 700.));
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
        assert_eq!(layout.rows_in(band, 1, 100), [1, 2, 3]);
        // A band inside one row selects that row.
        assert_eq!(
            layout.rows_in(Rect::between((0., 2.), (5., 9.)), 1, 100),
            [0]
        );
    }

    #[test]
    fn a_band_over_a_grid_selects_only_the_cells_it_touches() {
        let layout = ViewMode::Icons.layout(1); // 104 × 128-pixel cells
        // Columns 1-2 of lines 0-1, five cells per line.
        let band = Rect::between((130., 10.), (300., 200.));
        assert_eq!(layout.rows_in(band, 5, 100), [1, 2, 6, 7]);
    }

    #[test]
    fn a_band_is_clipped_to_the_entries_that_exist() {
        let layout = ViewMode::Icons.layout(1);
        // Seven entries, five per line: the second line has only two cells.
        let band = Rect::between((0., 0.), (2000., 2000.));
        assert_eq!(layout.rows_in(band, 5, 7), [0, 1, 2, 3, 4, 5, 6]);
        let lower_right = Rect::between((300., 130.), (640., 250.));
        assert_eq!(layout.rows_in(lower_right, 5, 7), Vec::<usize>::new());
        let list = ViewMode::Details.layout(DEFAULT_ZOOM);
        assert_eq!(
            list.rows_in(Rect::between((0., 0.), (10., 5000.)), 1, 3),
            [0, 1, 2]
        );
    }

    #[test]
    fn a_band_entirely_above_or_left_of_the_entries_selects_nothing() {
        let layout = ViewMode::Icons.layout(1);
        assert!(
            layout
                .rows_in(Rect::between((10., -50.), (60., -5.)), 5, 100)
                .is_empty()
        );
        assert!(
            layout
                .rows_in(Rect::between((-50., 10.), (-5., 60.)), 5, 100)
                .is_empty()
        );
        // Starting above the first row still reaches down into it.
        assert_eq!(
            layout.rows_in(Rect::between((10., -50.), (60., 5.)), 5, 100),
            [0]
        );
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

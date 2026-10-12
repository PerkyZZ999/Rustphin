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
/// The zoom level a new window starts at.
pub const DEFAULT_ZOOM: usize = 1;

/// Icon sizes of the two list-like modes, by zoom level.
const SMALL_ICONS: [f32; ZOOM_LEVELS] = [16., 20., 24., 32., 48.];
/// Icon sizes of the icons mode, by zoom level.
const LARGE_ICONS: [f32; ZOOM_LEVELS] = [48., 64., 96., 128., 192.];
/// Cell widths of the compact mode, by zoom level.
const COMPACT_WIDTHS: [f32; ZOOM_LEVELS] = [224., 224., 240., 256., 288.];
/// Space around a large icon and its two-line label: padding, gap and label.
const ICON_CELL_EXTRA: f32 = 64.;
/// Rows are at least this tall, and otherwise the icon plus this padding.
const MIN_ROW: f32 = 28.;
const ROW_PADDING: f32 = 8.;

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
                row_height: LARGE_ICONS[zoom] + ICON_CELL_EXTRA,
                cell_width: Some(LARGE_ICONS[zoom] + ICON_CELL_EXTRA),
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
}

/// The zoom level after stepping `delta` levels from `zoom`, kept in range.
pub fn step_zoom(zoom: usize, delta: isize) -> usize {
    zoom.saturating_add_signed(delta).min(ZOOM_LEVELS - 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn details_rows_span_the_width_and_grow_with_the_icon() {
        let default = ViewMode::Details.layout(DEFAULT_ZOOM);
        assert_eq!(default.cell_width, None);
        assert_eq!((default.icon, default.row_height), (20., 28.));
        let largest = ViewMode::Details.layout(ZOOM_LEVELS - 1);
        assert_eq!((largest.icon, largest.row_height), (48., 56.));
        assert_eq!(default.columns(2000.), 1);
    }

    #[test]
    fn icons_mode_uses_square_cells_larger_than_the_icon() {
        let layout = ViewMode::Icons.layout(DEFAULT_ZOOM);
        assert_eq!(layout.icon, 64.);
        assert_eq!(layout.cell_width, Some(128.));
        assert_eq!(layout.row_height, 128.);
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
        assert_eq!(layout.columns(128. * 5.), 5);
        assert_eq!(layout.columns(128. * 5. + 127.), 5);
        assert_eq!(layout.columns(128. * 6.), 6);
        // A window narrower than one cell still shows a column.
        assert_eq!(layout.columns(50.), 1);
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
}

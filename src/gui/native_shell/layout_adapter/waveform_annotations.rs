//! Slotized waveform annotation geometry for selection, slice previews, cursor, and playhead.

use crate::app::{NormalizedRangeModel, WaveformSlicePreviewModel};
use crate::gui::types::{Point, Rect};

/// Waveform annotation rectangles resolved from normalized waveform anchors.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct WaveformAnnotationRects {
    pub selection: Option<Rect>,
    pub cursor: Option<Rect>,
    pub playhead: Option<Rect>,
}

/// Rectangle and selection state for one detected waveform slice preview.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct WaveformSlicePreviewRects {
    pub rect: Rect,
    pub selected: bool,
    pub focused: bool,
    pub marked_for_export: bool,
    pub duplicate_cleanup_candidate: bool,
    pub duplicate_cleanup_exempted: bool,
}

/// Compute waveform annotation rectangles constrained to the waveform plot.
pub(crate) fn compute_waveform_annotation_rects(
    waveform_plot: Rect,
    border_width: f32,
    selection: Option<NormalizedRangeModel>,
    cursor_milli: Option<u16>,
    playhead_milli: Option<u16>,
    view_start_micros: impl Into<u32>,
    view_end_micros: impl Into<u32>,
) -> WaveformAnnotationRects {
    if waveform_plot.width() <= 0.0 || waveform_plot.height() <= 0.0 {
        return WaveformAnnotationRects::default();
    }
    let view = normalized_view_window(view_start_micros.into(), view_end_micros.into());
    WaveformAnnotationRects {
        selection: selection.and_then(|range| selection_rect(waveform_plot, range, view)),
        cursor: cursor_milli.and_then(|milli| {
            marker_rect(waveform_plot, border_width, u32::from(milli) * 1000, view)
        }),
        playhead: playhead_milli.and_then(|milli| {
            marker_rect(waveform_plot, border_width, u32::from(milli) * 1000, view)
        }),
    }
}

/// Compute all slice-preview rectangles constrained to the waveform plot.
pub(crate) fn compute_waveform_slice_preview_rects(
    waveform_plot: Rect,
    slices: &[WaveformSlicePreviewModel],
    view_start_micros: impl Into<u32>,
    view_end_micros: impl Into<u32>,
) -> Vec<WaveformSlicePreviewRects> {
    if waveform_plot.width() <= 0.0 || waveform_plot.height() <= 0.0 {
        return Vec::new();
    }
    let view = normalized_view_window(view_start_micros.into(), view_end_micros.into());
    slices
        .iter()
        .filter_map(|slice| {
            slice_rect(waveform_plot, slice.range, view).map(|rect| WaveformSlicePreviewRects {
                rect,
                selected: slice.selected,
                focused: slice.focused,
                marked_for_export: slice.marked_for_export,
                duplicate_cleanup_candidate: slice.duplicate_cleanup_candidate,
                duplicate_cleanup_exempted: slice.duplicate_cleanup_exempted,
            })
        })
        .collect()
}

fn slice_rect(
    waveform_plot: Rect,
    range: NormalizedRangeModel,
    view: WaveformViewWindow,
) -> Option<Rect> {
    let start = x_for_micros(waveform_plot, range.start_micros, view);
    let end = x_for_micros(waveform_plot, range.end_micros, view);
    let left = start
        .min(end)
        .clamp(waveform_plot.min.x, waveform_plot.max.x);
    let right = end
        .max(start)
        .clamp(waveform_plot.min.x, waveform_plot.max.x);
    let expanded_right = right.max((left + 1.0).min(waveform_plot.max.x));
    (expanded_right > left).then_some(Rect::from_min_max(
        Point::new(left, waveform_plot.min.y),
        Point::new(expanded_right, waveform_plot.max.y),
    ))
}

fn selection_rect(
    waveform_plot: Rect,
    selection: NormalizedRangeModel,
    view: WaveformViewWindow,
) -> Option<Rect> {
    let start = x_for_micros(waveform_plot, selection.start_micros, view);
    let end = x_for_micros(waveform_plot, selection.end_micros, view);
    let left = start
        .min(end)
        .clamp(waveform_plot.min.x, waveform_plot.max.x);
    let right = end
        .max(start)
        .clamp(waveform_plot.min.x, waveform_plot.max.x);
    let expanded_right = right.max((left + 1.0).min(waveform_plot.max.x));
    (expanded_right > left).then_some(Rect::from_min_max(
        Point::new(left, waveform_plot.min.y),
        Point::new(expanded_right, waveform_plot.max.y),
    ))
}

fn marker_rect(
    waveform_plot: Rect,
    border_width: f32,
    micros: u32,
    view: WaveformViewWindow,
) -> Option<Rect> {
    let marker_width = border_width.max(1.0).min(waveform_plot.width());
    if marker_width <= 0.0 {
        return None;
    }
    let raw_x = x_for_micros(waveform_plot, micros, view);
    let left = raw_x.clamp(waveform_plot.min.x, waveform_plot.max.x - marker_width);
    let right = (left + marker_width).min(waveform_plot.max.x);
    (right > left).then_some(Rect::from_min_max(
        Point::new(left, waveform_plot.min.y),
        Point::new(right, waveform_plot.max.y),
    ))
}

#[derive(Clone, Copy)]
struct WaveformViewWindow {
    start_ratio: f32,
    width_ratio: f32,
}

fn normalized_view_window(view_start_micros: u32, view_end_micros: u32) -> WaveformViewWindow {
    let start_micros = view_start_micros.min(1_000_000);
    let end_micros = view_end_micros.min(1_000_000).max(start_micros);
    let start_ratio = start_micros as f32 / 1_000_000.0;
    let width_ratio =
        ((end_micros.saturating_sub(start_micros)) as f32 / 1_000_000.0).max(f32::EPSILON);
    WaveformViewWindow {
        start_ratio,
        width_ratio,
    }
}

fn x_for_micros(waveform_plot: Rect, micros: u32, view: WaveformViewWindow) -> f32 {
    let absolute_ratio = micros.min(1_000_000) as f32 / 1_000_000.0;
    let ratio_in_view = ((absolute_ratio - view.start_ratio) / view.width_ratio).clamp(0.0, 1.0);
    waveform_plot.min.x + (waveform_plot.width() * ratio_in_view)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_inside(outer: Rect, inner: Rect) {
        assert!(inner.min.x >= outer.min.x);
        assert!(inner.min.y >= outer.min.y);
        assert!(inner.max.x <= outer.max.x);
        assert!(inner.max.y <= outer.max.y);
    }

    #[test]
    fn annotation_rects_stay_inside_waveform_plot() {
        let plot = Rect::from_min_max(Point::new(300.0, 120.0), Point::new(1160.0, 320.0));
        let rects = compute_waveform_annotation_rects(
            plot,
            1.5,
            Some(NormalizedRangeModel::new(120, 640)),
            Some(300),
            Some(780),
            0_u32,
            1_000_000_u32,
        );
        assert_inside(plot, rects.selection.expect("selection"));
        assert_inside(plot, rects.cursor.expect("cursor"));
        assert_inside(plot, rects.playhead.expect("playhead"));
    }

    #[test]
    fn marker_rects_clamp_to_plot_edges() {
        let plot = Rect::from_min_max(Point::new(100.0, 80.0), Point::new(300.0, 200.0));
        let left =
            compute_waveform_annotation_rects(plot, 2.0, None, Some(0), None, 0_u32, 1_000_000_u32);
        let right = compute_waveform_annotation_rects(
            plot,
            2.0,
            None,
            None,
            Some(1000),
            0_u32,
            1_000_000_u32,
        );
        assert_eq!(left.cursor.expect("left marker").min.x, plot.min.x);
        assert_eq!(right.playhead.expect("right marker").max.x, plot.max.x);
    }

    #[test]
    fn empty_plot_returns_no_annotation_rects() {
        let plot = Rect::from_min_max(Point::new(10.0, 10.0), Point::new(10.0, 10.0));
        let rects = compute_waveform_annotation_rects(
            plot,
            1.0,
            Some(NormalizedRangeModel::new(100, 200)),
            Some(150),
            Some(200),
            0_u32,
            1_000_000_u32,
        );
        assert_eq!(rects, WaveformAnnotationRects::default());
    }

    #[test]
    fn marker_rects_respect_view_window() {
        let plot = Rect::from_min_max(Point::new(200.0, 80.0), Point::new(1000.0, 220.0));
        let start = compute_waveform_annotation_rects(
            plot,
            2.0,
            None,
            Some(250),
            None,
            250_000_u32,
            750_000_u32,
        );
        let center = compute_waveform_annotation_rects(
            plot,
            2.0,
            None,
            Some(500),
            None,
            250_000_u32,
            750_000_u32,
        );
        let end = compute_waveform_annotation_rects(
            plot,
            2.0,
            None,
            Some(750),
            None,
            250_000_u32,
            750_000_u32,
        );
        assert_eq!(start.cursor.expect("start marker").min.x, plot.min.x);
        let center_marker = center.cursor.expect("center marker");
        assert!((center_marker.min.x - (plot.min.x + (plot.width() * 0.5))).abs() <= 2.0);
        assert_eq!(end.cursor.expect("end marker").max.x, plot.max.x);
    }

    #[test]
    fn selection_rects_use_micro_precision_inside_narrow_view_windows() {
        let plot = Rect::from_min_max(Point::new(100.0, 40.0), Point::new(300.0, 140.0));
        let rects = compute_waveform_annotation_rects(
            plot,
            1.0,
            Some(NormalizedRangeModel::from_micros(500_400, 500_600)),
            None,
            None,
            500_000_u32,
            501_000_u32,
        );

        let selection = rects.selection.expect("selection");
        assert!((selection.min.x - 180.0).abs() <= 1.0);
        assert!((selection.max.x - 220.0).abs() <= 1.0);
    }

    #[test]
    fn slice_preview_rects_preserve_selection_state_and_stay_inside_plot() {
        let plot = Rect::from_min_max(Point::new(100.0, 40.0), Point::new(300.0, 140.0));
        let slices = compute_waveform_slice_preview_rects(
            plot,
            &[
                WaveformSlicePreviewModel {
                    range: NormalizedRangeModel::new(100, 220),
                    selected: false,
                    focused: false,
                    marked_for_export: false,
                    duplicate_cleanup_candidate: false,
                    duplicate_cleanup_exempted: false,
                },
                WaveformSlicePreviewModel {
                    range: NormalizedRangeModel::new(500, 700),
                    selected: true,
                    focused: true,
                    marked_for_export: true,
                    duplicate_cleanup_candidate: false,
                    duplicate_cleanup_exempted: false,
                },
            ],
            0_u32,
            1_000_000_u32,
        );

        assert_eq!(slices.len(), 2);
        assert!(!slices[0].selected);
        assert!(slices[1].selected);
        assert!(!slices[0].focused);
        assert!(slices[1].focused);
        assert!(!slices[0].marked_for_export);
        assert!(slices[1].marked_for_export);
        assert_inside(plot, slices[0].rect);
        assert_inside(plot, slices[1].rect);
    }
}

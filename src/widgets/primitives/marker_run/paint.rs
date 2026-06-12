use crate::{
    gui::types::Rect,
    runtime::{PaintFillRect, PaintFillRectBatch, PaintPrimitive},
    widgets::contract::WidgetId,
};

use super::{
    geometry::{collect_marker_rects, for_each_marker_rect, marker_geometry},
    model::{ColorMarkerRunProps, MarkerRunProps},
};

pub(super) fn append_marker_run_paint(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: WidgetId,
    bounds: Rect,
    props: MarkerRunProps,
) {
    let Some(color) = props.color else {
        return;
    };
    let count = props.count as usize;
    let geometry = marker_geometry(props.side, props.gap, props.inset, props.align);
    if count == 1 {
        for_each_marker_rect(bounds, count, geometry, |_, rect| {
            primitives.push(PaintPrimitive::FillRect(PaintFillRect {
                widget_id,
                rect,
                color,
            }));
        });
        return;
    }

    let mut rects = Vec::with_capacity(count);
    collect_marker_rects(bounds, count, geometry, &mut rects);
    if !rects.is_empty() {
        primitives.push(PaintPrimitive::FillRectBatch(PaintFillRectBatch {
            widget_id,
            rects: rects.into(),
            color,
        }));
    }
}

pub(super) fn append_color_marker_run_paint(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: WidgetId,
    bounds: Rect,
    props: &ColorMarkerRunProps,
) {
    for_each_marker_rect(
        bounds,
        props.colors.len(),
        marker_geometry(props.side, props.gap, props.inset, props.align),
        |index, rect| {
            if let Some(color) = props.colors.get(index) {
                primitives.push(PaintPrimitive::FillRect(PaintFillRect {
                    widget_id,
                    rect,
                    color: *color,
                }));
            }
        },
    );
}

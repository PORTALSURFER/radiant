use super::{PaintClipEnd, PaintClipStart, PaintPrimitive, PaintStrokeRect, SurfacePaintPlan};
use crate::{
    gui::types::{Rect, Rgba8},
    layout::{LayoutOutput, NodeId},
};

const LAYOUT_DEBUG_STROKE: Rgba8 = Rgba8 {
    r: 255,
    g: 0,
    b: 0,
    a: 255,
};

pub(in crate::runtime) fn push_clip_start(
    primitives: &mut Vec<PaintPrimitive>,
    node_id: NodeId,
    rect: Rect,
) {
    primitives.push(PaintPrimitive::ClipStart(PaintClipStart { node_id, rect }));
}

pub(in crate::runtime) fn push_clip_end(primitives: &mut Vec<PaintPrimitive>, node_id: NodeId) {
    primitives.push(PaintPrimitive::ClipEnd(PaintClipEnd { node_id }));
}

pub(in crate::runtime) fn push_layout_debug_overlay_for_node(
    layout: &LayoutOutput,
    plan: &mut SurfacePaintPlan,
    node_id: NodeId,
) {
    plan.primitives.extend(
        layout
            .debug_primitives
            .iter()
            .filter(|primitive| {
                primitive.node_id == node_id && primitive.rect.has_finite_positive_area()
            })
            .map(|primitive| {
                PaintPrimitive::StrokeRect(PaintStrokeRect {
                    widget_id: primitive.node_id,
                    rect: primitive.rect,
                    color: LAYOUT_DEBUG_STROKE,
                    width: 1.0,
                })
            }),
    );
}

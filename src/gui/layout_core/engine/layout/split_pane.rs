//! Static two-pane split placement.

use super::super::{LayoutContext, round_rect};
use super::layout_node;
use crate::gui::layout_core::engine::LayoutDiagnosticCode;
use crate::gui::layout_core::tree::ContainerNode;
use crate::gui::panel::{SplitPaneAxis, SplitPaneLayout, SplitPaneLayoutParts};
use crate::gui::types::{Point, Rect};

pub(super) fn layout_split_pane(
    container: &ContainerNode,
    content: Rect,
    context: &mut LayoutContext,
) {
    // Keep the same observational boundary in the top-down pass. The value
    // must not influence placement, diagnostics, invalidation, or caching.
    let _ = context.container_state_read(container.id);
    if container.children.len() != 2 {
        context.push_diagnostic(
            container.id,
            LayoutDiagnosticCode::SplitPaneChildCountMismatch,
            "split pane requires exactly two children",
        );
        for child in &container.children {
            layout_node(&child.child, content, context);
        }
        return;
    }

    let policy = container.policy.split_pane;
    let resolved = SplitPaneLayout::from_parts(SplitPaneLayoutParts {
        bounds: content,
        axis: policy.axis,
        ratio: policy.initial_ratio,
        divider_extent: policy.divider_extent,
        first_min_extent: policy.first_min_extent,
        second_min_extent: policy.second_min_extent,
    });
    if !resolved.minima_satisfied {
        context.push_diagnostic(
            container.id,
            LayoutDiagnosticCode::SplitPaneMinimumsUnsatisfied,
            "split pane minimums were unsatisfied by the available bounds",
        );
    }
    let (first, _divider, second) = quantized_rects(resolved);
    layout_node(&container.children[0].child, first, context);
    layout_node(&container.children[1].child, second, context);
}

fn quantized_rects(resolved: SplitPaneLayout) -> (Rect, Rect, Rect) {
    let outer = round_rect(resolved.bounds);
    let total_extent = selected_extent(outer, resolved.axis).max(0.0);
    let divider_extent = if total_extent > 0.0 && resolved.divider_extent > 0.0 {
        resolved.divider_extent.round().max(1.0).min(total_extent)
    } else {
        0.0
    };
    let first_extent = selected_extent(resolved.first, resolved.axis)
        .round()
        .clamp(0.0, total_extent - divider_extent);
    let second_extent = total_extent - divider_extent - first_extent;

    let q0 = match resolved.axis {
        SplitPaneAxis::Horizontal => outer.min.x,
        SplitPaneAxis::Vertical => outer.min.y,
    };
    let q1 = q0 + first_extent;
    let q2 = q1 + divider_extent;
    let q3 = q2 + second_extent;

    (
        rect_for_axis_span(outer, resolved.axis, q0, q1),
        rect_for_axis_span(outer, resolved.axis, q1, q2),
        rect_for_axis_span(outer, resolved.axis, q2, q3),
    )
}

fn rect_for_axis_span(outer: Rect, axis: SplitPaneAxis, start: f32, end: f32) -> Rect {
    match axis {
        SplitPaneAxis::Horizontal => {
            Rect::from_min_max(Point::new(start, outer.min.y), Point::new(end, outer.max.y))
        }
        SplitPaneAxis::Vertical => {
            Rect::from_min_max(Point::new(outer.min.x, start), Point::new(outer.max.x, end))
        }
    }
}

fn selected_extent(rect: Rect, axis: SplitPaneAxis) -> f32 {
    match axis {
        SplitPaneAxis::Horizontal => rect.width(),
        SplitPaneAxis::Vertical => rect.height(),
    }
}

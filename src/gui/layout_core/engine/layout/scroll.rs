//! ScrollView layout including optional linear virtualization.

mod virtualization;

use super::super::{LayoutContext, LayoutDiagnosticCode};
use super::layout_node;
use super::scroll_helpers::clamp_scroll_offset;
use super::scroll_linear::known_linear_main_extent;
use crate::gui::layout_core::model::{ContainerKind, VirtualizationAxis};
use crate::gui::layout_core::tree::{ContainerNode, LayoutNode};
use crate::gui::types::{Point, Rect, Vector2};
use virtualization::layout_virtualized_child;

/// Layout a scroll container and optionally virtualize large linear child lists.
pub(super) fn layout_scroll_view(
    container: &ContainerNode,
    content: Rect,
    context: &mut LayoutContext,
) {
    let Some(child) = container.children.first() else {
        return;
    };
    let slot = child.slot;
    let viewport_w = (content.width() - slot.margin.left - slot.margin.right).max(0.0);
    let viewport_h = (content.height() - slot.margin.top - slot.margin.bottom).max(0.0);
    let measured = virtual_fixed_content_size(container, &child.child, viewport_w, viewport_h)
        .unwrap_or_else(|| {
            super::super::measure::measure_node(&child.child, slot.constraints, context)
        });
    let width = measured.x.max(viewport_w);
    let height = measured.y.max(viewport_h);
    let max_x = (width - viewport_w).max(0.0);
    let max_y = (height - viewport_h).max(0.0);

    let requested = context.scroll_offset(container.id);
    let (clamped, clamped_offset) = clamp_scroll_offset(requested, max_x, max_y);
    if clamped {
        context.push_diagnostic(
            container.id,
            LayoutDiagnosticCode::InvalidScrollOffsetClamped,
            "scroll offset was out of bounds and was clamped",
        );
    }

    let origin = Point::new(
        content.min.x + slot.margin.left - clamped_offset.x,
        content.min.y + slot.margin.top - clamped_offset.y,
    );
    let rect = Rect::from_min_size(origin, Vector2::new(width, height));
    context.record_slot_margin(child.child.id(), rect, slot.margin);

    let viewport_rect = Rect::from_min_size(
        Point::new(
            content.min.x + slot.margin.left,
            content.min.y + slot.margin.top,
        ),
        Vector2::new(viewport_w, viewport_h),
    );
    context.record_viewport_bounds(container.id, viewport_rect);

    if !layout_virtualized_child(
        container,
        child,
        rect,
        viewport_rect,
        clamped_offset,
        context,
    ) {
        layout_node(&child.child, rect, context);
    }

    if width > viewport_w || height > viewport_h {
        context.record_overflow(
            container.id,
            container.policy.overflow,
            width > viewport_w,
            height > viewport_h,
        );
    }
}

fn virtual_fixed_content_size(
    container: &ContainerNode,
    child: &LayoutNode,
    viewport_w: f32,
    viewport_h: f32,
) -> Option<Vector2> {
    let policy = container.policy.virtualization?;
    if !policy.enabled {
        return None;
    }
    let LayoutNode::Container(content) = child else {
        return None;
    };
    let horizontal = match (content.policy.kind, policy.axis) {
        (ContainerKind::Row, VirtualizationAxis::Horizontal) => true,
        (ContainerKind::Column, VirtualizationAxis::Vertical) => false,
        _ => return None,
    };
    let main = known_linear_main_extent(content, policy.axis)?;
    Some(if horizontal {
        Vector2::new(main, viewport_h)
    } else {
        Vector2::new(viewport_w, main)
    })
}

//! Wrap container layout strategy.

use super::layout_node;
use super::linear::resolve_nonfill_main;
use crate::gui::layout_core::engine::LayoutContext;
use crate::gui::layout_core::engine::direct::direct_widget_measure;
use crate::gui::layout_core::tree::ContainerNode;
use crate::gui::types::{Point, Rect, Vector2};

pub(super) fn layout_wrap(container: &ContainerNode, content: Rect, context: &mut LayoutContext) {
    let item_gap = container.policy.wrap.item_gap.max(0.0);
    let line_gap = container.policy.wrap.line_gap.max(0.0);

    let mut line_x = content.min.x;
    let mut line_y = content.min.y;
    let mut line_h = 0.0;

    for child in &container.children {
        let measured = direct_widget_measure(child).unwrap_or_else(|| {
            super::super::measure::measure_node(&child.child, child.slot.constraints, context)
        });
        let width = resolve_nonfill_main(
            true,
            child,
            measured,
            content.width(),
            context,
            child.child.id(),
        );
        let height = resolve_nonfill_main(
            false,
            child,
            measured,
            content.height(),
            context,
            child.child.id(),
        );
        let span_w = width + child.slot.margin.left + child.slot.margin.right;

        if line_x > content.min.x && (line_x + span_w) > content.max.x {
            line_x = content.min.x;
            line_y += line_h + line_gap;
            line_h = 0.0;
        }

        let item_rect = Rect::from_min_size(
            Point::new(
                line_x + child.slot.margin.left,
                line_y + child.slot.margin.top,
            ),
            Vector2::new(width, height),
        );
        context.record_slot_margin(child.child.id(), item_rect, child.slot.margin);
        layout_node(&child.child, item_rect, context);
        line_x += span_w + item_gap;
        line_h = line_h.max(height + child.slot.margin.top + child.slot.margin.bottom);
    }

    if (line_y + line_h) > content.max.y {
        context.record_overflow(container.id, container.policy.overflow, false, true);
    }
}

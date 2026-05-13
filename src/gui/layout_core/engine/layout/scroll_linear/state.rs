//! Virtualized linear child state measurement.

use crate::gui::layout_core::engine::LayoutContext;
use crate::gui::layout_core::engine::helpers::LinearLayoutState;
use crate::gui::layout_core::engine::measure::measure_node;
use crate::gui::layout_core::model::SizeModeMain;
use crate::gui::layout_core::tree::{ContainerNode, LayoutNode, SlotChild, WidgetNode};
use crate::gui::types::Vector2;

pub(super) fn collect_layout_states<'a>(
    container: &'a ContainerNode,
    context: &mut LayoutContext,
    horizontal: bool,
    available_main: f32,
) -> Vec<LinearLayoutState<'a>> {
    let mut states = Vec::with_capacity(container.children.len());
    for child in &container.children {
        let measured = if matches!(child.slot.size_main, SizeModeMain::Intrinsic) {
            direct_widget_intrinsic_size(&child.child)
                .unwrap_or_else(|| measure_node(&child.child, child.slot.constraints, context))
        } else {
            Vector2::default()
        };
        let main = resolve_main_for_virtual(horizontal, child, measured, available_main, context);
        states.push(LinearLayoutState::new(child, measured, main));
    }
    states
}

pub(super) fn direct_widget_intrinsic_size(node: &LayoutNode) -> Option<Vector2> {
    let LayoutNode::Widget(WidgetNode { intrinsic, .. }) = node else {
        return None;
    };
    (intrinsic.x.is_finite() && intrinsic.y.is_finite())
        .then_some(Vector2::new(intrinsic.x.max(0.0), intrinsic.y.max(0.0)))
}

fn resolve_main_for_virtual(
    horizontal: bool,
    slot_child: &SlotChild,
    measured: Vector2,
    available_main: f32,
    context: &mut LayoutContext,
) -> f32 {
    let raw = match slot_child.slot.size_main {
        SizeModeMain::Fixed(value) => value,
        SizeModeMain::Intrinsic => {
            if horizontal {
                measured.x
            } else {
                measured.y
            }
        }
        SizeModeMain::Percent(percent) => available_main * percent.clamp(0.0, 1.0),
        SizeModeMain::Fill(_) => available_main,
    };
    context.clamp_main(
        slot_child.child.id(),
        horizontal,
        slot_child.slot.constraints,
        raw,
    )
}

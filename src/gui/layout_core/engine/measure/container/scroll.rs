//! Scroll container measurement strategy.

use super::super::measure_node;
use crate::gui::layout_core::constraints::Constraints;
use crate::gui::layout_core::engine::LayoutContext;
use crate::gui::layout_core::tree::ContainerNode;
use crate::gui::types::Vector2;

pub(super) fn measure_scroll_view(
    container: &ContainerNode,
    constraints: Constraints,
    context: &mut LayoutContext,
) -> Vector2 {
    if container.policy.virtualization.is_none()
        && let Some(child) = container.children.first()
        && crate::gui::layout_core::validated_geometry::finite_inset_totals(child.slot.margin)
            .is_some()
    {
        let _ = measure_node(&child.child, child.slot.constraints, context);
    }
    Vector2::new(constraints.max_w, constraints.max_h)
}

//! Container measurement strategies for the layout engine.

mod boxes;
mod grid;
mod linear;
mod scroll;
mod wrap;

use super::super::LayoutContext;
use crate::gui::layout_core::constraints::Constraints;
use crate::gui::layout_core::model::ContainerKind;
use crate::gui::layout_core::tree::ContainerNode;
use crate::gui::types::Vector2;

pub(super) fn measure_container(
    container: &ContainerNode,
    constraints: Constraints,
    context: &mut LayoutContext,
) -> Vector2 {
    let policy = &container.policy;
    let inner = context.normalize_constraints(
        container.id,
        constraints.inset(
            policy.padding.horizontal() * 0.5,
            policy.padding.vertical() * 0.5,
        ),
    );
    let measured_inner = match policy.kind {
        ContainerKind::Row => {
            linear::measure_linear(true, &container.children, inner, policy.spacing, context)
        }
        ContainerKind::Column => {
            linear::measure_linear(false, &container.children, inner, policy.spacing, context)
        }
        ContainerKind::Stack | ContainerKind::AlignBox | ContainerKind::PaddingBox => {
            boxes::measure_stack(&container.children, inner, context)
        }
        ContainerKind::AspectBox => boxes::measure_aspect_box(container, inner, context),
        ContainerKind::Grid => grid::measure_grid(container, inner, context),
        ContainerKind::ScrollView => scroll::measure_scroll_view(container, inner, context),
        ContainerKind::Wrap => wrap::measure_wrap(container, inner, context),
        ContainerKind::SwitchLayout => boxes::measure_switch_layout(container, inner, context),
    };

    Vector2::new(
        constraints.clamp_w(measured_inner.x + policy.padding.horizontal()),
        constraints.clamp_h(measured_inner.y + policy.padding.vertical()),
    )
}

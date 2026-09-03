//! Container measurement strategies for the layout engine.

mod boxes;
mod custom;
mod grid;
mod linear;
mod scroll;
mod split_pane;
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
    let horizontal_padding = policy.padding.horizontal();
    let vertical_padding = policy.padding.vertical();
    let inner = context.normalize_constraints(
        container.id,
        constraints.inset(horizontal_padding * 0.5, vertical_padding * 0.5),
    );
    let measured_inner = if let Some(layout_policy) = container.layout_policy() {
        custom::measure_custom(container, layout_policy, inner, context)
    } else {
        match policy.kind {
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
            ContainerKind::FloatingLayer => boxes::measure_floating_layer(container, context),
            ContainerKind::SplitPane => split_pane::measure_split_pane(container, inner, context),
        }
    };

    Vector2::new(
        finite_padded_extent(
            measured_inner.x,
            horizontal_padding,
            constraints.min_w,
            constraints.max_w,
        ),
        finite_padded_extent(
            measured_inner.y,
            vertical_padding,
            constraints.min_h,
            constraints.max_h,
        ),
    )
}

fn finite_padded_extent(inner: f32, padding: f32, minimum: f32, maximum: f32) -> f32 {
    let padded = inner + padding;
    if !padded.is_finite() {
        return minimum;
    }

    let clamped = padded.clamp(minimum, maximum);
    if clamped.is_finite() {
        clamped
    } else {
        minimum
    }
}

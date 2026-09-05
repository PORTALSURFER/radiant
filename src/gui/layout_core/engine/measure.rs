//! Measure pass implementation for strict slot-based layout trees.

mod container;

use super::{LayoutContext, MeasureCacheKey};
use crate::gui::layout_core::constraints::Constraints;
use crate::gui::layout_core::tree::LayoutNode;
use crate::gui::types::Vector2;

pub(super) fn measure_node(
    node: &LayoutNode,
    constraints: Constraints,
    context: &mut LayoutContext,
) -> Vector2 {
    if let Some(trace) = context.fragment_trace.as_mut() {
        trace.begin_measure(node, constraints);
    }
    let measured = measure_node_inner(node, constraints, context);
    if let Some(trace) = context.fragment_trace.as_mut() {
        trace.end_measure();
    }
    measured
}

fn measure_node_inner(
    node: &LayoutNode,
    constraints: Constraints,
    context: &mut LayoutContext,
) -> Vector2 {
    let normalized = context.normalize_constraints(node.id(), constraints);
    let key = MeasureCacheKey::new(node, normalized);
    let is_container = matches!(node, LayoutNode::Container(_));
    let contains_custom_policy = node.contains_layout_policy();
    if !contains_custom_policy
        && let Some(size) = context.cached_measure(key, node.id(), is_container)
    {
        context.record_measured_size(node.id(), size);
        return size;
    }
    context.record_measure_miss();

    let measured = match node {
        LayoutNode::Widget(widget) => Vector2::new(
            context.clamp_width(widget.id, normalized, widget.intrinsic.x),
            context.clamp_height(widget.id, normalized, widget.intrinsic.y),
        ),
        LayoutNode::Container(container) => {
            container::measure_container(container, normalized, context)
        }
    };
    if contains_custom_policy {
        context.record_measured_size(node.id(), measured);
    } else {
        context.remember_measure(key, measured);
    }
    measured
}

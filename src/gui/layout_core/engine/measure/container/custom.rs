//! Custom layout-policy measurement.

use super::super::super::super::policy::{MeasureChildren, MeasureChildrenError};
use super::super::super::super::tree::ContainerNode;
use super::super::super::LayoutContext;
use crate::gui::layout_core::constraints::Constraints;
use crate::gui::layout_core::policy::LayoutPolicy;
use crate::gui::layout_core::tree::NodeId;
use crate::gui::types::Vector2;
use std::collections::HashMap;

pub(super) fn measure_custom(
    container: &ContainerNode,
    layout_policy: &dyn LayoutPolicy,
    inner: Constraints,
    context: &mut LayoutContext,
) -> Vector2 {
    let mut requests = HashMap::new();
    let mut errors = Vec::new();
    let count = container.children.len();
    let mut measure = |index: usize, requested: Constraints| {
        let requested = context.normalize_constraints(container.id, requested);
        let child = &container.children[index];
        let slot_constraints =
            context.normalize_constraints(child.child.id(), child.slot.constraints);
        let bounded = bounded_constraints(requested, slot_constraints);
        let key = ChildMeasureKey::new(index, child.child.id(), bounded);
        if let Some(measured) = requests.get(&key).copied() {
            return measured;
        }
        let measured = super::super::measure_node(&child.child, bounded, context);
        requests.insert(key, measured);
        measured
    };
    let hint = {
        let mut children = MeasureChildren::new(count, &mut measure, &mut errors);
        layout_policy.measure(&mut children, inner)
    };

    for error in errors {
        match error {
            MeasureChildrenError::InvalidIndex { .. } => context.push_diagnostic(
                container.id,
                crate::gui::layout_core::engine::LayoutDiagnosticCode::CustomLayoutInvalidChildIndex,
                "custom layout policy requested an invalid child index",
            ),
        }
    }
    if hint.has_non_finite_values() {
        context.push_diagnostic(
            container.id,
            crate::gui::layout_core::engine::LayoutDiagnosticCode::CustomLayoutHintNonFinite,
            "custom layout policy returned a non-finite size hint value",
        );
    }
    if hint.has_negative_values() {
        context.push_diagnostic(
            container.id,
            crate::gui::layout_core::engine::LayoutDiagnosticCode::CustomLayoutHintNegative,
            "custom layout policy returned a negative size hint value",
        );
    }
    if hint.has_contradictory_values() {
        context.push_diagnostic(
            container.id,
            crate::gui::layout_core::engine::LayoutDiagnosticCode::CustomLayoutHintContradictory,
            "custom layout policy returned contradictory size hint values",
        );
    }

    let measured_inner = hint.resolve(inner);
    Vector2::new(measured_inner.x, measured_inner.y)
}

fn bounded_constraints(requested: Constraints, slot: Constraints) -> Constraints {
    let (min_w, max_w) = bounded_axis(requested.min_w, requested.max_w, slot.min_w, slot.max_w);
    let (min_h, max_h) = bounded_axis(requested.min_h, requested.max_h, slot.min_h, slot.max_h);
    Constraints::new(min_w, max_w, min_h, max_h)
}

fn bounded_axis(
    requested_min: f32,
    requested_max: f32,
    slot_min: f32,
    slot_max: f32,
) -> (f32, f32) {
    if requested_max < slot_min {
        (slot_min, slot_min)
    } else if requested_min > slot_max {
        (slot_max, slot_max)
    } else {
        (requested_min.max(slot_min), requested_max.min(slot_max))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct ChildMeasureKey {
    index: usize,
    child_id: NodeId,
    min_w: u32,
    max_w: u32,
    min_h: u32,
    max_h: u32,
}

impl ChildMeasureKey {
    fn new(index: usize, child_id: NodeId, constraints: Constraints) -> Self {
        Self {
            index,
            child_id,
            min_w: constraints.min_w.to_bits(),
            max_w: constraints.max_w.to_bits(),
            min_h: constraints.min_h.to_bits(),
            max_h: constraints.max_h.to_bits(),
        }
    }
}

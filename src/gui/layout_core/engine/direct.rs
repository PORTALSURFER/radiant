//! Direct leaf sizing helpers for hot layout paths.

use crate::gui::layout_core::constraints::Constraints;
use crate::gui::layout_core::tree::{LayoutNode, SlotChild};
use crate::gui::types::Vector2;

pub(super) fn direct_widget_measure(child: &SlotChild) -> Option<Vector2> {
    let LayoutNode::Widget(widget) = &child.child else {
        return None;
    };
    let constraints = clean_constraints(child.slot.constraints)?;
    let intrinsic = widget.intrinsic;
    if !intrinsic.x.is_finite()
        || !intrinsic.y.is_finite()
        || intrinsic.x < 0.0
        || intrinsic.y < 0.0
    {
        return None;
    }
    Some(Vector2::new(
        constraints.clamp_w(intrinsic.x),
        constraints.clamp_h(intrinsic.y),
    ))
}

fn clean_constraints(constraints: Constraints) -> Option<Constraints> {
    (constraints == constraints.normalized()
        && constraints.min_w.is_finite()
        && constraints.min_h.is_finite()
        && constraints.max_w >= constraints.min_w
        && constraints.max_h >= constraints.min_h)
        .then_some(constraints)
}

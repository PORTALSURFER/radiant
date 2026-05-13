//! Known linear metric precomputation for derived layout-tree state.

use super::super::{LayoutNode, SlotChild};
use crate::gui::layout_core::model::SizeModeMain;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(in crate::gui::layout_core::tree) struct KnownMainMetrics {
    pub(in crate::gui::layout_core::tree) extent: Option<f32>,
    pub(in crate::gui::layout_core::tree) uniform_main: Option<f32>,
}

pub(super) fn known_main_metrics(
    horizontal: bool,
    spacing: f32,
    children: &[SlotChild],
) -> KnownMainMetrics {
    let spacing_total = spacing.max(0.0) * children.len().saturating_sub(1) as f32;
    let mut total = spacing_total;
    let mut uniform_main: Option<f32> = None;
    let mut uniform_valid = true;
    for child in children {
        let has_main_margin = has_main_axis_margin(horizontal, child);
        let size = match child.slot.size_main {
            SizeModeMain::Fixed(size) => size.max(0.0),
            SizeModeMain::Intrinsic => match direct_widget_intrinsic_main(horizontal, &child.child)
            {
                Some(size) => size,
                None => return KnownMainMetrics::default(),
            },
            SizeModeMain::Fill(_) | SizeModeMain::Percent(_) => return KnownMainMetrics::default(),
        };
        let constraints = child.slot.constraints.normalized();
        let main = if horizontal {
            constraints.clamp_w(size) + child.slot.margin.left + child.slot.margin.right
        } else {
            constraints.clamp_h(size) + child.slot.margin.top + child.slot.margin.bottom
        };
        let item_main = if horizontal {
            constraints.clamp_w(size)
        } else {
            constraints.clamp_h(size)
        };
        if has_main_margin {
            uniform_valid = false;
        } else if let Some(expected) = uniform_main {
            if (expected - item_main).abs() > f32::EPSILON {
                uniform_valid = false;
            }
        } else if uniform_main.is_none() {
            uniform_main = Some(item_main);
        }
        total += main;
    }
    KnownMainMetrics {
        extent: Some(total),
        uniform_main: uniform_valid.then_some(uniform_main).flatten(),
    }
}

fn direct_widget_intrinsic_main(horizontal: bool, node: &LayoutNode) -> Option<f32> {
    let LayoutNode::Widget(widget) = node else {
        return None;
    };
    let main = if horizontal {
        widget.intrinsic.x
    } else {
        widget.intrinsic.y
    };
    main.is_finite().then_some(main.max(0.0))
}

fn has_main_axis_margin(horizontal: bool, child: &SlotChild) -> bool {
    let (before, after) = if horizontal {
        (child.slot.margin.left, child.slot.margin.right)
    } else {
        (child.slot.margin.top, child.slot.margin.bottom)
    };
    before.abs() > f32::EPSILON || after.abs() > f32::EPSILON
}

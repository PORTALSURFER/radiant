//! Deterministic two-pass layout engine for strict slot-based trees.

mod helpers;
mod layout;
mod measure;

use super::constraints::Constraints;
use super::tree::{LayoutNode, NodeId};
use crate::gui::types::{Point, Rect, Vector2};
use std::collections::{BTreeMap, BTreeSet, HashMap};

/// Layout diagnostic emitted when invalid states are normalized.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LayoutDiagnostic {
    /// Node that triggered the diagnostic.
    pub node_id: NodeId,
    /// Human-readable diagnostic message.
    pub message: String,
}

/// Final layout output from `layout_tree`.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct LayoutOutput {
    /// Final rounded rectangles by node id.
    pub rects: BTreeMap<NodeId, Rect>,
    /// Node ids that overflowed available space.
    pub overflowed: BTreeSet<NodeId>,
    /// Diagnostics collected during measure/layout normalization.
    pub diagnostics: Vec<LayoutDiagnostic>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct ConstraintKey {
    min_w: u32,
    max_w: u32,
    min_h: u32,
    max_h: u32,
}

impl ConstraintKey {
    fn from_constraints(constraints: Constraints) -> Self {
        Self {
            min_w: constraints.min_w.to_bits(),
            max_w: constraints.max_w.to_bits(),
            min_h: constraints.min_h.to_bits(),
            max_h: constraints.max_h.to_bits(),
        }
    }
}

#[derive(Default)]
struct LayoutContext {
    measured: HashMap<(NodeId, ConstraintKey), Vector2>,
    output: LayoutOutput,
}

/// Measure and layout a strict slot tree into rounded rectangles.
pub(crate) fn layout_tree(root: &LayoutNode, root_rect: Rect) -> LayoutOutput {
    let constraints = Constraints::new(0.0, root_rect.width().max(0.0), 0.0, root_rect.height());
    let mut context = LayoutContext::default();
    measure::measure_node(root, constraints, &mut context);
    layout::layout_node(root, round_rect(root_rect), &mut context);
    context.output
}

fn round_rect(rect: Rect) -> Rect {
    let min_x = rect.min.x.floor();
    let min_y = rect.min.y.floor();
    let width = rect.width().round().max(0.0);
    let height = rect.height().round().max(0.0);
    Rect::from_min_size(Point::new(min_x, min_y), Vector2::new(width, height))
}

#[cfg(test)]
mod tests {
    use super::layout_tree;
    use crate::gui::layout_core::constraints::Constraints;
    use crate::gui::layout_core::model::{
        ContainerKind, ContainerPolicy, SizeModeCross, SizeModeMain, SlotParams,
    };
    use crate::gui::layout_core::tree::{LayoutNode, SlotChild};
    use crate::gui::types::{Point, Rect, Vector2};

    #[test]
    fn layout_tree_is_deterministic_for_same_input() {
        let child_a = LayoutNode::widget(2, Vector2::new(32.0, 20.0));
        let child_b = LayoutNode::widget(3, Vector2::new(64.0, 20.0));
        let root = LayoutNode::container(
            1,
            ContainerPolicy {
                kind: ContainerKind::Row,
                spacing: 8.0,
                ..ContainerPolicy::default()
            },
            vec![
                SlotChild {
                    slot: SlotParams::fill(),
                    child: child_a,
                },
                SlotChild {
                    slot: SlotParams::fill(),
                    child: child_b,
                },
            ],
        );
        let rect = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(400.0, 80.0));
        let first = layout_tree(&root, rect);
        let second = layout_tree(&root, rect);
        assert_eq!(first.rects, second.rects);
        assert_eq!(first.overflowed, second.overflowed);
    }

    #[test]
    fn fill_children_compress_before_fixed_children() {
        let fill_a = LayoutNode::widget(2, Vector2::new(200.0, 20.0));
        let fixed = LayoutNode::widget(3, Vector2::new(80.0, 20.0));
        let root = LayoutNode::container(
            1,
            ContainerPolicy {
                kind: ContainerKind::Row,
                ..ContainerPolicy::default()
            },
            vec![
                SlotChild {
                    slot: SlotParams::fill(),
                    child: fill_a,
                },
                SlotChild {
                    slot: SlotParams {
                        size_main: SizeModeMain::Fixed(80.0),
                        size_cross: SizeModeCross::Fill,
                        constraints: Constraints::new(80.0, 80.0, 0.0, f32::INFINITY),
                        margin: Default::default(),
                        align_cross_override: None,
                        allow_fixed_compress: false,
                    },
                    child: fixed,
                },
            ],
        );
        let rect = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(100.0, 40.0));
        let output = layout_tree(&root, rect);
        let fixed_rect = output.rects.get(&3).expect("fixed rect");
        assert!((fixed_rect.width() - 80.0).abs() < 0.5);
    }
}

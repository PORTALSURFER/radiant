//! Path-local evidence and leaf swapping for the interaction-only refresh.

use super::node::SurfaceLayerChildKind;
use super::revision::{InteractionLeafRevision, classify_interaction_leaf};
use super::{SurfaceNode, SurfaceWidget, UiSurface, WidgetPath};

/// Evidence returned after inspecting one selected path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct InteractionPathEvidence {
    pub(crate) relation: InteractionLeafRevision,
    pub(crate) previous_membership: [bool; 7],
    pub(crate) current_membership: [bool; 7],
}

impl<Message> UiSurface<Message> {
    /// Swap one already validated widget leaf with the corresponding leaf in a
    /// complete successor.  The operation is infallible after preflight.
    pub(in crate::runtime) fn swap_widget_at_path(
        &mut self,
        successor: &mut Self,
        widget_id: crate::widgets::WidgetId,
        child_path: &WidgetPath,
    ) -> bool {
        let Some(installed) = self.find_widget_mut_at_path(widget_id, child_path) else {
            return false;
        };
        let Some(replacement) = successor.find_widget_mut_at_path(widget_id, child_path) else {
            return false;
        };
        std::mem::swap(installed, replacement);
        true
    }
}

/// Inspect only the nodes on one selected root-to-leaf path.
pub(crate) fn inspect_interaction_path<Message>(
    previous: &SurfaceNode<Message>,
    current: &SurfaceNode<Message>,
    path: &[usize],
) -> Option<InteractionPathEvidence> {
    if !same_node_witness(previous, current) {
        return None;
    }
    match (previous, current, path.split_first()) {
        (SurfaceNode::Widget(previous), SurfaceNode::Widget(current), None) => {
            if !same_source_metadata(previous.source.as_deref(), current.source.as_deref()) {
                return None;
            }
            Some(InteractionPathEvidence {
                relation: classify_interaction_leaf(previous, current),
                previous_membership: membership(previous),
                current_membership: membership(current),
            })
        }
        (
            SurfaceNode::Container(previous),
            SurfaceNode::Container(current),
            Some((index, rest)),
        ) => {
            if previous.layout_policy.is_some()
                || current.layout_policy.is_some()
                || previous.layout_capabilities.is_some()
                || current.layout_capabilities.is_some()
                || previous.policy != current.policy
                || previous.style != current.style
                || previous.hoverable != current.hoverable
                || previous.split_pane_runtime != current.split_pane_runtime
                || previous.children.len() != current.children.len()
                || previous
                    .scroll_mapper_descriptor()
                    .relation(&current.scroll_mapper_descriptor())
                    != super::widget::MapperRelation::Unchanged
            {
                return None;
            }
            let (Some(previous_child), Some(current_child)) =
                (previous.children.get(*index), current.children.get(*index))
            else {
                return None;
            };
            if previous_child.slot != current_child.slot {
                return None;
            }
            inspect_interaction_path(&previous_child.child, &current_child.child, rest)
        }
        (SurfaceNode::Scene(previous), SurfaceNode::Scene(current), Some((index, rest))) => {
            if *index == 0 || previous.layers.len() != current.layers.len() {
                return None;
            }
            let previous_child_descriptor = previous.ordered_layer_child_for_child(*index - 1)?;
            let current_child_descriptor = current.ordered_layer_child_for_child(*index - 1)?;
            if previous_child_descriptor != current_child_descriptor {
                return None;
            }
            let (layer_index, kind) = previous_child_descriptor;
            let previous_child = match kind {
                SurfaceLayerChildKind::Input => previous.layers[layer_index].input.as_ref()?,
                SurfaceLayerChildKind::Foreground => &previous.layers[layer_index].node,
            };
            let current_child = match kind {
                SurfaceLayerChildKind::Input => current.layers[layer_index].input.as_ref()?,
                SurfaceLayerChildKind::Foreground => &current.layers[layer_index].node,
            };
            inspect_interaction_path(previous_child, current_child, rest)
        }
        (
            SurfaceNode::FloatingLayer(previous),
            SurfaceNode::FloatingLayer(current),
            Some((index, rest)),
        ) => {
            if previous.interactive != current.interactive
                || !previous.interactive
                || previous.container.children.len() != current.container.children.len()
            {
                return None;
            }
            let (Some(previous_child), Some(current_child)) = (
                previous.container.children.get(*index),
                current.container.children.get(*index),
            ) else {
                return None;
            };
            if previous_child.slot != current_child.slot {
                return None;
            }
            inspect_interaction_path(&previous_child.child, &current_child.child, rest)
        }
        _ => None,
    }
}

fn same_node_witness<Message>(
    previous: &SurfaceNode<Message>,
    current: &SurfaceNode<Message>,
) -> bool {
    use std::mem::discriminant;
    if discriminant(previous) != discriminant(current) || previous.id() != current.id() {
        return false;
    }
    same_source_metadata(
        previous.source_metadata_handle().as_deref(),
        current.source_metadata_handle().as_deref(),
    )
}

fn same_source_metadata(
    previous: Option<&super::SourceMetadata>,
    current: Option<&super::SourceMetadata>,
) -> bool {
    match (previous, current) {
        (Some(previous), Some(current)) => {
            previous.identity == current.identity && previous.compatibility == current.compatibility
        }
        (None, None) => true,
        _ => false,
    }
}

fn membership<Message>(widget: &SurfaceWidget<Message>) -> [bool; 7] {
    [
        widget.is_focusable(),
        widget.is_keyboard_focusable(),
        widget.receives_pointer_hit_testing(),
        widget.receives_wheel_input(),
        widget.accepts_native_file_drop(),
        widget.needs_state_synchronization(),
        widget.suppresses_container_hover(),
    ]
}

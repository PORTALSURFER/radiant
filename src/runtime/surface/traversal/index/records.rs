use crate::{
    gui::layout_core::{
        SplitPaneDividerDescriptor, SplitPaneRuntimeOwnership, SplitPaneRuntimePolicyRevision,
        SplitPaneRuntimeStateInput,
    },
    layout::{
        ContainerStateDeclaration, ContainerStateId, LayoutInteraction, LayoutInteractionRevision,
        LayoutTargetIdentity, NodeId,
    },
    widgets::WidgetId,
};
use std::rc::Rc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::runtime) enum WheelHitTarget {
    Widget(WidgetId),
    ScrollContainer(NodeId),
}

impl WheelHitTarget {
    pub(in crate::runtime) const fn node_id(self) -> NodeId {
        match self {
            Self::Widget(id) | Self::ScrollContainer(id) => id,
        }
    }
}

/// Non-authorizing source evidence for one runtime-owned split separator.
///
/// The marker records only the declarative boundary and its source behavior.
/// It becomes usable only when the committed controller finds one exact
/// current separator projection with the same evidence and mounted generation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::runtime) struct SurfaceSplitPaneFocusOrderCandidate {
    pub(in crate::runtime) widget_index: usize,
    pub(in crate::runtime) target: LayoutTargetIdentity,
    pub(in crate::runtime) state_id: ContainerStateId,
    pub(in crate::runtime) descriptor: SplitPaneDividerDescriptor,
    pub(in crate::runtime) ownership: SplitPaneRuntimeOwnership,
    pub(in crate::runtime) contract_version: u16,
    pub(in crate::runtime) state_schema_version: u16,
    pub(in crate::runtime) policy_revision: SplitPaneRuntimePolicyRevision,
}

/// Non-authorizing source evidence for one runtime-owned split ratio action.
///
/// This deliberately remains separate from the observational separator
/// projection and from the focus-order candidate. The controller pairs it
/// with one exact committed layout target and mounted state before creating an
/// action authority.
pub(in crate::runtime) struct SurfaceSplitPaneRatioActionCandidate<Message> {
    pub(in crate::runtime) target: LayoutTargetIdentity,
    pub(in crate::runtime) state_id: ContainerStateId,
    pub(in crate::runtime) descriptor: SplitPaneDividerDescriptor,
    pub(in crate::runtime) ownership: SplitPaneRuntimeOwnership,
    pub(in crate::runtime) contract_version: u16,
    pub(in crate::runtime) state_schema_version: u16,
    pub(in crate::runtime) policy_revision: SplitPaneRuntimePolicyRevision,
    pub(in crate::runtime) on_ratio_settled: Option<Rc<dyn Fn(f32) -> Message>>,
}

pub(in crate::runtime) struct SurfaceContainerTraversalRecord<'a, Message> {
    pub(in crate::runtime) id: NodeId,
    pub(in crate::runtime) clipped_by: &'a [NodeId],
    pub(in crate::runtime) scroll_content: Option<NodeId>,
    pub(in crate::runtime) styled_hoverable: bool,
    pub(in crate::runtime) layout_interaction: Option<SurfaceLayoutInteractionRecord<Message>>,
    pub(in crate::runtime) split_pane_runtime: Option<SplitPaneRuntimeStateInput>,
    pub(in crate::runtime) split_pane_divider: Option<SplitPaneDividerDescriptor>,
    pub(in crate::runtime) split_pane_ratio_action:
        Option<SurfaceSplitPaneRatioActionCandidate<Message>>,
    pub(in crate::runtime) virtual_layout:
        Option<super::super::super::VirtualLayoutRegistration<Message>>,
}

pub(in crate::runtime) struct SurfaceLayoutInteractionRecord<Message> {
    pub(in crate::runtime) path: super::WidgetPath,
    pub(in crate::runtime) gesture_qualified: bool,
    pub(in crate::runtime) id: NodeId,
    pub(in crate::runtime) contract_version: u16,
    pub(in crate::runtime) interaction: Rc<dyn LayoutInteraction<Message>>,
    pub(in crate::runtime) revision: LayoutInteractionRevision,
    pub(in crate::runtime) state: Option<ContainerStateDeclaration>,
    pub(in crate::runtime) foreign_state_declaration: bool,
}

pub(in crate::runtime) struct SurfaceWidgetTraversalRecord<'a> {
    pub(in crate::runtime) id: WidgetId,
    pub(in crate::runtime) child_path: &'a [usize],
    pub(in crate::runtime) clipped_by: &'a [NodeId],
    pub(in crate::runtime) focusable: bool,
    pub(in crate::runtime) keyboard_focusable: bool,
    pub(in crate::runtime) receives_pointer_hit_testing: bool,
    pub(in crate::runtime) receives_wheel_input: bool,
    pub(in crate::runtime) accepts_native_file_drop: bool,
    pub(in crate::runtime) needs_state_synchronization: bool,
    pub(in crate::runtime) suppresses_container_hover: bool,
}

use crate::{
    gui::layout_core::SplitPaneRuntimeStateInput,
    layout::{ContainerStateDeclaration, LayoutInteraction, LayoutInteractionRevision, NodeId},
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

pub(in crate::runtime) struct SurfaceContainerTraversalRecord<'a, Message> {
    pub(in crate::runtime) id: NodeId,
    pub(in crate::runtime) clipped_by: &'a [NodeId],
    pub(in crate::runtime) scroll_content: Option<NodeId>,
    pub(in crate::runtime) styled_hoverable: bool,
    pub(in crate::runtime) layout_interaction: Option<SurfaceLayoutInteractionRecord<Message>>,
    pub(in crate::runtime) split_pane_runtime: Option<SplitPaneRuntimeStateInput>,
    pub(in crate::runtime) virtual_layout:
        Option<super::super::super::VirtualLayoutRegistration<Message>>,
}

pub(in crate::runtime) struct SurfaceLayoutInteractionRecord<Message> {
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

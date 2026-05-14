use super::ViewLowering;
use crate::{
    layout::{ContainerPolicy, NodeId},
    runtime::{SurfaceChild, SurfaceNode},
    widgets::WidgetStyle,
};

impl ViewLowering<'_> {
    pub(super) fn lower_container<Message>(
        &mut self,
        id: NodeId,
        policy: ContainerPolicy,
        style: Option<WidgetStyle>,
        hoverable: bool,
        children: Vec<SurfaceChild<Message>>,
    ) -> SurfaceNode<Message> {
        if let Some(style) = style {
            SurfaceNode::styled_container(id, policy, style, children)
                .with_container_hoverable(hoverable)
        } else {
            SurfaceNode::container(id, policy, children)
        }
    }
}

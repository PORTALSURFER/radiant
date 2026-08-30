use super::ViewLowering;
use crate::{
    layout::{ContainerPolicy, LayoutPolicy, NodeId},
    runtime::{SurfaceChild, SurfaceNode},
    widgets::WidgetStyle,
};
use std::rc::Rc;

impl<Message> ViewLowering<'_, Message> {
    pub(super) fn lower_container(
        &mut self,
        id: NodeId,
        policy: ContainerPolicy,
        layout_policy: Option<Rc<dyn LayoutPolicy>>,
        style: Option<WidgetStyle>,
        hoverable: bool,
        children: Vec<SurfaceChild<Message>>,
    ) -> SurfaceNode<Message> {
        let container = if let Some(style) = style {
            SurfaceNode::styled_container(id, policy, style, children)
                .with_container_hoverable(hoverable)
        } else {
            SurfaceNode::container(id, policy, children)
        };
        if let Some(layout_policy) = layout_policy {
            container.with_layout_policy_erased(layout_policy)
        } else {
            container
        }
    }
}

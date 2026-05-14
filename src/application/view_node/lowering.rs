use super::{ViewNode, ViewNodeKind};
use crate::{
    application::{
        IdGenerator, IntoView, ROOT_KEY_SCOPE, WidgetViewContext,
        view_node::lowering_defaults::ViewNodeContainerDefaults,
    },
    layout::{
        ContainerKind, ContainerPolicy, GridPolicy, NodeId, VirtualizationAxis,
        VirtualizationPolicy,
    },
    runtime::{SurfaceChild, SurfaceNode},
};

#[path = "lowering/children.rs"]
mod children;
#[path = "lowering/containers.rs"]
mod containers;

impl<Message> IntoView<Message> for ViewNode<Message>
where
    Message: 'static,
{
    fn into_node(self) -> SurfaceNode<Message> {
        let mut reserved = Vec::new();
        self.collect_reserved_ids(ROOT_KEY_SCOPE, &mut reserved);
        let mut ids = IdGenerator::new(reserved);
        ViewLowering::new(&mut ids).lower_node(self, ROOT_KEY_SCOPE)
    }
}

pub(super) struct ViewLowering<'a> {
    ids: &'a mut IdGenerator,
}

impl<'a> ViewLowering<'a> {
    fn new(ids: &'a mut IdGenerator) -> Self {
        Self { ids }
    }

    fn next_node_id<Message>(&mut self, node: &ViewNode<Message>, scope: u64) -> NodeId {
        node.resolved_id(scope).unwrap_or_else(|| self.ids.next())
    }

    fn lower_node<Message>(&mut self, node: ViewNode<Message>, scope: u64) -> SurfaceNode<Message>
    where
        Message: 'static,
    {
        let id = self.next_node_id(&node, scope);
        let child_scope = id;
        let style = node.style;
        let hoverable = node.hoverable;
        let defaults =
            ViewNodeContainerDefaults::new(node.padding, node.align_main, node.align_cross, style);
        let base_policy = || defaults.base_policy();
        let styled_container =
            |lowering: &mut Self, policy: ContainerPolicy, children: Vec<SurfaceChild<Message>>| {
                lowering.lower_container(id, policy, style, hoverable, children)
            };

        match node.kind {
            ViewNodeKind::Runtime(node) => node,
            ViewNodeKind::Widget(widget) => widget.into_surface_node(WidgetViewContext {
                id,
                sizing: node.sizing,
                style,
                input_only: node.input_only,
                text_wrap: node.text_wrap,
                text_align: node.text_align,
            }),
            ViewNodeKind::Row { spacing, children } => {
                let policy = ContainerPolicy {
                    kind: ContainerKind::Row,
                    spacing,
                    ..base_policy()
                };
                let children = self.lower_slot_children(children, child_scope, true);
                styled_container(self, policy, children)
            }
            ViewNodeKind::Column { spacing, children } => {
                let policy = ContainerPolicy {
                    kind: ContainerKind::Column,
                    spacing,
                    ..base_policy()
                };
                let children = self.lower_slot_children(children, child_scope, false);
                styled_container(self, policy, children)
            }
            ViewNodeKind::Grid {
                columns,
                column_gap,
                row_gap,
                children,
            } => {
                let policy = ContainerPolicy {
                    kind: ContainerKind::Grid,
                    grid: GridPolicy {
                        columns,
                        column_gap,
                        row_gap,
                    },
                    ..base_policy()
                };
                let children = self.lower_slot_children(children, child_scope, false);
                styled_container(self, policy, children)
            }
            ViewNodeKind::Scroll { child } => {
                let policy = ContainerPolicy {
                    kind: ContainerKind::ScrollView,
                    overflow: crate::layout::OverflowPolicy::Scroll,
                    ..base_policy()
                };
                let children = vec![self.lower_fill_child(*child, child_scope)];
                styled_container(self, policy, children)
            }
            ViewNodeKind::VirtualScroll { child, overscan_px } => {
                let policy = ContainerPolicy {
                    kind: ContainerKind::ScrollView,
                    overflow: crate::layout::OverflowPolicy::Scroll,
                    virtualization: Some(VirtualizationPolicy {
                        enabled: true,
                        axis: VirtualizationAxis::Vertical,
                        overscan_px,
                    }),
                    ..base_policy()
                };
                let children = vec![self.lower_fill_child(*child, child_scope)];
                styled_container(self, policy, children)
            }
            ViewNodeKind::Stack { children } => {
                let policy = ContainerPolicy {
                    kind: ContainerKind::Stack,
                    ..base_policy()
                };
                let children = self.lower_fill_children(children, child_scope);
                styled_container(self, policy, children)
            }
            ViewNodeKind::OverlayPanel { rect, label } => {
                if let Some(label) = label {
                    SurfaceNode::overlay_panel(id, rect, label, style.unwrap_or_default())
                } else {
                    SurfaceNode::overlay_marker(id, rect, style.unwrap_or_default())
                }
            }
        }
    }
}

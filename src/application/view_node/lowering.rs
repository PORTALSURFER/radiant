use super::{ViewNode, ViewNodeKind};
use crate::{
    application::{IdGenerator, IntoView, ROOT_KEY_SCOPE, WidgetViewContext},
    layout::{
        ContainerKind, ContainerPolicy, CrossAlign, GridPolicy, MainAlign, VirtualizationAxis,
        VirtualizationPolicy,
    },
    runtime::{SurfaceChild, SurfaceNode},
};
use std::collections::HashSet;

impl<Message> IntoView<Message> for ViewNode<Message>
where
    Message: 'static,
{
    fn into_node(self) -> SurfaceNode<Message> {
        let mut reserved = HashSet::new();
        self.collect_reserved_ids(ROOT_KEY_SCOPE, &mut reserved);
        let mut ids = IdGenerator::new(reserved);
        self.lower(&mut ids, ROOT_KEY_SCOPE)
    }
}

impl<Message> ViewNode<Message>
where
    Message: 'static,
{
    fn lower(self, ids: &mut IdGenerator, scope: u64) -> SurfaceNode<Message> {
        let id = self.resolved_id(scope).unwrap_or_else(|| ids.next());
        let child_scope = id;
        match self.kind {
            ViewNodeKind::Runtime(node) => node,
            ViewNodeKind::Widget(widget) => widget.into_surface_node(WidgetViewContext {
                id,
                sizing: self.sizing,
                style: self.style,
                input_only: self.input_only,
                text_wrap: self.text_wrap,
                text_align: self.text_align,
            }),
            ViewNodeKind::Row { spacing, children } => {
                let policy = ContainerPolicy {
                    kind: ContainerKind::Row,
                    spacing,
                    padding: self.padding,
                    align_main: self.align_main.unwrap_or(MainAlign::Start),
                    align_cross: self.align_cross.unwrap_or(CrossAlign::Stretch),
                    ..ContainerPolicy::default()
                };
                let children = children
                    .into_iter()
                    .map(|child| child.lower_child(ids, child_scope, true))
                    .collect();
                if let Some(style) = self.style {
                    SurfaceNode::styled_container(id, policy, style, children)
                        .with_container_hoverable(self.hoverable)
                } else {
                    SurfaceNode::container(id, policy, children)
                }
            }
            ViewNodeKind::Column { spacing, children } => {
                let policy = ContainerPolicy {
                    kind: ContainerKind::Column,
                    spacing,
                    padding: self.padding,
                    align_main: self.align_main.unwrap_or(MainAlign::Start),
                    align_cross: self.align_cross.unwrap_or(CrossAlign::Stretch),
                    ..ContainerPolicy::default()
                };
                let children = children
                    .into_iter()
                    .map(|child| child.lower_child(ids, child_scope, false))
                    .collect();
                if let Some(style) = self.style {
                    SurfaceNode::styled_container(id, policy, style, children)
                        .with_container_hoverable(self.hoverable)
                } else {
                    SurfaceNode::container(id, policy, children)
                }
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
                    padding: self.padding,
                    align_main: self.align_main.unwrap_or(MainAlign::Start),
                    align_cross: self.align_cross.unwrap_or(CrossAlign::Stretch),
                    ..ContainerPolicy::default()
                };
                let children = children
                    .into_iter()
                    .map(|child| child.lower_child(ids, child_scope, false))
                    .collect();
                if let Some(style) = self.style {
                    SurfaceNode::styled_container(id, policy, style, children)
                        .with_container_hoverable(self.hoverable)
                } else {
                    SurfaceNode::container(id, policy, children)
                }
            }
            ViewNodeKind::Scroll { child } => {
                let policy = ContainerPolicy {
                    kind: ContainerKind::ScrollView,
                    overflow: crate::layout::OverflowPolicy::Scroll,
                    padding: self.padding,
                    align_main: self.align_main.unwrap_or(MainAlign::Start),
                    align_cross: self.align_cross.unwrap_or(CrossAlign::Stretch),
                    ..ContainerPolicy::default()
                };
                let children = vec![SurfaceChild::fill(child.lower(ids, child_scope))];
                if let Some(style) = self.style {
                    SurfaceNode::styled_container(id, policy, style, children)
                        .with_container_hoverable(self.hoverable)
                } else {
                    SurfaceNode::container(id, policy, children)
                }
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
                    padding: self.padding,
                    align_main: self.align_main.unwrap_or(MainAlign::Start),
                    align_cross: self.align_cross.unwrap_or(CrossAlign::Stretch),
                    ..ContainerPolicy::default()
                };
                let children = vec![SurfaceChild::fill(child.lower(ids, child_scope))];
                if let Some(style) = self.style {
                    SurfaceNode::styled_container(id, policy, style, children)
                        .with_container_hoverable(self.hoverable)
                } else {
                    SurfaceNode::container(id, policy, children)
                }
            }
            ViewNodeKind::Stack { children } => {
                let policy = ContainerPolicy {
                    kind: ContainerKind::Stack,
                    padding: self.padding,
                    align_main: self.align_main.unwrap_or(MainAlign::Start),
                    align_cross: self.align_cross.unwrap_or(CrossAlign::Stretch),
                    ..ContainerPolicy::default()
                };
                let children = children
                    .into_iter()
                    .map(|child| SurfaceChild::fill(child.lower(ids, child_scope)))
                    .collect();
                if let Some(style) = self.style {
                    SurfaceNode::styled_container(id, policy, style, children)
                        .with_container_hoverable(self.hoverable)
                } else {
                    SurfaceNode::container(id, policy, children)
                }
            }
            ViewNodeKind::OverlayPanel { rect, label } => {
                if let Some(label) = label {
                    SurfaceNode::overlay_panel(id, rect, label, self.style.unwrap_or_default())
                } else {
                    SurfaceNode::overlay_marker(id, rect, self.style.unwrap_or_default())
                }
            }
        }
    }

    fn lower_child(
        self,
        ids: &mut IdGenerator,
        scope: u64,
        parent_horizontal: bool,
    ) -> SurfaceChild<Message> {
        let slot = self.slot.to_slot_params(parent_horizontal);
        SurfaceChild::new(slot, self.lower(ids, scope))
    }
}

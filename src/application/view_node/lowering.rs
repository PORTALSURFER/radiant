use super::{ViewNode, ViewNodeKind};
use crate::{
    application::{
        AppBridgeLifecycle, IdGenerator, IntoView, Presentation, ROOT_KEY_SCOPE, WidgetViewContext,
        view_node::lowering_defaults::ViewNodeContainerDefaults,
    },
    layout::{
        ContainerKind, ContainerPolicy, GridPolicy, NodeId, VirtualizationAxis,
        VirtualizationPolicy, WrapPolicy,
    },
    runtime::{SurfaceChild, SurfaceLayer, SurfaceNode},
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

impl<Message: 'static> ViewNode<Message> {
    pub(in crate::application) fn apply_scene_presentation<State: 'static>(
        &mut self,
        lifecycle: &mut AppBridgeLifecycle<State, Message>,
    ) {
        self.drain_scene_presentation_into(lifecycle);
    }

    fn drain_scene_presentation_into<State: 'static>(
        &mut self,
        lifecycle: &mut AppBridgeLifecycle<State, Message>,
    ) {
        match &mut self.kind {
            ViewNodeKind::Scene {
                base,
                layers,
                presentation,
                shortcuts,
            } => {
                if lifecycle.scene_shortcuts.is_none() {
                    lifecycle.scene_shortcuts = shortcuts.take();
                }
                if let Some(presentation) = presentation.take()
                    && let Ok(presentation) =
                        presentation.downcast::<Presentation<State, Message>>()
                {
                    let presentation = *presentation;
                    presentation.apply_to_scene_lifecycle(lifecycle);
                }
                base.drain_scene_presentation_into(lifecycle);
                for layer in layers {
                    if let Some(input) = layer.input.as_mut() {
                        input.drain_scene_presentation_into(lifecycle);
                    }
                    layer.view.drain_scene_presentation_into(lifecycle);
                }
            }
            ViewNodeKind::Row { children, .. }
            | ViewNodeKind::Column { children, .. }
            | ViewNodeKind::Grid { children, .. }
            | ViewNodeKind::Wrap { children, .. }
            | ViewNodeKind::Stack { children } => {
                for child in children {
                    child.drain_scene_presentation_into(lifecycle);
                }
            }
            ViewNodeKind::Scroll { child }
            | ViewNodeKind::VirtualScroll { child, .. }
            | ViewNodeKind::FloatingLayer { child, .. } => {
                child.drain_scene_presentation_into(lifecycle);
            }
            ViewNodeKind::Runtime(_)
            | ViewNodeKind::Widget(_)
            | ViewNodeKind::OverlayPanel { .. } => {}
        }
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
        let scroll_message = node.scroll_message;
        let accepts_native_file_drop = node.accepts_native_file_drop;
        let native_file_drop = node.native_file_drop.clone();
        let defaults =
            ViewNodeContainerDefaults::new(node.padding, node.align_main, node.align_cross, style);
        let base_policy = || defaults.base_policy();
        let styled_container =
            |lowering: &mut Self, policy: ContainerPolicy, children: Vec<SurfaceChild<Message>>| {
                let mut container =
                    lowering.lower_container(id, policy, style, hoverable, children);
                if let Some(scroll_message) = scroll_message.clone() {
                    container = container.with_scroll_message(scroll_message);
                }
                container
            };

        let lowered = match node.kind {
            ViewNodeKind::Scene { base, layers, .. } => {
                let mut base = *base;
                let mut collected_layers = Vec::new();
                base.drain_transient_layers_in_declaration_order(&mut collected_layers);
                collected_layers.extend(layers);
                let base = self.lower_node(base, child_scope);
                let layers = collected_layers
                    .into_iter()
                    .map(|layer| {
                        let input = layer.input.map(|input| self.lower_node(input, child_scope));
                        let foreground = self.lower_node(layer.view, child_scope);
                        SurfaceLayer::with_input(layer.kind, input, foreground)
                    })
                    .collect();
                SurfaceNode::scene(id, base, layers)
            }
            ViewNodeKind::Runtime(node) => node,
            ViewNodeKind::Widget(widget) => widget.into_surface_node(WidgetViewContext {
                id,
                sizing: node.sizing,
                style,
                input_only: node.input_only,
                text_wrap: node.text_wrap,
                text_align: node.text_align,
                text_color: node.text_color,
                text_background: node.text_background,
                text_inset: node.text_inset,
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
            ViewNodeKind::Wrap {
                item_gap,
                line_gap,
                children,
            } => {
                let policy = ContainerPolicy {
                    kind: ContainerKind::Wrap,
                    wrap: WrapPolicy { item_gap, line_gap },
                    ..base_policy()
                };
                let children = self.lower_slot_children(children, child_scope, true);
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
            ViewNodeKind::FloatingLayer {
                offset,
                size,
                child,
                interactive,
            } => {
                let child = self.lower_node(*child, child_scope);
                SurfaceNode::floating_layer(id, offset, size, child, interactive)
            }
        };
        let lowered = if accepts_native_file_drop {
            lowered.accepting_native_file_drop()
        } else {
            lowered
        };
        if let Some(mapper) = native_file_drop {
            lowered.with_native_file_drop_mapper(mapper)
        } else {
            lowered
        }
    }
}

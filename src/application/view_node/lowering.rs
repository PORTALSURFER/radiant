use super::{ViewNode, ViewNodeKind};
use crate::{
    application::{
        DeclarativeSourceContext, IdGenerator, IntoView, ROOT_KEY_SCOPE, SourceIdentitySeed,
        ViewProjection, WidgetViewContext, ids::StructuralRole, launch::SceneProjection,
        view_node::lowering_defaults::ViewNodeContainerDefaults,
    },
    layout::{ContainerKind, ContainerPolicy, NodeId, VirtualizationAxis, VirtualizationPolicy},
    runtime::{
        KeyedNodeEvidence, SourceCompatibility, SourceIdentity, SourceMetadata, SourceTopology,
        SurfaceChild, SurfaceLayer, SurfaceNode, UiSurface,
    },
};
use std::{collections::HashMap, panic::panic_any, rc::Rc};

#[path = "lowering/children.rs"]
mod children;
#[path = "lowering/containers.rs"]
mod containers;

impl<Message> IntoView<Message> for ViewNode<Message>
where
    Message: 'static,
{
    fn into_projection(self) -> ViewProjection<Message> {
        let mut reserved = Vec::new();
        self.collect_reserved_ids(ROOT_KEY_SCOPE, &mut reserved);
        let mut ids = IdGenerator::new(reserved);
        let mut keyed_candidates = std::collections::HashSet::new();
        if self
            .collect_keyed_collisions(ROOT_KEY_SCOPE, &mut keyed_candidates)
            .is_err()
        {
            panic_any("ambiguous keyed identity");
        }
        let mut continuity_keys = std::collections::HashSet::new();
        let mut explicit_ids = std::collections::HashSet::new();
        if self
            .collect_explicit_identity_collisions(
                ROOT_KEY_SCOPE,
                &mut continuity_keys,
                &mut explicit_ids,
            )
            .is_err()
        {
            panic_any("ambiguous keyed identity");
        }
        let mut scene = SceneProjection::default();
        let root = ViewLowering::new(&mut ids, &mut scene).lower_node(
            self,
            ROOT_KEY_SCOPE,
            StructuralRole::Root,
        );
        ViewProjection::with_scene(UiSurface::new(root), scene)
    }
}

pub(super) struct ViewLowering<'a, Message> {
    ids: &'a mut IdGenerator,
    scene: &'a mut SceneProjection<Message>,
    source_context: DeclarativeSourceContext,
    keyed_candidates: HashMap<NodeId, Rc<KeyedNodeEvidence>>,
}

impl<'a, Message: 'static> ViewLowering<'a, Message> {
    pub(super) fn new(ids: &'a mut IdGenerator, scene: &'a mut SceneProjection<Message>) -> Self {
        Self {
            ids,
            scene,
            source_context: DeclarativeSourceContext::default(),
            keyed_candidates: HashMap::new(),
        }
    }

    fn keyed_candidate(&mut self, seed: SourceIdentitySeed) -> Rc<KeyedNodeEvidence> {
        if let Some(candidate) = self.keyed_candidates.get(&seed.structural_scope) {
            return Rc::clone(candidate);
        }
        let candidate = Rc::new(KeyedNodeEvidence::new(seed));
        self.keyed_candidates
            .insert(seed.structural_scope, Rc::clone(&candidate));
        candidate
    }

    fn source_topology(&mut self, context: &DeclarativeSourceContext) -> SourceTopology {
        SourceTopology::from_context(context, |seed| self.keyed_candidate(seed))
    }

    fn lower_node_with_context(
        &mut self,
        node: ViewNode<Message>,
        scope: u64,
        role: StructuralRole,
        context: DeclarativeSourceContext,
    ) -> SurfaceNode<Message> {
        let previous_context = std::mem::replace(&mut self.source_context, context);
        let lowered = self.lower_node(node, scope, role);
        self.source_context = previous_context;
        lowered
    }

    fn next_node_identity(
        &mut self,
        node: &ViewNode<Message>,
        scope: u64,
        role: StructuralRole,
    ) -> crate::application::ids::StructuralIdentity {
        if let Some(id) = node.resolved_id(scope) {
            self.ids.claim_explicit(id);
            return crate::application::ids::StructuralIdentity {
                id,
                scope: node.unprobed_structural_scope(scope, role),
            };
        }
        if let Some(keyed) = node.keyed_identity {
            return self.ids.next_keyed_structural(
                scope,
                node.structural_kind(),
                keyed.key_type,
                keyed.key_fingerprint,
            );
        }
        self.ids
            .next_structural(scope, node.structural_kind(), role)
    }

    pub(super) fn lower_node(
        &mut self,
        node: ViewNode<Message>,
        scope: u64,
        role: StructuralRole,
    ) -> SurfaceNode<Message> {
        let identity = self.next_node_identity(&node, scope, role);
        let id = identity.id;
        let child_scope = identity.scope;
        let source_seed = node.source_identity_seed(scope, role);
        let source_origin = source_seed.origin;
        let source_seed = SourceIdentitySeed {
            resolved_id: match source_origin {
                crate::application::DeclarativeIdentityOrigin::UnreidentifiedDirectRuntimeRoot => {
                    source_seed.resolved_id
                }
                _ => identity.id,
            },
            ..source_seed
        };
        let source_identity = SourceIdentity {
            resolved_id: source_seed.resolved_id,
            structural_scope: source_seed.structural_scope,
            origin: source_origin,
        };
        let node_context = self.source_context.with_node(source_seed);
        let current_keyed_candidate = source_origin.is_keyed().then(|| {
            let candidate = self.keyed_candidate(source_seed);
            candidate.set_identity(source_identity);
            candidate
        });
        let source_topology = self.source_topology(&node_context);
        let previous_context = std::mem::replace(&mut self.source_context, node_context);
        let reidentify_runtime_root = node.id.is_some() || node.key.is_some();
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
                    container = container.with_scroll_message_local(scroll_message);
                }
                container
            };

        let lowered = match node.kind {
            ViewNodeKind::Scene {
                base,
                mut layers,
                presentation,
                shortcuts,
            } => {
                self.scene.capture(presentation, shortcuts);
                let mut base = *base;
                let mut collected_layers = Vec::new();
                base.drain_overlay_layers_in_declaration_order(
                    child_scope,
                    StructuralRole::SceneBase,
                    &self.source_context,
                    &mut collected_layers,
                );
                ViewNode::drain_layer_list_in_declaration_order(
                    &mut layers,
                    child_scope,
                    &self.source_context,
                    &mut collected_layers,
                );
                let base = self.lower_node(base, child_scope, StructuralRole::SceneBase);
                let layers = collected_layers
                    .into_iter()
                    .enumerate()
                    .map(|(index, layer)| {
                        let layer_context = layer.source_context.clone();
                        let input = layer.input.map(|input| {
                            self.lower_node_with_context(
                                input,
                                child_scope,
                                StructuralRole::SceneInput(index),
                                layer_context.clone(),
                            )
                        });
                        let foreground = self.lower_node_with_context(
                            layer.view,
                            child_scope,
                            StructuralRole::SceneLayer(index),
                            layer_context,
                        );
                        SurfaceLayer::with_input(layer.kind, input, foreground)
                    })
                    .collect();
                SurfaceNode::scene(id, base, layers)
            }
            ViewNodeKind::Runtime(node) if reidentify_runtime_root => node.with_id(id),
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
                tooltip: node.tooltip,
            }),
            ViewNodeKind::Container {
                mut policy,
                children,
            } => {
                let defaults = base_policy();
                policy.padding = defaults.padding;
                policy.align_main = defaults.align_main;
                policy.align_cross = defaults.align_cross;
                let children = if policy.kind == ContainerKind::Stack {
                    self.lower_fill_children(children, child_scope)
                } else {
                    let parent_horizontal =
                        matches!(policy.kind, ContainerKind::Row | ContainerKind::Wrap);
                    self.lower_slot_children(children, child_scope, parent_horizontal)
                };
                styled_container(self, policy, children)
            }
            ViewNodeKind::Scroll { child } => {
                let policy = ContainerPolicy {
                    kind: ContainerKind::ScrollView,
                    overflow: crate::layout::OverflowPolicy::Scroll,
                    ..base_policy()
                };
                let children =
                    vec![self.lower_fill_child(*child, child_scope, StructuralRole::ScrollChild)];
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
                let children = vec![self.lower_fill_child(
                    *child,
                    child_scope,
                    StructuralRole::VirtualScrollChild,
                )];
                styled_container(self, policy, children)
            }
            ViewNodeKind::OverlayPanel { rect, label } => {
                if let Some(label) = label {
                    SurfaceNode::overlay_panel(
                        id,
                        rect,
                        label.into_paint_text(),
                        style.unwrap_or_default(),
                    )
                } else {
                    SurfaceNode::overlay_marker(id, rect, style.unwrap_or_default())
                }
            }
            ViewNodeKind::FloatingLayer {
                offset,
                size,
                child,
                interactive,
                horizontal_overflow,
                vertical_overflow,
            } => {
                let child =
                    self.lower_node(*child, child_scope, StructuralRole::FloatingLayerChild);
                SurfaceNode::floating_layer_with_vertical_overflow(
                    id,
                    offset,
                    size,
                    child,
                    interactive,
                    horizontal_overflow,
                    vertical_overflow,
                )
            }
        };
        let mut lowered = if accepts_native_file_drop {
            lowered.accepting_native_file_drop()
        } else {
            lowered
        };
        if let Some(mapper) = native_file_drop {
            lowered = lowered.with_native_file_drop_mapper(mapper);
        }
        let compatibility = SourceCompatibility::from_surface_node(&lowered);
        if let Some(candidate) = current_keyed_candidate {
            candidate.set_compatibility(compatibility);
        }
        let lowered = lowered.with_source_metadata(SourceMetadata::new(
            source_identity,
            compatibility,
            source_topology,
        ));
        self.source_context = previous_context;
        lowered
    }
}

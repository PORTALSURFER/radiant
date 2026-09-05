use super::{ExtractedLayerRoot, ViewNode, ViewNodeKind};
use crate::{
    UiAffinity,
    application::{
        DeclarativeSourceContext, IdGenerator, IntoView, ROOT_KEY_SCOPE, SourceIdentitySeed,
        ViewProjection, WidgetViewContext, ids::StructuralRole, launch::SceneProjection,
        view_node::lowering_defaults::ViewNodeContainerDefaults,
    },
    layout::{
        ContainerKind, ContainerPolicy, LayoutPolicy, NodeId, VirtualizationAxis,
        VirtualizationPolicy,
    },
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
    _ui_affinity: UiAffinity,
    ids: &'a mut IdGenerator,
    scene: &'a mut SceneProjection<Message>,
    source_context: DeclarativeSourceContext,
    keyed_candidates: HashMap<NodeId, Rc<KeyedNodeEvidence>>,
}

impl<'a, Message: 'static> ViewLowering<'a, Message> {
    pub(super) fn new(ids: &'a mut IdGenerator, scene: &'a mut SceneProjection<Message>) -> Self {
        Self {
            _ui_affinity: UiAffinity::new(),
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
        source_seed: SourceIdentitySeed,
    ) -> SurfaceNode<Message> {
        let previous_context = std::mem::replace(&mut self.source_context, context);
        let lowered = self.lower_node_with_source_seed(node, scope, role, source_seed);
        self.source_context = previous_context;
        lowered
    }

    fn lower_extracted_layer_root(
        &mut self,
        root: ExtractedLayerRoot<Message>,
        scope: u64,
        role: StructuralRole,
    ) -> SurfaceNode<Message> {
        self.lower_node_with_context(root.node, scope, role, root.context, root.seed)
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
        let source_seed = node.source_identity_seed(scope, role);
        self.lower_node_with_source_seed(node, scope, role, source_seed)
    }

    fn lower_node_with_source_seed(
        &mut self,
        node: ViewNode<Message>,
        scope: u64,
        role: StructuralRole,
        source_seed: SourceIdentitySeed,
    ) -> SurfaceNode<Message> {
        let identity = self.next_node_identity(&node, scope, role);
        let id = identity.id;
        let child_scope = identity.scope;
        let source_origin = source_seed.origin;
        let reidentify_runtime_root = node.id.is_some() || node.key.is_some();
        let resolved_id = match (&node.kind, source_origin, reidentify_runtime_root) {
            (
                ViewNodeKind::Runtime(runtime),
                crate::application::DeclarativeIdentityOrigin::UnreidentifiedDirectRuntimeRoot,
                false,
            ) => runtime.id(),
            _ => identity.id,
        };
        let source_identity = SourceIdentity {
            resolved_id,
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
        let style = node.style;
        let hoverable = node.hoverable;
        let split_pane_runtime = node.split_pane_runtime;
        let split_pane_ratio_settled = node.split_pane_ratio_settled;
        let scroll_message = node.scroll_message;
        let scroll_policy = node.scroll_policy;
        let initial_offset = node.initial_offset;
        let controlled_offset = node.controlled_offset;
        let scroll_request = node.scroll_request;
        let offset_settled = node.offset_settled;
        let accepts_native_file_drop = node.accepts_native_file_drop;
        let native_file_drop = node.native_file_drop.clone();
        let defaults =
            ViewNodeContainerDefaults::new(node.padding, node.align_main, node.align_cross, style);
        let base_policy = || defaults.base_policy();
        let styled_container =
            |lowering: &mut Self,
             policy: ContainerPolicy,
             layout_policy: Option<Rc<dyn LayoutPolicy>>,
             children: Vec<SurfaceChild<Message>>| {
                let runtime_split_policy = (policy.kind == ContainerKind::SplitPane)
                    .then(|| {
                        split_pane_runtime.and_then(|mode| {
                            matches!(
                                mode,
                                crate::gui::layout_core::SplitPaneRuntimeMode::RuntimeOwned { .. }
                            )
                            .then_some((policy.split_pane, mode.collapse_policy()))
                        })
                    })
                    .flatten();
                let mut container =
                    lowering.lower_container(id, policy, layout_policy, style, hoverable, children);
                if let Some(scroll_message) = scroll_message.clone() {
                    container = container.with_scroll_message_local(scroll_message);
                }
                container = container.with_scroll_declaration(
                    scroll_policy.map(crate::layout::ScrollPolicy::normalized),
                    initial_offset,
                    controlled_offset,
                    scroll_request.clone(),
                );
                if let Some(map) = offset_settled.clone() {
                    container = container.on_offset_settled(move |offset| map(offset));
                }
                container = container.with_split_pane_runtime_mode(split_pane_runtime);
                container =
                    container.with_split_pane_ratio_settled(split_pane_ratio_settled.clone());
                if let Some((policy, collapse_policy)) = runtime_split_policy {
                    let capabilities = match split_pane_ratio_settled.clone() {
                        Some(map) => crate::gui::layout_core::
                            runtime_owned_split_pane_capabilities_with_ratio_settled(
                                policy,
                                collapse_policy,
                                Some(map),
                            ),
                        None => crate::gui::layout_core::runtime_owned_split_pane_capabilities(
                            policy,
                            collapse_policy,
                        ),
                    };
                    container = container.with_layout_capabilities(capabilities);
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
                        let input = layer.input.map(|input| {
                            self.lower_extracted_layer_root(
                                input,
                                child_scope,
                                StructuralRole::SceneInput(index),
                            )
                        });
                        let foreground = self.lower_extracted_layer_root(
                            layer.foreground,
                            child_scope,
                            StructuralRole::SceneLayer(index),
                        );
                        SurfaceLayer::with_input(layer.kind, input, foreground)
                    })
                    .collect();
                SurfaceNode::scene(id, base, layers)
            }
            ViewNodeKind::Runtime(node) if reidentify_runtime_root => node.with_id(id),
            ViewNodeKind::Runtime(node) => node,
            ViewNodeKind::VirtualLayout(parts) => {
                crate::runtime::lower_public_virtual_layout(id, parts)
            }
            ViewNodeKind::Widget(widget) => widget.into_surface_node(WidgetViewContext {
                _ui_affinity: UiAffinity::new(),
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
                    let parent_horizontal = match policy.kind {
                        ContainerKind::Row | ContainerKind::Wrap => true,
                        ContainerKind::SplitPane => matches!(
                            policy.split_pane.axis,
                            crate::gui::panel::SplitPaneAxis::Horizontal
                        ),
                        _ => false,
                    };
                    self.lower_slot_children(children, child_scope, parent_horizontal)
                };
                styled_container(self, policy, None, children)
            }
            ViewNodeKind::CustomLayout { policy, children } => {
                let policy_metadata = base_policy();
                let children = self.lower_slot_children(children, child_scope, false);
                styled_container(self, policy_metadata, Some(policy), children)
            }
            ViewNodeKind::Scroll { child } => {
                let policy = ContainerPolicy {
                    kind: ContainerKind::ScrollView,
                    overflow: crate::layout::OverflowPolicy::Scroll,
                    ..base_policy()
                };
                let children =
                    vec![self.lower_fill_child(*child, child_scope, StructuralRole::ScrollChild)];
                styled_container(self, policy, None, children)
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
                styled_container(self, policy, None, children)
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

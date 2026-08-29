use super::{
    SourceTraversalIndex, SurfaceContainer, SurfaceContainerTraversalRecord, SurfaceNode,
    SurfaceScene, SurfaceSplitPaneFocusOrderCandidate, SurfaceSplitPaneRatioActionCandidate,
    SurfaceTraversalIndex, SurfaceTraversalStats, UiSurface,
};
use super::{SurfaceWidget, SurfaceWidgetTraversalRecord};
use crate::layout::supports_layout_capabilities_contract;
use crate::layout::{ContainerKind, LayoutNode, NodeId, SlotChild, Vector2};
use crate::layout::{ContainerPolicy, SlotParams};

pub(in crate::runtime) struct SurfaceRuntimeProjection<Message> {
    pub(in crate::runtime) layout_root: LayoutNode,
    pub(in crate::runtime) traversal: SurfaceTraversalIndex<Message>,
    pub(in crate::runtime) source: SourceTraversalIndex,
}

impl<Message> UiSurface<Message> {
    pub(in crate::runtime) fn runtime_projection(&self) -> SurfaceRuntimeProjection<Message> {
        let stats = self.root.runtime_traversal_stats();
        let mut traversal = SurfaceTraversalIndex::with_stats(stats);
        let mut source = SourceTraversalIndex::with_stats(stats);
        let layout_root =
            self.runtime_projection_into_with_source(&mut traversal, stats, &mut source);
        SurfaceRuntimeProjection {
            layout_root,
            traversal,
            source,
        }
    }

    pub(in crate::runtime) fn runtime_projection_into_with_source(
        &self,
        traversal: &mut SurfaceTraversalIndex<Message>,
        stats: SurfaceTraversalStats,
        source: &mut SourceTraversalIndex,
    ) -> LayoutNode {
        traversal.clear_for_stats(stats);
        source.clear_for_stats(stats);
        self.root.project_runtime(
            &mut Vec::with_capacity(stats.max_scroll_depth),
            &mut Vec::with_capacity(stats.max_depth),
            traversal,
            source,
        )
    }

    pub(in crate::runtime) fn runtime_projection_reusing_with_scratch(
        &self,
        traversal: &mut SurfaceTraversalIndex<Message>,
        scroll_stack: &mut Vec<NodeId>,
        child_path: &mut Vec<usize>,
        source: &mut SourceTraversalIndex,
    ) -> LayoutNode {
        self.runtime_projection_reusing_with_scratch_and_source(
            traversal,
            scroll_stack,
            child_path,
            source,
        )
    }

    pub(in crate::runtime) fn runtime_projection_reusing_with_scratch_and_source(
        &self,
        traversal: &mut SurfaceTraversalIndex<Message>,
        scroll_stack: &mut Vec<NodeId>,
        child_path: &mut Vec<usize>,
        source: &mut SourceTraversalIndex,
    ) -> LayoutNode {
        traversal.clear_for_reuse();
        source.clear_for_reuse();
        scroll_stack.clear();
        child_path.clear();
        self.root
            .project_runtime(scroll_stack, child_path, traversal, source)
    }

    pub(in crate::runtime) fn runtime_source_traversal_index_reusing(
        &self,
        source: &mut SourceTraversalIndex,
    ) {
        let stats = self.root.runtime_traversal_stats();
        source.clear_for_stats(stats);
        self.root.collect_source_traversal(source);
    }
}

impl<Message> SurfaceNode<Message> {
    pub(super) fn layout_node(&self) -> LayoutNode {
        match self {
            Self::Scene(scene) => scene_layout_node(scene, |_, child| child.layout_node()),
            Self::Container(container) => {
                let children = container_layout_children(container, |_, child| child.layout_node());
                LayoutNode::container_with_split_pane_runtime_mode(
                    container.id,
                    container.policy.clone(),
                    children,
                    container.split_pane_runtime,
                )
            }
            Self::Widget(widget) => widget.layout_node(),
            Self::Overlay(overlay) => LayoutNode::widget(overlay.id, Vector2::new(0.0, 0.0)),
            Self::FloatingLayer(layer) => {
                let children =
                    container_layout_children(&layer.container, |_, child| child.layout_node());
                LayoutNode::container(layer.container.id, layer.container.policy.clone(), children)
            }
        }
    }

    fn project_runtime(
        &self,
        scroll_stack: &mut Vec<NodeId>,
        child_path: &mut Vec<usize>,
        traversal: &mut SurfaceTraversalIndex<Message>,
        source: &mut SourceTraversalIndex,
    ) -> LayoutNode {
        source.record_node(self);
        match self {
            Self::Scene(scene) => {
                if !scene.has_layers() {
                    return scene
                        .base
                        .project_runtime(scroll_stack, child_path, traversal, source);
                }
                scene_layout_node(scene, |scene_child_index, child| {
                    child_path.push(scene_child_index);
                    let layout = child.project_runtime(scroll_stack, child_path, traversal, source);
                    child_path.pop();
                    layout
                })
            }
            Self::Container(container) => {
                let is_scroll = begin_container_runtime(container, scroll_stack, traversal);
                let focus_order_candidate = split_pane_focus_order_candidate(container);
                let children = container_layout_children(container, |child_index, child| {
                    child_path.push(child_index);
                    let layout = child.project_runtime(scroll_stack, child_path, traversal, source);
                    child_path.pop();
                    if child_index == 0
                        && let Some(candidate) = focus_order_candidate
                    {
                        traversal.record_split_pane_focus_order_candidate(candidate);
                    }
                    layout
                });
                end_container_runtime(is_scroll, scroll_stack);
                LayoutNode::container_with_split_pane_runtime_mode(
                    container.id,
                    container.policy.clone(),
                    children,
                    container.split_pane_runtime,
                )
            }
            Self::Widget(widget) => {
                record_widget_runtime(widget, scroll_stack, child_path, traversal);
                widget.layout_node()
            }
            Self::Overlay(overlay) => LayoutNode::widget(overlay.id, Vector2::new(0.0, 0.0)),
            Self::FloatingLayer(layer) => {
                if layer.interactive {
                    let is_scroll =
                        begin_container_runtime(&layer.container, scroll_stack, traversal);
                    let focus_order_candidate = split_pane_focus_order_candidate(&layer.container);
                    let children =
                        container_layout_children(&layer.container, |child_index, child| {
                            child_path.push(child_index);
                            let layout =
                                child.project_runtime(scroll_stack, child_path, traversal, source);
                            child_path.pop();
                            if child_index == 0
                                && let Some(candidate) = focus_order_candidate
                            {
                                traversal.record_split_pane_focus_order_candidate(candidate);
                            }
                            layout
                        });
                    end_container_runtime(is_scroll, scroll_stack);
                    LayoutNode::container(
                        layer.container.id,
                        layer.container.policy.clone(),
                        children,
                    )
                } else {
                    let children = container_layout_children(&layer.container, |_, child| {
                        child.collect_source_traversal(source);
                        child.layout_node()
                    });
                    LayoutNode::container(
                        layer.container.id,
                        layer.container.policy.clone(),
                        children,
                    )
                }
            }
        }
    }

    #[cfg(test)]
    pub(in crate::runtime) fn project_runtime_index(
        &self,
        scroll_stack: &mut Vec<NodeId>,
        child_path: &mut Vec<usize>,
        traversal: &mut SurfaceTraversalIndex<Message>,
    ) {
        self.collect_runtime_index(scroll_stack, child_path, traversal);
    }

    #[cfg(test)]
    fn collect_runtime_index(
        &self,
        scroll_stack: &mut Vec<NodeId>,
        child_path: &mut Vec<usize>,
        traversal: &mut SurfaceTraversalIndex<Message>,
    ) {
        match self {
            Self::Scene(scene) => {
                visit_scene_children(scene, child_path, |child, child_path| {
                    child.collect_runtime_index(scroll_stack, child_path, traversal);
                });
            }
            Self::Container(container) => {
                let is_scroll = begin_container_runtime(container, scroll_stack, traversal);
                let focus_order_candidate = split_pane_focus_order_candidate(container);
                visit_container_children(
                    container,
                    child_path,
                    |child_index, child, child_path| {
                        child.collect_runtime_index(scroll_stack, child_path, traversal);
                        if child_index == 0
                            && let Some(candidate) = focus_order_candidate
                        {
                            traversal.record_split_pane_focus_order_candidate(candidate);
                        }
                    },
                );
                end_container_runtime(is_scroll, scroll_stack);
            }
            Self::Widget(widget) => {
                record_widget_runtime(widget, scroll_stack, child_path, traversal);
            }
            Self::Overlay(_) => {}
            Self::FloatingLayer(layer) => {
                if !layer.interactive {
                    return;
                }
                let is_scroll = begin_container_runtime(&layer.container, scroll_stack, traversal);
                let focus_order_candidate = split_pane_focus_order_candidate(&layer.container);
                visit_container_children(
                    &layer.container,
                    child_path,
                    |child_index, child, child_path| {
                        child.collect_runtime_index(scroll_stack, child_path, traversal);
                        if child_index == 0
                            && let Some(candidate) = focus_order_candidate
                        {
                            traversal.record_split_pane_focus_order_candidate(candidate);
                        }
                    },
                );
                end_container_runtime(is_scroll, scroll_stack);
            }
        }
    }
}

fn scene_layout_node<Message>(
    scene: &SurfaceScene<Message>,
    mut child_layout: impl FnMut(usize, &SurfaceNode<Message>) -> LayoutNode,
) -> LayoutNode {
    if !scene.has_layers() {
        return child_layout(0, &scene.base);
    }
    let mut children = Vec::with_capacity(1 + scene.ordered_layer_child_count());
    push_scene_layout_children(scene, &mut children, child_layout);
    LayoutNode::container(
        scene.id,
        ContainerPolicy {
            kind: ContainerKind::Stack,
            ..ContainerPolicy::default()
        },
        children,
    )
}

fn push_scene_layout_children<Message>(
    scene: &SurfaceScene<Message>,
    children: &mut Vec<SlotChild>,
    mut child_layout: impl FnMut(usize, &SurfaceNode<Message>) -> LayoutNode,
) {
    children.push(SlotChild::new(
        SlotParams::fill(),
        child_layout(0, &scene.base),
    ));

    let mut scene_child_index = 1;
    for layer in scene.ordered_layers() {
        if let Some(input) = &layer.input {
            children.push(SlotChild::new(
                SlotParams::fill(),
                child_layout(scene_child_index, input),
            ));
            scene_child_index += 1;
        }

        children.push(SlotChild::new(
            SlotParams::fill(),
            child_layout(scene_child_index, &layer.node),
        ));
        scene_child_index += 1;
    }
}

#[cfg(test)]
fn visit_scene_children<Message>(
    scene: &SurfaceScene<Message>,
    child_path: &mut Vec<usize>,
    mut visit_child: impl FnMut(&SurfaceNode<Message>, &mut Vec<usize>),
) {
    if !scene.has_layers() {
        visit_child(&scene.base, child_path);
        return;
    }

    child_path.push(0);
    visit_child(&scene.base, child_path);
    child_path.pop();

    let mut scene_child_index = 1;
    for layer in scene.ordered_layers() {
        if let Some(input) = &layer.input {
            child_path.push(scene_child_index);
            visit_child(input, child_path);
            child_path.pop();
            scene_child_index += 1;
        }

        child_path.push(scene_child_index);
        visit_child(&layer.node, child_path);
        child_path.pop();
        scene_child_index += 1;
    }
}

fn container_layout_children<Message>(
    container: &SurfaceContainer<Message>,
    mut child_layout: impl FnMut(usize, &SurfaceNode<Message>) -> LayoutNode,
) -> Vec<SlotChild> {
    let mut children = Vec::with_capacity(container.children.len());
    for (child_index, child) in container.children.iter().enumerate() {
        children.push(SlotChild::new(
            child.slot,
            child_layout(child_index, &child.child),
        ));
    }
    children
}

#[cfg(test)]
fn visit_container_children<Message>(
    container: &SurfaceContainer<Message>,
    child_path: &mut Vec<usize>,
    mut visit_child: impl FnMut(usize, &SurfaceNode<Message>, &mut Vec<usize>),
) {
    for (child_index, child) in container.children.iter().enumerate() {
        child_path.push(child_index);
        visit_child(child_index, &child.child, child_path);
        child_path.pop();
    }
}

fn split_pane_focus_order_candidate<Message>(
    container: &SurfaceContainer<Message>,
) -> Option<SurfaceSplitPaneFocusOrderCandidate> {
    let (state_id, descriptor, policy_revision, contract_version) =
        runtime_owned_split_pane_source_evidence(container)?;
    Some(SurfaceSplitPaneFocusOrderCandidate {
        widget_index: 0,
        target: crate::layout::LayoutTargetIdentity::new(
            container.id,
            crate::gui::layout_core::SPLIT_PANE_DIVIDER_REGION_ID,
        ),
        state_id,
        descriptor,
        ownership: crate::gui::layout_core::SplitPaneRuntimeOwnership::RuntimeOwned,
        contract_version,
        state_schema_version: state_id.schema_version(),
        policy_revision,
    })
}

fn split_pane_ratio_action_candidate<Message>(
    container: &SurfaceContainer<Message>,
) -> Option<SurfaceSplitPaneRatioActionCandidate<Message>> {
    let (state_id, descriptor, policy_revision, contract_version) =
        runtime_owned_split_pane_source_evidence(container)?;
    Some(SurfaceSplitPaneRatioActionCandidate {
        target: crate::layout::LayoutTargetIdentity::new(
            container.id,
            crate::gui::layout_core::SPLIT_PANE_DIVIDER_REGION_ID,
        ),
        state_id,
        descriptor,
        ownership: crate::gui::layout_core::SplitPaneRuntimeOwnership::RuntimeOwned,
        contract_version,
        state_schema_version: state_id.schema_version(),
        policy_revision,
        on_ratio_settled: container.split_pane_ratio_settled.clone(),
    })
}

fn runtime_owned_split_pane_source_evidence<Message>(
    container: &SurfaceContainer<Message>,
) -> Option<(
    crate::gui::layout_core::ContainerStateId,
    crate::gui::layout_core::SplitPaneDividerDescriptor,
    crate::gui::layout_core::SplitPaneRuntimePolicyRevision,
    u16,
)> {
    let Some(crate::gui::layout_core::SplitPaneRuntimeMode::RuntimeOwned { collapse_policy }) =
        container.split_pane_runtime
    else {
        return None;
    };
    if container.policy.kind != ContainerKind::SplitPane {
        return None;
    }
    let [first, second] = container.children.as_slice() else {
        return None;
    };
    let child_ids = [first.child.id(), second.child.id()];
    let descriptor = crate::gui::layout_core::SplitPaneDividerDescriptor::from_policy(
        container.id,
        container.policy.split_pane,
        &child_ids,
    )?;
    let policy_revision = crate::gui::layout_core::SplitPaneRuntimePolicyRevision::new(
        container.policy.split_pane,
        collapse_policy,
    );
    let state_id = crate::gui::layout_core::SplitPaneRuntimeStateInput {
        container_id: container.id,
        initial_ratio: container.policy.split_pane.initial_ratio,
        mode: crate::gui::layout_core::SplitPaneRuntimeMode::RuntimeOwned { collapse_policy },
        policy_revision,
    }
    .state_id();
    let contract_version = container
        .layout_capabilities
        .as_ref()
        .map_or(0, |capabilities| capabilities.contract_version);
    Some((state_id, descriptor, policy_revision, contract_version))
}

fn begin_container_runtime<Message>(
    container: &SurfaceContainer<Message>,
    scroll_stack: &mut Vec<NodeId>,
    traversal: &mut SurfaceTraversalIndex<Message>,
) -> bool {
    let is_scroll = container.policy.kind == ContainerKind::ScrollView;
    traversal.record_container(SurfaceContainerTraversalRecord {
        id: container.id,
        clipped_by: scroll_stack,
        scroll_content: if is_scroll {
            container.children.first().map(|content| content.child.id())
        } else {
            None
        },
        styled_hoverable: container.style.is_some() && container.hoverable,
        layout_interaction: container
            .layout_capabilities
            .as_ref()
            .filter(|capabilities| {
                supports_layout_capabilities_contract(capabilities.contract_version)
            })
            .and_then(|capabilities| {
                capabilities.interaction.as_ref().map(|interaction| {
                    let state = crate::layout::supports_layout_state_input_contract(
                        capabilities.contract_version,
                    )
                    .then(|| interaction.state(container.id))
                    .flatten();
                    let foreign_state_declaration = state
                        .as_ref()
                        .is_some_and(|declaration| declaration.container_id() != container.id);
                    super::SurfaceLayoutInteractionRecord {
                        id: container.id,
                        contract_version: capabilities.contract_version,
                        interaction: interaction.clone(),
                        revision: interaction.revision(),
                        state: state
                            .filter(|declaration| declaration.container_id() == container.id),
                        foreign_state_declaration,
                    }
                })
            }),
        split_pane_runtime: (container.policy.kind == ContainerKind::SplitPane)
            .then(|| {
                container.split_pane_runtime.map(|mode| {
                    crate::gui::layout_core::SplitPaneRuntimeStateInput {
                        container_id: container.id,
                        initial_ratio: container.policy.split_pane.initial_ratio,
                        mode,
                        policy_revision:
                            crate::gui::layout_core::SplitPaneRuntimePolicyRevision::new(
                                container.policy.split_pane,
                                mode.collapse_policy(),
                            ),
                    }
                })
            })
            .flatten(),
        split_pane_divider: (container.policy.kind == ContainerKind::SplitPane
            && matches!(
                container.split_pane_runtime,
                Some(crate::gui::layout_core::SplitPaneRuntimeMode::RuntimeOwned { .. })
            )
            && container
                .layout_capabilities
                .as_ref()
                .is_some_and(|capabilities| {
                    crate::layout::supports_layout_input_contract(capabilities.contract_version)
                        && capabilities.interaction.is_some()
                }))
        .then(|| {
            let children = container
                .children
                .iter()
                .map(|child| child.child.id())
                .collect::<Vec<_>>();
            crate::gui::layout_core::SplitPaneDividerDescriptor::from_policy(
                container.id,
                container.policy.split_pane,
                &children,
            )
        })
        .flatten(),
        split_pane_ratio_action: split_pane_ratio_action_candidate(container),
        virtual_layout: container.virtual_layout.clone(),
    });
    if is_scroll {
        scroll_stack.push(container.id);
    }
    is_scroll
}

fn end_container_runtime(is_scroll: bool, scroll_stack: &mut Vec<NodeId>) {
    if is_scroll {
        scroll_stack.pop();
    }
}

fn record_widget_runtime<Message>(
    widget: &SurfaceWidget<Message>,
    scroll_stack: &[NodeId],
    child_path: &[usize],
    traversal: &mut SurfaceTraversalIndex<Message>,
) {
    traversal.record_widget(SurfaceWidgetTraversalRecord {
        id: widget.id(),
        child_path,
        clipped_by: scroll_stack,
        focusable: widget.is_focusable(),
        keyboard_focusable: widget.is_keyboard_focusable(),
        receives_pointer_hit_testing: widget.receives_pointer_hit_testing(),
        receives_wheel_input: widget.receives_wheel_input(),
        accepts_native_file_drop: widget.accepts_native_file_drop(),
        needs_state_synchronization: widget.needs_state_synchronization(),
        suppresses_container_hover: widget.suppresses_container_hover(),
    });
}

#[cfg(test)]
#[path = "layout/tests.rs"]
mod tests;

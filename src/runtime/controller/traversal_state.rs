//! Traversal indexes and lookup caches derived from the projected surface tree.

use super::{ClipAncestors, WidgetPath, hit_order::HitOrderIndex};
use crate::runtime::surface::SurfaceSplitPaneRatioActionCandidate;
use crate::{
    gui::layout_core::{SplitPaneRuntimeOwnership, SplitPaneRuntimeState},
    layout::{
        LayoutHitRegion, LayoutHitRegionDiagnostics, LayoutHitTarget, LayoutInteraction,
        LayoutInteractionRevision, LayoutOutput, NodeId, Rect,
    },
    runtime::{
        RuntimeLifecyclePhase, SurfaceLayoutInteractionRecord, SurfaceSplitPaneFocusOrderCandidate,
        WheelHitTarget,
    },
    widgets::WidgetId,
};
use std::collections::{HashMap, HashSet};

pub(super) struct RuntimeTraversalState<Message = ()> {
    pub(super) command_scopes: crate::runtime::surface::SurfaceCommandScopes,
    pub(super) widgets: RuntimeWidgetTraversal,
    pub(super) containers: RuntimeContainerTraversal<Message>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum RuntimeFocusOrderEntry {
    Widget(WidgetId),
    SplitPaneSeparator(super::split_pane_separator::SplitPaneSeparatorProjection),
}

impl<Message> Default for RuntimeTraversalState<Message> {
    fn default() -> Self {
        Self {
            command_scopes: Default::default(),
            widgets: RuntimeWidgetTraversal::default(),
            containers: RuntimeContainerTraversal::default(),
        }
    }
}

#[derive(Default)]
pub(super) struct RuntimeWidgetTraversal {
    pub(super) hit_order: Vec<WidgetId>,
    pub(super) focusable: HitOrderIndex,
    pub(super) pointer: HitOrderIndex,
    pub(super) native_file_drop: HitOrderIndex,
    pub(super) keyboard_focus: HitOrderIndex,
    pub(super) keyboard_focus_order_candidates: Vec<SurfaceSplitPaneFocusOrderCandidate>,
    pub(super) mixed_focus_order: Vec<RuntimeFocusOrderEntry>,
    pub(super) wheel: HitOrderIndex,
    pub(super) wheel_targets: RuntimeWheelTargetTraversal,
    pub(super) stateful_order: Vec<WidgetId>,
    pub(super) stateful_ordinals: HashMap<WidgetId, usize>,
    pub(super) paths: RuntimeWidgetPathState,
    pub(super) duplicate_widget_ids: HashSet<WidgetId>,
    pub(super) membership: HashMap<WidgetId, [bool; 7]>,
}

#[derive(Default)]
pub(super) struct RuntimeWheelTargetTraversal {
    order: Vec<WheelHitTarget>,
    visible: Vec<WheelHitTarget>,
}

impl RuntimeWheelTargetTraversal {
    #[allow(dead_code)]
    pub(super) fn order(&self) -> &[WheelHitTarget] {
        &self.order
    }

    pub(super) fn set_order(&mut self, order: Vec<WheelHitTarget>) {
        self.order = order;
        self.visible.clear();
    }

    pub(super) fn refresh_visible(&mut self, layout: &LayoutOutput) {
        self.visible.clear();
        self.visible.extend(
            self.order
                .iter()
                .copied()
                .filter(|target: &WheelHitTarget| layout.rects.contains_key(&target.node_id())),
        );
    }

    pub(super) fn visible(&self) -> &[WheelHitTarget] {
        &self.visible
    }

    pub(super) fn take_order(&mut self) -> Vec<WheelHitTarget> {
        self.visible.clear();
        std::mem::take(&mut self.order)
    }
}

#[derive(Default)]
pub(super) struct RuntimeWidgetPathState {
    pub(super) current: HashMap<WidgetId, WidgetPath>,
    pub(super) previous: HashMap<WidgetId, WidgetPath>,
    pub(super) clip_ancestors: HashMap<WidgetId, ClipAncestors>,
    pub(super) container_hover_suppression: HashSet<WidgetId>,
}

pub(super) struct RuntimeContainerTraversal<Message = ()> {
    pub(super) styled: HitOrderIndex,
    pub(super) scroll: HitOrderIndex,
    pub(super) clip_ancestors: HashMap<NodeId, ClipAncestors>,
    pub(super) scroll_content_by_container: HashMap<NodeId, NodeId>,
    pub(super) layout_interactions: Vec<SurfaceLayoutInteractionRecord<Message>>,
    pub(super) split_pane_runtime: Vec<crate::gui::layout_core::SplitPaneRuntimeStateInput>,
    pub(super) split_pane_dividers: Vec<crate::gui::layout_core::SplitPaneDividerDescriptor>,
    pub(super) split_pane_ratio_action_candidates:
        Vec<SurfaceSplitPaneRatioActionCandidate<Message>>,
    pub(super) split_pane_ratio_action_authorities:
        Vec<super::split_pane_ratio_action::SplitPaneRatioActionAuthority<Message>>,
    pub(super) split_pane_ratio_action_capacity_exhausted: bool,
    pub(super) virtual_layout_registrations:
        Vec<crate::runtime::surface::VirtualLayoutRegistration<Message>>,
    pub(super) layout_targets: Vec<RuntimeLayoutHitTarget<Message>>,
    pub(super) split_pane_separator_projections:
        Vec<super::split_pane_separator::SplitPaneSeparatorProjection>,
    pub(super) layout_hit_region_diagnostics: LayoutHitRegionDiagnostics,
    layout_region_declarations: Vec<LayoutHitRegion>,
}

pub(super) struct RuntimeLayoutHitTarget<Message> {
    pub(super) target: LayoutHitTarget,
    pub(super) contract_version: u16,
    pub(super) state_id: Option<crate::layout::ContainerStateId>,
    pub(super) interaction: std::rc::Rc<dyn LayoutInteraction<Message>>,
    pub(super) revision: LayoutInteractionRevision,
    pub(super) container_bounds: Option<Rect>,
    pub(super) target_bounds: Option<Rect>,
    pub(super) divider_bounds: Option<Rect>,
    pub(super) mounted_state_id: Option<crate::gui::layout_core::MountedContainerStateId>,
    pub(super) split_capture_witness: Option<crate::gui::layout_core::SplitPaneCaptureWitness>,
}

impl<Message> Default for RuntimeContainerTraversal<Message> {
    fn default() -> Self {
        Self {
            styled: HitOrderIndex::default(),
            scroll: HitOrderIndex::default(),
            clip_ancestors: HashMap::new(),
            scroll_content_by_container: HashMap::new(),
            layout_interactions: Vec::new(),
            split_pane_runtime: Vec::new(),
            split_pane_dividers: Vec::new(),
            split_pane_ratio_action_candidates: Vec::new(),
            split_pane_ratio_action_authorities: Vec::new(),
            split_pane_ratio_action_capacity_exhausted: false,
            virtual_layout_registrations: Vec::new(),
            layout_targets: Vec::new(),
            split_pane_separator_projections: Vec::new(),
            layout_hit_region_diagnostics: LayoutHitRegionDiagnostics::default(),
            layout_region_declarations: Vec::new(),
        }
    }
}

impl<Message> RuntimeTraversalState<Message> {
    /// Reconcile private mixed-focus evidence only after a committed boundary.
    ///
    /// The ordinary widget order remains the sole key-routing and public-order
    /// source. A separator is copied into this private traversal sidecar only
    /// when every source marker and current committed projection pair exactly;
    /// any incomplete or malformed set falls back to the unchanged widget-only
    /// sequence.
    pub(super) fn rebuild_mixed_focus_order(
        &mut self,
        lifecycle_phase: RuntimeLifecyclePhase,
        state_store: &super::layout_state::RuntimeLayoutContainerStateStore,
    ) {
        let widget_order = self.widgets.keyboard_focus.order();
        let candidates = &self.widgets.keyboard_focus_order_candidates;
        let projections = &self.containers.split_pane_separator_projections;
        let evidence_is_current = mixed_focus_order_evidence_is_current(
            lifecycle_phase,
            widget_order.len(),
            candidates,
            projections,
            state_store,
        );
        let required_capacity = widget_order.len().saturating_add(projections.len());

        self.widgets.mixed_focus_order.clear();
        if self.widgets.mixed_focus_order.capacity() < required_capacity {
            self.widgets
                .mixed_focus_order
                .reserve(required_capacity - self.widgets.mixed_focus_order.capacity());
        }

        if !evidence_is_current {
            self.widgets.mixed_focus_order.extend(
                widget_order
                    .iter()
                    .copied()
                    .map(RuntimeFocusOrderEntry::Widget),
            );
            return;
        }

        let mut candidate_index = 0;
        for widget_index in 0..=widget_order.len() {
            while candidates
                .get(candidate_index)
                .is_some_and(|candidate| candidate.widget_index == widget_index)
            {
                let candidate = candidates[candidate_index];
                if let Some(projection) = projections
                    .iter()
                    .find(|projection| {
                        split_pane_focus_order_candidate_matches(
                            &candidate,
                            projection,
                            state_store,
                        )
                    })
                    .copied()
                {
                    self.widgets
                        .mixed_focus_order
                        .push(RuntimeFocusOrderEntry::SplitPaneSeparator(projection));
                }
                candidate_index += 1;
            }
            if let Some(widget_id) = widget_order.get(widget_index).copied() {
                self.widgets
                    .mixed_focus_order
                    .push(RuntimeFocusOrderEntry::Widget(widget_id));
            }
        }
    }

    #[cfg(test)]
    pub(in crate::runtime::controller) fn mixed_focus_order(&self) -> &[RuntimeFocusOrderEntry] {
        &self.widgets.mixed_focus_order
    }
}

fn mixed_focus_order_evidence_is_current(
    lifecycle_phase: RuntimeLifecyclePhase,
    widget_count: usize,
    candidates: &[SurfaceSplitPaneFocusOrderCandidate],
    projections: &[super::split_pane_separator::SplitPaneSeparatorProjection],
    state_store: &super::layout_state::RuntimeLayoutContainerStateStore,
) -> bool {
    if lifecycle_phase != RuntimeLifecyclePhase::Running || candidates.len() != projections.len() {
        return false;
    }

    for (index, candidate) in candidates.iter().enumerate() {
        if candidate.widget_index > widget_count
            || candidate.ownership != SplitPaneRuntimeOwnership::RuntimeOwned
            || candidate.target.region_id != crate::gui::layout_core::SPLIT_PANE_DIVIDER_REGION_ID
            || candidate.target.container_id != candidate.state_id.container_id()
            || candidate.target.container_id != candidate.descriptor.container_id
            || candidate.state_schema_version != candidate.state_id.schema_version()
            || !candidate.descriptor.first_min_extent.is_finite()
            || candidate.descriptor.first_min_extent < 0.0
            || !candidate.descriptor.second_min_extent.is_finite()
            || candidate.descriptor.second_min_extent < 0.0
            || !candidate.descriptor.divider_extent.is_finite()
            || candidate.descriptor.divider_extent <= 0.0
            || candidate.descriptor.first_child == candidate.descriptor.second_child
            || (index > 0 && candidate.widget_index < candidates[index - 1].widget_index)
            || candidates[..index]
                .iter()
                .any(|previous| previous.target == candidate.target)
        {
            return false;
        }

        let matching_projections = projections
            .iter()
            .filter(|projection| {
                split_pane_focus_order_candidate_matches(candidate, projection, state_store)
            })
            .count();
        if matching_projections != 1 {
            return false;
        }
    }

    for (index, projection) in projections.iter().enumerate() {
        if projections[..index]
            .iter()
            .any(|previous| previous.target == projection.target)
        {
            return false;
        }

        let matching_candidates = candidates
            .iter()
            .filter(|candidate| {
                split_pane_focus_order_candidate_matches(candidate, projection, state_store)
            })
            .count();
        if matching_candidates != 1 {
            return false;
        }
    }

    true
}

fn split_pane_focus_order_candidate_matches(
    candidate: &SurfaceSplitPaneFocusOrderCandidate,
    projection: &super::split_pane_separator::SplitPaneSeparatorProjection,
    state_store: &super::layout_state::RuntimeLayoutContainerStateStore,
) -> bool {
    candidate.target == projection.target
        && candidate.state_id == projection.state_id
        && candidate.descriptor == projection.descriptor
        && candidate.ownership == projection.ownership
        && candidate.descriptor.axis == projection.axis
        && candidate.contract_version == projection.behavior.contract_version
        && candidate.state_schema_version == projection.behavior.state_schema_version
        && candidate.policy_revision == projection.behavior.policy_revision
        && projection.state_id.is::<SplitPaneRuntimeState>()
        && projection.state_id.schema_version() == candidate.state_schema_version
        && projection.target.region_id == crate::gui::layout_core::SPLIT_PANE_DIVIDER_REGION_ID
        && projection.divider_bounds.has_finite_positive_area()
        && projection.live_ratio.is_finite()
        && (0.0..=1.0).contains(&projection.live_ratio)
        && state_store.current_mounted_state_id(candidate.state_id)
            == Some(projection.mounted_state_id)
}

impl<Message> RuntimeContainerTraversal<Message> {
    pub(super) fn project_layout_targets(
        &mut self,
        layout: &LayoutOutput,
        direction: crate::application::WritingDirection,
    ) {
        self.layout_targets.clear();
        self.layout_hit_region_diagnostics = LayoutHitRegionDiagnostics::default();
        for interaction in &self.layout_interactions {
            let Some(container_bounds) = layout.rects.get(&interaction.id).copied() else {
                continue;
            };
            if !container_bounds.has_finite_positive_area() {
                continue;
            }

            self.layout_region_declarations.clear();
            interaction
                .interaction
                .visit_hit_regions(container_bounds, &mut |region| {
                    if !self
                        .layout_region_declarations
                        .iter()
                        .any(|candidate| candidate.id() == region.id())
                    {
                        self.layout_region_declarations.push(region);
                    } else {
                        self.layout_hit_region_diagnostics.record_duplicate();
                    }
                });

            for region in self.layout_region_declarations.iter().copied() {
                let Some(bounds) = project_layout_region(container_bounds, region.bounds()) else {
                    continue;
                };
                let Some(bounds) = self
                    .layout_clip_for_container(interaction.id, layout)
                    .try_fold(bounds, |bounds, clip| bounds.intersection(clip))
                    .filter(|rect| rect.has_finite_positive_area())
                else {
                    continue;
                };
                self.layout_targets.push(RuntimeLayoutHitTarget {
                    target: LayoutHitTarget {
                        container_id: interaction.id,
                        region_id: region.id(),
                        bounds,
                    },
                    contract_version: interaction.contract_version,
                    state_id: interaction.state.as_ref().map(|state| state.id()),
                    interaction: std::rc::Rc::clone(&interaction.interaction),
                    revision: interaction.revision.clone(),
                    container_bounds: Some(container_bounds),
                    target_bounds: Some(bounds),
                    divider_bounds: None,
                    mounted_state_id: None,
                    split_capture_witness: None,
                });
            }

            let Some(descriptor) = self
                .split_pane_dividers
                .iter()
                .find(|descriptor| descriptor.container_id == interaction.id)
                .copied()
            else {
                continue;
            };
            let Some(first) = layout.rects.get(&descriptor.first_child).copied() else {
                continue;
            };
            let Some(second) = layout.rects.get(&descriptor.second_child).copied() else {
                continue;
            };
            let Some((divider_bounds, _split_bounds)) =
                split_divider_geometry(first, second, descriptor.axis)
            else {
                continue;
            };
            let Some(target_bounds) = std::iter::once(container_bounds)
                .chain(self.layout_clip_for_container(interaction.id, layout))
                .try_fold(divider_bounds, |bounds, clip| bounds.intersection(clip))
                .filter(|rect| rect.has_finite_positive_area())
            else {
                continue;
            };
            self.layout_targets.push(RuntimeLayoutHitTarget {
                target: LayoutHitTarget {
                    container_id: interaction.id,
                    region_id: crate::gui::layout_core::SPLIT_PANE_DIVIDER_REGION_ID,
                    bounds: target_bounds,
                },
                contract_version: interaction.contract_version,
                state_id: interaction.state.as_ref().map(|state| state.id()),
                interaction: std::rc::Rc::clone(&interaction.interaction),
                revision: interaction.revision.clone(),
                container_bounds: Some(container_bounds),
                target_bounds: Some(target_bounds),
                divider_bounds: Some(divider_bounds),
                mounted_state_id: None,
                split_capture_witness: Some(descriptor.witness(container_bounds, direction)),
            });
        }
    }

    pub(super) fn bind_committed_mounted_state_ids(
        &mut self,
        state_store: &super::layout_state::RuntimeLayoutContainerStateStore,
    ) {
        for target in &mut self.layout_targets {
            target.mounted_state_id = target
                .state_id
                .and_then(|state_id| state_store.current_mounted_state_id(state_id));
        }
    }

    pub(super) fn rebuild_split_pane_separator_projections(
        &mut self,
        state_store: &super::layout_state::RuntimeLayoutContainerStateStore,
    ) {
        let desired_capacity = self
            .split_pane_runtime
            .len()
            .min(self.split_pane_dividers.len());
        self.split_pane_separator_projections.clear();
        if self.split_pane_separator_projections.capacity() < desired_capacity {
            self.split_pane_separator_projections
                .reserve_exact(desired_capacity - self.split_pane_separator_projections.capacity());
        }
        for input in &self.split_pane_runtime {
            if self
                .split_pane_runtime
                .iter()
                .filter(|candidate| candidate.container_id == input.container_id)
                .count()
                != 1
            {
                continue;
            }

            let descriptor_count = self
                .split_pane_dividers
                .iter()
                .filter(|descriptor| descriptor.container_id == input.container_id)
                .count();
            if descriptor_count != 1 {
                continue;
            }
            let Some(descriptor) = self
                .split_pane_dividers
                .iter()
                .find(|descriptor| descriptor.container_id == input.container_id)
                .copied()
            else {
                continue;
            };

            let identity = crate::layout::LayoutTargetIdentity::new(
                input.container_id,
                crate::gui::layout_core::SPLIT_PANE_DIVIDER_REGION_ID,
            );
            let target_count = self
                .layout_targets
                .iter()
                .filter(|target| target.target.identity() == identity)
                .count();
            if target_count != 1 {
                continue;
            }
            let Some(target) = self
                .layout_targets
                .iter()
                .find(|target| target.target.identity() == identity)
            else {
                continue;
            };
            if let Some(projection) =
                super::split_pane_separator::build_split_pane_separator_projection(
                    target,
                    *input,
                    descriptor,
                    state_store,
                )
            {
                self.split_pane_separator_projections.push(projection);
            }
        }
    }

    pub(super) fn rebuild_split_pane_ratio_action_authorities(
        &mut self,
        state_store: &super::layout_state::RuntimeLayoutContainerStateStore,
    ) {
        use crate::gui::layout_core::{
            LAYOUT_CAPABILITIES_STATE_CONTRACT_VERSION, SPLIT_PANE_DIVIDER_REGION_ID,
            SplitPaneRuntimeOwnership, SplitPaneRuntimeState,
        };

        const MAX_AUTHORITIES: usize =
            super::split_pane_ratio_action::MAX_SPLIT_PANE_RATIO_ACTION_AUTHORITIES;
        self.split_pane_ratio_action_authorities.clear();
        self.split_pane_ratio_action_capacity_exhausted =
            self.split_pane_ratio_action_candidates.len() > MAX_AUTHORITIES;
        if self.split_pane_ratio_action_capacity_exhausted {
            return;
        }
        let desired_capacity = self.split_pane_ratio_action_candidates.len();
        if self.split_pane_ratio_action_authorities.capacity() < desired_capacity {
            self.split_pane_ratio_action_authorities.reserve_exact(
                desired_capacity - self.split_pane_ratio_action_authorities.capacity(),
            );
        }

        for candidate in &self.split_pane_ratio_action_candidates {
            if self
                .split_pane_ratio_action_candidates
                .iter()
                .filter(|previous| previous.target == candidate.target)
                .count()
                != 1
                || candidate.ownership != SplitPaneRuntimeOwnership::RuntimeOwned
                || candidate.target.region_id != SPLIT_PANE_DIVIDER_REGION_ID
                || candidate.target.container_id != candidate.state_id.container_id()
                || candidate.target.container_id != candidate.descriptor.container_id
                || !candidate.state_id.is::<SplitPaneRuntimeState>()
                || candidate.state_schema_version != candidate.state_id.schema_version()
                || candidate.contract_version != LAYOUT_CAPABILITIES_STATE_CONTRACT_VERSION
                || candidate.descriptor.axis != candidate.policy_revision.axis
                || candidate.descriptor.first_child == candidate.descriptor.second_child
                || !candidate.descriptor.first_min_extent.is_finite()
                || candidate.descriptor.first_min_extent < 0.0
                || !candidate.descriptor.second_min_extent.is_finite()
                || candidate.descriptor.second_min_extent < 0.0
                || !candidate.descriptor.divider_extent.is_finite()
                || candidate.descriptor.divider_extent <= 0.0
            {
                continue;
            }

            let target_count = self
                .layout_targets
                .iter()
                .filter(|target| target.target.identity() == candidate.target)
                .count();
            if target_count != 1 {
                continue;
            }
            let Some(target) = self
                .layout_targets
                .iter()
                .find(|target| target.target.identity() == candidate.target)
            else {
                continue;
            };
            if target.target.region_id != SPLIT_PANE_DIVIDER_REGION_ID
                || target.state_id != Some(candidate.state_id)
                || target.contract_version != candidate.contract_version
                || target.revision != LayoutInteractionRevision::exact(candidate.policy_revision)
                || target.target_bounds != Some(target.target.bounds)
                || !target
                    .container_bounds
                    .is_some_and(crate::gui::types::Rect::has_finite_positive_area)
                || !target
                    .target_bounds
                    .is_some_and(crate::gui::types::Rect::has_finite_positive_area)
                || !target
                    .divider_bounds
                    .is_some_and(crate::gui::types::Rect::has_finite_positive_area)
            {
                continue;
            }

            let descriptor_count = self
                .split_pane_dividers
                .iter()
                .filter(|descriptor| descriptor.container_id == candidate.target.container_id)
                .count();
            if descriptor_count != 1
                || self
                    .split_pane_dividers
                    .iter()
                    .find(|descriptor| descriptor.container_id == candidate.target.container_id)
                    .copied()
                    != Some(candidate.descriptor)
            {
                continue;
            }

            let Some(mounted_state_id) = target.mounted_state_id else {
                continue;
            };
            if state_store.current_mounted_state_id(candidate.state_id) != Some(mounted_state_id) {
                continue;
            }
            let Some(committed) = state_store.lookup_current_state_view(mounted_state_id) else {
                continue;
            };
            let Some(state) = committed.downcast_ref::<SplitPaneRuntimeState>() else {
                continue;
            };
            if state.ownership != SplitPaneRuntimeOwnership::RuntimeOwned
                || !state.ratio.is_finite()
                || !(0.0..=1.0).contains(&state.ratio)
                || !state
                    .policy_revision
                    .runtime_state_compatible(candidate.policy_revision)
            {
                continue;
            }

            let (Some(container_bounds), Some(target_bounds), Some(divider_bounds)) = (
                target.container_bounds,
                target.target_bounds,
                target.divider_bounds,
            ) else {
                continue;
            };

            self.split_pane_ratio_action_authorities.push(
                super::split_pane_ratio_action::SplitPaneRatioActionAuthority {
                    target: candidate.target,
                    state_id: candidate.state_id,
                    mounted_state_id,
                    descriptor: candidate.descriptor,
                    ownership: candidate.ownership,
                    axis: candidate.descriptor.axis,
                    contract_version: candidate.contract_version,
                    state_schema_version: candidate.state_schema_version,
                    policy_revision: candidate.policy_revision,
                    container_bounds,
                    target_bounds,
                    divider_bounds,
                    on_ratio_settled: candidate.on_ratio_settled.clone(),
                },
            );
        }
    }

    fn layout_clip_for_container<'a>(
        &'a self,
        container_id: NodeId,
        layout: &'a LayoutOutput,
    ) -> impl Iterator<Item = Rect> + 'a {
        let own_viewport = layout.viewport_bounds.get(&container_id).copied();
        let ancestors = self
            .clip_ancestors
            .get(&container_id)
            .into_iter()
            .flat_map(|ancestors| ancestors.as_slice().iter().copied())
            .filter_map(|ancestor| {
                layout
                    .viewport_bounds
                    .get(&ancestor)
                    .copied()
                    .or_else(|| layout.rects.get(&ancestor).copied())
            });
        own_viewport.into_iter().chain(ancestors)
    }
}

fn split_divider_geometry(
    first: Rect,
    second: Rect,
    axis: crate::gui::panel::SplitPaneAxis,
) -> Option<(Rect, Rect)> {
    if !first.is_finite() || !second.is_finite() {
        return None;
    }
    let (divider, split) = match axis {
        crate::gui::panel::SplitPaneAxis::Horizontal => {
            let start = first.max.x;
            let end = second.min.x;
            if !start.is_finite() || !end.is_finite() || end <= start {
                return None;
            }
            (
                Rect::from_min_max(
                    crate::gui::types::Point::new(start, first.min.y.min(second.min.y)),
                    crate::gui::types::Point::new(end, first.max.y.max(second.max.y)),
                ),
                Rect::from_min_max(
                    crate::gui::types::Point::new(
                        first.min.x.min(second.min.x),
                        first.min.y.min(second.min.y),
                    ),
                    crate::gui::types::Point::new(
                        first.max.x.max(second.max.x),
                        first.max.y.max(second.max.y),
                    ),
                ),
            )
        }
        crate::gui::panel::SplitPaneAxis::Vertical => {
            let start = first.max.y;
            let end = second.min.y;
            if !start.is_finite() || !end.is_finite() || end <= start {
                return None;
            }
            (
                Rect::from_min_max(
                    crate::gui::types::Point::new(first.min.x.min(second.min.x), start),
                    crate::gui::types::Point::new(first.max.x.max(second.max.x), end),
                ),
                Rect::from_min_max(
                    crate::gui::types::Point::new(
                        first.min.x.min(second.min.x),
                        first.min.y.min(second.min.y),
                    ),
                    crate::gui::types::Point::new(
                        first.max.x.max(second.max.x),
                        first.max.y.max(second.max.y),
                    ),
                ),
            )
        }
    };
    (divider.has_finite_positive_area() && split.has_finite_positive_area())
        .then_some((divider, split))
}

fn project_layout_region(container_bounds: Rect, local_bounds: Rect) -> Option<Rect> {
    let projected = Rect::from_min_max(
        crate::gui::types::Point::new(
            container_bounds.min.x + container_bounds.width() * local_bounds.min.x,
            container_bounds.min.y + container_bounds.height() * local_bounds.min.y,
        ),
        crate::gui::types::Point::new(
            container_bounds.min.x + container_bounds.width() * local_bounds.max.x,
            container_bounds.min.y + container_bounds.height() * local_bounds.max.y,
        ),
    );
    projected.has_finite_positive_area().then_some(projected)
}

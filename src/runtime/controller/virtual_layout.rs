//! Private synchronous virtual-layout registration and materialization bridge.
//!
//! This module is deliberately owned by `SurfaceRuntime`. It does not expose a
//! registration API, schedule policy work, or let a bridge/application object
//! retain materialized slots.

use super::SurfaceRuntime;
use crate::{
    gui::layout_core::{
        VirtualLayoutCompletion, VirtualLayoutLifecycleAdapter, VirtualLayoutMaterializationError,
        VirtualLayoutMaterializationReentry, VirtualLayoutMaterializationStore,
        VirtualLayoutProjectionEvidence, VirtualLayoutProjectionKind, VirtualLayoutRetainReason,
        VirtualLayoutWindowCoordinator,
    },
    gui::types::Rect,
    layout::VirtualLayoutQueryInputParts,
    runtime::{
        SurfaceNode, SurfaceTraversalIndex, UiSurface,
        surface::{
            MAX_VIRTUAL_LAYOUT_REGISTRATIONS, SourceTraversalIndex, VirtualLayoutRegistration,
        },
    },
};
use std::convert::Infallible;

#[derive(Default)]
struct RuntimeVirtualLayoutLifecycle;

impl<Message> VirtualLayoutLifecycleAdapter<SurfaceNode<Message>>
    for RuntimeVirtualLayoutLifecycle
{
    type Error = Infallible;

    fn compatible(
        &self,
        _previous: &VirtualLayoutProjectionKind,
        _next: &VirtualLayoutProjectionKind,
    ) -> Option<bool> {
        Some(true)
    }

    fn unmount(
        &mut self,
        _payload: &SurfaceNode<Message>,
        _evidence: VirtualLayoutProjectionEvidence<'_>,
        _reentry: &VirtualLayoutMaterializationReentry<'_>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    fn reset(
        &mut self,
        _payload: &SurfaceNode<Message>,
        _evidence: VirtualLayoutProjectionEvidence<'_>,
        _reentry: &VirtualLayoutMaterializationReentry<'_>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    fn reconcile(
        &mut self,
        _previous: &SurfaceNode<Message>,
        _next: &SurfaceNode<Message>,
        _evidence: VirtualLayoutProjectionEvidence<'_>,
        _reentry: &VirtualLayoutMaterializationReentry<'_>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    fn mount(
        &mut self,
        _recycled_shell: Option<&SurfaceNode<Message>>,
        _next: &SurfaceNode<Message>,
        _evidence: VirtualLayoutProjectionEvidence<'_>,
        _reentry: &VirtualLayoutMaterializationReentry<'_>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
}

type RuntimeMaterialization<Message> =
    VirtualLayoutMaterializationStore<SurfaceNode<Message>, RuntimeVirtualLayoutLifecycle>;

struct RuntimeVirtualLayoutSubtree<Message> {
    shell: SurfaceNode<Message>,
    items: Vec<SurfaceNode<Message>>,
    registration: VirtualLayoutRegistration<Message>,
}

impl<Message> Clone for RuntimeVirtualLayoutSubtree<Message> {
    fn clone(&self) -> Self {
        Self {
            shell: self.shell.clone(),
            items: self.items.clone(),
            registration: self.registration.clone(),
        }
    }
}

struct RuntimeVirtualLayoutCommittedBatch<Message> {
    query: VirtualLayoutQueryInputParts,
    subtree: RuntimeVirtualLayoutSubtree<Message>,
}

enum RuntimeVirtualLayoutMaterialization<Message> {
    Reused,
    Retained,
    Suppressed,
    Committed(Box<RuntimeVirtualLayoutCommittedBatch<Message>>),
    Retired,
}

struct RuntimeVirtualLayoutRecord<Message> {
    registration: VirtualLayoutRegistration<Message>,
    mount_generation: u64,
    coordinator: VirtualLayoutWindowCoordinator,
    materialization: RuntimeMaterialization<Message>,
    last_query: Option<VirtualLayoutQueryInputParts>,
    cached_subtree: Option<RuntimeVirtualLayoutSubtree<Message>>,
    retired: bool,
}

pub(super) struct RuntimeVirtualLayoutProjectionProbe<Message> {
    pub(super) traversal: SurfaceTraversalIndex<Message>,
    pub(super) source: SourceTraversalIndex,
}

impl<Message> RuntimeVirtualLayoutRecord<Message> {
    fn new(registration: VirtualLayoutRegistration<Message>, mount_generation: u64) -> Self {
        let coordinator = VirtualLayoutWindowCoordinator::new(
            registration.container_id,
            registration.policy_identity.clone(),
            mount_generation,
        );
        let materialization = RuntimeMaterialization::new(&coordinator, Default::default());
        Self {
            registration,
            mount_generation,
            coordinator,
            materialization,
            last_query: None,
            cached_subtree: None,
            retired: false,
        }
    }

    fn update_registration(&mut self, registration: VirtualLayoutRegistration<Message>) {
        self.registration = registration;
        if let Some(cached) = &mut self.cached_subtree {
            cached.registration = self.registration.clone();
        }
    }

    fn needs_query(&self, parts: &VirtualLayoutQueryInputParts) -> bool {
        let Some(previous) = &self.last_query else {
            return true;
        };
        previous.container_id != parts.container_id
            || previous.policy_identity != parts.policy_identity
            || previous.mount_generation != parts.mount_generation
            || previous.viewport != parts.viewport
            || previous.coordinate_space != parts.coordinate_space
            || previous.overscan != parts.overscan
            || previous.budget != parts.budget
            || previous.viewport_revision != parts.viewport_revision
            || previous.data_revision != parts.data_revision
            || previous.policy_revision != parts.policy_revision
            || previous.measurement_revision != parts.measurement_revision
            || previous.semantic_revision != parts.semantic_revision
    }

    fn needs_query_for_viewport(&self, viewport: Rect) -> bool {
        self.needs_query(
            &self
                .registration
                .query_parts(viewport, self.mount_generation),
        )
    }

    fn materialize(&mut self, viewport: Rect) -> RuntimeVirtualLayoutMaterialization<Message> {
        if self.retired {
            return RuntimeVirtualLayoutMaterialization::Retired;
        }
        let parts = self
            .registration
            .query_parts(viewport, self.mount_generation);
        if !self.needs_query(&parts) {
            return RuntimeVirtualLayoutMaterialization::Reused;
        }
        let pending = match self.coordinator.begin_query(parts.clone()) {
            Ok(pending) => pending,
            Err(_) => {
                self.retire();
                return RuntimeVirtualLayoutMaterialization::Retired;
            }
        };
        let outcome = pending.execute(&*self.registration.policy);
        let completion = self.coordinator.complete(pending, outcome);
        let fallback_authorized = self.fallback_authorizes_cached_window(&completion);
        match completion {
            VirtualLayoutCompletion::Committed(commit) => {
                let projector = self.registration.projector();
                match self.materialization.publish(&commit, &projector) {
                    Ok(()) => {
                        let Some(shell) = projector.take_shell() else {
                            self.retire();
                            return RuntimeVirtualLayoutMaterialization::Retired;
                        };
                        let items = self.active_payloads();
                        RuntimeVirtualLayoutMaterialization::Committed(Box::new(
                            RuntimeVirtualLayoutCommittedBatch {
                                query: parts,
                                subtree: RuntimeVirtualLayoutSubtree {
                                    shell,
                                    items,
                                    registration: self.registration.clone(),
                                },
                            },
                        ))
                    }
                    Err(
                        VirtualLayoutMaterializationError::Lifecycle(_)
                        | VirtualLayoutMaterializationError::Reentrant
                        | VirtualLayoutMaterializationError::LifecycleIndeterminate
                        | VirtualLayoutMaterializationError::ForeignContainer
                        | VirtualLayoutMaterializationError::ForeignPolicy
                        | VirtualLayoutMaterializationError::ForeignMount
                        | VirtualLayoutMaterializationError::ForeignOwner
                        | VirtualLayoutMaterializationError::UnstablePolicyIdentity
                        | VirtualLayoutMaterializationError::Unmounted
                        | VirtualLayoutMaterializationError::InvalidCommit
                        | VirtualLayoutMaterializationError::CapacityViolation
                        | VirtualLayoutMaterializationError::DuplicateKey
                        | VirtualLayoutMaterializationError::UnstableKey
                        | VirtualLayoutMaterializationError::DuplicateLogicalIndex
                        | VirtualLayoutMaterializationError::OlderRevision
                        | VirtualLayoutMaterializationError::DuplicateRevision
                        | VirtualLayoutMaterializationError::OlderFence
                        | VirtualLayoutMaterializationError::SlotArithmeticOverflow
                        | VirtualLayoutMaterializationError::GenerationOverflow
                        | VirtualLayoutMaterializationError::UnstableCompatibility
                        | VirtualLayoutMaterializationError::Projection(_)
                        | VirtualLayoutMaterializationError::ProjectionKindChanged,
                    ) => {
                        self.retire();
                        RuntimeVirtualLayoutMaterialization::Retired
                    }
                }
            }
            VirtualLayoutCompletion::Retained { reason, .. } => match reason {
                VirtualLayoutRetainReason::Pending
                | VirtualLayoutRetainReason::Deferred(_)
                | VirtualLayoutRetainReason::Unavailable(_) => {
                    if fallback_authorized {
                        RuntimeVirtualLayoutMaterialization::Retained
                    } else {
                        RuntimeVirtualLayoutMaterialization::Suppressed
                    }
                }
                VirtualLayoutRetainReason::Invalid => {
                    self.retire();
                    RuntimeVirtualLayoutMaterialization::Retired
                }
            },
            VirtualLayoutCompletion::Stale(_) | VirtualLayoutCompletion::Rejected(_) => {
                self.retire();
                RuntimeVirtualLayoutMaterialization::Retired
            }
        }
    }

    fn fallback_authorizes_cached_window(&self, completion: &VirtualLayoutCompletion) -> bool {
        let VirtualLayoutCompletion::Retained { view, .. } = completion else {
            return false;
        };
        if !view.fallback || view.extent.is_none() {
            return false;
        }
        let Some(accepted_revision) = view.accepted_revision else {
            return false;
        };
        if self.materialization.authoritative_revision() != Some(accepted_revision) {
            return false;
        }
        let Some(cached) = &self.cached_subtree else {
            return false;
        };
        let active_slots = self.materialization.active_slots();
        if active_slots.len() != cached.items.len() {
            return false;
        }

        let mut active_items: Vec<_> = active_slots
            .into_iter()
            .map(|slot| slot.item().clone())
            .collect();
        let mut fallback_items = view.entries.clone();
        active_items.sort_by_key(|item| item.logical_index());
        fallback_items.sort_by_key(|item| item.logical_index());
        active_items == fallback_items
    }

    fn active_payloads(&self) -> Vec<SurfaceNode<Message>> {
        self.materialization
            .active_slots()
            .into_iter()
            .map(|slot| slot.payload().clone())
            .collect()
    }

    fn commit_batch(&mut self, batch: RuntimeVirtualLayoutCommittedBatch<Message>) {
        self.last_query = Some(batch.query);
        self.cached_subtree = Some(batch.subtree);
    }

    fn retire(&mut self) {
        if !self.retired {
            self.retired = true;
            let _ = self.materialization.unmount();
        }
        self.cached_subtree = None;
    }
}

impl<Message> Drop for RuntimeVirtualLayoutRecord<Message> {
    fn drop(&mut self) {
        self.retire();
    }
}

/// Runtime-owned bounded registry of mounted virtual-layout records.
pub(in crate::runtime) struct RuntimeVirtualLayoutState<Message> {
    records: Vec<RuntimeVirtualLayoutRecord<Message>>,
    next_mount_generation: u64,
    projection_probe: Option<RuntimeVirtualLayoutProjectionProbe<Message>>,
    #[cfg(test)]
    materialization_passes: u32,
}

impl<Message> Default for RuntimeVirtualLayoutState<Message> {
    fn default() -> Self {
        Self {
            records: Vec::new(),
            next_mount_generation: 0,
            projection_probe: None,
            #[cfg(test)]
            materialization_passes: 0,
        }
    }
}

impl<Message> RuntimeVirtualLayoutState<Message> {
    pub(super) fn prepare_surface(
        &mut self,
        surface: &mut UiSurface<Message>,
        registrations: &[VirtualLayoutRegistration<Message>],
    ) {
        self.clear_projection_probe_if_empty();
        if registrations.len() > MAX_VIRTUAL_LAYOUT_REGISTRATIONS {
            self.retire_all();
            return;
        }
        let mut duplicate_containers = Vec::new();
        for (index, registration) in registrations.iter().enumerate() {
            if registrations[..index]
                .iter()
                .any(|previous| previous.container_id == registration.container_id)
                && !duplicate_containers.contains(&registration.container_id)
            {
                duplicate_containers.push(registration.container_id);
            }
        }
        let accepted: Vec<_> = registrations
            .iter()
            .filter(|registration| !duplicate_containers.contains(&registration.container_id))
            .cloned()
            .collect();

        let mut index = 0;
        while index < self.records.len() {
            if !accepted.iter().any(|registration| {
                registration.container_id == self.records[index].registration.container_id
            }) {
                self.records.remove(index).retire();
            } else {
                index += 1;
            }
        }

        for registration in accepted {
            let Some(existing_index) = self
                .records
                .iter()
                .position(|record| record.registration.container_id == registration.container_id)
            else {
                let Some(generation) = self.allocate_generation() else {
                    continue;
                };
                self.records
                    .push(RuntimeVirtualLayoutRecord::new(registration, generation));
                continue;
            };
            if self.records[existing_index]
                .registration
                .same_scope(&registration)
            {
                self.records[existing_index].update_registration(registration);
            } else {
                self.records[existing_index].retire();
                let Some(generation) = self.allocate_generation() else {
                    continue;
                };
                self.records[existing_index] =
                    RuntimeVirtualLayoutRecord::new(registration, generation);
            }
        }

        for index in 0..self.records.len() {
            if self.records[index].retired {
                continue;
            }
            let container_id = self.records[index].registration.container_id;
            if let Some(cached) = &self.records[index].cached_subtree {
                let installed = surface.install_virtual_layout_subtree(
                    container_id,
                    &cached.shell,
                    &cached.registration,
                    &cached.items,
                );
                if !installed {
                    // The pulled surface no longer admits this mounted record.
                    // Retire it without attempting to lower or retry the old
                    // retained payloads.
                    self.records[index].retire();
                }
                continue;
            }
            let Some(shell) = self.records[index].registration.lowered_shell() else {
                self.records[index].retire();
                continue;
            };
            if !surface.replace_virtual_layout_shell(
                container_id,
                shell,
                self.records[index].registration.clone(),
            ) {
                self.records[index].retire();
            }
        }
        self.clear_projection_probe_if_empty();
    }

    pub(super) fn requires_materialization(
        &self,
        layout: &crate::layout::LayoutOutput,
        force_pass: bool,
    ) -> bool {
        self.records.iter().any(|record| {
            if record.retired {
                return false;
            }
            if force_pass || record.cached_subtree.is_none() {
                return true;
            }
            let Some(viewport) = layout
                .viewport_bounds
                .get(&record.registration.container_id)
                .copied()
            else {
                return true;
            };
            record.needs_query_for_viewport(viewport)
        })
    }

    pub(super) fn materialize_surface(
        &mut self,
        surface: &mut UiSurface<Message>,
        layout: &crate::layout::LayoutOutput,
    ) {
        #[cfg(test)]
        {
            self.materialization_passes = self.materialization_passes.saturating_add(1);
        }
        for index in 0..self.records.len() {
            if self.records[index].retired {
                continue;
            }
            let previous_subtree = self.records[index].cached_subtree.clone();
            let Some(viewport) = layout
                .viewport_bounds
                .get(&self.records[index].registration.container_id)
                .copied()
            else {
                self.records[index].retire();
                suppress_cached_virtual_layout_subtree(
                    surface,
                    self.records[index].registration.container_id,
                    previous_subtree,
                );
                continue;
            };
            match self.records[index].materialize(viewport) {
                RuntimeVirtualLayoutMaterialization::Reused
                | RuntimeVirtualLayoutMaterialization::Retained => {}
                RuntimeVirtualLayoutMaterialization::Suppressed => {
                    suppress_cached_virtual_layout_subtree(
                        surface,
                        self.records[index].registration.container_id,
                        previous_subtree,
                    );
                }
                RuntimeVirtualLayoutMaterialization::Retired => {
                    suppress_cached_virtual_layout_subtree(
                        surface,
                        self.records[index].registration.container_id,
                        previous_subtree,
                    );
                }
                RuntimeVirtualLayoutMaterialization::Committed(batch) => {
                    let container_id = self.records[index].registration.container_id;
                    let installed = surface.install_virtual_layout_subtree(
                        container_id,
                        &batch.subtree.shell,
                        &batch.subtree.registration,
                        &batch.subtree.items,
                    );
                    if installed {
                        self.records[index].commit_batch(*batch);
                    } else {
                        self.records[index].retire();
                        suppress_cached_virtual_layout_subtree(
                            surface,
                            container_id,
                            previous_subtree,
                        );
                    }
                }
            }
        }
        self.clear_projection_probe_if_empty();
    }

    pub(super) fn retire_all(&mut self) {
        for record in &mut self.records {
            record.retire();
        }
        self.records.clear();
        self.projection_probe = None;
    }

    pub(super) fn take_projection_probe(
        &mut self,
    ) -> Option<RuntimeVirtualLayoutProjectionProbe<Message>> {
        self.projection_probe.take()
    }

    pub(super) fn store_projection_probe(
        &mut self,
        probe: RuntimeVirtualLayoutProjectionProbe<Message>,
    ) {
        self.projection_probe = Some(probe);
    }

    fn clear_projection_probe_if_empty(&mut self) {
        if self.is_empty() {
            self.projection_probe = None;
        }
    }

    pub(super) fn is_empty(&self) -> bool {
        self.records.iter().all(|record| record.retired)
    }

    fn allocate_generation(&mut self) -> Option<u64> {
        let next = self.next_mount_generation.checked_add(1)?;
        self.next_mount_generation = next;
        Some(next)
    }
}

fn suppress_cached_virtual_layout_subtree<Message>(
    surface: &mut UiSurface<Message>,
    container_id: crate::layout::NodeId,
    subtree: Option<RuntimeVirtualLayoutSubtree<Message>>,
) {
    let Some(subtree) = subtree else {
        return;
    };
    let _ = surface.replace_virtual_layout_shell(container_id, subtree.shell, subtree.registration);
}

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: crate::runtime::RuntimeBridge<Message>,
{
    pub(super) fn prepare_virtual_layout_surface(
        &mut self,
        registrations: &[VirtualLayoutRegistration<Message>],
    ) {
        self.virtual_layout
            .prepare_surface(&mut self.surface, registrations);
    }

    pub(super) fn rebuild_virtual_layout_shell_layout(&mut self) {
        self.layout_engine.layout_with_state_into(
            &self.layout_root,
            self.viewport,
            &self.layout_state,
            self.layout_debug_options,
            &mut self.layout,
        );
    }

    pub(super) fn materialize_virtual_layout_surface(&mut self) {
        self.virtual_layout
            .materialize_surface(&mut self.surface, &self.layout);
    }

    pub(super) fn requires_virtual_layout_materialization(&self, force_pass: bool) -> bool {
        self.virtual_layout
            .requires_materialization(&self.layout, force_pass)
    }

    pub(super) fn relayout_virtual_layout_for_geometry(&mut self) -> bool {
        if self.virtual_layout.is_empty() {
            return false;
        }
        let registrations = self
            .traversal
            .containers
            .virtual_layout_registrations
            .clone();
        self.prepare_virtual_layout_surface(&registrations);
        let mut traversal = self.take_reusable_traversal_index(true);
        self.layout_root = self.surface.runtime_projection_reusing_with_scratch(
            &mut traversal,
            &mut self.scratch.projection_scroll_stack,
            &mut self.scratch.projection_child_path,
            &mut self.scratch.projection_source,
        );
        self.rebuild_virtual_layout_shell_layout();
        self.materialize_virtual_layout_surface();
        self.layout_root = self.surface.runtime_projection_reusing_with_scratch(
            &mut traversal,
            &mut self.scratch.projection_scroll_stack,
            &mut self.scratch.projection_child_path,
            &mut self.scratch.projection_source,
        );
        self.relayout_with_traversal(traversal);
        self.install_declarative_owner_projection();
        true
    }

    pub(super) fn retire_virtual_layout(&mut self) {
        self.virtual_layout.retire_all();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        application::{View, empty, scroll, spacer, text},
        gui::types::{Rect, Vector2},
        layout::{
            ContainerKind, ContainerPolicy, OverflowPolicy, VirtualLayoutBoundsConfidence,
            VirtualLayoutBudget, VirtualLayoutCoordinateSpace, VirtualLayoutDeferredReason,
            VirtualLayoutExtentCandidate, VirtualLayoutItemCandidate, VirtualLayoutItemKey,
            VirtualLayoutOverscan, VirtualLayoutPolicy, VirtualLayoutPolicyDecision,
            VirtualLayoutPolicyIdentity, VirtualLayoutQueryInput, VirtualLayoutQuerySink,
            VirtualLayoutUnavailableReason, VirtualLayoutVisibility,
        },
        runtime::{
            RuntimeBridge, SurfaceChild, SurfaceNode, UiSurface,
            surface::VirtualLayoutRegistrationRevisions,
        },
        widgets::WidgetSizing,
    };
    use std::{cell::Cell, rc::Rc, sync::Arc};

    const CONTAINER_ID: u64 = 710;
    const ROOT_ID: u64 = 711;
    const ORDINARY_CHILD_ID: u64 = 712;

    struct ReadyPolicy {
        calls: Rc<Cell<u32>>,
        key: u32,
    }

    impl VirtualLayoutPolicy for ReadyPolicy {
        fn query(
            &self,
            _input: &VirtualLayoutQueryInput,
            sink: &mut VirtualLayoutQuerySink,
        ) -> VirtualLayoutPolicyDecision {
            self.calls.set(self.calls.get().saturating_add(1));
            assert!(
                sink.visit(VirtualLayoutItemCandidate::new(
                    VirtualLayoutItemKey::new(self.key),
                    0,
                    Rect::from_xy_size(0.0, 0.0, 100.0, 20.0),
                    VirtualLayoutVisibility::Visible,
                    VirtualLayoutBoundsConfidence::Exact,
                ))
                .is_ok()
            );
            assert!(
                sink.set_extent(VirtualLayoutExtentCandidate::exact(Vector2::new(
                    100.0, 20.0,
                )))
                .is_ok()
            );
            VirtualLayoutPolicyDecision::Ready
        }
    }

    struct ControlledPolicy {
        calls: Rc<Cell<u32>>,
        decision: Cell<VirtualLayoutPolicyDecision>,
        key: u32,
    }

    impl VirtualLayoutPolicy for ControlledPolicy {
        fn query(
            &self,
            _input: &VirtualLayoutQueryInput,
            sink: &mut VirtualLayoutQuerySink,
        ) -> VirtualLayoutPolicyDecision {
            self.calls.set(self.calls.get().saturating_add(1));
            let decision = self.decision.get();
            if decision == VirtualLayoutPolicyDecision::Ready {
                assert!(
                    sink.visit(VirtualLayoutItemCandidate::new(
                        VirtualLayoutItemKey::new(self.key),
                        0,
                        Rect::from_xy_size(0.0, 0.0, 100.0, 20.0),
                        VirtualLayoutVisibility::Visible,
                        VirtualLayoutBoundsConfidence::Exact,
                    ))
                    .is_ok()
                );
                assert!(
                    sink.set_extent(VirtualLayoutExtentCandidate::exact(Vector2::new(
                        100.0, 20.0,
                    )))
                    .is_ok()
                );
            }
            decision
        }
    }

    type VirtualLayoutItemFactory = Rc<dyn Fn(&crate::layout::VirtualLayoutItem) -> View<()>>;

    struct RegistrationParts {
        policy: Rc<dyn VirtualLayoutPolicy>,
        policy_identity: VirtualLayoutPolicyIdentity,
        revisions: VirtualLayoutRegistrationRevisions,
        shell: Rc<dyn Fn() -> View<()>>,
        item: VirtualLayoutItemFactory,
        kind: Rc<dyn Fn(&crate::layout::VirtualLayoutItem) -> VirtualLayoutPolicyIdentity>,
    }

    fn registration_with_parts(parts: RegistrationParts) -> VirtualLayoutRegistration<()> {
        VirtualLayoutRegistration::new(
            CONTAINER_ID,
            parts.policy_identity,
            parts.policy,
            VirtualLayoutCoordinateSpace::logical(),
            VirtualLayoutOverscan::new(0.0, 0.0).expect("finite overscan"),
            VirtualLayoutBudget::new(4),
            parts.revisions,
            parts.shell,
            parts.item,
            parts.kind,
        )
    }

    fn registration(
        policy: Rc<dyn VirtualLayoutPolicy>,
        policy_identity: VirtualLayoutPolicyIdentity,
    ) -> VirtualLayoutRegistration<()> {
        registration_with_parts(RegistrationParts {
            policy,
            policy_identity,
            revisions: Default::default(),
            shell: Rc::new(|| scroll(spacer::<()>())),
            item: Rc::new(|_| text::<()>("virtual item")),
            kind: Rc::new(|_| VirtualLayoutPolicyIdentity::new("item-kind")),
        })
    }

    fn surface(registration: VirtualLayoutRegistration<()>) -> UiSurface<()> {
        UiSurface::new(
            SurfaceNode::container(
                CONTAINER_ID,
                ContainerPolicy {
                    kind: ContainerKind::ScrollView,
                    overflow: OverflowPolicy::Scroll,
                    ..ContainerPolicy::default()
                },
                Vec::<SurfaceChild<()>>::new(),
            )
            .with_virtual_layout_registration(registration),
        )
    }

    fn ordinary_surface() -> UiSurface<()> {
        UiSurface::new(SurfaceNode::container(
            CONTAINER_ID,
            ContainerPolicy::default(),
            vec![SurfaceChild::new(
                crate::layout::SlotParams {
                    size_main: crate::layout::SizeModeMain::Fixed(20.0),
                    size_cross: crate::layout::SizeModeCross::Fixed(48.0),
                    constraints: crate::layout::Constraints::unconstrained(),
                    margin: Default::default(),
                    align_cross_override: None,
                    allow_fixed_compress: false,
                },
                SurfaceNode::text(
                    ORDINARY_CHILD_ID,
                    "ordinary child",
                    WidgetSizing::fixed(Vector2::new(48.0, 20.0)),
                ),
            )],
        ))
    }

    fn duplicate_surface(
        first: VirtualLayoutRegistration<()>,
        second: VirtualLayoutRegistration<()>,
    ) -> UiSurface<()> {
        let container = |registration| {
            SurfaceNode::container(
                CONTAINER_ID,
                ContainerPolicy {
                    kind: ContainerKind::ScrollView,
                    overflow: OverflowPolicy::Scroll,
                    ..ContainerPolicy::default()
                },
                Vec::<SurfaceChild<()>>::new(),
            )
            .with_virtual_layout_registration(registration)
        };
        UiSurface::new(SurfaceNode::container(
            ROOT_ID,
            ContainerPolicy::default(),
            vec![
                SurfaceChild::fill(container(first)),
                SurfaceChild::fill(container(second)),
            ],
        ))
    }

    #[test]
    fn duplicate_registration_rejects_all_candidates_without_a_winner() {
        let first_calls = Rc::new(Cell::new(0));
        let second_calls = Rc::new(Cell::new(0));
        let first = registration_with_parts(RegistrationParts {
            policy: Rc::new(ReadyPolicy {
                calls: Rc::clone(&first_calls),
                key: 9,
            }),
            policy_identity: VirtualLayoutPolicyIdentity::new("duplicate-first-policy"),
            revisions: Default::default(),
            shell: Rc::new(|| scroll(spacer::<()>())),
            item: Rc::new(|_| text::<()>("first item")),
            kind: Rc::new(|_| VirtualLayoutPolicyIdentity::new("duplicate-first-kind")),
        });
        let second = registration_with_parts(RegistrationParts {
            policy: Rc::new(ReadyPolicy {
                calls: Rc::clone(&second_calls),
                key: 10,
            }),
            policy_identity: VirtualLayoutPolicyIdentity::new("duplicate-second-policy"),
            revisions: Default::default(),
            shell: Rc::new(|| scroll(spacer::<()>())),
            item: Rc::new(|_| text::<()>("second item")),
            kind: Rc::new(|_| VirtualLayoutPolicyIdentity::new("duplicate-second-kind")),
        });
        let mut runtime = SurfaceRuntime::new(
            TestBridge {
                surface: duplicate_surface(first, second),
            },
            Vector2::new(160.0, 80.0),
        );

        assert_eq!(runtime.surface().layout_node().id(), ROOT_ID);
        assert!(runtime.virtual_layout.records.is_empty());
        assert_eq!(first_calls.get(), 0);
        assert_eq!(second_calls.get(), 0);
        let crate::layout::LayoutNode::Container(root) = runtime.surface().layout_node() else {
            panic!("duplicate-registration surface should retain its root container");
        };
        assert_eq!(root.children.len(), 2);

        runtime.refresh_with_scope(crate::runtime::RepaintScope::Projection);

        assert!(runtime.virtual_layout.records.is_empty());
        assert_eq!(first_calls.get(), 0);
        assert_eq!(second_calls.get(), 0);
        assert_eq!(runtime.surface().layout_node().id(), ROOT_ID);
    }

    struct TestBridge {
        surface: UiSurface<()>,
    }

    impl RuntimeBridge<()> for TestBridge {
        fn project_surface(&mut self) -> Arc<UiSurface<()>> {
            crate::runtime::test_arc_surface(self.surface.clone())
        }

        fn pull_surface(&mut self) -> UiSurface<()> {
            self.surface.clone()
        }
    }

    fn source_node_ids(surface: &UiSurface<()>) -> Vec<u64> {
        surface
            .runtime_source_traversal_index()
            .records
            .into_iter()
            .map(|record| record.node_id)
            .collect()
    }

    fn assert_authoritative_source(runtime: &SurfaceRuntime<TestBridge, ()>) {
        let expected = source_node_ids(runtime.surface());
        let actual = runtime
            .scratch
            .projection_source
            .records
            .iter()
            .map(|record| record.node_id)
            .collect::<Vec<_>>();
        assert_eq!(actual, expected);
    }

    #[test]
    fn ordinary_startup_keeps_projection_source_authoritative() {
        let runtime = SurfaceRuntime::new(
            TestBridge {
                surface: ordinary_surface(),
            },
            Vector2::new(160.0, 80.0),
        );

        assert_authoritative_source(&runtime);
        assert_eq!(
            runtime.declarative_owner_projection().installation_count(),
            1
        );
    }

    #[test]
    fn runtime_admits_shell_and_complete_batch_before_installing_children() {
        let calls = Rc::new(Cell::new(0));
        let policy = Rc::new(ReadyPolicy {
            calls: Rc::clone(&calls),
            key: 1,
        });
        let runtime = SurfaceRuntime::new(
            TestBridge {
                surface: surface(registration(
                    policy,
                    VirtualLayoutPolicyIdentity::new("policy"),
                )),
            },
            Vector2::new(160.0, 80.0),
        );

        assert_eq!(calls.get(), 1);
        assert_eq!(runtime.virtual_layout.records.len(), 1);
        assert_eq!(
            runtime.virtual_layout.records[0]
                .materialization
                .active_slots()
                .len(),
            1
        );
        assert_eq!(runtime.surface().layout_node().id(), CONTAINER_ID);
        let crate::layout::LayoutNode::Container(root) = runtime.surface().layout_node() else {
            panic!("virtual shell should remain a layout container");
        };
        assert_eq!(root.children.len(), 2, "shell plus one admitted item");
        assert_authoritative_source(&runtime);
        assert_eq!(
            runtime.declarative_owner_projection().installation_count(),
            1
        );
    }

    #[test]
    fn virtual_geometry_relayout_keeps_projection_source_authoritative() {
        let mut runtime = SurfaceRuntime::new(
            TestBridge {
                surface: surface(registration(
                    Rc::new(ReadyPolicy {
                        calls: Rc::new(Cell::new(0)),
                        key: 3,
                    }),
                    VirtualLayoutPolicyIdentity::new("geometry-policy"),
                )),
            },
            Vector2::new(160.0, 80.0),
        );
        let source_capacity = runtime.scratch.projection_source.records.capacity();

        assert!(runtime.relayout_virtual_layout_for_geometry());

        assert_authoritative_source(&runtime);
        assert!(runtime.scratch.projection_source.records.capacity() >= source_capacity);
        assert_eq!(
            runtime.declarative_owner_projection().installation_count(),
            2
        );
    }

    #[test]
    fn unchanged_projection_reuses_the_active_window_without_requerying() {
        let calls = Rc::new(Cell::new(0));
        let shell_constructions = Rc::new(Cell::new(0_u32));
        let item_projections = Rc::new(Cell::new(0_u32));
        let kind_projections = Rc::new(Cell::new(0_u32));
        let policy = Rc::new(ReadyPolicy {
            calls: Rc::clone(&calls),
            key: 2,
        });
        let shell_counter = Rc::clone(&shell_constructions);
        let item_counter = Rc::clone(&item_projections);
        let kind_counter = Rc::clone(&kind_projections);
        let registration = registration_with_parts(RegistrationParts {
            policy,
            policy_identity: VirtualLayoutPolicyIdentity::new("policy"),
            revisions: Default::default(),
            shell: Rc::new(move || {
                shell_counter.set(shell_counter.get().saturating_add(1));
                scroll(spacer::<()>())
            }),
            item: Rc::new(move |_| {
                item_counter.set(item_counter.get().saturating_add(1));
                text::<()>("virtual item")
            }),
            kind: Rc::new(move |_| {
                kind_counter.set(kind_counter.get().saturating_add(1));
                VirtualLayoutPolicyIdentity::new("item-kind")
            }),
        });
        let mut runtime = SurfaceRuntime::new(
            TestBridge {
                surface: surface(registration),
            },
            Vector2::new(160.0, 80.0),
        );

        let expected_hit_order = runtime.traversal.widgets.hit_order.clone();
        let expected_focus_order = runtime.traversal.widgets.focusable.order().to_vec();
        let expected_widget_paths = runtime.traversal.widgets.paths.current.clone();
        let expected_virtual_registration_count = runtime
            .traversal
            .containers
            .virtual_layout_registrations
            .len();

        let before_refresh = (
            calls.get(),
            shell_constructions.get(),
            item_projections.get(),
            kind_projections.get(),
            runtime.refresh_counters().runtime_projection,
            runtime.refresh_counters().layout,
            runtime.virtual_layout.materialization_passes,
        );
        let owner_installations = runtime.declarative_owner_projection().installation_count();
        let source_capacity = runtime.scratch.projection_source.records.capacity();
        let stale_source_record = runtime
            .scratch
            .projection_source
            .records
            .first()
            .cloned()
            .expect("virtual startup should have source records");
        runtime
            .scratch
            .projection_source
            .records
            .push(stale_source_record);
        let _ = runtime.take_frame_refresh_diagnostics();
        runtime.refresh_with_scope(crate::runtime::RepaintScope::Projection);

        let frame = runtime.take_frame_refresh_diagnostics();
        assert_eq!(
            frame.effective_scope,
            crate::runtime::RepaintScope::Projection,
            "unchanged cached refresh evidence: {frame:?}"
        );
        assert_eq!(calls.get(), before_refresh.0);
        assert_eq!(shell_constructions.get(), before_refresh.1);
        assert_eq!(item_projections.get(), before_refresh.2);
        assert_eq!(kind_projections.get(), before_refresh.3);
        assert_eq!(
            runtime.refresh_counters().runtime_projection,
            before_refresh.4 + 1,
            "unchanged cached refresh must use only its initial runtime projection"
        );
        assert_eq!(runtime.refresh_counters().layout, before_refresh.5);
        assert_eq!(
            runtime.virtual_layout.materialization_passes,
            before_refresh.6
        );
        assert!(runtime.base_paint_plan_reuse_eligible());
        assert_authoritative_source(&runtime);
        assert!(runtime.scratch.projection_source.records.capacity() >= source_capacity);
        assert_eq!(
            runtime.declarative_owner_projection().installation_count(),
            owner_installations + 1
        );
        assert_eq!(
            frame.view_delta.effect,
            crate::runtime::surface::ViewDeltaEffect::Unchanged
        );
        assert_eq!(
            runtime.virtual_layout.records[0]
                .last_query
                .as_ref()
                .unwrap()
                .viewport_revision,
            0
        );
        assert_eq!(
            runtime.virtual_layout.records[0]
                .materialization
                .active_slots()
                .len(),
            1
        );
        assert!(runtime.virtual_layout.records[0].cached_subtree.is_some());
        assert_eq!(runtime.traversal.widgets.hit_order, expected_hit_order);
        assert_eq!(
            runtime.traversal.widgets.focusable.order(),
            expected_focus_order.as_slice()
        );
        assert_eq!(
            runtime.traversal.widgets.paths.current,
            expected_widget_paths
        );
        assert_eq!(
            runtime
                .traversal
                .containers
                .virtual_layout_registrations
                .len(),
            expected_virtual_registration_count
        );
        assert!(runtime.virtual_layout.projection_probe.is_some());
        runtime.virtual_layout.retire_all();
        assert!(runtime.virtual_layout.projection_probe.is_none());
    }

    #[test]
    fn provisional_virtual_probe_does_not_replace_accepted_owner_projection() {
        let registration = registration_with_parts(RegistrationParts {
            policy: Rc::new(ReadyPolicy {
                calls: Rc::new(Cell::new(0)),
                key: 12,
            }),
            policy_identity: VirtualLayoutPolicyIdentity::new("probe-isolation-policy"),
            revisions: Default::default(),
            shell: Rc::new(|| scroll(spacer::<()>())),
            item: Rc::new(|_| text::<()>("virtual item").key("probe-owner")),
            kind: Rc::new(|_| VirtualLayoutPolicyIdentity::new("item-kind")),
        });
        let mut runtime = SurfaceRuntime::new(
            TestBridge {
                surface: surface(registration),
            },
            Vector2::new(160.0, 80.0),
        );

        runtime.refresh_with_scope(crate::runtime::RepaintScope::Projection);
        let accepted_before_probe_mutation = runtime
            .declarative_owner_projection()
            .accepted_keyed_nodes()
            .to_vec();
        let installations_before_probe_mutation =
            runtime.declarative_owner_projection().installation_count();
        let accepted_keyed_node = accepted_before_probe_mutation
            .first()
            .expect("keyed virtual item should have accepted owner metadata");
        let accepted_identity =
            super::super::declarative_owner::DeclarativeOwnerIdentity::KeyedNode {
                structural_scope: accepted_keyed_node.identity.structural_scope,
            };
        let accepted_token_before = runtime
            .declarative_owner_ledger()
            .live_records()
            .iter()
            .find(|record| record.token.identity() == accepted_identity)
            .map(|record| record.token.clone())
            .expect("keyed virtual item should have a live owner token");
        let generation_before = accepted_token_before.generation();
        let next_generation_before = runtime.declarative_owner_ledger().next_generation();
        let reconciliations_before = runtime.declarative_owner_ledger().reconciliation_count();
        {
            let probe = runtime
                .virtual_layout
                .projection_probe
                .as_mut()
                .expect("unchanged virtual refresh should retain a provisional probe");
            probe.source.records.clear();
        }

        assert!(
            runtime
                .declarative_owner_ledger()
                .is_live(&accepted_token_before)
        );
        assert_eq!(
            runtime.declarative_owner_ledger().next_generation(),
            next_generation_before
        );
        assert_eq!(
            runtime.declarative_owner_ledger().reconciliation_count(),
            reconciliations_before
        );

        runtime.refresh_with_scope(crate::runtime::RepaintScope::Projection);

        let accepted_token_after = runtime
            .declarative_owner_ledger()
            .live_records()
            .iter()
            .find(|record| record.token.identity() == accepted_identity)
            .map(|record| record.token.clone())
            .expect("authoritative materialization should retain the keyed owner token");

        assert_eq!(
            runtime
                .declarative_owner_projection()
                .accepted_keyed_nodes(),
            accepted_before_probe_mutation.as_slice()
        );
        assert_eq!(
            runtime.declarative_owner_projection().installation_count(),
            installations_before_probe_mutation + 1
        );
        assert_eq!(accepted_token_after, accepted_token_before);
        assert_eq!(accepted_token_after.generation(), generation_before);
        assert_eq!(
            runtime.declarative_owner_ledger().next_generation(),
            next_generation_before
        );
        assert_eq!(
            runtime.declarative_owner_ledger().reconciliation_count(),
            reconciliations_before + 1
        );
        assert_authoritative_source(&runtime);
    }

    #[test]
    fn same_id_ordinary_container_replaces_admitted_virtual_container() {
        let calls = Rc::new(Cell::new(0));
        let registration = registration(
            Rc::new(ReadyPolicy {
                calls: Rc::clone(&calls),
                key: 11,
            }),
            VirtualLayoutPolicyIdentity::new("same-id-transition-policy"),
        );
        let mut runtime = SurfaceRuntime::new(
            TestBridge {
                surface: surface(registration),
            },
            Vector2::new(160.0, 80.0),
        );
        assert_eq!(calls.get(), 1);

        runtime.refresh_with_scope(crate::runtime::RepaintScope::Projection);
        assert!(runtime.virtual_layout.projection_probe.is_some());
        let calls_before_transition = calls.get();

        runtime.bridge_mut().surface = ordinary_surface();
        runtime.refresh_with_scope(crate::runtime::RepaintScope::Projection);

        assert!(runtime.virtual_layout.records.is_empty());
        assert!(runtime.virtual_layout.projection_probe.is_none());
        assert_eq!(calls.get(), calls_before_transition);

        let crate::layout::LayoutNode::Container(installed_root) = runtime.surface().layout_node()
        else {
            panic!("same-ID ordinary transition should retain its container");
        };
        assert_eq!(installed_root.id, CONTAINER_ID);
        assert_eq!(installed_root.children.len(), 1);
        assert_eq!(installed_root.children[0].child.id(), ORDINARY_CHILD_ID);

        let installed_projection = runtime.surface().runtime_projection();
        assert_eq!(
            installed_projection.layout_root,
            runtime.surface().layout_node()
        );
        assert!(
            installed_projection
                .traversal
                .widget_paint_order
                .contains(&ORDINARY_CHILD_ID)
        );
        assert!(
            installed_projection
                .traversal
                .virtual_layout_registrations
                .is_empty()
        );
        assert_authoritative_source(&runtime);

        let installed_traversal = runtime.surface().runtime_traversal_index();
        assert!(
            installed_traversal
                .widget_paint_order
                .contains(&ORDINARY_CHILD_ID)
        );
        assert!(installed_traversal.virtual_layout_registrations.is_empty());

        let crate::layout::LayoutNode::Container(layout_root) = &runtime.layout_root else {
            panic!("final layout root should retain the ordinary container");
        };
        assert_eq!(layout_root.id, CONTAINER_ID);
        assert_eq!(layout_root.children.len(), 1);
        assert_eq!(layout_root.children[0].child.id(), ORDINARY_CHILD_ID);
        assert!(runtime.layout().rects.contains_key(&CONTAINER_ID));
        assert!(runtime.layout().rects.contains_key(&ORDINARY_CHILD_ID));
    }

    #[test]
    fn conservative_shell_evidence_keeps_the_normal_fallback_path() {
        let calls = Rc::new(Cell::new(0));
        let shell_constructions = Rc::new(Cell::new(0_u32));
        let shell_counter = Rc::clone(&shell_constructions);
        let registration = registration_with_parts(RegistrationParts {
            policy: Rc::new(ReadyPolicy {
                calls: Rc::clone(&calls),
                key: 6,
            }),
            policy_identity: VirtualLayoutPolicyIdentity::new("conservative-shell-policy"),
            revisions: Default::default(),
            shell: Rc::new(move || {
                shell_counter.set(shell_counter.get().saturating_add(1));
                scroll(empty::<()>())
            }),
            item: Rc::new(|_| text::<()>("virtual item")),
            kind: Rc::new(|_| VirtualLayoutPolicyIdentity::new("item-kind")),
        });
        let mut runtime = SurfaceRuntime::new(
            TestBridge {
                surface: surface(registration),
            },
            Vector2::new(160.0, 80.0),
        );
        let before_layout = runtime.refresh_counters().layout;
        let before_materialization = runtime.virtual_layout.materialization_passes;
        let before_shells = shell_constructions.get();
        let before_calls = calls.get();

        let _ = runtime.take_frame_refresh_diagnostics();
        runtime.refresh_with_scope(crate::runtime::RepaintScope::Projection);

        let frame = runtime.take_frame_refresh_diagnostics();
        assert_eq!(frame.effective_scope, crate::runtime::RepaintScope::Surface);
        assert_eq!(
            frame.view_delta.effect,
            crate::runtime::surface::ViewDeltaEffect::Structural
        );
        assert_eq!(runtime.refresh_counters().layout, before_layout + 1);
        assert_eq!(
            runtime.virtual_layout.materialization_passes,
            before_materialization + 1
        );
        assert_eq!(shell_constructions.get(), before_shells);
        assert_eq!(calls.get(), before_calls);
        assert!(!runtime.base_paint_plan_reuse_eligible());
    }

    #[test]
    fn deferred_query_retains_only_a_complete_matching_fallback_window() {
        let calls = Rc::new(Cell::new(0));
        let controlled = Rc::new(ControlledPolicy {
            calls: Rc::clone(&calls),
            decision: Cell::new(VirtualLayoutPolicyDecision::Ready),
            key: 7,
        });
        let policy: Rc<dyn VirtualLayoutPolicy> = controlled.clone();
        let initial = registration_with_parts(RegistrationParts {
            policy: Rc::clone(&policy),
            policy_identity: VirtualLayoutPolicyIdentity::new("deferred-policy"),
            revisions: Default::default(),
            shell: Rc::new(|| scroll(spacer::<()>())),
            item: Rc::new(|_| text::<()>("virtual item")),
            kind: Rc::new(|_| VirtualLayoutPolicyIdentity::new("item-kind")),
        });
        let mut runtime = SurfaceRuntime::new(
            TestBridge {
                surface: surface(initial),
            },
            Vector2::new(160.0, 80.0),
        );
        assert_eq!(calls.get(), 1);
        let before_materialization = runtime.virtual_layout.materialization_passes;

        controlled
            .decision
            .set(VirtualLayoutPolicyDecision::Deferred(
                VirtualLayoutDeferredReason::DataPending,
            ));
        runtime.bridge_mut().surface = surface(registration_with_parts(RegistrationParts {
            policy,
            policy_identity: VirtualLayoutPolicyIdentity::new("deferred-policy"),
            revisions: VirtualLayoutRegistrationRevisions {
                viewport: 1,
                ..Default::default()
            },
            shell: Rc::new(|| scroll(spacer::<()>())),
            item: Rc::new(|_| text::<()>("virtual item")),
            kind: Rc::new(|_| VirtualLayoutPolicyIdentity::new("item-kind")),
        }));
        runtime.refresh_with_scope(crate::runtime::RepaintScope::Projection);

        assert_eq!(calls.get(), 2);
        assert_eq!(
            runtime.virtual_layout.materialization_passes,
            before_materialization + 1
        );
        assert!(!runtime.virtual_layout.records[0].retired);
        assert!(runtime.virtual_layout.records[0].cached_subtree.is_some());
        assert_eq!(
            runtime.virtual_layout.records[0]
                .materialization
                .active_slots()
                .len(),
            1
        );
        assert_eq!(
            runtime.virtual_layout.records[0]
                .last_query
                .as_ref()
                .expect("the deferred query must remain retryable")
                .viewport_revision,
            0
        );
        let crate::layout::LayoutNode::Container(root) = runtime.surface().layout_node() else {
            panic!("matching fallback should retain the active item");
        };
        assert_eq!(root.children.len(), 2);
    }

    #[test]
    fn unavailable_query_suppresses_stale_items_and_remains_retryable() {
        let calls = Rc::new(Cell::new(0));
        let controlled = Rc::new(ControlledPolicy {
            calls: Rc::clone(&calls),
            decision: Cell::new(VirtualLayoutPolicyDecision::Ready),
            key: 8,
        });
        let policy: Rc<dyn VirtualLayoutPolicy> = controlled.clone();
        let initial = registration_with_parts(RegistrationParts {
            policy: Rc::clone(&policy),
            policy_identity: VirtualLayoutPolicyIdentity::new("unavailable-policy"),
            revisions: Default::default(),
            shell: Rc::new(|| scroll(spacer::<()>())),
            item: Rc::new(|_| text::<()>("virtual item")),
            kind: Rc::new(|_| VirtualLayoutPolicyIdentity::new("item-kind")),
        });
        let mut runtime = SurfaceRuntime::new(
            TestBridge {
                surface: surface(initial),
            },
            Vector2::new(160.0, 80.0),
        );
        assert_eq!(calls.get(), 1);

        controlled
            .decision
            .set(VirtualLayoutPolicyDecision::Unavailable(
                VirtualLayoutUnavailableReason::DataUnavailable,
            ));
        runtime.bridge_mut().surface = surface(registration_with_parts(RegistrationParts {
            policy: Rc::clone(&policy),
            policy_identity: VirtualLayoutPolicyIdentity::new("unavailable-policy"),
            revisions: VirtualLayoutRegistrationRevisions {
                data: 1,
                ..Default::default()
            },
            shell: Rc::new(|| scroll(spacer::<()>())),
            item: Rc::new(|_| text::<()>("virtual item")),
            kind: Rc::new(|_| VirtualLayoutPolicyIdentity::new("item-kind")),
        }));
        runtime.refresh_with_scope(crate::runtime::RepaintScope::Projection);

        assert_eq!(calls.get(), 2);
        assert!(!runtime.virtual_layout.records[0].retired);
        assert!(runtime.virtual_layout.records[0].cached_subtree.is_some());
        assert_eq!(
            runtime.virtual_layout.records[0]
                .last_query
                .as_ref()
                .expect("the unavailable query must remain retryable")
                .data_revision,
            0
        );
        let crate::layout::LayoutNode::Container(root) = runtime.surface().layout_node() else {
            panic!("unavailable fallback should retain the shell container");
        };
        assert_eq!(root.children.len(), 1);

        controlled.decision.set(VirtualLayoutPolicyDecision::Ready);
        runtime.refresh_with_scope(crate::runtime::RepaintScope::Projection);

        assert_eq!(calls.get(), 3);
        assert!(!runtime.virtual_layout.records[0].retired);
        assert_eq!(
            runtime.virtual_layout.records[0]
                .last_query
                .as_ref()
                .expect("the retry should commit a new query")
                .data_revision,
            1
        );
        let crate::layout::LayoutNode::Container(root) = runtime.surface().layout_node() else {
            panic!("a successful retry should reinstall the active item");
        };
        assert_eq!(root.children.len(), 2);
    }

    #[test]
    fn invalid_shell_lowering_retires_and_suppresses_without_retrying() {
        let shell_constructions = Rc::new(Cell::new(0_u32));
        let policy_calls = Rc::new(Cell::new(0));
        let shell_counter = Rc::clone(&shell_constructions);
        let registration = registration_with_parts(RegistrationParts {
            policy: Rc::new(ReadyPolicy {
                calls: Rc::clone(&policy_calls),
                key: 3,
            }),
            policy_identity: VirtualLayoutPolicyIdentity::new("invalid-shell-policy"),
            revisions: Default::default(),
            shell: Rc::new(move || {
                shell_counter.set(shell_counter.get().saturating_add(1));
                text::<()>("invalid shell").id(CONTAINER_ID + 1)
            }),
            item: Rc::new(|_| text::<()>("virtual item")),
            kind: Rc::new(|_| VirtualLayoutPolicyIdentity::new("item-kind")),
        });
        let mut runtime = SurfaceRuntime::new(
            TestBridge {
                surface: surface(registration),
            },
            Vector2::new(160.0, 80.0),
        );

        assert_eq!(shell_constructions.get(), 1);
        assert_eq!(policy_calls.get(), 0);
        assert!(runtime.virtual_layout.records[0].retired);
        assert!(runtime.virtual_layout.records[0].cached_subtree.is_none());
        assert!(
            runtime.virtual_layout.records[0]
                .materialization
                .active_slots()
                .is_empty()
        );
        let crate::layout::LayoutNode::Container(root) = runtime.surface().layout_node() else {
            panic!("invalid shell test should retain the application container");
        };
        assert!(root.children.is_empty());

        runtime.refresh_with_scope(crate::runtime::RepaintScope::Projection);

        assert_eq!(shell_constructions.get(), 1);
        assert_eq!(policy_calls.get(), 0);
        assert!(runtime.virtual_layout.records[0].retired);
        let crate::layout::LayoutNode::Container(root) = runtime.surface().layout_node() else {
            panic!("invalid shell test should retain the application container");
        };
        assert!(root.children.is_empty());
    }

    #[test]
    fn invalid_complete_batch_admission_retires_and_suppresses_without_retrying() {
        let policy_calls = Rc::new(Cell::new(0));
        let invalid_item_projections = Rc::new(Cell::new(0_u32));
        let policy: Rc<dyn VirtualLayoutPolicy> = Rc::new(ReadyPolicy {
            calls: Rc::clone(&policy_calls),
            key: 4,
        });
        let valid_registration = registration_with_parts(RegistrationParts {
            policy: Rc::clone(&policy),
            policy_identity: VirtualLayoutPolicyIdentity::new("batch-policy"),
            revisions: Default::default(),
            shell: Rc::new(|| scroll(spacer::<()>())),
            item: Rc::new(|_| text::<()>("valid item")),
            kind: Rc::new(|_| VirtualLayoutPolicyIdentity::new("item-kind")),
        });
        let invalid_item_counter = Rc::clone(&invalid_item_projections);
        let invalid_registration = registration_with_parts(RegistrationParts {
            policy,
            policy_identity: VirtualLayoutPolicyIdentity::new("batch-policy"),
            revisions: VirtualLayoutRegistrationRevisions {
                data: 1,
                ..Default::default()
            },
            shell: Rc::new(|| scroll(spacer::<()>())),
            item: Rc::new(move |_| {
                invalid_item_counter.set(invalid_item_counter.get().saturating_add(1));
                text::<()>("invalid item").id(CONTAINER_ID + 1)
            }),
            kind: Rc::new(|_| VirtualLayoutPolicyIdentity::new("item-kind")),
        });
        let mut runtime = SurfaceRuntime::new(
            TestBridge {
                surface: surface(valid_registration),
            },
            Vector2::new(160.0, 80.0),
        );
        assert_eq!(policy_calls.get(), 1);
        let crate::layout::LayoutNode::Container(root) = runtime.surface().layout_node() else {
            panic!("valid virtual shell should remain a layout container");
        };
        assert_eq!(root.children.len(), 2);

        runtime.bridge_mut().surface = surface(invalid_registration);
        runtime.refresh_with_scope(crate::runtime::RepaintScope::Projection);

        assert_eq!(policy_calls.get(), 2);
        assert_eq!(invalid_item_projections.get(), 1);
        assert!(runtime.virtual_layout.records[0].retired);
        assert!(runtime.virtual_layout.records[0].cached_subtree.is_none());
        assert!(
            runtime.virtual_layout.records[0]
                .materialization
                .active_slots()
                .is_empty()
        );
        let crate::layout::LayoutNode::Container(root) = runtime.surface().layout_node() else {
            panic!("retired virtual batch should retain only the shell");
        };
        assert_eq!(root.children.len(), 1);

        runtime.refresh_with_scope(crate::runtime::RepaintScope::Projection);

        assert_eq!(policy_calls.get(), 2);
        assert_eq!(invalid_item_projections.get(), 1);
        assert!(runtime.virtual_layout.records[0].retired);
        let crate::layout::LayoutNode::Container(root) = runtime.surface().layout_node() else {
            panic!("retired virtual batch should remain suppressed");
        };
        assert!(root.children.len() <= 1);
    }

    #[test]
    fn coordinator_begin_error_retires_and_suppresses_without_retrying() {
        let policy_calls = Rc::new(Cell::new(0));
        let policy: Rc<dyn VirtualLayoutPolicy> = Rc::new(ReadyPolicy {
            calls: Rc::clone(&policy_calls),
            key: 5,
        });
        let initial_registration = registration_with_parts(RegistrationParts {
            policy: Rc::clone(&policy),
            policy_identity: VirtualLayoutPolicyIdentity::new("regression-policy"),
            revisions: VirtualLayoutRegistrationRevisions {
                data: 2,
                ..Default::default()
            },
            shell: Rc::new(|| scroll(spacer::<()>())),
            item: Rc::new(|_| text::<()>("virtual item")),
            kind: Rc::new(|_| VirtualLayoutPolicyIdentity::new("item-kind")),
        });
        let regressed_registration = registration_with_parts(RegistrationParts {
            policy,
            policy_identity: VirtualLayoutPolicyIdentity::new("regression-policy"),
            revisions: VirtualLayoutRegistrationRevisions {
                data: 1,
                ..Default::default()
            },
            shell: Rc::new(|| scroll(spacer::<()>())),
            item: Rc::new(|_| text::<()>("virtual item")),
            kind: Rc::new(|_| VirtualLayoutPolicyIdentity::new("item-kind")),
        });
        let mut runtime = SurfaceRuntime::new(
            TestBridge {
                surface: surface(initial_registration),
            },
            Vector2::new(160.0, 80.0),
        );
        assert_eq!(policy_calls.get(), 1);

        runtime.bridge_mut().surface = surface(regressed_registration);
        runtime.refresh_with_scope(crate::runtime::RepaintScope::Projection);

        assert_eq!(policy_calls.get(), 1);
        assert!(runtime.virtual_layout.records[0].retired);
        assert!(runtime.virtual_layout.records[0].cached_subtree.is_none());
        assert!(
            runtime.virtual_layout.records[0]
                .materialization
                .active_slots()
                .is_empty()
        );
        let crate::layout::LayoutNode::Container(root) = runtime.surface().layout_node() else {
            panic!("coordinator failure should retain only the shell");
        };
        assert_eq!(root.children.len(), 1);

        runtime.refresh_with_scope(crate::runtime::RepaintScope::Projection);

        assert_eq!(policy_calls.get(), 1);
        assert!(runtime.virtual_layout.records[0].retired);
        let crate::layout::LayoutNode::Container(root) = runtime.surface().layout_node() else {
            panic!("coordinator failure should remain suppressed");
        };
        assert!(root.children.len() <= 1);
    }

    #[test]
    fn registry_preserves_equal_scope_and_retires_changed_policy_scope() {
        let mut state = RuntimeVirtualLayoutState::default();
        let first = registration(
            Rc::new(ReadyPolicy {
                calls: Rc::new(Cell::new(0)),
                key: 3,
            }),
            VirtualLayoutPolicyIdentity::new("policy"),
        );
        let mut first_surface = surface(first.clone());
        state.prepare_surface(&mut first_surface, &[first]);
        assert_eq!(state.records[0].mount_generation, 1);

        let equal = registration(
            Rc::new(ReadyPolicy {
                calls: Rc::new(Cell::new(0)),
                key: 3,
            }),
            VirtualLayoutPolicyIdentity::new("policy"),
        );
        let mut equal_surface = surface(equal.clone());
        state.prepare_surface(&mut equal_surface, &[equal]);
        assert_eq!(state.records[0].mount_generation, 1);

        let changed = registration(
            Rc::new(ReadyPolicy {
                calls: Rc::new(Cell::new(0)),
                key: 4,
            }),
            VirtualLayoutPolicyIdentity::new("new-policy"),
        );
        let mut changed_surface = surface(changed.clone());
        state.prepare_surface(&mut changed_surface, &[changed]);
        assert_eq!(state.records[0].mount_generation, 2);
        assert!(!state.records[0].retired);
    }
}

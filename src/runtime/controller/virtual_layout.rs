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
        VirtualLayoutProjectionEvidence, VirtualLayoutProjectionKind,
        VirtualLayoutWindowCoordinator,
    },
    gui::types::Rect,
    layout::VirtualLayoutQueryInputParts,
    runtime::{
        SurfaceNode, UiSurface,
        surface::{MAX_VIRTUAL_LAYOUT_REGISTRATIONS, VirtualLayoutRegistration},
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

struct RuntimeVirtualLayoutRecord<Message> {
    registration: VirtualLayoutRegistration<Message>,
    mount_generation: u64,
    coordinator: VirtualLayoutWindowCoordinator,
    materialization: RuntimeMaterialization<Message>,
    last_query: Option<VirtualLayoutQueryInputParts>,
    retired: bool,
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
            retired: false,
        }
    }

    fn update_registration(&mut self, registration: VirtualLayoutRegistration<Message>) {
        self.registration = registration;
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

    fn materialize(&mut self, viewport: Rect) -> Option<SurfaceNode<Message>> {
        if self.retired {
            return None;
        }
        let parts = self
            .registration
            .query_parts(viewport, self.mount_generation);
        if !self.needs_query(&parts) {
            return None;
        }
        let pending = match self.coordinator.begin_query(parts.clone()) {
            Ok(pending) => pending,
            Err(_) => return None,
        };
        let outcome = pending.execute(&*self.registration.policy);
        let completion = self.coordinator.complete(pending, outcome);
        let VirtualLayoutCompletion::Committed(commit) = completion else {
            self.last_query = Some(parts);
            return None;
        };

        let projector = self.registration.projector();
        match self.materialization.publish(&commit, &projector) {
            Ok(()) => {
                self.last_query = Some(parts);
                projector.take_shell()
            }
            Err(
                VirtualLayoutMaterializationError::Lifecycle(_)
                | VirtualLayoutMaterializationError::Reentrant
                | VirtualLayoutMaterializationError::LifecycleIndeterminate,
            ) => {
                self.retired = true;
                None
            }
            Err(_) => None,
        }
    }

    fn active_payloads(&self) -> Vec<SurfaceNode<Message>> {
        self.materialization
            .active_slots()
            .into_iter()
            .map(|slot| slot.payload().clone())
            .collect()
    }

    fn retire(&mut self) {
        if self.retired {
            return;
        }
        self.retired = true;
        let _ = self.materialization.unmount();
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
}

impl<Message> Default for RuntimeVirtualLayoutState<Message> {
    fn default() -> Self {
        Self {
            records: Vec::new(),
            next_mount_generation: 0,
        }
    }
}

impl<Message> RuntimeVirtualLayoutState<Message> {
    pub(super) fn prepare_surface(
        &mut self,
        surface: &mut UiSurface<Message>,
        registrations: &[VirtualLayoutRegistration<Message>],
    ) {
        if registrations.len() > MAX_VIRTUAL_LAYOUT_REGISTRATIONS {
            self.retire_all();
            return;
        }
        let mut accepted = Vec::with_capacity(registrations.len());
        for registration in registrations.iter().cloned() {
            if accepted
                .iter()
                .any(|current: &VirtualLayoutRegistration<Message>| {
                    current.container_id == registration.container_id
                })
            {
                continue;
            }
            accepted.push(registration);
        }

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

        for record in &self.records {
            if record.retired {
                continue;
            }
            let Some(shell) = record.registration.lowered_shell() else {
                continue;
            };
            let _ = surface.replace_virtual_layout_shell(
                record.registration.container_id,
                shell,
                record.registration.clone(),
            );
        }
    }

    pub(super) fn materialize_surface(
        &mut self,
        surface: &mut UiSurface<Message>,
        layout: &crate::layout::LayoutOutput,
    ) {
        for record in &mut self.records {
            if record.retired {
                continue;
            }
            let Some(viewport) = layout
                .viewport_bounds
                .get(&record.registration.container_id)
                .copied()
            else {
                let items = record.active_payloads();
                let _ =
                    surface.append_virtual_layout_items(record.registration.container_id, &items);
                continue;
            };
            if let Some(shell) = record.materialize(viewport) {
                let items = record.active_payloads();
                let _ = surface.install_virtual_layout_batch(
                    record.registration.container_id,
                    shell,
                    record.registration.clone(),
                    &items,
                );
            } else {
                let items = record.active_payloads();
                let _ =
                    surface.append_virtual_layout_items(record.registration.container_id, &items);
            }
        }
    }

    pub(super) fn retire_all(&mut self) {
        for record in &mut self.records {
            record.retire();
        }
        self.records.clear();
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
        let shell_projection = self.surface.runtime_projection();
        self.layout_root = shell_projection.layout_root;
        self.rebuild_virtual_layout_shell_layout();
        self.materialize_virtual_layout_surface();
        let final_projection = self.surface.runtime_projection();
        self.layout_root = final_projection.layout_root;
        self.relayout_with_traversal(final_projection.traversal);
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
        application::{empty, scroll, text},
        gui::types::{Rect, Vector2},
        layout::{
            ContainerKind, ContainerPolicy, OverflowPolicy, VirtualLayoutBoundsConfidence,
            VirtualLayoutBudget, VirtualLayoutCoordinateSpace, VirtualLayoutExtentCandidate,
            VirtualLayoutItemCandidate, VirtualLayoutItemKey, VirtualLayoutOverscan,
            VirtualLayoutPolicy, VirtualLayoutPolicyDecision, VirtualLayoutPolicyIdentity,
            VirtualLayoutQueryInput, VirtualLayoutQuerySink, VirtualLayoutVisibility,
        },
        runtime::{RuntimeBridge, SurfaceChild, SurfaceNode, UiSurface},
    };
    use std::{cell::Cell, rc::Rc, sync::Arc};

    const CONTAINER_ID: u64 = 710;

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

    fn registration(
        policy: Rc<dyn VirtualLayoutPolicy>,
        policy_identity: VirtualLayoutPolicyIdentity,
    ) -> VirtualLayoutRegistration<()> {
        VirtualLayoutRegistration::new(
            CONTAINER_ID,
            policy_identity,
            policy,
            VirtualLayoutCoordinateSpace::logical(),
            VirtualLayoutOverscan::new(0.0, 0.0).expect("finite overscan"),
            VirtualLayoutBudget::new(4),
            Default::default(),
            Rc::new(|| scroll(empty::<()>())),
            Rc::new(|_| text::<()>("virtual item")),
            Rc::new(|_| VirtualLayoutPolicyIdentity::new("item-kind")),
        )
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
    }

    #[test]
    fn unchanged_projection_reuses_the_active_window_without_requerying() {
        let calls = Rc::new(Cell::new(0));
        let policy = Rc::new(ReadyPolicy {
            calls: Rc::clone(&calls),
            key: 2,
        });
        let mut runtime = SurfaceRuntime::new(
            TestBridge {
                surface: surface(registration(
                    policy,
                    VirtualLayoutPolicyIdentity::new("policy"),
                )),
            },
            Vector2::new(160.0, 80.0),
        );

        runtime.refresh_with_scope(crate::runtime::RepaintScope::Projection);

        assert_eq!(calls.get(), 1);
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

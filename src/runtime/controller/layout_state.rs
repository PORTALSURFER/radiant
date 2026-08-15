//! Runtime-owned bounded state for state-aware layout interactions.

use crate::{
    gui::layout_core::{MountedContainerStateId, MountedContainerStateRead},
    layout::{ContainerStateDeclaration, ContainerStateId, LayoutContainerStateContext, NodeId},
};
use std::any::Any;
use std::num::NonZeroU64;

use super::SurfaceRuntime;
use crate::runtime::RuntimeBridge;

/// Maximum number of mounted layout-interaction state slots in one window.
pub(super) const MAX_LAYOUT_CONTAINER_STATE_SLOTS: usize = 64;

/// Maximum number of type/schema replacement records retained for one refresh.
pub(crate) const MAX_LAYOUT_STATE_REPLACEMENTS_PER_REFRESH: usize = 4;

/// One bounded diagnostic for a changed layout-interaction state identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SurfaceLayoutStateReplacement {
    /// Container whose state slot was replaced.
    pub container_id: NodeId,
    /// Previous concrete type/schema identity.
    pub previous: ContainerStateId,
    /// Current concrete type/schema identity.
    pub current: ContainerStateId,
}

/// Bounded diagnostics for one runtime-owned layout-state reconciliation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SurfaceLayoutStateDiagnostics {
    /// First replacements in deterministic traversal order.
    pub replacements:
        [Option<SurfaceLayoutStateReplacement>; MAX_LAYOUT_STATE_REPLACEMENTS_PER_REFRESH],
    /// Number of changed type/schema identities, including omitted records.
    pub replacement_count: u32,
    /// Number of mounted slots dropped because their container was unmounted.
    pub dropped_count: u32,
    /// Number of slots initialized during this reconciliation.
    pub initialized_count: u32,
    /// Number of declarations that could not receive a slot at the bound.
    pub capacity_exceeded_count: u32,
    /// Number of v4 declarations rejected because they named another container.
    pub foreign_declaration_count: u32,
    /// Number of declarations that could not receive a mount generation.
    pub generation_exhaustion_count: u32,
}

impl SurfaceLayoutStateDiagnostics {
    pub(crate) const fn startup() -> Self {
        Self {
            replacements: [None; MAX_LAYOUT_STATE_REPLACEMENTS_PER_REFRESH],
            replacement_count: 0,
            dropped_count: 0,
            initialized_count: 0,
            capacity_exceeded_count: 0,
            foreign_declaration_count: 0,
            generation_exhaustion_count: 0,
        }
    }

    fn push_replacement(&mut self, replacement: SurfaceLayoutStateReplacement) {
        let index = self.replacement_count as usize;
        if index < self.replacements.len() {
            self.replacements[index] = Some(replacement);
        }
        self.replacement_count = self.replacement_count.saturating_add(1);
    }

    pub(crate) fn merge(&mut self, other: Self) {
        let base = self.replacement_count as usize;
        for (offset, replacement) in other.replacements.into_iter().enumerate() {
            let Some(replacement) = replacement else {
                continue;
            };
            let index = base.saturating_add(offset);
            if index < self.replacements.len() {
                self.replacements[index] = Some(replacement);
            }
        }
        self.replacement_count = self
            .replacement_count
            .saturating_add(other.replacement_count);
        self.dropped_count = self.dropped_count.saturating_add(other.dropped_count);
        self.initialized_count = self
            .initialized_count
            .saturating_add(other.initialized_count);
        self.capacity_exceeded_count = self
            .capacity_exceeded_count
            .saturating_add(other.capacity_exceeded_count);
        self.foreign_declaration_count = self
            .foreign_declaration_count
            .saturating_add(other.foreign_declaration_count);
        self.generation_exhaustion_count = self
            .generation_exhaustion_count
            .saturating_add(other.generation_exhaustion_count);
    }

    pub(crate) fn record_foreign_declaration(&mut self) {
        self.foreign_declaration_count = self.foreign_declaration_count.saturating_add(1);
    }
}

struct RuntimeLayoutContainerStateSlot {
    id: ContainerStateId,
    mounted_id: MountedContainerStateId,
    value: Box<dyn Any>,
}

pub(super) struct RuntimeLayoutContainerStateStore {
    slots: Vec<RuntimeLayoutContainerStateSlot>,
    next_mount_generation: u64,
}

impl Default for RuntimeLayoutContainerStateStore {
    fn default() -> Self {
        Self {
            slots: Vec::new(),
            next_mount_generation: 1,
        }
    }
}

impl RuntimeLayoutContainerStateStore {
    fn allocate_mounted_state_id(
        &mut self,
        state_id: ContainerStateId,
    ) -> Option<MountedContainerStateId> {
        let generation = NonZeroU64::new(self.next_mount_generation)?;
        let next_mount_generation = self.next_mount_generation.checked_add(1)?;
        self.next_mount_generation = next_mount_generation;
        Some(MountedContainerStateId::new(state_id, generation))
    }

    fn current_mounted_state_id(
        &self,
        state_id: ContainerStateId,
    ) -> Option<MountedContainerStateId> {
        self.slots
            .iter()
            .find(|slot| slot.id == state_id)
            .map(|slot| slot.mounted_id)
    }

    fn lookup_current_state(
        &mut self,
        mounted_id: MountedContainerStateId,
    ) -> Option<&mut dyn Any> {
        self.slots
            .iter_mut()
            .find(|slot| {
                slot.mounted_id == mounted_id
                    && slot.mounted_id.generation() == mounted_id.generation()
            })
            .map(|slot| slot.value.as_mut())
    }

    #[cfg_attr(not(test), expect(dead_code))]
    pub(super) fn lookup_current_state_view(
        &self,
        mounted_id: MountedContainerStateId,
    ) -> Option<MountedContainerStateRead<'_>> {
        self.slots
            .iter()
            .find(|slot| {
                slot.mounted_id == mounted_id
                    && slot.mounted_id.generation() == mounted_id.generation()
            })
            .map(|slot| MountedContainerStateRead::new(slot.mounted_id, slot.value.as_ref()))
    }

    pub(super) fn reconcile(
        &mut self,
        declarations: &[ContainerStateDeclaration],
    ) -> SurfaceLayoutStateDiagnostics {
        let mut diagnostics = SurfaceLayoutStateDiagnostics::default();
        let mut mounted = Vec::with_capacity(declarations.len());

        let mut index = 0;
        while index < self.slots.len() {
            if declarations.iter().any(|declaration| {
                declaration.container_id() == self.slots[index].id.container_id()
            }) {
                index += 1;
                continue;
            }
            let removed = self.slots.remove(index);
            drop(removed);
            diagnostics.dropped_count = diagnostics.dropped_count.saturating_add(1);
        }

        for declaration in declarations {
            let id = declaration.id();
            if mounted.contains(&id) {
                continue;
            }
            mounted.push(id);

            if self.slots.iter().any(|slot| slot.id == id) {
                continue;
            }

            if let Some(index) = self
                .slots
                .iter()
                .position(|slot| slot.id.same_container(id))
            {
                let previous = self.slots.remove(index);
                let previous_id = previous.id;
                drop(previous);
                diagnostics.push_replacement(SurfaceLayoutStateReplacement {
                    container_id: id.container_id(),
                    previous: previous_id,
                    current: id,
                });
            } else if self.slots.len() >= MAX_LAYOUT_CONTAINER_STATE_SLOTS {
                diagnostics.capacity_exceeded_count =
                    diagnostics.capacity_exceeded_count.saturating_add(1);
                continue;
            }

            let Some(mounted_id) = self.allocate_mounted_state_id(id) else {
                diagnostics.generation_exhaustion_count =
                    diagnostics.generation_exhaustion_count.saturating_add(1);
                continue;
            };

            self.slots.push(RuntimeLayoutContainerStateSlot {
                id,
                mounted_id,
                value: declaration.initialize(),
            });
            diagnostics.initialized_count = diagnostics.initialized_count.saturating_add(1);
        }

        diagnostics
    }

    pub(super) fn context<'a>(
        &'a mut self,
        container_id: NodeId,
        state_id: Option<ContainerStateId>,
    ) -> LayoutContainerStateContext<'a> {
        let state = state_id
            .and_then(|id| self.current_mounted_state_id(id))
            .and_then(|mounted_id| self.lookup_current_state(mounted_id));
        LayoutContainerStateContext::from_runtime(container_id, state_id, state)
    }

    #[cfg(test)]
    pub(super) fn slot_count(&self) -> usize {
        self.slots.len()
    }

    #[cfg(test)]
    pub(super) fn set_next_mount_generation_for_test(&mut self, generation: u64) {
        self.next_mount_generation = generation;
    }

    #[cfg(test)]
    pub(super) fn next_mount_generation_for_test(&self) -> u64 {
        self.next_mount_generation
    }
}

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(super) fn reconcile_layout_container_state(&mut self) {
        let declarations = self
            .traversal
            .containers
            .layout_interactions
            .iter()
            .filter_map(|interaction| interaction.state.clone())
            .collect::<Vec<_>>();
        let mut diagnostics = self.interaction.layout_state.reconcile(&declarations);
        for interaction in &self.traversal.containers.layout_interactions {
            if interaction.foreign_state_declaration {
                diagnostics.record_foreign_declaration();
            }
        }
        self.last_layout_state_diagnostics = diagnostics;
    }

    pub(super) fn layout_container_state_context<'a>(
        &'a mut self,
        container_id: NodeId,
        state_id: Option<ContainerStateId>,
    ) -> LayoutContainerStateContext<'a> {
        self.interaction
            .layout_state
            .context(container_id, state_id)
    }

    #[cfg(test)]
    pub(super) fn layout_container_state_slot_count(&self) -> usize {
        self.interaction.layout_state.slot_count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::rc::Rc;

    #[test]
    fn matching_identity_reuses_a_non_send_slot() {
        let initialized = Rc::new(Cell::new(0));
        let declaration = ContainerStateDeclaration::new::<Rc<Cell<u32>>, _>(7, 1, {
            let initialized = Rc::clone(&initialized);
            move || {
                initialized.set(initialized.get() + 1);
                Rc::new(Cell::new(3))
            }
        });
        let mut store = RuntimeLayoutContainerStateStore::default();

        let first = store.reconcile(std::slice::from_ref(&declaration));
        let first_token = store
            .current_mounted_state_id(declaration.id())
            .expect("first mount token");
        let second = store.reconcile(std::slice::from_ref(&declaration));
        let second_token = store
            .current_mounted_state_id(declaration.id())
            .expect("matching mount token");
        assert_eq!(initialized.get(), 1);
        assert_eq!(first.initialized_count, 1);
        assert_eq!(second.initialized_count, 0);
        assert_eq!(first_token, second_token);
        assert_eq!(first_token.generation().get(), 1);

        let mut context = store.context(7, Some(declaration.id()));
        context
            .state_mut::<Rc<Cell<u32>>>()
            .expect("typed state")
            .set(9);
        assert_eq!(
            context
                .state_mut::<Rc<Cell<u32>>>()
                .expect("typed state")
                .get(),
            9
        );

        let view = store
            .lookup_current_state_view(first_token)
            .expect("matching immutable mount view");
        assert_eq!(view.mounted_id(), first_token);
        assert_eq!(
            view.downcast_ref::<Rc<Cell<u32>>>()
                .map(|value| value.get()),
            Some(9)
        );
        assert_eq!(
            view.get::<Rc<Cell<u32>>>().map(|value| value.get()),
            Some(9)
        );
        assert!(view.get::<u32>().is_none());
    }

    #[test]
    fn changed_schema_replaces_and_unmounted_state_drops() {
        let drops = Rc::new(Cell::new(0));
        struct Droppable(Rc<Cell<u8>>);
        impl Drop for Droppable {
            fn drop(&mut self) {
                self.0.set(self.0.get() + 1);
            }
        }

        let old = ContainerStateDeclaration::new::<Droppable, _>(9, 1, {
            let drops = Rc::clone(&drops);
            move || Droppable(Rc::clone(&drops))
        });
        let new = ContainerStateDeclaration::new::<Droppable, _>(9, 2, {
            let drops = Rc::clone(&drops);
            move || Droppable(Rc::clone(&drops))
        });
        let mut store = RuntimeLayoutContainerStateStore::default();
        let _ = store.reconcile(std::slice::from_ref(&old));
        let old_token = store
            .current_mounted_state_id(old.id())
            .expect("old mount token");
        let replacement = store.reconcile(std::slice::from_ref(&new));
        let new_token = store
            .current_mounted_state_id(new.id())
            .expect("new mount token");
        assert_eq!(replacement.replacement_count, 1);
        assert_eq!(replacement.initialized_count, 1);
        assert_eq!(drops.get(), 1);
        assert_ne!(old_token, new_token);
        assert!(store.lookup_current_state(old_token).is_none());
        assert!(store.lookup_current_state(new_token).is_some());
        assert!(store.lookup_current_state_view(old_token).is_none());
        assert_eq!(
            store
                .lookup_current_state_view(new_token)
                .expect("replacement immutable mount view")
                .mounted_id(),
            new_token
        );

        let dropped = store.reconcile(&[]);
        assert_eq!(dropped.dropped_count, 1);
        assert_eq!(drops.get(), 2);
        assert_eq!(store.slot_count(), 0);
        assert!(store.lookup_current_state_view(new_token).is_none());
    }

    #[test]
    fn disjoint_replacement_refills_all_slots_and_bounds_a_true_extra_declaration() {
        let initialized = Rc::new(Cell::new(0));
        let old_declarations = (0..MAX_LAYOUT_CONTAINER_STATE_SLOTS as u64)
            .map(|container_id| {
                let initialized = Rc::clone(&initialized);
                ContainerStateDeclaration::new::<u32, _>(container_id, 1, move || {
                    initialized.set(initialized.get() + 1);
                    0
                })
            })
            .collect::<Vec<_>>();
        let new_declarations = (MAX_LAYOUT_CONTAINER_STATE_SLOTS as u64
            ..(MAX_LAYOUT_CONTAINER_STATE_SLOTS * 2) as u64)
            .map(|container_id| {
                let initialized = Rc::clone(&initialized);
                ContainerStateDeclaration::new::<u32, _>(container_id, 1, move || {
                    initialized.set(initialized.get() + 1);
                    0
                })
            })
            .collect::<Vec<_>>();
        let mut store = RuntimeLayoutContainerStateStore::default();

        let first = store.reconcile(&old_declarations);
        let replacement = store.reconcile(&new_declarations);

        assert_eq!(
            first.initialized_count,
            MAX_LAYOUT_CONTAINER_STATE_SLOTS as u32
        );
        assert_eq!(
            replacement.dropped_count,
            MAX_LAYOUT_CONTAINER_STATE_SLOTS as u32
        );
        assert_eq!(
            replacement.initialized_count,
            MAX_LAYOUT_CONTAINER_STATE_SLOTS as u32
        );
        assert_eq!(replacement.capacity_exceeded_count, 0);
        assert_eq!(store.slot_count(), MAX_LAYOUT_CONTAINER_STATE_SLOTS);
        assert_eq!(
            initialized.get(),
            (MAX_LAYOUT_CONTAINER_STATE_SLOTS * 2) as u32
        );

        let extra_declarations = (0..(MAX_LAYOUT_CONTAINER_STATE_SLOTS + 1) as u64)
            .map(|container_id| {
                let initialized = Rc::clone(&initialized);
                ContainerStateDeclaration::new::<u32, _>(
                    container_id + (MAX_LAYOUT_CONTAINER_STATE_SLOTS * 2) as u64,
                    1,
                    move || {
                        initialized.set(initialized.get() + 1);
                        0
                    },
                )
            })
            .collect::<Vec<_>>();
        let bounded = store.reconcile(&extra_declarations);

        assert_eq!(
            bounded.initialized_count,
            MAX_LAYOUT_CONTAINER_STATE_SLOTS as u32
        );
        assert_eq!(bounded.capacity_exceeded_count, 1);
        assert_eq!(store.slot_count(), MAX_LAYOUT_CONTAINER_STATE_SLOTS);
        assert_eq!(
            initialized.get(),
            (MAX_LAYOUT_CONTAINER_STATE_SLOTS * 3) as u32
        );
    }

    #[test]
    fn unmount_and_reinsert_gets_a_fresh_mount_token() {
        let declaration = ContainerStateDeclaration::new::<u32, _>(11, 1, || 0);
        let mut store = RuntimeLayoutContainerStateStore::default();

        store.reconcile(std::slice::from_ref(&declaration));
        let first_token = store
            .current_mounted_state_id(declaration.id())
            .expect("first mount token");
        store.reconcile(&[]);
        assert!(store.lookup_current_state(first_token).is_none());
        assert!(store.lookup_current_state_view(first_token).is_none());

        store.reconcile(std::slice::from_ref(&declaration));
        let second_token = store
            .current_mounted_state_id(declaration.id())
            .expect("reinserted mount token");
        assert_ne!(first_token, second_token);
        assert_eq!(second_token.generation().get(), 2);
        assert_eq!(
            store
                .lookup_current_state_view(second_token)
                .expect("reinserted immutable mount view")
                .mounted_id(),
            second_token
        );
    }

    #[test]
    fn stale_mount_token_cannot_resolve_after_unmount() {
        let declaration = ContainerStateDeclaration::new::<u32, _>(12, 1, || 0);
        let mut store = RuntimeLayoutContainerStateStore::default();

        store.reconcile(std::slice::from_ref(&declaration));
        let stale_token = store
            .current_mounted_state_id(declaration.id())
            .expect("mount token");
        store.reconcile(&[]);

        assert!(store.lookup_current_state(stale_token).is_none());
        assert!(store.lookup_current_state_view(stale_token).is_none());
        assert_eq!(store.slot_count(), 0);
    }

    #[test]
    fn foreign_mount_token_cannot_resolve_in_the_current_store() {
        let declaration = ContainerStateDeclaration::new::<u32, _>(15, 1, || 3);
        let mut store = RuntimeLayoutContainerStateStore::default();
        store.reconcile(std::slice::from_ref(&declaration));
        let current_token = store
            .current_mounted_state_id(declaration.id())
            .expect("mount token");
        let foreign_id = ContainerStateId::new::<u32>(16, declaration.schema_version());
        let foreign_token = MountedContainerStateId::new(foreign_id, current_token.generation());

        assert!(store.lookup_current_state_view(foreign_token).is_none());
    }

    #[test]
    fn duplicate_declaration_creates_one_mount_token() {
        let initialized = Rc::new(Cell::new(0));
        let declaration = ContainerStateDeclaration::new::<u32, _>(13, 1, {
            let initialized = Rc::clone(&initialized);
            move || {
                initialized.set(initialized.get() + 1);
                0
            }
        });
        let mut store = RuntimeLayoutContainerStateStore::default();

        let diagnostics = store.reconcile(&[declaration.clone(), declaration.clone()]);

        assert_eq!(diagnostics.initialized_count, 1);
        assert_eq!(initialized.get(), 1);
        assert_eq!(store.slot_count(), 1);
        assert_eq!(store.next_mount_generation_for_test(), 2);
    }

    #[test]
    fn capacity_denial_does_not_allocate_a_mount_token_or_initialize() {
        let initialized = Rc::new(Cell::new(0));
        let declarations = (0..MAX_LAYOUT_CONTAINER_STATE_SLOTS as u64)
            .map(|container_id| {
                let initialized = Rc::clone(&initialized);
                ContainerStateDeclaration::new::<u32, _>(container_id, 1, move || {
                    initialized.set(initialized.get() + 1);
                    0
                })
            })
            .collect::<Vec<_>>();
        let denied =
            ContainerStateDeclaration::new::<u32, _>(MAX_LAYOUT_CONTAINER_STATE_SLOTS as u64, 1, {
                let initialized = Rc::clone(&initialized);
                move || {
                    initialized.set(initialized.get() + 1);
                    0
                }
            });
        let mut store = RuntimeLayoutContainerStateStore::default();
        store.reconcile(&declarations);
        let next_generation = store.next_mount_generation_for_test();

        let mut combined = declarations.clone();
        combined.push(denied.clone());
        let diagnostics = store.reconcile(&combined);

        assert_eq!(diagnostics.capacity_exceeded_count, 1);
        assert_eq!(diagnostics.generation_exhaustion_count, 0);
        assert_eq!(diagnostics.initialized_count, 0);
        assert_eq!(initialized.get(), MAX_LAYOUT_CONTAINER_STATE_SLOTS as u32);
        assert_eq!(store.next_mount_generation_for_test(), next_generation);
        assert!(store.current_mounted_state_id(denied.id()).is_none());
        let denied_token = MountedContainerStateId::new(
            denied.id(),
            NonZeroU64::new(next_generation).expect("capacity denial generation"),
        );
        assert!(store.lookup_current_state_view(denied_token).is_none());
        assert_eq!(store.slot_count(), MAX_LAYOUT_CONTAINER_STATE_SLOTS);
    }

    #[test]
    fn generation_exhaustion_is_bounded_without_token_or_initializer() {
        let initialized = Rc::new(Cell::new(0));
        let declaration = ContainerStateDeclaration::new::<u32, _>(14, 1, {
            let initialized = Rc::clone(&initialized);
            move || {
                initialized.set(initialized.get() + 1);
                0
            }
        });
        let mut store = RuntimeLayoutContainerStateStore::default();
        store.set_next_mount_generation_for_test(u64::MAX);

        let first = store.reconcile(std::slice::from_ref(&declaration));
        let second = store.reconcile(std::slice::from_ref(&declaration));

        assert_eq!(first.generation_exhaustion_count, 1);
        assert_eq!(second.generation_exhaustion_count, 1);
        assert_eq!(initialized.get(), 0);
        assert_eq!(store.slot_count(), 0);
        assert_eq!(store.next_mount_generation_for_test(), u64::MAX);
        assert!(store.current_mounted_state_id(declaration.id()).is_none());
        let exhausted_token = MountedContainerStateId::new(
            declaration.id(),
            NonZeroU64::new(u64::MAX).expect("exhausted generation"),
        );
        assert!(store.lookup_current_state_view(exhausted_token).is_none());
    }
}

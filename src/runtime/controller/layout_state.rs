//! Runtime-owned bounded state for state-aware layout interactions.

use crate::layout::{
    ContainerStateDeclaration, ContainerStateId, LayoutContainerStateContext, NodeId,
};
use std::any::Any;

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
    }

    pub(crate) fn record_foreign_declaration(&mut self) {
        self.foreign_declaration_count = self.foreign_declaration_count.saturating_add(1);
    }
}

struct RuntimeLayoutContainerStateSlot {
    id: ContainerStateId,
    value: Box<dyn Any>,
}

#[derive(Default)]
pub(super) struct RuntimeLayoutContainerStateStore {
    slots: Vec<RuntimeLayoutContainerStateSlot>,
}

impl RuntimeLayoutContainerStateStore {
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

            self.slots.push(RuntimeLayoutContainerStateSlot {
                id,
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
        let state = state_id.and_then(|id| {
            self.slots
                .iter_mut()
                .find(|slot| slot.id == id)
                .map(|slot| slot.value.as_mut())
        });
        LayoutContainerStateContext::from_runtime(container_id, state_id, state)
    }

    #[cfg(test)]
    pub(super) fn slot_count(&self) -> usize {
        self.slots.len()
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
        let second = store.reconcile(std::slice::from_ref(&declaration));
        assert_eq!(initialized.get(), 1);
        assert_eq!(first.initialized_count, 1);
        assert_eq!(second.initialized_count, 0);

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
        let replacement = store.reconcile(std::slice::from_ref(&new));
        assert_eq!(replacement.replacement_count, 1);
        assert_eq!(replacement.initialized_count, 1);
        assert_eq!(drops.get(), 1);

        let dropped = store.reconcile(&[]);
        assert_eq!(dropped.dropped_count, 1);
        assert_eq!(drops.get(), 2);
        assert_eq!(store.slot_count(), 0);
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
}

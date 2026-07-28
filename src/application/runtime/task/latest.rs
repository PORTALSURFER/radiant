use super::{TaskCompletion, TaskTicket};
use std::{
    cell::Cell,
    collections::{HashMap, HashSet},
    sync::atomic::{AtomicU64, Ordering},
    sync::{Arc, Mutex, Weak},
};

static NEXT_EFFECT_SLOT_ID: AtomicU64 = AtomicU64::new(1);

#[cfg(test)]
#[path = "latest/tests.rs"]
mod tests;

#[derive(Debug)]
struct LatestState {
    next_id: u64,
    active: Option<TaskTicket>,
    predecessors: HashMap<TaskTicket, Option<TaskTicket>>,
    rejected: HashSet<TaskTicket>,
}

#[derive(Debug)]
enum LatestStorage {
    Inline {
        next_id: u64,
        active: Option<TaskTicket>,
    },
    Shared {
        slot: u64,
        state: Arc<Mutex<LatestState>>,
    },
}

/// Tracks the latest in-flight task for one host-owned resource.
///
/// The application owns one tracker per logical resource and keeps it with the
/// UI state that starts and accepts the work. Starting another request advances
/// the ticket; a completion is current only when [`Self::is_active`] accepts
/// its ticket, and the reducer should call [`Self::finish`] before applying the
/// output. This is the normal app-facing stale-result contract used by
/// `context.business().latest(...)` and
/// [`crate::application::UiUpdateContext::after_latest`].
///
/// Custom hosts may use the same tracker with an explicit
/// [`crate::runtime::RuntimeBridge`] or [`crate::runtime::RuntimeTaskHost`]
/// integration, but they must preserve ticket ownership and perform the
/// active-ticket check on the UI owner. A timer or worker thread must not invoke
/// the mapper or mutate this tracker.
#[derive(Debug)]
pub struct LatestTask {
    storage: LatestStorage,
    effect_id: u64,
}

impl Clone for LatestTask {
    fn clone(&self) -> Self {
        let (next_id, active) = self.snapshot();
        Self {
            storage: LatestStorage::Inline { next_id, active },
            effect_id: 0,
        }
    }
}

impl PartialEq for LatestTask {
    fn eq(&self, other: &Self) -> bool {
        self.snapshot() == other.snapshot()
    }
}

impl Eq for LatestTask {}

impl Default for LatestTask {
    fn default() -> Self {
        Self::new()
    }
}

impl LatestTask {
    /// Build an idle task tracker.
    pub const fn new() -> Self {
        Self {
            storage: LatestStorage::Inline {
                next_id: 1,
                active: None,
            },
            effect_id: 0,
        }
    }

    /// Start a new latest task and return its ticket.
    pub fn begin(&mut self) -> TaskTicket {
        match &mut self.storage {
            LatestStorage::Inline { next_id, active } => {
                let ticket = TaskTicket::new(*next_id);
                *next_id = next_id.saturating_add(1);
                *active = Some(ticket);
                ticket
            }
            LatestStorage::Shared { state, .. } => {
                let mut state = lock_state(state);
                let ticket = TaskTicket::new(state.next_id);
                state.next_id = state.next_id.saturating_add(1);
                state.active = Some(ticket);
                state.predecessors.clear();
                state.rejected.clear();
                ticket
            }
        }
    }

    /// Reserve a replacement and publish its ticket transactionally.
    pub(crate) fn begin_replacement(&mut self) -> LatestTaskTransaction {
        let (slot, state) = match &mut self.storage {
            LatestStorage::Inline { next_id, active } => {
                let shared = Arc::new(Mutex::new(LatestState {
                    next_id: *next_id,
                    active: *active,
                    predecessors: HashMap::new(),
                    rejected: HashSet::new(),
                }));
                let slot = NEXT_EFFECT_SLOT_ID.fetch_add(1, Ordering::Relaxed);
                self.storage = LatestStorage::Shared {
                    slot,
                    state: Arc::clone(&shared),
                };
                (slot, shared)
            }
            LatestStorage::Shared { slot, state } => (*slot, Arc::clone(state)),
        };
        let mut state_guard = lock_state(&state);
        let previous = state_guard.active;
        let replacement = TaskTicket::new(state_guard.next_id);
        state_guard.next_id = state_guard.next_id.saturating_add(1);
        state_guard.active = Some(replacement);
        state_guard.predecessors.insert(replacement, previous);
        state_guard.rejected.remove(&replacement);
        LatestTaskTransaction {
            slot,
            replacement,
            previous,
            state: Arc::downgrade(&state),
            committed: Cell::new(false),
            rejection_hook: None,
        }
    }

    /// Reserve a timer replacement and publish its ticket transactionally.
    pub(crate) fn begin_timer_replacement(&mut self) -> LatestTaskTransaction {
        self.begin_replacement()
    }

    /// Return the currently active latest task, if any.
    pub fn active(&self) -> Option<TaskTicket> {
        self.snapshot().1
    }

    pub(crate) fn effect_id(&mut self) -> u64 {
        if self.effect_id == 0 {
            self.effect_id = NEXT_EFFECT_SLOT_ID.fetch_add(1, Ordering::Relaxed);
        }
        self.effect_id
    }

    /// Return whether this ticket is still the active latest task.
    pub fn is_active(&self, ticket: TaskTicket) -> bool {
        self.active() == Some(ticket)
    }

    /// Return whether this completion belongs to the active latest task.
    pub fn is_active_completion<Output>(&self, completion: &TaskCompletion<Output>) -> bool {
        self.is_active(completion.ticket)
    }

    /// Clear this task if `ticket` is still active.
    pub fn finish(&mut self, ticket: TaskTicket) -> bool {
        if !self.is_active(ticket) {
            return false;
        }
        self.set_active(None);
        self.invalidate_transactions();
        true
    }

    /// Clear this task for a current completion and return its output.
    pub fn finish_completion<Output>(
        &mut self,
        completion: TaskCompletion<Output>,
    ) -> Option<Output> {
        self.finish(completion.ticket).then_some(completion.output)
    }

    /// Clear any active task.
    pub fn cancel(&mut self) {
        self.set_active(None);
        self.invalidate_transactions();
    }

    fn snapshot(&self) -> (u64, Option<TaskTicket>) {
        match &self.storage {
            LatestStorage::Inline { next_id, active } => (*next_id, *active),
            LatestStorage::Shared { state, .. } => {
                let state = lock_state(state);
                (state.next_id, resolve_active(&state))
            }
        }
    }

    fn set_active(&mut self, active: Option<TaskTicket>) {
        match &mut self.storage {
            LatestStorage::Inline {
                active: current, ..
            } => *current = active,
            LatestStorage::Shared { state, .. } => lock_state(state).active = active,
        }
    }

    fn invalidate_transactions(&mut self) {
        if let LatestStorage::Shared { state, .. } = &mut self.storage {
            let mut state = lock_state(state);
            state.predecessors.clear();
            state.rejected.clear();
        }
    }
}

impl Drop for LatestTask {
    fn drop(&mut self) {
        self.invalidate_transactions();
    }
}

/// A latest-task replacement that can be committed or rolled back by the controller.
pub(crate) struct LatestTaskTransaction {
    slot: u64,
    replacement: TaskTicket,
    previous: Option<TaskTicket>,
    state: Weak<Mutex<LatestState>>,
    committed: Cell<bool>,
    rejection_hook: Option<Arc<dyn Fn() + Send + Sync + 'static>>,
}

impl LatestTaskTransaction {
    pub(crate) fn cancellation_probe(
        &self,
    ) -> std::sync::Arc<dyn Fn() -> bool + Send + Sync + 'static> {
        let state = self.state.clone();
        let replacement = self.replacement;
        std::sync::Arc::new(move || {
            state.upgrade().is_none_or(|state| {
                let state = lock_state(&state);
                resolve_active(&state) != Some(replacement)
            })
        })
    }

    pub(crate) fn with_rejection_hook(
        mut self,
        hook: Arc<dyn Fn() + Send + Sync + 'static>,
    ) -> Self {
        self.rejection_hook = Some(hook);
        self
    }

    pub(crate) fn replacement(&self) -> TaskTicket {
        self.replacement
    }

    pub(crate) fn slot(&self) -> u64 {
        self.slot
    }

    pub(crate) fn generation(&self) -> u64 {
        self.replacement.id()
    }

    pub(crate) fn is_active(&self) -> bool {
        self.state.upgrade().is_some_and(|state| {
            let state = lock_state(&state);
            resolve_active(&state) == Some(self.replacement)
        })
    }

    /// Commit the replacement after the host accepts its timer registration or worker admission.
    /// Publication already happened in [`LatestTask::begin_replacement`];
    /// once committed, its predecessor link is no longer needed for rollback.
    pub(crate) fn accept(&self) {
        if self.committed.replace(true) {
            return;
        }
        let Some(state) = self.state.upgrade() else {
            return;
        };
        let mut state = lock_state(&state);
        state.predecessors.remove(&self.replacement);
        state.rejected.remove(&self.replacement);
    }

    pub(crate) fn reject(&self) {
        if self.committed.replace(true) {
            return;
        }
        self.release_rejection_hook();
        let Some(state) = self.state.upgrade() else {
            return;
        };
        let mut state = lock_state(&state);
        state.rejected.insert(self.replacement);
        if state.active == Some(self.replacement) {
            state.active = resolve_ticket(&state, self.previous);
        }
    }

    fn release_rejection_hook(&self) {
        if let Some(hook) = &self.rejection_hook {
            hook();
        }
    }
}

impl Drop for LatestTaskTransaction {
    fn drop(&mut self) {
        if self.committed.replace(true) {
            return;
        }
        self.release_rejection_hook();
        let Some(state) = self.state.upgrade() else {
            return;
        };
        let mut state = lock_state(&state);
        state.rejected.insert(self.replacement);
        if state.active == Some(self.replacement) {
            state.active = resolve_ticket(&state, self.previous);
        }
    }
}

pub(crate) type LatestTimerTransaction = LatestTaskTransaction;

fn resolve_active(state: &LatestState) -> Option<TaskTicket> {
    resolve_ticket(state, state.active)
}

fn resolve_ticket(state: &LatestState, mut ticket: Option<TaskTicket>) -> Option<TaskTicket> {
    while let Some(current) = ticket {
        if !state.rejected.contains(&current) {
            return Some(current);
        }
        ticket = state.predecessors.get(&current).copied().flatten();
    }
    None
}

fn lock_state(state: &Mutex<LatestState>) -> std::sync::MutexGuard<'_, LatestState> {
    state
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

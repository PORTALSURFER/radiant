use super::{TaskCompletion, TaskTicket};
use std::{
    collections::HashMap,
    sync::atomic::{AtomicU64, Ordering},
    sync::{Mutex, OnceLock},
};

static NEXT_LATEST_SLOT_ID: AtomicU64 = AtomicU64::new(1);
static ACTIVE_LATEST: OnceLock<Mutex<HashMap<u64, u64>>> = OnceLock::new();

#[cfg(test)]
#[path = "latest/tests.rs"]
mod tests;

/// Tracks the latest in-flight task for one host-owned resource.
#[derive(Debug)]
pub struct LatestTask {
    effect_id: u64,
    next_id: u64,
    active: Option<TaskTicket>,
}

impl Clone for LatestTask {
    fn clone(&self) -> Self {
        Self {
            effect_id: 0,
            next_id: self.next_id,
            active: self.active,
        }
    }
}

impl PartialEq for LatestTask {
    fn eq(&self, other: &Self) -> bool {
        self.next_id == other.next_id && self.active == other.active
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
            effect_id: 0,
            next_id: 1,
            active: None,
        }
    }

    /// Start a new latest task and return its ticket.
    pub fn begin(&mut self) -> TaskTicket {
        let ticket = TaskTicket::new(self.next_id);
        self.next_id = self.next_id.saturating_add(1);
        self.active = Some(ticket);
        ticket
    }

    /// Return the currently active latest task, if any.
    pub const fn active(&self) -> Option<TaskTicket> {
        self.active
    }

    pub(crate) fn effect_id(&mut self) -> u64 {
        if self.effect_id == 0 {
            self.effect_id = NEXT_LATEST_SLOT_ID.fetch_add(1, Ordering::Relaxed);
        }
        if let Some(ticket) = self.active {
            latest_registry()
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .insert(self.effect_id, ticket.id());
        }
        self.effect_id
    }

    pub(crate) fn generation_active(slot: u64, generation: u64) -> bool {
        latest_registry()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(&slot)
            .copied()
            == Some(generation)
    }

    pub(crate) fn restore_generation(slot: u64, generation: Option<u64>) {
        let mut registry = latest_registry()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(generation) = generation {
            registry.insert(slot, generation);
        } else {
            registry.remove(&slot);
        }
    }

    /// Return whether this ticket is still the active latest task.
    pub fn is_active(&self, ticket: TaskTicket) -> bool {
        self.active == Some(ticket)
    }

    /// Return whether this completion belongs to the active latest task.
    pub fn is_active_completion<Output>(&self, completion: &TaskCompletion<Output>) -> bool {
        self.is_active(completion.ticket)
    }

    /// Clear this task if `ticket` is still active.
    pub fn finish(&mut self, ticket: TaskTicket) -> bool {
        if self.is_active(ticket) {
            self.active = None;
            if self.effect_id != 0 {
                latest_registry()
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .remove(&self.effect_id);
            }
            true
        } else {
            false
        }
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
        self.active = None;
        if self.effect_id != 0 {
            latest_registry()
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .remove(&self.effect_id);
        }
    }
}

impl Drop for LatestTask {
    fn drop(&mut self) {
        if self.effect_id != 0 {
            latest_registry()
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .remove(&self.effect_id);
        }
    }
}

fn latest_registry() -> &'static Mutex<HashMap<u64, u64>> {
    ACTIVE_LATEST.get_or_init(|| Mutex::new(HashMap::new()))
}

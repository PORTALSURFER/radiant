use std::collections::HashMap;

use crate::runtime::ResourceKey;

use super::{KeyedLatestTasks, TaskTicket};

/// Tracks in-flight business work by generic resource key.
///
/// Use this when application work is naturally scoped to a resource such as a
/// document, file, cache entry, device, or viewport. Latest work replaces older
/// work for the same key, while exclusive work refuses duplicate submissions
/// until the active request finishes or is cancelled.
#[derive(Clone, Debug, Default)]
pub struct ResourceTasks {
    latest: KeyedLatestTasks<ResourceKey>,
    exclusive: HashMap<ResourceKey, TaskTicket>,
}

impl ResourceTasks {
    /// Build an idle resource-task registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Return whether no resource keys are currently tracked.
    pub fn is_empty(&self) -> bool {
        self.latest.is_empty() && self.exclusive.is_empty()
    }

    /// Clear all latest and exclusive resource work.
    pub fn clear(&mut self) {
        self.latest.clear();
        self.exclusive.clear();
    }

    /// Start replace-latest work for one resource key.
    pub(crate) fn begin_latest(&mut self, key: ResourceKey) -> ResourceTaskTicket {
        let ticket = self.latest.begin(key.clone());
        ResourceTaskTicket { key, ticket }
    }

    /// Start exclusive work for one resource key.
    ///
    /// Returns `None` when the same key already has an active exclusive task.
    pub(crate) fn begin_exclusive(&mut self, key: ResourceKey) -> Option<ResourceTaskTicket> {
        if self.exclusive.contains_key(&key) {
            return None;
        }
        let ticket = self.latest.begin(key.clone());
        self.exclusive.insert(key.clone(), ticket);
        Some(ResourceTaskTicket { key, ticket })
    }

    /// Return the active task for a resource key, if any.
    pub fn active(&self, key: &ResourceKey) -> Option<TaskTicket> {
        self.exclusive
            .get(key)
            .copied()
            .or_else(|| self.latest.active(key))
    }

    /// Return whether a resource task ticket is still current.
    pub fn is_active(&self, task: &ResourceTaskTicket) -> bool {
        self.is_active_key(task.key(), task.ticket())
    }

    /// Return whether a resource key and task ticket are still current.
    pub fn is_active_key(&self, key: &ResourceKey, ticket: TaskTicket) -> bool {
        self.latest.is_active(key, ticket) || self.exclusive.get(key).copied() == Some(ticket)
    }

    /// Finish a resource task only when the ticket is still current.
    pub fn finish(&mut self, task: &ResourceTaskTicket) -> bool {
        self.finish_key(task.key(), task.ticket())
    }

    /// Finish a resource task by key and ticket only when it is still current.
    pub fn finish_key(&mut self, key: &ResourceKey, ticket: TaskTicket) -> bool {
        let latest_finished = self.latest.finish(key, ticket);
        let exclusive_finished = self.exclusive.get(key).copied() == Some(ticket);
        if exclusive_finished {
            self.exclusive.remove(key);
        }
        latest_finished || exclusive_finished
    }

    /// Cancel all active latest and exclusive work for one resource key.
    pub fn cancel(&mut self, key: &ResourceKey) -> bool {
        let latest_cancelled = self.latest.cancel(key);
        let exclusive_cancelled = self.exclusive.remove(key).is_some();
        latest_cancelled || exclusive_cancelled
    }
}

/// Ticket assigned to one resource-keyed business task.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ResourceTaskTicket {
    key: ResourceKey,
    ticket: TaskTicket,
}

impl ResourceTaskTicket {
    /// Return the resource key for this task.
    pub fn key(&self) -> &ResourceKey {
        &self.key
    }

    /// Return the underlying monotonic task ticket.
    pub fn ticket(&self) -> TaskTicket {
        self.ticket
    }

    /// Numeric task id suitable for host logs or progress events.
    pub fn id(&self) -> u64 {
        self.ticket.id()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exclusive_resource_tasks_reject_duplicate_active_key() {
        let mut tasks = ResourceTasks::new();
        let key = ResourceKey::scoped("sample", "C:/kick.wav");

        let first = tasks
            .begin_exclusive(key.clone())
            .expect("first task starts");
        assert!(tasks.begin_exclusive(key.clone()).is_none());

        assert!(tasks.finish(&first));
        assert!(tasks.begin_exclusive(key).is_some());
    }

    #[test]
    fn latest_resource_tasks_replace_previous_ticket_for_same_key() {
        let mut tasks = ResourceTasks::new();
        let key = ResourceKey::scoped("sample", "C:/kick.wav");

        let first = tasks.begin_latest(key.clone());
        let second = tasks.begin_latest(key);

        assert!(!tasks.is_active(&first));
        assert!(tasks.is_active(&second));
    }
}

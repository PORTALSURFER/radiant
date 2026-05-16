//! Small keyed-task helpers for application-owned background work.

use std::{collections::HashMap, hash::Hash};

/// Monotonic ticket for a host background task.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TaskTicket {
    id: u64,
}

impl TaskTicket {
    /// Numeric task id suitable for host messages, logs, or progress events.
    pub const fn id(self) -> u64 {
        self.id
    }
}

/// Completion payload tagged with the ticket that started the work.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskCompletion<Output> {
    /// Ticket assigned when the task was started.
    pub ticket: TaskTicket,
    /// Output returned by the background work.
    pub output: Output,
}

impl<Output> TaskCompletion<Output> {
    /// Numeric task id suitable for matching existing host message contracts.
    pub const fn task_id(&self) -> u64 {
        self.ticket.id()
    }
}

/// Completion payload tagged with the key and ticket that started the work.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyedTaskCompletion<Key, Output> {
    /// Host-defined key for the resource or operation.
    pub key: Key,
    /// Ticket assigned when the task was started for this key.
    pub ticket: TaskTicket,
    /// Output returned by the background work.
    pub output: Output,
}

impl<Key, Output> KeyedTaskCompletion<Key, Output> {
    /// Numeric task id suitable for matching existing host message contracts.
    pub const fn task_id(&self) -> u64 {
        self.ticket.id()
    }
}

/// Tracks the latest in-flight task for one host-owned resource.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LatestTask {
    next_id: u64,
    active: Option<TaskTicket>,
}

impl Default for LatestTask {
    fn default() -> Self {
        Self::new()
    }
}

/// Tracks latest in-flight tasks for multiple host-owned keys.
///
/// Use this when an application can start independent replace-latest work for
/// many resources, such as row previews, file scans, or per-document loads.
#[derive(Clone, Debug)]
pub struct KeyedLatestTasks<Key> {
    tasks: HashMap<Key, LatestTask>,
}

impl<Key> Default for KeyedLatestTasks<Key> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Key> KeyedLatestTasks<Key> {
    /// Build an idle keyed task registry.
    pub fn new() -> Self {
        Self {
            tasks: HashMap::new(),
        }
    }

    /// Return whether the registry has no active or remembered keys.
    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }

    /// Return the number of keys currently tracked by this registry.
    pub fn len(&self) -> usize {
        self.tasks.len()
    }

    /// Clear every tracked key.
    pub fn clear(&mut self) {
        self.tasks.clear();
    }
}

impl<Key> KeyedLatestTasks<Key>
where
    Key: Eq + Hash,
{
    /// Return the currently active task for `key`, if any.
    pub fn active(&self, key: &Key) -> Option<TaskTicket> {
        self.tasks.get(key).and_then(LatestTask::active)
    }

    /// Return whether `ticket` is still the active latest task for `key`.
    pub fn is_active(&self, key: &Key, ticket: TaskTicket) -> bool {
        self.tasks
            .get(key)
            .is_some_and(|task| task.is_active(ticket))
    }

    /// Clear the active task for `key` when `ticket` is still current.
    pub fn finish(&mut self, key: &Key, ticket: TaskTicket) -> bool {
        let Some(task) = self.tasks.get_mut(key) else {
            return false;
        };
        task.finish(ticket)
    }

    /// Cancel any active task for `key` while keeping the key remembered.
    pub fn cancel(&mut self, key: &Key) -> bool {
        let Some(task) = self.tasks.get_mut(key) else {
            return false;
        };
        task.cancel();
        true
    }

    /// Remove a tracked key and any active task associated with it.
    pub fn remove(&mut self, key: &Key) -> Option<LatestTask> {
        self.tasks.remove(key)
    }
}

impl<Key> KeyedLatestTasks<Key>
where
    Key: Clone + Eq + Hash,
{
    /// Start a new latest task for `key` and return its ticket.
    pub fn begin(&mut self, key: Key) -> TaskTicket {
        self.tasks.entry(key).or_default().begin()
    }
}

impl LatestTask {
    /// Build an idle task tracker.
    pub const fn new() -> Self {
        Self {
            next_id: 1,
            active: None,
        }
    }

    /// Start a new latest task and return its ticket.
    pub fn begin(&mut self) -> TaskTicket {
        let ticket = TaskTicket { id: self.next_id };
        self.next_id = self.next_id.saturating_add(1);
        self.active = Some(ticket);
        ticket
    }

    /// Return the currently active latest task, if any.
    pub const fn active(&self) -> Option<TaskTicket> {
        self.active
    }

    /// Return whether this ticket is still the active latest task.
    pub fn is_active(&self, ticket: TaskTicket) -> bool {
        self.active == Some(ticket)
    }

    /// Clear this task if `ticket` is still active.
    pub fn finish(&mut self, ticket: TaskTicket) -> bool {
        if self.is_active(ticket) {
            self.active = None;
            true
        } else {
            false
        }
    }

    /// Clear any active task.
    pub fn cancel(&mut self) {
        self.active = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn latest_task_rejects_stale_tickets_after_newer_begin() {
        let mut task = LatestTask::new();
        let first = task.begin();
        let second = task.begin();

        assert!(!task.is_active(first));
        assert!(task.is_active(second));
        assert!(!task.finish(first));
        assert!(task.finish(second));
        assert_eq!(task.active(), None);
    }

    #[test]
    fn keyed_latest_tasks_reject_stale_tickets_per_key() {
        let mut tasks = KeyedLatestTasks::new();

        let first_a = tasks.begin("a");
        let current_a = tasks.begin("a");
        let only_b = tasks.begin("b");

        assert!(!tasks.is_active(&"a", first_a));
        assert!(tasks.is_active(&"a", current_a));
        assert!(tasks.is_active(&"b", only_b));

        assert!(!tasks.finish(&"a", first_a));
        assert!(tasks.finish(&"a", current_a));
        assert!(tasks.finish(&"b", only_b));
        assert_eq!(tasks.active(&"a"), None);
        assert_eq!(tasks.active(&"b"), None);
    }

    #[test]
    fn keyed_latest_tasks_can_cancel_and_remove_keys() {
        let mut tasks = KeyedLatestTasks::new();

        let ticket = tasks.begin(String::from("preview"));
        assert_eq!(tasks.len(), 1);
        assert!(tasks.cancel(&String::from("preview")));
        assert!(!tasks.is_active(&String::from("preview"), ticket));
        assert!(tasks.remove(&String::from("preview")).is_some());
        assert!(tasks.is_empty());
    }
}

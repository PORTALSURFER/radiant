use std::{collections::HashMap, hash::Hash};

use super::{KeyedTaskCompletion, LatestTask, TaskTicket};

#[cfg(test)]
#[path = "keyed_latest/tests.rs"]
mod tests;

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

    /// Return whether this completion belongs to the active latest task for its key.
    pub fn is_active_completion<Output>(
        &self,
        completion: &KeyedTaskCompletion<Key, Output>,
    ) -> bool {
        self.is_active(&completion.key, completion.ticket)
    }

    /// Clear the active task for `key` when `ticket` is still current.
    pub fn finish(&mut self, key: &Key, ticket: TaskTicket) -> bool {
        let Some(task) = self.tasks.get_mut(key) else {
            return false;
        };
        task.finish(ticket)
    }

    /// Clear the active task for a current keyed completion and return its output.
    pub fn finish_completion<Output>(
        &mut self,
        completion: KeyedTaskCompletion<Key, Output>,
    ) -> Option<Output> {
        self.finish(&completion.key, completion.ticket)
            .then_some(completion.output)
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

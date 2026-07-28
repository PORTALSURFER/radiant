use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::runtime::ResourceKey;

use super::{KeyedLatestTasks, KeyedTaskCompletion, LatestTaskTransaction, TaskTicket};

/// Tracks in-flight business work by generic resource key.
///
/// Use this when application work is naturally scoped to a resource such as a
/// document, file, cache entry, device, or viewport. Latest work replaces older
/// work for the same key, while exclusive work refuses duplicate submissions
/// until the active request finishes or is cancelled.
#[derive(Debug, Default)]
pub struct ResourceTasks {
    latest: KeyedLatestTasks<ResourceKey>,
    exclusive: Arc<Mutex<HashMap<ResourceKey, TaskTicket>>>,
}

impl Clone for ResourceTasks {
    fn clone(&self) -> Self {
        Self {
            latest: self.latest.clone(),
            exclusive: Arc::new(Mutex::new(lock_exclusive(&self.exclusive).clone())),
        }
    }
}

impl ResourceTasks {
    /// Build an idle resource-task registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Return whether no resource keys are currently tracked.
    pub fn is_empty(&self) -> bool {
        self.latest.is_empty() && lock_exclusive(&self.exclusive).is_empty()
    }

    /// Clear all latest and exclusive resource work.
    pub fn clear(&mut self) {
        self.latest.clear();
        lock_exclusive(&self.exclusive).clear();
    }

    /// Start replace-latest work for one resource key.
    #[cfg(test)]
    pub(crate) fn begin_latest(&mut self, key: ResourceKey) -> ResourceTaskTicket {
        let ticket = self.latest.begin(key.clone());
        ResourceTaskTicket { key, ticket }
    }

    pub(crate) fn begin_latest_transaction(
        &mut self,
        key: ResourceKey,
    ) -> (ResourceTaskTicket, LatestTaskTransaction, u64) {
        let (ticket, transaction, effect_id) = self.latest.begin_replacement(key.clone());
        (ResourceTaskTicket { key, ticket }, transaction, effect_id)
    }

    /// Reserve exclusive work transactionally so abandoned or rejected work
    /// restores the previous task and releases only this key's reservation.
    pub(crate) fn begin_exclusive_transaction(
        &mut self,
        key: ResourceKey,
    ) -> Option<(ResourceTaskTicket, LatestTaskTransaction, u64)> {
        let mut exclusive = lock_exclusive(&self.exclusive);
        if exclusive.contains_key(&key) {
            return None;
        }
        let (ticket, transaction, effect_id) = self.latest.begin_replacement(key.clone());
        exclusive.insert(key.clone(), ticket);
        let reservations = Arc::clone(&self.exclusive);
        let reservation_key = key.clone();
        let release = Arc::new(move || {
            let mut reservations = lock_exclusive(&reservations);
            if reservations.get(&reservation_key).copied() == Some(ticket) {
                reservations.remove(&reservation_key);
            }
        });
        Some((
            ResourceTaskTicket { key, ticket },
            transaction.with_rejection_hook(release),
            effect_id,
        ))
    }

    /// Return the active task for a resource key, if any.
    pub fn active(&self, key: &ResourceKey) -> Option<TaskTicket> {
        lock_exclusive(&self.exclusive)
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
        self.latest.is_active(key, ticket)
            || lock_exclusive(&self.exclusive).get(key).copied() == Some(ticket)
    }

    /// Return whether this resource completion still belongs to active work.
    pub fn is_active_completion<Output>(
        &self,
        completion: &KeyedTaskCompletion<ResourceKey, Output>,
    ) -> bool {
        self.is_active_key(&completion.key, completion.ticket)
    }

    /// Finish a resource task only when the ticket is still current.
    pub fn finish(&mut self, task: &ResourceTaskTicket) -> bool {
        self.finish_key(task.key(), task.ticket())
    }

    /// Finish a resource task by key and ticket only when it is still current.
    pub fn finish_key(&mut self, key: &ResourceKey, ticket: TaskTicket) -> bool {
        let latest_finished = self.latest.finish(key, ticket);
        let exclusive_finished = lock_exclusive(&self.exclusive).get(key).copied() == Some(ticket);
        if exclusive_finished {
            lock_exclusive(&self.exclusive).remove(key);
        }
        latest_finished || exclusive_finished
    }

    /// Finish a current resource completion and return its output.
    pub fn finish_completion<Output>(
        &mut self,
        completion: KeyedTaskCompletion<ResourceKey, Output>,
    ) -> Option<Output> {
        self.finish_key(&completion.key, completion.ticket)
            .then_some(completion.output)
    }

    /// Cancel all active latest and exclusive work for one resource key.
    pub fn cancel(&mut self, key: &ResourceKey) -> bool {
        let latest_cancelled = self.latest.cancel(key);
        let exclusive_cancelled = lock_exclusive(&self.exclusive).remove(key).is_some();
        latest_cancelled || exclusive_cancelled
    }
}

fn lock_exclusive(
    exclusive: &Mutex<HashMap<ResourceKey, TaskTicket>>,
) -> std::sync::MutexGuard<'_, HashMap<ResourceKey, TaskTicket>> {
    exclusive
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
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

        let (first, first_transaction, _) = tasks
            .begin_exclusive_transaction(key.clone())
            .expect("first task starts");
        first_transaction.accept();
        assert!(tasks.begin_exclusive_transaction(key.clone()).is_none());

        assert!(tasks.finish(&first));
        let (_, second_transaction, _) = tasks
            .begin_exclusive_transaction(key)
            .expect("finished task releases reservation");
        second_transaction.reject();
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

    #[test]
    fn resource_tasks_finish_completion_returns_only_current_output() {
        let mut tasks = ResourceTasks::new();
        let key = ResourceKey::scoped("sample", "C:/kick.wav");

        let stale = tasks.begin_latest(key.clone());
        let (current, current_transaction, _) = tasks
            .begin_exclusive_transaction(key.clone())
            .expect("exclusive task starts");
        current_transaction.accept();

        let stale_completion = KeyedTaskCompletion {
            key: key.clone(),
            ticket: stale.ticket(),
            output: "stale",
        };
        assert!(!tasks.is_active_completion(&stale_completion));
        assert_eq!(tasks.finish_completion(stale_completion), None);

        assert_eq!(
            tasks.finish_completion(KeyedTaskCompletion {
                key: key.clone(),
                ticket: current.ticket(),
                output: "current",
            }),
            Some("current")
        );
        assert_eq!(tasks.active(&key), None);
        let (_, transaction, _) = tasks
            .begin_exclusive_transaction(key)
            .expect("finished task releases reservation");
        transaction.reject();
    }

    #[test]
    fn exclusive_transactions_rollback_only_the_rejected_key() {
        let key_a = ResourceKey::scoped("sample", "C:/a.wav");
        let key_b = ResourceKey::scoped("sample", "C:/b.wav");
        let mut tasks = ResourceTasks::new();
        let (_ticket_a, transaction_a, _) = tasks
            .begin_exclusive_transaction(key_a.clone())
            .expect("key A reservation");
        let (ticket_b, transaction_b, _) = tasks
            .begin_exclusive_transaction(key_b.clone())
            .expect("key B reservation");

        transaction_a.reject();
        assert_eq!(tasks.active(&key_a), None);
        assert_eq!(tasks.active(&key_b), Some(ticket_b.ticket()));
        let (_, replacement_a, _) = tasks
            .begin_exclusive_transaction(key_a.clone())
            .expect("rejected key can be reserved again");
        replacement_a.accept();
        assert!(tasks.begin_exclusive_transaction(key_b).is_none());

        transaction_b.accept();
        assert!(tasks.active(&key_a).is_some());
    }

    #[test]
    fn accepted_exclusive_transaction_persists_until_finish_or_cancel() {
        let key = ResourceKey::scoped("sample", "C:/accepted.wav");
        let mut tasks = ResourceTasks::new();
        let (ticket, transaction, _) = tasks
            .begin_exclusive_transaction(key.clone())
            .expect("exclusive reservation");
        transaction.accept();
        drop(transaction);

        assert_eq!(tasks.active(&key), Some(ticket.ticket()));
        assert!(tasks.begin_exclusive_transaction(key.clone()).is_none());
        assert!(tasks.finish(&ticket));
        let (_, replacement_transaction, _) = tasks
            .begin_exclusive_transaction(key.clone())
            .expect("finish releases reservation");
        replacement_transaction.accept();
        assert!(tasks.cancel(&key));
        let (_, replacement_transaction, _) = tasks
            .begin_exclusive_transaction(key)
            .expect("cancel releases reservation");
        replacement_transaction.reject();
    }

    #[test]
    fn rejected_exclusive_completion_is_stale_and_reservation_is_released() {
        let key = ResourceKey::scoped("sample", "C:/rejected.wav");
        let mut tasks = ResourceTasks::new();
        let (predecessor, predecessor_transaction, _) = tasks.begin_latest_transaction(key.clone());
        predecessor_transaction.accept();

        let (rejected, transaction, _) = tasks
            .begin_exclusive_transaction(key.clone())
            .expect("exclusive reservation");
        assert_eq!(tasks.active(&key), Some(rejected.ticket()));

        transaction.reject();
        assert_eq!(tasks.active(&key), Some(predecessor.ticket()));
        assert!(!tasks.is_active_key(&key, rejected.ticket()));
        assert!(!tasks.finish(&rejected));

        let (_replacement, replacement_transaction, _) = tasks
            .begin_exclusive_transaction(key)
            .expect("rejected key reservation should be released");
        replacement_transaction.reject();
    }

    #[test]
    fn cloned_resource_tasks_keep_exclusive_reservations_isolated() {
        let key = ResourceKey::scoped("sample", "C:/cloned.wav");
        let mut tasks = ResourceTasks::new();
        let (ticket, transaction, _) = tasks
            .begin_exclusive_transaction(key.clone())
            .expect("exclusive reservation");
        transaction.accept();
        drop(transaction);

        let mut clone = tasks.clone();
        assert_eq!(tasks.active(&key), Some(ticket.ticket()));
        assert_eq!(clone.active(&key), Some(ticket.ticket()));

        assert!(tasks.finish(&ticket));
        assert_eq!(tasks.active(&key), None);
        assert_eq!(clone.active(&key), Some(ticket.ticket()));
        assert!(clone.finish(&ticket));
        assert_eq!(clone.active(&key), None);
    }
}

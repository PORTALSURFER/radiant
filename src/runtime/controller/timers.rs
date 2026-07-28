use super::owner::{LifecycleDescriptor, RuntimeOwner};
use crate::application::LatestTimerTransaction;
use crate::runtime::{RuntimeTimerWake, command::TimerEffect};
use std::{collections::HashMap, time::Duration};

struct Registered<Message> {
    wake: RuntimeTimerWake,
    transaction: Option<LatestTimerTransaction>,
    map: Option<Box<dyn FnOnce() -> Message + 'static>>,
    lifecycle: LifecycleDescriptor,
}

pub(super) struct TimerEffects<Message> {
    owner: RuntimeOwner,
    registry: HashMap<u64, Registered<Message>>,
    latest: HashMap<u64, u64>,
    epoch: u64,
    next_id: u64,
}

impl<Message> Default for TimerEffects<Message> {
    fn default() -> Self {
        Self::new(RuntimeOwner::new())
    }
}

impl<Message> TimerEffects<Message> {
    pub(super) fn new(owner: RuntimeOwner) -> Self {
        Self {
            owner,
            registry: HashMap::new(),
            latest: HashMap::new(),
            epoch: 1,
            next_id: 1,
        }
    }
    pub(super) fn schedule(
        &mut self,
        effect: TimerEffect<Message>,
        mut host_schedule: impl FnMut(Duration, RuntimeTimerWake) -> bool,
    ) -> bool {
        let id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        let transaction = effect.transaction;
        let slot = transaction.as_ref().map(LatestTimerTransaction::slot);
        let generation = transaction
            .as_ref()
            .map_or(0, LatestTimerTransaction::generation);
        let cancellation = transaction
            .as_ref()
            .map(LatestTimerTransaction::cancellation_probe);
        let wake = RuntimeTimerWake::controller(id, generation, self.epoch);
        let previous = slot.and_then(|slot| {
            let old = self.latest.insert(slot, id);
            old.and_then(|old_id| self.registry.remove(&old_id))
        });
        self.registry.insert(
            id,
            Registered {
                wake,
                transaction,
                map: Some(effect.map),
                lifecycle: LifecycleDescriptor::new(
                    self.owner.clone(),
                    id,
                    slot,
                    generation,
                    cancellation,
                ),
            },
        );
        if host_schedule(effect.delay, wake) {
            if let Some(transaction) = self
                .registry
                .get(&id)
                .and_then(|registered| registered.transaction.as_ref())
            {
                transaction.accept();
            }
            return true;
        }

        if let Some(registered) = self.registry.remove(&id)
            && let Some(transaction) = registered.transaction
        {
            transaction.reject();
        }
        if let Some(slot) = slot {
            if let Some(previous) = previous {
                self.latest.insert(slot, previous.wake.id);
                self.registry.insert(previous.wake.id, previous);
            } else {
                self.latest.remove(&slot);
            }
        }
        false
    }

    /// Map one controller wake on the UI turn. Unknown, stale, or superseded
    /// wakes are consumed without invoking their mapper.
    pub(super) fn map_wake(&mut self, wake: RuntimeTimerWake) -> Option<Message> {
        let registered = self.registry.get(&wake.id)?;
        if registered.wake != wake {
            return None;
        }
        if !registered.lifecycle.admits(
            &self.owner,
            wake.id,
            wake.generation,
            wake.epoch == self.epoch,
        ) {
            let slot = registered.lifecycle.slot();
            self.registry.remove(&wake.id);
            if let Some(slot) = slot
                && self.latest.get(&slot).copied() == Some(wake.id)
            {
                self.latest.remove(&slot);
            }
            return None;
        }
        let stale_slot = registered
            .transaction
            .as_ref()
            .filter(|transaction| !transaction.is_active())
            .map(LatestTimerTransaction::slot);
        if let Some(slot) = stale_slot {
            self.registry.remove(&wake.id);
            if self.latest.get(&slot).copied() == Some(wake.id) {
                self.latest.remove(&slot);
            }
            return None;
        }
        let mut registered = self.registry.remove(&wake.id)?;
        if let Some(transaction) = registered.transaction.as_ref()
            && self.latest.get(&transaction.slot()).copied() == Some(wake.id)
        {
            self.latest.remove(&transaction.slot());
        }
        registered.map.take().map(|map| map())
    }

    pub(super) fn shutdown(&mut self) {
        self.epoch = self.epoch.saturating_add(1);
        self.registry.clear();
        self.latest.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    #[test]
    fn maps_only_on_ui_drain_and_drops_superseded_wake() {
        let calls = Arc::new(AtomicUsize::new(0));
        let mut effects = TimerEffects::default();
        let mut latest = crate::application::LatestTask::new();
        let transaction1 = latest.begin_timer_replacement();
        let slot = transaction1.slot();
        let first_calls = Arc::clone(&calls);
        assert!(effects.schedule(
            TimerEffect {
                delay: Duration::ZERO,
                transaction: Some(transaction1),
                map: Box::new(move || {
                    first_calls.fetch_add(1, Ordering::SeqCst);
                    1
                }),
            },
            |_, _| true,
        ));
        let old = *effects.latest.get(&slot).unwrap();
        let transaction2 = latest.begin_timer_replacement();
        let second_calls = Arc::clone(&calls);
        let mut current_wake = None;
        assert!(effects.schedule(
            TimerEffect {
                delay: Duration::ZERO,
                transaction: Some(transaction2),
                map: Box::new(move || {
                    second_calls.fetch_add(1, Ordering::SeqCst);
                    2
                }),
            },
            |_, wake| {
                current_wake = Some(wake);
                true
            },
        ));
        assert!(
            effects
                .map_wake(RuntimeTimerWake::controller(old, 1, 1))
                .is_none()
        );
        assert_eq!(effects.map_wake(current_wake.unwrap()), Some(2));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn rejected_latest_replacement_restores_prior_generation_and_mapper() {
        let mut effects = TimerEffects::default();
        let mut latest = crate::application::LatestTask::new();
        let first_transaction = latest.begin_timer_replacement();
        let first_ticket = first_transaction.replacement();
        let slot = first_transaction.slot();
        let mut first_wake = None;
        assert!(effects.schedule(
            TimerEffect {
                delay: Duration::ZERO,
                transaction: Some(first_transaction),
                map: Box::new(|| 1),
            },
            |_, wake| {
                first_wake = Some(wake);
                true
            },
        ));
        let replacement_transaction = latest.begin_timer_replacement();
        assert!(!effects.schedule(
            TimerEffect {
                delay: Duration::ZERO,
                transaction: Some(replacement_transaction),
                map: Box::new(|| 2),
            },
            |_, _| false,
        ));
        assert_eq!(latest.active(), Some(first_ticket));
        assert!(effects.latest.contains_key(&slot));
        assert_eq!(effects.map_wake(first_wake.unwrap()), Some(1));
    }

    #[test]
    fn shutdown_ignores_late_wakes() {
        let mut effects = TimerEffects::default();
        let mut wake = None;
        assert!(effects.schedule(
            TimerEffect {
                delay: Duration::ZERO,
                transaction: None,
                map: Box::new(|| 1),
            },
            |_, timer_wake| {
                wake = Some(timer_wake);
                true
            },
        ));
        effects.shutdown();
        assert!(effects.map_wake(wake.unwrap()).is_none());
    }

    #[test]
    fn maps_each_one_shot_once_in_fifo_order() {
        const COUNT: usize = 2_048;
        let mut effects = TimerEffects::default();
        let mut wakes = Vec::with_capacity(COUNT);
        for value in 0..COUNT {
            assert!(effects.schedule(
                TimerEffect {
                    delay: Duration::ZERO,
                    transaction: None,
                    map: Box::new(move || value),
                },
                |_, wake| {
                    wakes.push(wake);
                    true
                },
            ));
        }
        let mapped = wakes
            .into_iter()
            .map(|wake| effects.map_wake(wake).expect("scheduled wake"))
            .collect::<Vec<_>>();
        assert_eq!(mapped, (0..COUNT).collect::<Vec<_>>());
    }

    struct DropSentinel(Arc<AtomicUsize>);
    impl Drop for DropSentinel {
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[test]
    fn stale_latest_wake_releases_mapper_after_finish_cancel_or_drop() {
        for action in 0..3 {
            let drops = Arc::new(AtomicUsize::new(0));
            let mut effects = TimerEffects::default();
            let mut latest = crate::application::LatestTask::new();
            let transaction = latest.begin_timer_replacement();
            let ticket = transaction.replacement();
            let mut wake = None;
            let sentinel = DropSentinel(Arc::clone(&drops));
            assert!(effects.schedule(
                TimerEffect {
                    delay: Duration::ZERO,
                    transaction: Some(transaction),
                    map: Box::new(move || {
                        let _sentinel = sentinel;
                        1
                    }),
                },
                |_, timer_wake| {
                    wake = Some(timer_wake);
                    true
                },
            ));
            match action {
                0 => assert!(latest.finish(ticket)),
                1 => latest.cancel(),
                _ => drop(latest),
            }
            assert!(effects.map_wake(wake.unwrap()).is_none());
            assert_eq!(drops.load(Ordering::SeqCst), 1);
        }
    }
}

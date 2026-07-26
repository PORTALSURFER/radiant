use crate::runtime::{RuntimeTimerWake, command::TimerEffect};
use std::{
    collections::{HashMap, VecDeque},
    time::Duration,
};

struct Registered<Message> {
    wake: RuntimeTimerWake,
    latest_slot: Option<u64>,
    map: Option<Box<dyn FnOnce() -> Message + 'static>>,
}

pub(super) struct TimerEffects<Message> {
    registry: HashMap<u64, Registered<Message>>,
    latest: HashMap<u64, u64>,
    ingress: VecDeque<RuntimeTimerWake>,
    epoch: u64,
    next_id: u64,
}
impl<Message> Default for TimerEffects<Message> {
    fn default() -> Self {
        Self {
            registry: HashMap::new(),
            latest: HashMap::new(),
            ingress: VecDeque::new(),
            epoch: 1,
            next_id: 1,
        }
    }
}
impl<Message> TimerEffects<Message> {
    pub(super) fn schedule(
        &mut self,
        effect: TimerEffect<Message>,
        mut host_schedule: impl FnMut(Duration, RuntimeTimerWake) -> bool,
    ) -> bool {
        let id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        let previous_generation = effect.previous_generation;
        let wake = RuntimeTimerWake::controller(id, effect.generation, self.epoch);
        let previous = effect.latest_slot.and_then(|slot| {
            let old = self.latest.insert(slot, id);
            old.and_then(|old_id| self.registry.remove(&old_id))
        });
        self.registry.insert(
            id,
            Registered {
                wake,
                latest_slot: effect.latest_slot,
                map: Some(effect.map),
            },
        );
        if host_schedule(effect.delay, wake) {
            return true;
        }
        self.registry.remove(&id);
        if let Some(slot) = effect.latest_slot {
            if let Some(previous) = previous {
                self.latest.insert(slot, previous.wake.id);
                self.registry.insert(previous.wake.id, previous);
            } else {
                self.latest.remove(&slot);
            }
            crate::application::LatestTask::restore_generation(slot, previous_generation);
        }
        false
    }
    pub(super) fn enqueue(&mut self, wakes: impl IntoIterator<Item = RuntimeTimerWake>) {
        self.ingress.extend(wakes);
    }
    pub(super) fn drain(&mut self, budget: usize) -> Vec<Message> {
        let high_water = self.ingress.len().min(budget.max(1));
        let mut messages = Vec::new();
        for _ in 0..high_water {
            let Some(wake) = self.ingress.pop_front() else {
                break;
            };
            let Some(registered) = self.registry.get_mut(&wake.id) else {
                continue;
            };
            if registered.wake != wake {
                continue;
            };
            if let Some(slot) = registered.latest_slot
                && !crate::application::LatestTask::generation_active(slot, wake.generation)
            {
                self.registry.remove(&wake.id);
                if self.latest.get(&slot).copied() == Some(wake.id) {
                    self.latest.remove(&slot);
                }
                continue;
            }
            let Some(map) = registered.map.take() else {
                continue;
            };
            self.registry.remove(&wake.id);
            messages.push(map());
        }
        messages
    }
    pub(super) fn shutdown(&mut self) {
        self.epoch = self.epoch.saturating_add(1);
        self.registry.clear();
        self.latest.clear();
        self.ingress.clear();
    }

    pub(super) fn has_remaining_work(&self) -> bool {
        !self.ingress.is_empty()
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
        let ticket1 = latest.begin();
        let slot = latest.effect_id();
        let first_calls = Arc::clone(&calls);
        assert!(effects.schedule(
            TimerEffect {
                delay: Duration::ZERO,
                generation: ticket1.id(),
                latest_slot: Some(slot),
                previous_generation: None,
                map: Box::new(move || {
                    first_calls.fetch_add(1, Ordering::SeqCst);
                    1
                })
            },
            |_, _| true
        ));
        let old = *effects.latest.get(&slot).unwrap();
        let ticket2 = latest.begin();
        latest.effect_id();
        let second_calls = Arc::clone(&calls);
        let second = TimerEffect {
            delay: Duration::ZERO,
            generation: ticket2.id(),
            latest_slot: Some(slot),
            previous_generation: Some(ticket1.id()),
            map: Box::new(move || {
                second_calls.fetch_add(1, Ordering::SeqCst);
                2
            }),
        };
        let mut current_wake = None;
        assert!(effects.schedule(second, |_, wake| {
            current_wake = Some(wake);
            true
        }));
        effects.enqueue([RuntimeTimerWake::controller(old, 1, 1)]);
        assert!(effects.drain(64).is_empty());
        effects.enqueue([current_wake.unwrap()]);
        assert_eq!(effects.drain(64), vec![2]);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn rejected_latest_replacement_restores_prior_generation_and_mapper() {
        let mut effects = TimerEffects::default();
        let mut latest = crate::application::LatestTask::new();
        let first_ticket = latest.begin();
        let slot = latest.effect_id();
        let mut first_wake = None;
        assert!(effects.schedule(
            TimerEffect {
                delay: Duration::ZERO,
                generation: first_ticket.id(),
                latest_slot: Some(slot),
                previous_generation: None,
                map: Box::new(|| 1),
            },
            |_, wake| {
                first_wake = Some(wake);
                true
            },
        ));

        let replacement_ticket = latest.begin();
        latest.effect_id();
        assert!(!effects.schedule(
            TimerEffect {
                delay: Duration::ZERO,
                generation: replacement_ticket.id(),
                latest_slot: Some(slot),
                previous_generation: Some(first_ticket.id()),
                map: Box::new(|| 2),
            },
            |_, _| false,
        ));

        effects.enqueue([first_wake.expect("accepted first wake")]);
        assert_eq!(effects.drain(64), vec![1]);
    }

    #[test]
    fn shutdown_ignores_late_wakes() {
        let mut effects = TimerEffects::default();
        let mut wake = None;
        assert!(effects.schedule(
            TimerEffect {
                delay: Duration::ZERO,
                generation: 0,
                latest_slot: None,
                previous_generation: None,
                map: Box::new(|| 1)
            },
            |_, timer_wake| {
                wake = Some(timer_wake);
                true
            }
        ));
        effects.shutdown();
        effects.enqueue([wake.unwrap()]);
        assert!(effects.drain(64).is_empty());
    }

    #[test]
    fn retains_excess_ingress_and_maps_each_one_shot_once_in_fifo_order() {
        const COUNT: usize = 2_048;
        let mut effects = TimerEffects::default();
        let mut wakes = Vec::with_capacity(COUNT);
        for value in 0..COUNT {
            assert!(effects.schedule(
                TimerEffect {
                    delay: Duration::ZERO,
                    generation: 0,
                    latest_slot: None,
                    previous_generation: None,
                    map: Box::new(move || value),
                },
                |_, wake| {
                    wakes.push(wake);
                    true
                },
            ));
        }

        effects.enqueue(wakes);
        let mut mapped = Vec::with_capacity(COUNT);
        while mapped.len() < COUNT {
            mapped.extend(effects.drain(64));
        }

        assert_eq!(mapped, (0..COUNT).collect::<Vec<_>>());
        assert!(effects.drain(64).is_empty());
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
            let ticket = latest.begin();
            let slot = latest.effect_id();
            let sentinel = DropSentinel(Arc::clone(&drops));
            let mut wake = None;
            assert!(effects.schedule(
                TimerEffect {
                    delay: Duration::ZERO,
                    generation: ticket.id(),
                    latest_slot: Some(slot),
                    previous_generation: None,
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
            effects.enqueue([wake.expect("scheduled wake")]);
            assert!(effects.drain(64).is_empty());
            assert_eq!(drops.load(Ordering::SeqCst), 1);
        }
    }
}

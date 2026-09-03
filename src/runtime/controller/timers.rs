use super::owner::{AuxiliaryWindowOwner, EffectOrigin, LifecycleDescriptor, RuntimeOwner};
use crate::application::LatestTimerTransaction;
use crate::runtime::{RuntimeTimerOwner, RuntimeTimerWake, command::TimerEffect};
use std::{collections::HashMap, sync::Arc, time::Duration};

struct Registered<Message> {
    wake: RuntimeTimerWake,
    transaction: Option<LatestTimerTransaction>,
    map: Option<Box<dyn FnOnce() -> Message + 'static>>,
    lifecycle: LifecycleDescriptor,
    owner_generation: Option<u64>,
    origin: EffectOrigin,
}

pub(super) struct MappedTimerMessage<Message> {
    pub(super) message: Message,
    pub(super) origin: EffectOrigin,
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
        origin: EffectOrigin,
        mut host_schedule: impl FnMut(Duration, RuntimeTimerWake) -> bool,
    ) -> bool {
        let id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        let effect_lifecycle = effect.lifecycle.as_ref();
        let transaction = effect.transaction;
        let slot = transaction.as_ref().map(LatestTimerTransaction::slot);
        // Keep the wake generation's existing latest-ticket meaning. The
        // declarative generation is retained separately in the registration
        // and fenced by the exact private origin witness.
        let generation = transaction
            .as_ref()
            .map_or(0, LatestTimerTransaction::generation);
        let owner_generation = origin.declarative_generation();
        let cancellation = transaction
            .as_ref()
            .map(LatestTimerTransaction::cancellation_probe);
        let cancellation = combine_cancellation_probes(
            cancellation,
            effect_lifecycle.map(|lifecycle| Arc::clone(&lifecycle.cancellation)),
        );
        let cancellation = combine_cancellation_probes(cancellation, origin.cancellation_probe());
        let wake = RuntimeTimerWake::controller(id, generation, self.epoch);
        let lifecycle = if let Some(effect_lifecycle) = effect_lifecycle {
            LifecycleDescriptor::new_for_effect(
                self.owner.clone(),
                id,
                effect_lifecycle,
                cancellation,
            )
        } else {
            LifecycleDescriptor::new(self.owner.clone(), id, slot, generation, cancellation)
        };
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
                lifecycle,
                owner_generation,
                origin,
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
    pub(super) fn map_wake(
        &mut self,
        wake: RuntimeTimerWake,
    ) -> Option<MappedTimerMessage<Message>> {
        if wake.owner != RuntimeTimerOwner::Controller {
            return None;
        }
        let registered = self.registry.get(&wake.id)?;
        if registered.wake != wake {
            return None;
        }

        let latest_slot_current = registered.transaction.as_ref().is_none_or(|transaction| {
            self.latest.get(&transaction.slot()).copied() == Some(wake.id)
        });
        let transaction_current = registered
            .transaction
            .as_ref()
            .is_none_or(LatestTimerTransaction::is_active);
        let current = wake.epoch == self.epoch
            && latest_slot_current
            && transaction_current
            && registered.owner_generation == registered.origin.declarative_generation()
            && registered.origin.is_live()
            && registered.lifecycle.admits_effect(
                &self.owner,
                wake.id,
                registered.lifecycle.identity(),
                wake.generation,
                latest_slot_current,
            );
        if !current {
            let slot = registered.lifecycle.slot();
            self.registry.remove(&wake.id);
            if let Some(slot) = slot
                && self.latest.get(&slot).copied() == Some(wake.id)
            {
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
        let origin = registered.origin.clone();
        let message = registered.map.take().map(|map| map())?;
        let current_after_mapping = wake.epoch == self.epoch
            && registered.owner_generation == origin.declarative_generation()
            && registered.origin.is_live()
            && registered
                .transaction
                .as_ref()
                .is_none_or(LatestTimerTransaction::is_active)
            && registered.lifecycle.admits_effect(
                &self.owner,
                wake.id,
                registered.lifecycle.identity(),
                wake.generation,
                true,
            );
        current_after_mapping.then_some(MappedTimerMessage { message, origin })
    }

    pub(super) fn retire_origin(&mut self, origin: &EffectOrigin) {
        let current_ids = self
            .registry
            .iter()
            .filter(|(_, registered)| registered.origin.eq(origin))
            .map(|(id, _)| *id)
            .collect::<Vec<_>>();
        for id in current_ids {
            let Some(registered) = self.registry.remove(&id) else {
                continue;
            };
            if let Some(slot) = registered
                .transaction
                .as_ref()
                .map(LatestTimerTransaction::slot)
                && self.latest.get(&slot).copied() == Some(id)
            {
                self.latest.remove(&slot);
            }
            drop(registered);
        }
    }

    pub(super) fn retire_auxiliary_owner(&mut self, owner: &AuxiliaryWindowOwner) {
        owner.retire();
        let origin = EffectOrigin::Auxiliary(owner.clone());
        self.retire_origin(&origin);
    }

    #[cfg(test)]
    pub(super) fn registered_origin(&self, id: u64) -> Option<EffectOrigin> {
        self.registry
            .get(&id)
            .map(|registered| registered.origin.clone())
    }

    #[cfg(test)]
    pub(super) fn contains_registration(&self, id: u64) -> bool {
        self.registry.contains_key(&id)
    }

    #[cfg(test)]
    pub(super) fn is_empty(&self) -> bool {
        self.registry.is_empty()
    }

    pub(super) fn shutdown(&mut self) {
        self.epoch = self.epoch.saturating_add(1);
        self.registry.clear();
        self.latest.clear();
    }
}

fn combine_cancellation_probes(
    first: Option<super::owner::CancellationProbe>,
    second: Option<super::owner::CancellationProbe>,
) -> Option<super::owner::CancellationProbe> {
    match (first, second) {
        (None, None) => None,
        (Some(probe), None) | (None, Some(probe)) => Some(probe),
        (Some(first), Some(second)) => Some(Arc::new(move || first() || second())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        application::{IntoView, column, text},
        gui::types::Vector2,
        runtime::SurfaceRuntime,
    };
    use std::{
        cell::Cell,
        rc::Rc,
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
    };

    fn map_message(effects: &mut TimerEffects<usize>, wake: RuntimeTimerWake) -> Option<usize> {
        effects.map_wake(wake).map(|mapped| mapped.message)
    }

    fn declarative_origins() -> (EffectOrigin, EffectOrigin, EffectOrigin) {
        let phase = Rc::new(Cell::new(0_u8));
        let project_phase = Rc::clone(&phase);
        let mut runtime = SurfaceRuntime::new_declarative_owned(
            (),
            Vector2::new(80.0, 40.0),
            move |_| {
                if project_phase.get() == 1 {
                    text::<usize>("raw").into_surface()
                } else {
                    column([text::<usize>("old").key("old")]).into_surface()
                }
            },
            |_, _| {},
        );
        let old = runtime
            .declarative_owner_ledger()
            .live_records()
            .first()
            .expect("old declarative owner")
            .token
            .clone();
        let sibling_runtime = SurfaceRuntime::new_declarative_owned(
            (),
            Vector2::new(80.0, 40.0),
            |_| column([text::<usize>("sibling").key("sibling")]).into_surface(),
            |_, _| {},
        );
        let sibling = sibling_runtime
            .declarative_owner_ledger()
            .live_records()
            .first()
            .expect("sibling declarative owner")
            .token
            .clone();
        phase.set(1);
        runtime.refresh();
        phase.set(2);
        runtime.refresh();
        let new = runtime
            .declarative_owner_ledger()
            .live_records()
            .first()
            .expect("later declarative owner generation")
            .token
            .clone();
        (
            EffectOrigin::Declarative(old),
            EffectOrigin::Declarative(sibling),
            EffectOrigin::Declarative(new),
        )
    }

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
                owner: None,
                lifecycle: None,
                map: Box::new(move || {
                    first_calls.fetch_add(1, Ordering::SeqCst);
                    1
                }),
            },
            EffectOrigin::Application,
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
                owner: None,
                lifecycle: None,
                map: Box::new(move || {
                    second_calls.fetch_add(1, Ordering::SeqCst);
                    2
                }),
            },
            EffectOrigin::Application,
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
        assert_eq!(map_message(&mut effects, current_wake.unwrap()), Some(2));
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
                owner: None,
                lifecycle: None,
                map: Box::new(|| 1),
            },
            EffectOrigin::Application,
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
                owner: None,
                lifecycle: None,
                map: Box::new(|| 2),
            },
            EffectOrigin::Application,
            |_, _| false,
        ));
        assert_eq!(latest.active(), Some(first_ticket));
        assert!(effects.latest.contains_key(&slot));
        assert_eq!(map_message(&mut effects, first_wake.unwrap()), Some(1));
    }

    #[test]
    fn shutdown_ignores_late_wakes() {
        let mut effects = TimerEffects::default();
        let mut wake = None;
        assert!(effects.schedule(
            TimerEffect {
                delay: Duration::ZERO,
                transaction: None,
                owner: None,
                lifecycle: None,
                map: Box::new(|| 1),
            },
            EffectOrigin::Application,
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
                    owner: None,
                    lifecycle: None,
                    map: Box::new(move || value),
                },
                EffectOrigin::Application,
                |_, wake| {
                    wakes.push(wake);
                    true
                },
            ));
        }
        let mapped = wakes
            .into_iter()
            .map(|wake| map_message(&mut effects, wake).expect("scheduled wake"))
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
                    owner: None,
                    lifecycle: None,
                    map: Box::new(move || {
                        let _sentinel = sentinel;
                        1
                    }),
                },
                EffectOrigin::Application,
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
            assert!(map_message(&mut effects, wake.unwrap()).is_none());
            assert_eq!(drops.load(Ordering::SeqCst), 1);
        }
    }

    #[test]
    fn auxiliary_origin_is_returned_with_ui_mapped_timer_message() {
        let owner = AuxiliaryWindowOwner::new("settings");
        let origin = EffectOrigin::Auxiliary(owner.clone());
        let mut effects = TimerEffects::default();
        let mut wake = None;
        assert!(effects.schedule(
            TimerEffect {
                delay: Duration::ZERO,
                transaction: None,
                owner: None,
                lifecycle: None,
                map: Box::new(|| 7),
            },
            origin.clone(),
            |_, timer_wake| {
                wake = Some(timer_wake);
                true
            },
        ));

        let mapped = effects.map_wake(wake.unwrap()).expect("scheduled wake");
        assert_eq!(mapped.message, 7);
        assert!(mapped.origin == origin);
    }

    #[test]
    fn declarative_origin_is_returned_and_retired_before_timer_mapper() {
        let mut owner_runtime = SurfaceRuntime::new_declarative_owned(
            (),
            Vector2::new(80.0, 40.0),
            |_| column([text::<()>("keyed").key("keyed")]).into_surface(),
            |_, _| {},
        );
        let token = owner_runtime
            .declarative_owner_ledger()
            .live_records()
            .first()
            .expect("keyed declarative owner")
            .token
            .clone();
        let origin = EffectOrigin::Declarative(token.clone());
        let mut effects = TimerEffects::default();
        let mut wake = None;
        assert!(effects.schedule(
            TimerEffect {
                delay: Duration::ZERO,
                transaction: None,
                owner: None,
                lifecycle: None,
                map: Box::new(|| 7),
            },
            origin.clone(),
            |_, timer_wake| {
                wake = Some(timer_wake);
                true
            },
        ));
        let mapped = effects.map_wake(wake.unwrap()).expect("live timer wake");
        assert_eq!(mapped.message, 7);
        assert!(mapped.origin == origin);

        let calls = Arc::new(AtomicUsize::new(0));
        let calls_for_mapper = Arc::clone(&calls);
        let mut late_wake = None;
        assert!(effects.schedule(
            TimerEffect {
                delay: Duration::ZERO,
                transaction: None,
                owner: None,
                lifecycle: None,
                map: Box::new(move || {
                    calls_for_mapper.fetch_add(1, Ordering::SeqCst);
                    8
                }),
            },
            origin,
            |_, timer_wake| {
                late_wake = Some(timer_wake);
                true
            },
        ));
        assert!(owner_runtime.begin_closing());
        assert!(!token.is_live());
        assert!(map_message(&mut effects, late_wake.unwrap()).is_none());
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn auxiliary_retirement_drops_mapper_repairs_latest_and_is_idempotent() {
        let drops = Arc::new(AtomicUsize::new(0));
        let owner = AuxiliaryWindowOwner::new("settings");
        let sibling = AuxiliaryWindowOwner::new("inspector");
        let mut effects = TimerEffects::default();
        let mut latest = crate::application::LatestTask::new();
        let transaction = latest.begin_timer_replacement();
        let slot = transaction.slot();
        let mut retired_wake = None;
        let retired_sentinel = DropSentinel(Arc::clone(&drops));
        assert!(effects.schedule(
            TimerEffect {
                delay: Duration::ZERO,
                transaction: Some(transaction),
                owner: None,
                lifecycle: None,
                map: Box::new(move || {
                    let _sentinel = retired_sentinel;
                    1
                }),
            },
            EffectOrigin::Auxiliary(owner.clone()),
            |_, timer_wake| {
                retired_wake = Some(timer_wake);
                true
            },
        ));
        let mut sibling_wake = None;
        assert!(effects.schedule(
            TimerEffect {
                delay: Duration::ZERO,
                transaction: None,
                owner: None,
                lifecycle: None,
                map: Box::new(|| 2),
            },
            EffectOrigin::Auxiliary(sibling.clone()),
            |_, timer_wake| {
                sibling_wake = Some(timer_wake);
                true
            },
        ));
        let mut application_wake = None;
        assert!(effects.schedule(
            TimerEffect {
                delay: Duration::ZERO,
                transaction: None,
                owner: None,
                lifecycle: None,
                map: Box::new(|| 3),
            },
            EffectOrigin::Application,
            |_, timer_wake| {
                application_wake = Some(timer_wake);
                true
            },
        ));

        effects.retire_auxiliary_owner(&owner);

        assert_eq!(drops.load(Ordering::SeqCst), 1);
        assert!(!effects.latest.contains_key(&slot));
        assert!(map_message(&mut effects, retired_wake.unwrap()).is_none());
        assert_eq!(map_message(&mut effects, sibling_wake.unwrap()), Some(2));
        assert_eq!(
            map_message(&mut effects, application_wake.unwrap()),
            Some(3)
        );

        effects.retire_auxiliary_owner(&owner);
        assert_eq!(drops.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn declarative_retirement_drops_mapper_repairs_latest_and_isolates_origins() {
        let (old_origin, sibling_origin, new_origin) = declarative_origins();
        let drops = Arc::new(AtomicUsize::new(0));
        let mut effects = TimerEffects::default();
        let mut latest = crate::application::LatestTask::new();
        let transaction = latest.begin_timer_replacement();
        let slot = transaction.slot();
        let retired_sentinel = DropSentinel(Arc::clone(&drops));
        let mut old_wake = None;
        assert!(effects.schedule(
            TimerEffect {
                delay: Duration::ZERO,
                transaction: Some(transaction),
                owner: None,
                lifecycle: None,
                map: Box::new(move || {
                    let _sentinel = retired_sentinel;
                    1
                }),
            },
            old_origin.clone(),
            |_, wake| {
                old_wake = Some(wake);
                true
            },
        ));
        let mut sibling_wake = None;
        assert!(effects.schedule(
            TimerEffect {
                delay: Duration::ZERO,
                transaction: None,
                owner: None,
                lifecycle: None,
                map: Box::new(|| 2),
            },
            sibling_origin,
            |_, wake| {
                sibling_wake = Some(wake);
                true
            },
        ));
        let mut application_wake = None;
        assert!(effects.schedule(
            TimerEffect {
                delay: Duration::ZERO,
                transaction: None,
                owner: None,
                lifecycle: None,
                map: Box::new(|| 3),
            },
            EffectOrigin::Application,
            |_, wake| {
                application_wake = Some(wake);
                true
            },
        ));
        let mut new_wake = None;
        assert!(effects.schedule(
            TimerEffect {
                delay: Duration::ZERO,
                transaction: None,
                owner: None,
                lifecycle: None,
                map: Box::new(|| 4),
            },
            new_origin,
            |_, wake| {
                new_wake = Some(wake);
                true
            },
        ));

        effects.retire_origin(&old_origin);

        assert_eq!(drops.load(Ordering::SeqCst), 1);
        assert!(!effects.latest.contains_key(&slot));
        assert!(map_message(&mut effects, old_wake.unwrap()).is_none());
        assert_eq!(map_message(&mut effects, sibling_wake.unwrap()), Some(2));
        assert_eq!(
            map_message(&mut effects, application_wake.unwrap()),
            Some(3)
        );
        assert_eq!(map_message(&mut effects, new_wake.unwrap()), Some(4));

        effects.retire_origin(&old_origin);
        assert_eq!(drops.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn stale_same_key_retirement_does_not_remove_new_generation() {
        let old_owner = AuxiliaryWindowOwner::new("settings");
        let new_owner = AuxiliaryWindowOwner::new("settings");
        let mut effects = TimerEffects::default();
        let mut old_wake = None;
        assert!(effects.schedule(
            TimerEffect {
                delay: Duration::ZERO,
                transaction: None,
                owner: None,
                lifecycle: None,
                map: Box::new(|| 1),
            },
            EffectOrigin::Auxiliary(old_owner.clone()),
            |_, timer_wake| {
                old_wake = Some(timer_wake);
                true
            },
        ));
        effects.retire_auxiliary_owner(&old_owner);

        let mut new_wake = None;
        assert!(effects.schedule(
            TimerEffect {
                delay: Duration::ZERO,
                transaction: None,
                owner: None,
                lifecycle: None,
                map: Box::new(|| 2),
            },
            EffectOrigin::Auxiliary(new_owner.clone()),
            |_, timer_wake| {
                new_wake = Some(timer_wake);
                true
            },
        ));
        effects.retire_auxiliary_owner(&old_owner);

        assert!(map_message(&mut effects, old_wake.unwrap()).is_none());
        assert_eq!(map_message(&mut effects, new_wake.unwrap()), Some(2));
    }

    #[test]
    fn latest_slot_mismatch_and_wrong_runtime_owner_never_invoke_mapper() {
        let calls = Arc::new(AtomicUsize::new(0));
        let mut effects = TimerEffects::default();
        let mut latest = crate::application::LatestTask::new();
        let transaction = latest.begin_timer_replacement();
        let slot = transaction.slot();
        let calls_for_mapper = Arc::clone(&calls);
        let mut wake = None;
        assert!(effects.schedule(
            TimerEffect {
                delay: Duration::ZERO,
                transaction: Some(transaction),
                owner: None,
                lifecycle: None,
                map: Box::new(move || {
                    calls_for_mapper.fetch_add(1, Ordering::SeqCst);
                    1
                }),
            },
            EffectOrigin::Application,
            |_, timer_wake| {
                wake = Some(timer_wake);
                true
            },
        ));
        let wake = wake.unwrap();
        let wrong_owner = RuntimeTimerWake::application(wake.id, wake.generation, wake.epoch);
        assert!(map_message(&mut effects, wrong_owner).is_none());
        effects.latest.remove(&slot);
        assert!(map_message(&mut effects, wake).is_none());
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert!(effects.registry.is_empty());
    }
}

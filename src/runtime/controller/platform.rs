use crate::runtime::{PlatformCompletion, PlatformCompletionIdentity, PlatformResultDelivery};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, Weak};

use super::SurfaceRuntime;
use super::owner::{LifecycleDescriptor, RuntimeOwner};
use crate::runtime::RuntimeBridge;

pub(super) struct PlatformCompletionRegistry<Message> {
    owner: RuntimeOwner,
    entries: HashMap<PlatformCompletionIdentity, RegisteredPlatformCompletion<Message>>,
    next_id: u64,
    epoch: u64,
}

impl<Message> Default for PlatformCompletionRegistry<Message> {
    fn default() -> Self {
        Self::new(RuntimeOwner::new())
    }
}

impl<Message> PlatformCompletionRegistry<Message> {
    pub(super) fn new(owner: RuntimeOwner) -> Self {
        Self {
            owner,
            entries: HashMap::new(),
            next_id: 1,
            epoch: 1,
        }
    }
    pub(super) fn register(
        &mut self,
        completion: PlatformCompletion<Message>,
    ) -> PlatformCompletionIdentity {
        let identity = PlatformCompletionIdentity {
            id: self.next_id,
            epoch: self.epoch,
        };
        self.next_id = self.next_id.saturating_add(1);
        self.entries.insert(
            identity,
            RegisteredPlatformCompletion {
                completion,
                lifecycle: LifecycleDescriptor::new(
                    self.owner.clone(),
                    identity.id,
                    None,
                    identity.epoch,
                    None,
                ),
            },
        );
        identity
    }

    pub(super) fn map_delivery(&mut self, delivery: PlatformResultDelivery) -> Option<Message> {
        match delivery {
            PlatformResultDelivery::Completed { identity, result } => {
                let mapper = self.entries.get(&identity)?;
                if !mapper.lifecycle.admits(
                    &self.owner,
                    identity.id,
                    identity.epoch,
                    mapper.lifecycle.slot().is_none(),
                ) {
                    self.entries.remove(&identity);
                    return None;
                }
                let mapper = self.entries.remove(&identity)?;
                Some((mapper.completion)(result))
            }
            PlatformResultDelivery::Discarded { identity } => {
                self.entries.remove(&identity);
                None
            }
        }
    }

    pub(super) fn remove(
        &mut self,
        identity: PlatformCompletionIdentity,
    ) -> Option<PlatformCompletion<Message>> {
        self.entries.remove(&identity).map(|entry| entry.completion)
    }

    pub(super) fn clear(&mut self) {
        self.entries.clear();
        self.epoch = self.epoch.saturating_add(1);
    }
}

struct RegisteredPlatformCompletion<Message> {
    completion: PlatformCompletion<Message>,
    lifecycle: LifecycleDescriptor,
}

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(super) fn shutdown_platform_services(&mut self) {
        {
            let mut ingress = self
                .platform_results
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            ingress.close();
        }
        self.platform_registry.clear();
    }
}

#[derive(Default)]
pub(super) struct PlatformResultIngress {
    pending: Vec<PlatformResultDelivery>,
    overflow: Option<PlatformResultDelivery>,
    reservations: usize,
    closed: bool,
}

impl PlatformResultIngress {
    const CAPACITY: usize = 64;

    pub(super) fn reserve(ingress: &Arc<Mutex<Self>>) -> Option<PlatformResultReservation> {
        let mut state = ingress
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if state.closed || state.pending.len().saturating_add(state.reservations) >= Self::CAPACITY
        {
            return None;
        }
        state.reservations += 1;
        Some(PlatformResultReservation {
            ingress: Arc::downgrade(ingress),
            committed: false,
        })
    }

    #[cfg(test)]
    pub(super) fn take_pending(&mut self) -> Vec<PlatformResultDelivery> {
        self.take_frozen_pending_batch(self.pending_len(), usize::MAX)
            .0
    }

    pub(super) fn take_frozen_pending_batch(
        &mut self,
        frozen_count: usize,
        max_deliveries: usize,
    ) -> (Vec<PlatformResultDelivery>, bool) {
        let take_count = frozen_count.min(max_deliveries);
        if take_count == 0 {
            return (Vec::new(), frozen_count != 0);
        }
        let take = self.pending.len().min(take_count);
        let mut pending = self.pending.drain(..take).collect::<Vec<_>>();
        if pending.len() < take_count
            && let Some(delivery) = self.overflow.take()
        {
            pending.push(delivery);
        }
        // Keep an older overflow delivery ahead of arrivals committed while
        // the frozen prefix is being mapped. The remaining pending entries
        // are older than that overflow, so appending preserves global FIFO.
        if let Some(delivery) = self.overflow.take() {
            self.pending.push(delivery);
        }
        (pending, frozen_count > max_deliveries)
    }

    /// Snapshot the eligible prefix and remove its budgeted portion while the
    /// caller still holds the ingress lock. Later reservations therefore
    /// cannot enter the frozen turn ahead of an older overflow delivery.
    pub(super) fn take_budgeted_pending_batch(
        &mut self,
        max_deliveries: usize,
    ) -> (Vec<PlatformResultDelivery>, bool) {
        let frozen_count = self.pending_len();
        self.take_frozen_pending_batch(frozen_count, max_deliveries)
    }

    pub(super) fn pending_len(&self) -> usize {
        self.pending.len() + usize::from(self.overflow.is_some())
    }

    pub(super) fn close(&mut self) {
        self.closed = true;
        self.pending.clear();
        self.overflow = None;
        self.reservations = 0;
    }

    pub(super) fn enqueue_overflow(&mut self, delivery: PlatformResultDelivery) -> bool {
        if self.closed || self.overflow.is_some() {
            false
        } else {
            self.overflow = Some(delivery);
            true
        }
    }
}

pub(super) struct PlatformResultReservation {
    ingress: Weak<Mutex<PlatformResultIngress>>,
    committed: bool,
}

impl PlatformResultReservation {
    pub(super) fn commit(mut self, delivery: PlatformResultDelivery) -> bool {
        let Some(ingress) = self.ingress.upgrade() else {
            self.committed = true;
            return false;
        };
        let mut state = ingress
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if state.closed || state.reservations == 0 {
            self.committed = true;
            return false;
        }
        state.reservations -= 1;
        state.pending.push(delivery);
        self.committed = true;
        true
    }
}

impl Drop for PlatformResultReservation {
    fn drop(&mut self) {
        if self.committed {
            return;
        }
        if let Some(ingress) = self.ingress.upgrade() {
            let mut state = ingress
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            state.reservations = state.reservations.saturating_sub(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::PlatformResponse;
    use std::{
        cell::RefCell,
        rc::Rc,
        sync::{Arc, Mutex},
    };

    #[test]
    fn mapper_runs_once_and_duplicate_delivery_is_ignored() {
        let calls = Rc::new(RefCell::new(0));
        let marker = Rc::clone(&calls);
        let mut registry = PlatformCompletionRegistry::<usize>::default();
        let identity = registry.register(Box::new(move |_| {
            *marker.borrow_mut() += 1;
            1
        }));
        let delivery = || PlatformResultDelivery::Completed {
            identity,
            result: Ok(PlatformResponse::Completed),
        };
        assert_eq!(registry.map_delivery(delivery()), Some(1));
        assert_eq!(registry.map_delivery(delivery()), None);
        assert_eq!(*calls.borrow(), 1);
    }

    #[test]
    fn clear_fences_stale_delivery_and_releases_mapper() {
        let marker = Rc::new(());
        let captured = Rc::clone(&marker);
        let mut registry = PlatformCompletionRegistry::<()>::default();
        let identity = registry.register(Box::new(move |_| {
            let _ = &captured;
        }));
        assert_eq!(Rc::strong_count(&marker), 2);
        registry.clear();
        assert_eq!(Rc::strong_count(&marker), 1);
        assert!(
            registry
                .map_delivery(PlatformResultDelivery::Completed {
                    identity,
                    result: Ok(PlatformResponse::Completed),
                })
                .is_none()
        );
    }

    #[test]
    fn saturated_overflow_releases_rejected_mapper() {
        let marker = Rc::new(());
        let mut registry = PlatformCompletionRegistry::<()>::default();
        let first_marker = Rc::clone(&marker);
        let first = registry.register(Box::new(move |_| {
            let _ = &first_marker;
        }));
        let second_marker = Rc::clone(&marker);
        let second = registry.register(Box::new(move |_| {
            let _ = &second_marker;
        }));
        assert_eq!(Rc::strong_count(&marker), 3);

        let ingress = Arc::new(Mutex::new(PlatformResultIngress::default()));
        let mut ingress_state = ingress.lock().expect("ingress lock");
        assert!(
            ingress_state.enqueue_overflow(PlatformResultDelivery::Completed {
                identity: first,
                result: Ok(PlatformResponse::Completed),
            })
        );
        assert!(
            !ingress_state.enqueue_overflow(PlatformResultDelivery::Completed {
                identity: second,
                result: Ok(PlatformResponse::Completed),
            })
        );
        drop(ingress_state);
        let _ = registry.remove(second);
        assert_eq!(Rc::strong_count(&marker), 2);
        let delivery = ingress
            .lock()
            .expect("ingress lock")
            .take_pending()
            .pop()
            .expect("bounded overflow delivery");
        assert!(registry.map_delivery(delivery).is_some());
        assert_eq!(Rc::strong_count(&marker), 1);
    }

    #[test]
    fn frozen_batch_excludes_arrivals_after_turn_snapshot() {
        let identity = PlatformCompletionIdentity { id: 1, epoch: 1 };
        let delivery = || PlatformResultDelivery::Completed {
            identity,
            result: Ok(PlatformResponse::Completed),
        };
        let mut ingress = PlatformResultIngress::default();
        ingress.pending.push(delivery());
        ingress.pending.push(delivery());
        let frozen_count = ingress.pending_len();
        ingress.pending.push(delivery());

        let (batch, frozen_remainder) = ingress.take_frozen_pending_batch(frozen_count, 64);

        assert_eq!(batch.len(), 2);
        assert!(!frozen_remainder);
        assert_eq!(ingress.pending_len(), 1);
    }

    #[test]
    fn atomic_frozen_batch_keeps_late_reservation_behind_older_overflow() {
        let ingress = Arc::new(Mutex::new(PlatformResultIngress::default()));
        for id in 0..63 {
            let reservation = PlatformResultIngress::reserve(&ingress).expect("old reservation");
            assert!(reservation.commit(PlatformResultDelivery::Completed {
                identity: PlatformCompletionIdentity { id, epoch: 1 },
                result: Ok(PlatformResponse::Completed),
            }));
        }
        let late_reservation =
            PlatformResultIngress::reserve(&ingress).expect("outstanding late reservation");
        {
            let mut state = ingress.lock().expect("ingress lock");
            assert!(state.enqueue_overflow(PlatformResultDelivery::Completed {
                identity: PlatformCompletionIdentity { id: 100, epoch: 1 },
                result: Ok(PlatformResponse::Completed),
            }));
            let (frozen, frozen_remainder) = state.take_budgeted_pending_batch(8);
            assert!(frozen_remainder);
            assert_eq!(frozen.len(), 8);
            assert!(frozen.iter().all(|delivery| match delivery {
                PlatformResultDelivery::Completed { identity, .. } => identity.id < 8,
                PlatformResultDelivery::Discarded { .. } => false,
            }));
        }
        assert!(late_reservation.commit(PlatformResultDelivery::Completed {
            identity: PlatformCompletionIdentity { id: 101, epoch: 1 },
            result: Ok(PlatformResponse::Completed),
        }));

        let remainder = ingress.lock().expect("ingress lock").take_pending();
        let ids = remainder
            .into_iter()
            .map(|delivery| match delivery {
                PlatformResultDelivery::Completed { identity, .. } => identity.id,
                PlatformResultDelivery::Discarded { identity } => identity.id,
            })
            .collect::<Vec<_>>();
        assert_eq!(ids.first(), Some(&8));
        assert_eq!(ids.last(), Some(&101));
    }
}

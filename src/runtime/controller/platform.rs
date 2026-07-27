use crate::runtime::{PlatformCompletion, PlatformCompletionIdentity, PlatformResultDelivery};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, Weak};

pub(super) struct PlatformCompletionRegistry<Message> {
    entries: HashMap<PlatformCompletionIdentity, PlatformCompletion<Message>>,
    next_id: u64,
    epoch: u64,
}

impl<Message> Default for PlatformCompletionRegistry<Message> {
    fn default() -> Self {
        Self {
            entries: HashMap::new(),
            next_id: 1,
            epoch: 1,
        }
    }
}

impl<Message> PlatformCompletionRegistry<Message> {
    pub(super) fn register(
        &mut self,
        completion: PlatformCompletion<Message>,
    ) -> PlatformCompletionIdentity {
        let identity = PlatformCompletionIdentity {
            id: self.next_id,
            epoch: self.epoch,
        };
        self.next_id = self.next_id.saturating_add(1);
        self.entries.insert(identity, completion);
        identity
    }

    pub(super) fn map_delivery(&mut self, delivery: PlatformResultDelivery) -> Option<Message> {
        match delivery {
            PlatformResultDelivery::Completed { identity, result } => {
                let mapper = self.entries.remove(&identity)?;
                Some(mapper(result))
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
        self.entries.remove(&identity)
    }

    pub(super) fn clear(&mut self) {
        self.entries.clear();
        self.epoch = self.epoch.saturating_add(1);
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

    pub(super) fn take_pending(&mut self) -> Vec<PlatformResultDelivery> {
        let mut pending = std::mem::take(&mut self.pending);
        if let Some(delivery) = self.overflow.take() {
            pending.push(delivery);
        }
        pending
    }

    pub(super) fn close(&mut self) {
        self.closed = true;
        self.pending.clear();
        self.overflow = None;
        self.reservations = 0;
    }

    pub(super) fn enqueue_overflow(&mut self, delivery: PlatformResultDelivery) -> bool {
        if self.overflow.is_none() {
            self.overflow = Some(delivery);
            true
        } else {
            false
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
}

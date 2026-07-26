use std::{any::Any, collections::HashMap};

/// Opaque identity for one worker subscription registration.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct WorkerSubscriptionIdentity {
    pub(crate) id: u64,
    pub(crate) epoch: u64,
}

/// Worker delivery transported from a subscription thread to the UI queue.
pub(crate) enum WorkerSubscriptionDelivery {
    Payload {
        identity: WorkerSubscriptionIdentity,
        payload: Box<dyn Any + Send>,
    },
    Disconnected {
        identity: WorkerSubscriptionIdentity,
    },
}

struct Registration<Message> {
    map: Box<dyn Fn(Box<dyn Any + Send>) -> Option<Message> + 'static>,
}

pub(crate) struct WorkerSubscriptionRegistry<Message> {
    entries: HashMap<WorkerSubscriptionIdentity, Registration<Message>>,
    next_id: u64,
    epoch: u64,
}

impl<Message> Default for WorkerSubscriptionRegistry<Message> {
    fn default() -> Self {
        Self {
            entries: HashMap::new(),
            next_id: 1,
            epoch: 1,
        }
    }
}

impl<Message> WorkerSubscriptionRegistry<Message> {
    pub(crate) fn register(
        &mut self,
        map: Box<dyn Fn(Box<dyn Any + Send>) -> Option<Message> + 'static>,
    ) -> WorkerSubscriptionIdentity {
        let identity = WorkerSubscriptionIdentity {
            id: self.next_id,
            epoch: self.epoch,
        };
        self.next_id = self.next_id.saturating_add(1);
        self.entries.insert(identity, Registration { map });
        identity
    }

    pub(crate) fn remove(&mut self, identity: WorkerSubscriptionIdentity) {
        self.entries.remove(&identity);
    }

    pub(crate) fn map(
        &mut self,
        identity: WorkerSubscriptionIdentity,
        payload: Box<dyn Any + Send>,
    ) -> Option<Message> {
        let entry = self.entries.get(&identity)?;
        (entry.map)(payload)
    }

    pub(crate) fn disconnect(&mut self, identity: WorkerSubscriptionIdentity) {
        self.entries.remove(&identity);
    }

    pub(crate) fn map_delivery(&mut self, delivery: WorkerSubscriptionDelivery) -> Option<Message> {
        match delivery {
            WorkerSubscriptionDelivery::Payload { identity, payload } => {
                self.map(identity, payload)
            }
            WorkerSubscriptionDelivery::Disconnected { identity } => {
                self.disconnect(identity);
                None
            }
        }
    }

    pub(crate) fn clear(&mut self) {
        self.entries.clear();
        self.epoch = self.epoch.saturating_add(1);
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
    fn disconnect_is_ordered_after_payload_mapping() {
        let mut registry = WorkerSubscriptionRegistry::default();
        let drops = Arc::new(AtomicUsize::new(0));
        let drops_mapper = Arc::clone(&drops);
        let identity = registry.register(Box::new(move |payload| {
            drops_mapper.fetch_add(1, Ordering::AcqRel);
            Some(*payload.downcast::<u32>().expect("u32 payload"))
        }));

        assert_eq!(
            registry.map_delivery(WorkerSubscriptionDelivery::Payload {
                identity,
                payload: Box::new(7_u32),
            }),
            Some(7)
        );
        assert_eq!(drops.load(Ordering::Acquire), 1);
        assert_eq!(
            registry.map_delivery(WorkerSubscriptionDelivery::Disconnected { identity }),
            None
        );
        assert_eq!(registry.entries.len(), 0);
    }

    #[test]
    fn stale_identity_does_not_invoke_mapper() {
        let mut registry = WorkerSubscriptionRegistry::default();
        let invoked = Arc::new(AtomicUsize::new(0));
        let invoked_mapper = Arc::clone(&invoked);
        let identity = registry.register(Box::new(move |payload| {
            invoked_mapper.fetch_add(1, Ordering::AcqRel);
            Some(*payload.downcast::<u32>().expect("u32 payload"))
        }));
        registry.clear();

        assert_eq!(registry.map(identity, Box::new(9_u32)), None);
        assert_eq!(invoked.load(Ordering::Acquire), 0);
    }

    #[test]
    fn unknown_identity_does_not_invoke_mapper() {
        let mut registry = WorkerSubscriptionRegistry::default();
        let invoked = Arc::new(AtomicUsize::new(0));
        let invoked_mapper = Arc::clone(&invoked);
        registry.register(Box::new(move |payload| {
            invoked_mapper.fetch_add(1, Ordering::AcqRel);
            Some(*payload.downcast::<u32>().expect("u32 payload"))
        }));

        assert_eq!(
            registry.map(
                WorkerSubscriptionIdentity { id: 999, epoch: 1 },
                Box::new(9_u32),
            ),
            None
        );
        assert_eq!(invoked.load(Ordering::Acquire), 0);
    }
}

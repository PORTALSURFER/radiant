use crate::runtime::{PlatformCompletion, PlatformResult};
use std::collections::HashMap;

/// Opaque identity for one UI-owned platform completion mapper.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct PlatformCompletionIdentity {
    pub(crate) id: u64,
    pub(crate) epoch: u64,
}

/// Owned platform result transported to the UI queue.
pub(crate) struct PlatformCompletionDelivery {
    pub(crate) identity: PlatformCompletionIdentity,
    pub(crate) result: PlatformResult,
}

pub(crate) struct PlatformCompletionRegistry<Message> {
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
    pub(crate) fn register(
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

    pub(crate) fn remove(
        &mut self,
        identity: PlatformCompletionIdentity,
    ) -> Option<PlatformCompletion<Message>> {
        self.entries.remove(&identity)
    }

    pub(crate) fn map_delivery(&mut self, delivery: PlatformCompletionDelivery) -> Option<Message> {
        let mapper = self.entries.remove(&delivery.identity)?;
        Some(mapper(delivery.result))
    }

    pub(crate) fn clear(&mut self) {
        self.entries.clear();
        self.epoch = self.epoch.saturating_add(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::PlatformResponse;
    use std::{
        cell::RefCell,
        rc::Rc,
        thread::{self, ThreadId},
    };

    #[test]
    fn completion_mapper_runs_once_on_the_ui_owner() {
        #[derive(Clone)]
        struct UiOnlyMessage(Rc<RefCell<Vec<ThreadId>>>);

        let ui_thread = thread::current().id();
        let calls = Rc::new(RefCell::new(Vec::new()));
        let mapper_calls = Rc::clone(&calls);
        let mut registry = PlatformCompletionRegistry::default();
        let identity = registry.register(Box::new(move |_| {
            mapper_calls.borrow_mut().push(thread::current().id());
            UiOnlyMessage(Rc::clone(&mapper_calls))
        }));

        let message = registry
            .map_delivery(PlatformCompletionDelivery {
                identity,
                result: Ok(PlatformResponse::Completed),
            })
            .expect("registered completion should map");
        assert!(Rc::ptr_eq(&message.0, &calls));
        assert_eq!(calls.borrow().as_slice(), &[ui_thread]);
        assert!(
            registry
                .map_delivery(PlatformCompletionDelivery {
                    identity,
                    result: Ok(PlatformResponse::Completed),
                })
                .is_none()
        );
    }

    #[test]
    fn cleared_generation_rejects_late_completion_and_drops_mapper() {
        let marker = Rc::new(());
        let mapper_marker = Rc::clone(&marker);
        let mut registry = PlatformCompletionRegistry::<()>::default();
        let identity = registry.register(Box::new(move |_| {
            let _ = &mapper_marker;
        }));
        assert_eq!(Rc::strong_count(&marker), 2);

        registry.clear();

        assert_eq!(Rc::strong_count(&marker), 1);
        assert!(
            registry
                .map_delivery(PlatformCompletionDelivery {
                    identity,
                    result: Ok(PlatformResponse::Completed),
                })
                .is_none()
        );
    }
}

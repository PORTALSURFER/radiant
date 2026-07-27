use super::queue::{TimerIdentity, TimerWake};
use crate::application::runtime::queue::SharedRuntimeIngress;
use std::{collections::HashMap, sync::Arc, time::Duration};

pub(crate) struct TimerRegistry<Message> {
    entries: HashMap<TimerIdentity, TimerRegistration<Message>>,
}

enum TimerRegistration<Message> {
    Interval(Arc<dyn Fn() -> Message + 'static>),
}

impl<Message> Default for TimerRegistry<Message> {
    fn default() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }
}

impl<Message> TimerRegistry<Message> {
    pub(crate) fn schedule_interval(
        &mut self,
        runtime: &Arc<SharedRuntimeIngress>,
        every: Duration,
        message: Arc<dyn Fn() -> Message + 'static>,
    ) -> bool {
        let identity = runtime.allocate_timer_identity(0);
        self.entries
            .insert(identity, TimerRegistration::Interval(message));
        if runtime.schedule_timer(every, identity, true) {
            true
        } else {
            self.entries.remove(&identity);
            false
        }
    }

    pub(crate) fn map_wake(&mut self, wake: TimerWake) -> Option<Message> {
        if wake.owner != crate::runtime::RuntimeTimerOwner::Application {
            return None;
        }
        let entry = self.entries.get_mut(&wake)?;
        let TimerRegistration::Interval(mapper) = entry;
        Some(mapper())
    }

    pub(crate) fn clear(&mut self) {
        self.entries.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::RuntimeTimerWake;

    #[test]
    fn application_and_controller_wakes_with_equal_ids_route_to_their_owner() {
        let mut registry = TimerRegistry::default();
        let application = RuntimeTimerWake::application(1, 0, 1);
        let controller = RuntimeTimerWake::controller(1, 0, 1);
        registry
            .entries
            .insert(application, TimerRegistration::Interval(Arc::new(|| 7_u32)));
        assert_eq!(registry.map_wake(application), Some(7));
        assert_eq!(registry.map_wake(controller), None);
    }
}

use super::RuntimeUserEvent;
use crate::gui::repaint::{CoalescingRepaintSignal, RepaintSignal, try_mark_repaint_pending};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use winit::event_loop::EventLoopProxy;

pub(super) struct RuntimeWakeup {
    pending: Arc<AtomicBool>,
    proxy: Option<EventLoopProxy<RuntimeUserEvent>>,
}

impl Default for RuntimeWakeup {
    fn default() -> Self {
        Self {
            pending: Arc::new(AtomicBool::new(false)),
            proxy: None,
        }
    }
}

impl RuntimeWakeup {
    pub(super) fn install_proxy(
        &mut self,
        proxy: EventLoopProxy<RuntimeUserEvent>,
    ) -> Arc<dyn RepaintSignal> {
        self.proxy = Some(proxy.clone());
        Arc::new(CoalescingRepaintSignal::new(
            Arc::clone(&self.pending),
            move || proxy.send_event(RuntimeUserEvent::RepaintRequested).is_ok(),
        ))
    }

    pub(super) fn clear_pending(&self) {
        self.pending.store(false, Ordering::Release);
    }

    pub(super) fn request_if(&self, should_request: bool) {
        if !should_request || !try_mark_repaint_pending(self.pending.as_ref()) {
            return;
        }
        let Some(proxy) = self.proxy.as_ref() else {
            self.clear_pending();
            return;
        };
        if proxy
            .send_event(RuntimeUserEvent::RepaintRequested)
            .is_err()
        {
            self.clear_pending();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_without_installed_proxy_does_not_leave_wakeup_pending() {
        let wakeup = RuntimeWakeup::default();

        wakeup.request_if(true);

        assert!(!wakeup.pending.load(Ordering::Acquire));
    }

    #[test]
    fn skipped_request_does_not_mark_wakeup_pending() {
        let wakeup = RuntimeWakeup::default();

        wakeup.request_if(false);

        assert!(!wakeup.pending.load(Ordering::Acquire));
    }
}

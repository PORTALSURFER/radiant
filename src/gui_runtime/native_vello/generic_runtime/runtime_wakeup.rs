use super::RuntimeUserEvent;
use crate::gui::repaint::{CoalescingRepaintSignal, RepaintSignal, try_mark_repaint_pending};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use winit::event_loop::EventLoopProxy;

pub(super) struct RuntimeWakeup {
    pending: Arc<AtomicBool>,
    summary_pending: Arc<AtomicBool>,
    custom_shader_pending: Arc<AtomicBool>,
    proxy: Option<EventLoopProxy<RuntimeUserEvent>>,
}

impl Default for RuntimeWakeup {
    fn default() -> Self {
        Self {
            pending: Arc::new(AtomicBool::new(false)),
            summary_pending: Arc::new(AtomicBool::new(false)),
            custom_shader_pending: Arc::new(AtomicBool::new(false)),
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

    pub(super) fn event_loop_proxy(&self) -> Option<EventLoopProxy<RuntimeUserEvent>> {
        self.proxy.clone()
    }

    /// Install the private wake used by the shared signal-summary broker.  It
    /// is deliberately distinct from a redraw wake: completing CPU work must
    /// first be reconciled against the current native target and paint plan.
    pub(super) fn install_summary_work_signal(&self) -> Arc<dyn RepaintSignal> {
        let pending = Arc::clone(&self.summary_pending);
        let proxy = self.proxy.clone();
        Arc::new(CoalescingRepaintSignal::new(pending, move || {
            proxy.as_ref().is_some_and(|proxy| {
                proxy
                    .send_event(RuntimeUserEvent::SignalSummaryWorkReady)
                    .is_ok()
            })
        }))
    }

    pub(super) fn clear_summary_work_pending(&self) {
        self.summary_pending.store(false, Ordering::Release);
    }

    pub(super) fn install_custom_shader_work_signal(&self) -> Arc<dyn RepaintSignal> {
        let pending = Arc::clone(&self.custom_shader_pending);
        let proxy = self.proxy.clone();
        Arc::new(CoalescingRepaintSignal::new(pending, move || {
            proxy.as_ref().is_some_and(|proxy| {
                proxy
                    .send_event(RuntimeUserEvent::CustomShaderWorkReady)
                    .is_ok()
            })
        }))
    }

    pub(super) fn clear_custom_shader_work_pending(&self) {
        self.custom_shader_pending.store(false, Ordering::Release);
    }

    pub(super) fn clear_pending(&self) {
        self.pending.store(false, Ordering::Release);
    }

    #[cfg(test)]
    pub(super) fn is_pending(&self) -> bool {
        self.pending.load(Ordering::Acquire)
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

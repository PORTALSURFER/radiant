use super::*;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};

#[path = "lifecycle/animation.rs"]
mod animation;
#[path = "lifecycle/startup_and_exit.rs"]
mod startup_and_exit;

struct CountingRepaintSignal {
    called: Arc<AtomicBool>,
}

impl RepaintSignal for CountingRepaintSignal {
    fn request_repaint(&self) {
        self.called.store(true, Ordering::Release);
    }
}

fn drain_until_messages<Bridge>(
    runtime: &mut SurfaceRuntime<Bridge, DemoMessage>,
    min_messages: usize,
) -> radiant::runtime::CommandOutcome
where
    Bridge: RuntimeBridge<DemoMessage>,
{
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut drained = radiant::runtime::CommandOutcome::default();
    loop {
        let outcome = runtime.drain_runtime_messages();
        drained.messages_dispatched += outcome.messages_dispatched;
        drained.repaint_requested |= outcome.repaint_requested;
        drained.surface_refresh_requested |= outcome.surface_refresh_requested;
        drained.exit_requested |= outcome.exit_requested;
        if drained.messages_dispatched >= min_messages || Instant::now() >= deadline {
            return drained;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
}

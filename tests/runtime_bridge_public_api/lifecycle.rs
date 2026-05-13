use super::*;
use radiant::gui::repaint::RepaintSignal;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

#[test]
fn runtime_bridge_accepts_repaint_signal_for_host_background_work() {
    let called = Arc::new(AtomicBool::new(false));
    let mut bridge = RepaintSignalBridge::default();

    bridge.install_repaint_signal(Arc::new(CountingRepaintSignal {
        called: Arc::clone(&called),
    }));
    bridge.request_worker_repaint();

    assert!(called.load(Ordering::Acquire));
}

#[test]
fn runtime_bridge_exposes_host_owned_runtime_exit_artifact() {
    let mut bridge = RuntimeExitBridge;

    assert_eq!(
        bridge.on_runtime_exit(),
        Some(serde_json::json!({
            "status": "clean",
            "phase": "host-owned"
        }))
    );
}

#[derive(Default)]
struct RepaintSignalBridge {
    signal: Option<Arc<dyn RepaintSignal>>,
}

impl RepaintSignalBridge {
    fn request_worker_repaint(&self) {
        if let Some(signal) = self.signal.as_ref() {
            signal.request_repaint();
        }
    }
}

impl RuntimeBridge<DemoMessage> for RepaintSignalBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<DemoMessage>> {
        project_surface(&mut DemoState::default())
    }

    fn install_repaint_signal(&mut self, signal: Arc<dyn RepaintSignal>) {
        self.signal = Some(signal);
    }
}

struct CountingRepaintSignal {
    called: Arc<AtomicBool>,
}

impl RepaintSignal for CountingRepaintSignal {
    fn request_repaint(&self) {
        self.called.store(true, Ordering::Release);
    }
}

struct RuntimeExitBridge;

impl RuntimeBridge<DemoMessage> for RuntimeExitBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<DemoMessage>> {
        project_surface(&mut DemoState::default())
    }

    fn on_runtime_exit(&mut self) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "status": "clean",
            "phase": "host-owned"
        }))
    }
}

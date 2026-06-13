use super::*;
use radiant::runtime::{UiUpdateHandlerDiagnosticsMode, UiUpdateHandlerDiagnosticsPolicy};
use std::{panic, time::Duration};

#[test]
fn update_handler_diagnostics_record_handler_and_message_identity() {
    let bridge = PaintOnlyBridge::default();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(180.0, 40.0));
    runtime.set_update_handler_diagnostics_policy(UiUpdateHandlerDiagnosticsPolicy::warn_at(
        Duration::ZERO,
    ));

    runtime.dispatch_message(DemoMessage::Increment);

    let diagnostics = runtime.runtime_diagnostics();
    let slow = diagnostics
        .ui
        .last_slow_update_handler
        .expect("zero threshold should record the handler deterministically");
    assert_eq!(diagnostics.ui.update_handlers, 1);
    assert_eq!(diagnostics.ui.slow_update_handlers, 1);
    assert_eq!(slow.threshold, Duration::ZERO);
    assert!(slow.handler.contains("PaintOnlyBridge"));
    assert!(slow.message.contains("DemoMessage"));
    assert!(slow.guidance.contains("context.business()"));
}

#[test]
fn strict_update_handler_diagnostics_panic_after_recording_slow_handler() {
    let bridge = PaintOnlyBridge::default();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(180.0, 40.0));
    runtime.set_update_handler_diagnostics_policy(UiUpdateHandlerDiagnosticsPolicy::panic_at(
        Duration::ZERO,
    ));

    let failure = panic::catch_unwind(panic::AssertUnwindSafe(|| {
        runtime.dispatch_message(DemoMessage::Increment);
    }))
    .expect_err("zero-threshold strict diagnostics should panic");

    let failure = panic_message(failure);
    assert!(failure.contains("PaintOnlyBridge"));
    assert!(failure.contains("DemoMessage"));
    assert!(failure.contains("context.business()"));

    let diagnostics = runtime.runtime_diagnostics();
    assert_eq!(diagnostics.ui.update_handlers, 1);
    assert_eq!(diagnostics.ui.slow_update_handlers, 1);
}

#[test]
fn disabled_update_handler_diagnostics_skip_timing_records() {
    let bridge = PaintOnlyBridge::default();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(180.0, 40.0));
    runtime.set_update_handler_diagnostics_policy(UiUpdateHandlerDiagnosticsPolicy::disabled());

    runtime.dispatch_message(DemoMessage::Increment);

    let diagnostics = runtime.runtime_diagnostics();
    assert_eq!(diagnostics.ui.update_handlers, 0);
    assert_eq!(diagnostics.ui.slow_update_handlers, 0);
    assert!(diagnostics.ui.last_slow_update_handler.is_none());
}

#[test]
fn business_worker_runtime_is_not_counted_as_slow_ui_handler() {
    let bridge = app(DemoState::default())
        .view(|state| {
            SurfaceNode::static_widget(TextWidget::new(
                10,
                format!("Count {}", state.count),
                WidgetSizing::fixed(Vector2::new(120.0, 20.0)).with_baseline(14.0),
            ))
        })
        .handle_message(|state, message, context| match message {
            DemoMessage::Increment => {
                state.count += 1;
                context.business().background("diagnostic-worker").run(
                    |_| {
                        std::thread::sleep(Duration::from_millis(25));
                        DemoMessage::Noop
                    },
                    |message| message,
                );
            }
            DemoMessage::Noop => {}
            DemoMessage::GpuInput(_) | DemoMessage::VirtualListWindowChanged(_) => {}
        })
        .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(180.0, 40.0));
    runtime.set_update_handler_diagnostics_policy(UiUpdateHandlerDiagnosticsPolicy::warn_at(
        Duration::from_secs(60),
    ));

    runtime.dispatch_message(DemoMessage::Increment);

    let diagnostics = runtime.runtime_diagnostics();
    assert_eq!(diagnostics.ui.update_handlers, 1);
    assert_eq!(diagnostics.ui.slow_update_handlers, 0);
    assert_eq!(diagnostics.business.queued, 1);
}

#[test]
fn update_handler_diagnostics_policy_exposes_mode_and_threshold() {
    let policy = UiUpdateHandlerDiagnosticsPolicy::panic_at(Duration::from_millis(7));

    assert_eq!(policy.threshold(), Some(Duration::from_millis(7)));
    assert_eq!(policy.mode(), UiUpdateHandlerDiagnosticsMode::Panic);
    assert_eq!(
        UiUpdateHandlerDiagnosticsPolicy::disabled().threshold(),
        None
    );
}

fn panic_message(payload: Box<dyn std::any::Any + Send>) -> String {
    match payload.downcast::<String>() {
        Ok(message) => *message,
        Err(payload) => match payload.downcast::<&'static str>() {
            Ok(message) => (*message).to_string(),
            Err(_) => String::from("<non-string panic>"),
        },
    }
}

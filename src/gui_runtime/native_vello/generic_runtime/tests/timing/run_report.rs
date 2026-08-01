use super::super::super::{GenericNativeVelloRunner, NativeGenericRunError};
use super::{fixtures::*, shared::*};

fn make_runner() -> GenericNativeVelloRunner<TestFrameMessageBridge, DemoMessage> {
    GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        TestFrameMessageBridge::default(),
        Vector2::new(320.0, 40.0),
    )
}

#[test]
fn terminal_cause_recording_is_first_write_wins_and_take_is_one_shot() {
    let mut runner = make_runner();

    assert_eq!(runner.take_terminal_cause(), None);
    assert!(runner.record_terminal_cause(NativeGenericRunError::SurfaceAcquireOutOfMemory));
    assert!(
        !runner
            .record_terminal_cause(NativeGenericRunError::EventLoopRun("secondary".to_string(),))
    );
    assert_eq!(
        runner.take_terminal_cause(),
        Some(NativeGenericRunError::SurfaceAcquireOutOfMemory)
    );
    assert_eq!(runner.take_terminal_cause(), None);
}

#[test]
fn terminal_cause_takes_precedence_over_successful_or_failed_event_loop_result() {
    let mut runner = make_runner();
    runner.record_terminal_cause(NativeGenericRunError::SurfaceAcquireOutOfMemory);
    assert_eq!(
        runner.resolve_run_result(Ok(())),
        Err(NativeGenericRunError::SurfaceAcquireOutOfMemory)
    );

    let mut runner = make_runner();
    runner.record_terminal_cause(NativeGenericRunError::SurfaceAcquireOutOfMemory);
    assert_eq!(
        runner.resolve_run_result(Err(NativeGenericRunError::EventLoopRun(
            "secondary".to_string(),
        ))),
        Err(NativeGenericRunError::SurfaceAcquireOutOfMemory)
    );
}

#[test]
fn result_resolution_preserves_ordinary_close_and_event_loop_errors_without_terminal_cause() {
    let mut runner = make_runner();
    assert_eq!(runner.resolve_run_result(Ok(())), Ok(()));

    let mut runner = make_runner();
    let event_loop_error = NativeGenericRunError::EventLoopRun("stopped".to_string());
    assert_eq!(
        runner.resolve_run_result(Err(event_loop_error.clone())),
        Err(event_loop_error)
    );
}

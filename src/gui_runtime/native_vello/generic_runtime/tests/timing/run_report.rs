use super::super::super::{
    GenericNativeVelloRunner, NativeGenericRunError, NativeInitializationStage,
};
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
fn native_initialization_stage_display_and_error_display_are_stable() {
    let stages = [
        (
            NativeInitializationStage::WindowCreation,
            "native window creation",
        ),
        (
            NativeInitializationStage::WgpuSurfaceCreation,
            "WGPU surface creation",
        ),
        (
            NativeInitializationStage::DeviceAcquisition,
            "WGPU device acquisition",
        ),
        (
            NativeInitializationStage::RenderSurfaceCreation,
            "render-surface creation",
        ),
        (
            NativeInitializationStage::RendererCreation,
            "renderer creation",
        ),
    ];

    for (stage, label) in stages {
        let error = NativeGenericRunError::NativeInitialization {
            stage,
            message: String::from("backend detail"),
        };
        assert_eq!(stage.to_string(), label);
        assert_eq!(
            error.to_string(),
            format!("native initialization failed during {label}: backend detail")
        );
    }
}

#[test]
fn initialization_terminal_cause_preserves_first_failure_over_later_causes() {
    let mut runner = make_runner();
    let initialization = NativeGenericRunError::NativeInitialization {
        stage: NativeInitializationStage::WgpuSurfaceCreation,
        message: String::from("surface rejected window"),
    };

    assert!(runner.record_terminal_cause(initialization.clone()));
    assert!(!runner.record_terminal_cause(NativeGenericRunError::SurfaceAcquireOutOfMemory));
    assert_eq!(runner.take_terminal_cause(), Some(initialization));
}

#[test]
fn terminal_cause_admission_predicates_block_later_initialization_work() {
    let mut runner = make_runner();
    assert!(runner.should_initialize_runtime());
    assert!(runner.should_admit_auxiliary_sync());

    runner.record_terminal_cause(NativeGenericRunError::NativeInitialization {
        stage: NativeInitializationStage::RendererCreation,
        message: String::from("renderer rejected device"),
    });

    assert!(!runner.should_initialize_runtime());
    assert!(!runner.should_admit_auxiliary_sync());
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

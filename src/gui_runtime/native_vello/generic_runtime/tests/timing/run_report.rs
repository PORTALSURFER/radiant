use super::super::super::{
    DeviceLossRegistration, GenericNativeAdapterOwner, GenericNativeVelloRunner,
    NativeAdapterGeneration, NativeGenericRunError, NativeInitializationStage,
    NativeRenderDeviceErrorKind,
};
use super::{fixtures::*, shared::*};
use std::sync::Arc;

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
fn current_primary_device_loss_witness_admits_only_current_owner() {
    let mut runner = make_runner();
    let current = Arc::new(DeviceLossRegistration::new());
    let stale = Arc::new(DeviceLossRegistration::new());
    let generation = NativeAdapterGeneration::from_test_serial(1);
    runner.adapter = Some(GenericNativeAdapterOwner::with_test_registration(
        generation,
        Arc::clone(&current),
    ));

    assert!(runner.device_loss_event_is_current(generation, &current));
    assert!(!runner.device_loss_event_is_current(generation, &stale));
    assert!(
        !runner
            .device_loss_event_is_current(NativeAdapterGeneration::from_test_serial(2), &current,)
    );
    assert!(!runner.device_loss_event_is_current(NativeAdapterGeneration::unknown(), &current,));

    runner.adapter = None;
    assert!(!runner.device_loss_event_is_current(generation, &current));
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
fn frame_render_error_display_is_stable_and_preserves_first_cause() {
    let mut runner = make_runner();
    let frame_error = NativeGenericRunError::FrameRender(String::from("backend rejected scene"));

    assert_eq!(
        frame_error.to_string(),
        "native frame rendering failed: backend rejected scene"
    );
    assert!(runner.record_terminal_cause(frame_error.clone()));
    assert!(!runner.record_terminal_cause(NativeGenericRunError::SurfaceAcquireOutOfMemory));
    assert_eq!(runner.take_terminal_cause(), Some(frame_error));
}

#[test]
fn render_device_loss_display_is_stable_and_preserves_first_cause() {
    let mut runner = make_runner();
    let device_loss = NativeGenericRunError::RenderDeviceLost(String::from("driver reset"));

    assert_eq!(
        device_loss.to_string(),
        "native render device lost: driver reset"
    );
    assert!(runner.record_terminal_cause(device_loss.clone()));
    assert!(
        !runner.record_terminal_cause(NativeGenericRunError::FrameRender(String::from(
            "secondary frame failure"
        ),))
    );
    assert!(!runner.record_terminal_cause(NativeGenericRunError::SurfaceAcquireOutOfMemory));
    assert_eq!(runner.take_terminal_cause(), Some(device_loss));
}

#[test]
fn render_device_error_kind_and_display_are_stable() {
    let kinds = [
        (NativeRenderDeviceErrorKind::OutOfMemory, "out of memory"),
        (NativeRenderDeviceErrorKind::Validation, "validation"),
        (NativeRenderDeviceErrorKind::Internal, "internal"),
    ];

    for (kind, label) in kinds {
        assert_eq!(kind.to_string(), label);
        let error = NativeGenericRunError::RenderDeviceError {
            kind,
            message: String::from("backend detail"),
        };
        assert_eq!(
            error.to_string(),
            format!("native render device error ({label}): backend detail")
        );
    }
}

#[test]
fn render_device_error_takes_first_cause_precedence_over_other_terminal_failures() {
    let render_device_error = NativeGenericRunError::RenderDeviceError {
        kind: NativeRenderDeviceErrorKind::Internal,
        message: String::from("driver fault"),
    };
    let later_causes = [
        NativeGenericRunError::RenderDeviceLost(String::from("device lost")),
        NativeGenericRunError::FrameRender(String::from("frame rejected")),
        NativeGenericRunError::NativeInitialization {
            stage: NativeInitializationStage::RendererCreation,
            message: String::from("renderer rejected device"),
        },
        NativeGenericRunError::EventLoopRun(String::from("stopped")),
    ];

    for later_cause in later_causes {
        let mut runner = make_runner();
        assert!(runner.record_terminal_cause(render_device_error.clone()));
        assert!(!runner.record_terminal_cause(later_cause));
        assert_eq!(
            runner.take_terminal_cause(),
            Some(render_device_error.clone())
        );
    }
}

#[test]
fn render_device_error_terminal_cause_blocks_later_initialization_work() {
    let mut runner = make_runner();
    assert!(runner.should_initialize_runtime());
    assert!(runner.should_admit_auxiliary_sync());

    let render_device_error = NativeGenericRunError::RenderDeviceError {
        kind: NativeRenderDeviceErrorKind::Validation,
        message: String::from("uncaptured validation"),
    };
    runner.record_terminal_cause(render_device_error.clone());

    assert!(!runner.should_initialize_runtime());
    assert!(!runner.should_admit_auxiliary_sync());
    assert_eq!(runner.take_terminal_cause(), Some(render_device_error));
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

    let initialization = NativeGenericRunError::NativeInitialization {
        stage: NativeInitializationStage::RendererCreation,
        message: String::from("renderer rejected device"),
    };
    runner.record_terminal_cause(initialization.clone());

    assert!(!runner.should_initialize_runtime());
    assert!(!runner.should_admit_auxiliary_sync());
    assert_eq!(runner.take_terminal_cause(), Some(initialization));
}

#[test]
fn render_device_loss_terminal_cause_blocks_later_initialization_work() {
    let mut runner = make_runner();
    assert!(runner.should_initialize_runtime());
    assert!(runner.should_admit_auxiliary_sync());

    let device_loss = NativeGenericRunError::RenderDeviceLost(String::from("driver reset"));
    runner.record_terminal_cause(device_loss.clone());

    assert!(!runner.should_initialize_runtime());
    assert!(!runner.should_admit_auxiliary_sync());
    assert_eq!(runner.take_terminal_cause(), Some(device_loss));
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

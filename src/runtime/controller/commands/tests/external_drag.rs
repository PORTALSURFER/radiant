use super::{super::*, fixtures::QueuedCommandBridge};
use crate::runtime::{ExternalDragEffect, ExternalDragOutcome, ExternalDragRequest};
use std::path::PathBuf;

#[test]
fn external_drag_command_arms_and_clears_native_session() {
    let bridge = QueuedCommandBridge::default();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(100.0, 100.0));
    let request = ExternalDragRequest::files([PathBuf::from(r"C:\samples\kick.wav")], "kick.wav");

    let outcome = runtime.execute_command(Command::begin_external_drag_without_completion(
        request.clone(),
    ));

    assert!(runtime.external_drag_armed());
    assert_eq!(outcome.messages_dispatched, 0);
    assert_eq!(
        runtime
            .take_external_drag_session()
            .expect("external drag session")
            .request,
        request
    );

    runtime.execute_command(Command::end_external_drag());

    assert!(!runtime.external_drag_armed());
}

#[test]
fn external_drag_completion_dispatches_host_message() {
    let bridge = QueuedCommandBridge::default();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(100.0, 100.0));
    let request = ExternalDragRequest::files([PathBuf::from(r"C:\samples\kick.wav")], "kick.wav");

    runtime.execute_command(Command::begin_external_drag(request, |result| {
        if result.expect("external drag should complete").accepted() {
            1
        } else {
            0
        }
    }));
    let session = runtime
        .take_external_drag_session()
        .expect("external drag session");
    let outcome = runtime.dispatch_external_drag_result(
        session,
        Ok(ExternalDragOutcome {
            effect: ExternalDragEffect::Copy,
        }),
    );

    assert_eq!(outcome.messages_dispatched, 1);
    assert_eq!(runtime.bridge().dispatched, vec![1]);
}

use super::{super::*, fixtures::QueuedCommandBridge};
use crate::runtime::{ExternalDragEffect, ExternalDragOutcome, ExternalDragRequest};
use std::{path::PathBuf, rc::Rc};

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
            .take_external_drag_launch()
            .expect("external drag launch")
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
    let launch = runtime
        .take_external_drag_launch()
        .expect("external drag launch");
    let outcome = runtime.dispatch_external_drag_launch_result(
        launch.identity,
        Ok(ExternalDragOutcome {
            effect: ExternalDragEffect::Copy,
        }),
    );

    assert_eq!(outcome.messages_dispatched, 0);
    assert!(outcome.runtime_work_remaining);
    assert_eq!(runtime.bridge().dispatched, Vec::<usize>::new());
    assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 1);
    assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 0);
    assert_eq!(runtime.bridge().dispatched, vec![1]);
}

#[test]
fn external_drag_completion_mapper_is_ui_local_and_released_by_replacement() {
    let bridge = QueuedCommandBridge::default();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(100.0, 100.0));
    let mapper_token = Rc::new(());
    let mapper_token_for_callback = Rc::clone(&mapper_token);
    let request = ExternalDragRequest::files([PathBuf::from("kick.wav")], "kick.wav");

    runtime.execute_command(Command::begin_external_drag(request.clone(), move |_| {
        drop(mapper_token_for_callback);
        1
    }));
    assert_eq!(Rc::strong_count(&mapper_token), 2);

    let launch = runtime
        .take_external_drag_launch()
        .expect("external drag launch");
    assert_eq!(
        runtime
            .dispatch_external_drag_launch_result(
                launch.identity,
                Ok(ExternalDragOutcome {
                    effect: ExternalDragEffect::Copy,
                }),
            )
            .messages_dispatched,
        0
    );
    assert_eq!(Rc::strong_count(&mapper_token), 2);

    runtime.execute_command(Command::begin_external_drag_without_completion(request));
    assert_eq!(Rc::strong_count(&mapper_token), 1);
}

#[test]
fn external_drag_end_and_runtime_exit_release_pending_mapper() {
    let bridge = QueuedCommandBridge::default();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(100.0, 100.0));
    let mapper_token = Rc::new(());
    let mapper_token_for_callback = Rc::clone(&mapper_token);
    let request = ExternalDragRequest::files([PathBuf::from("kick.wav")], "kick.wav");

    runtime.execute_command(Command::begin_external_drag(request, move |_| {
        drop(mapper_token_for_callback);
        1
    }));
    let launch = runtime
        .take_external_drag_launch()
        .expect("external drag launch");
    runtime.dispatch_external_drag_launch_result(launch.identity, Err(String::from("cancelled")));
    assert_eq!(Rc::strong_count(&mapper_token), 2);
    runtime.execute_command(Command::end_external_drag());
    assert_eq!(Rc::strong_count(&mapper_token), 1);

    let mapper_token_for_callback = Rc::clone(&mapper_token);
    runtime.execute_command(Command::begin_external_drag(
        ExternalDragRequest::files([PathBuf::from("snare.wav")], "snare.wav"),
        move |_| {
            drop(mapper_token_for_callback);
            1
        },
    ));
    let launch = runtime
        .take_external_drag_launch()
        .expect("external drag launch");
    runtime.dispatch_external_drag_launch_result(launch.identity, Err(String::from("cancelled")));
    assert_eq!(Rc::strong_count(&mapper_token), 2);
    runtime.host_on_runtime_exit();
    assert_eq!(Rc::strong_count(&mapper_token), 1);
    assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 0);
}

#[test]
fn external_drag_launch_results_are_deferred_for_all_outcomes() {
    let bridge = QueuedCommandBridge::default();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(100.0, 100.0));
    let outcomes = [
        Ok(ExternalDragOutcome {
            effect: ExternalDragEffect::Copy,
        }),
        Ok(ExternalDragOutcome {
            effect: ExternalDragEffect::None,
        }),
        Err(String::from("native failure")),
    ];

    for (index, result) in outcomes.into_iter().enumerate() {
        runtime.execute_command(Command::begin_external_drag(
            ExternalDragRequest::files([PathBuf::from("kick.wav")], "kick.wav"),
            move |result| {
                if result.is_ok_and(ExternalDragOutcome::accepted) {
                    index
                } else {
                    index + 10
                }
            },
        ));
        let launch = runtime
            .take_external_drag_launch()
            .expect("external drag launch");
        assert_eq!(
            runtime
                .dispatch_external_drag_launch_result(launch.identity, result)
                .messages_dispatched,
            0
        );
        assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 1);
        assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 0);
    }
    assert_eq!(runtime.bridge().dispatched, vec![0, 11, 12]);
}

#[test]
fn external_drag_stale_and_duplicate_results_are_ignored_after_replacement() {
    let bridge = QueuedCommandBridge::default();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(100.0, 100.0));
    runtime.execute_command(Command::begin_external_drag(
        ExternalDragRequest::files([PathBuf::from("old.wav")], "old.wav"),
        |_| 1,
    ));
    let old_launch = runtime
        .take_external_drag_launch()
        .expect("old external drag launch");
    runtime.execute_command(Command::begin_external_drag_without_completion(
        ExternalDragRequest::files([PathBuf::from("new.wav")], "new.wav"),
    ));
    let stale_outcome = runtime.dispatch_external_drag_launch_result(
        old_launch.identity,
        Ok(ExternalDragOutcome {
            effect: ExternalDragEffect::Copy,
        }),
    );
    assert_eq!(stale_outcome.messages_dispatched, 0);
    assert!(!stale_outcome.runtime_work_remaining);
    assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 0);
}

#[test]
fn external_drag_exit_and_runtime_drop_release_mapper() {
    let mapper_token = Rc::new(());
    {
        let bridge = QueuedCommandBridge::default();
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(100.0, 100.0));
        let mapper_token_for_callback = Rc::clone(&mapper_token);
        runtime.execute_command(Command::begin_external_drag(
            ExternalDragRequest::files([PathBuf::from("kick.wav")], "kick.wav"),
            move |_| {
                drop(mapper_token_for_callback);
                1
            },
        ));
        runtime.execute_command(Command::Exit);
        assert_eq!(Rc::strong_count(&mapper_token), 1);
    }

    let mapper_token_for_callback = Rc::clone(&mapper_token);
    {
        let bridge = QueuedCommandBridge::default();
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(100.0, 100.0));
        runtime.execute_command(Command::begin_external_drag(
            ExternalDragRequest::files([PathBuf::from("snare.wav")], "snare.wav"),
            move |_| {
                drop(mapper_token_for_callback);
                1
            },
        ));
        assert_eq!(Rc::strong_count(&mapper_token), 2);
    }
    assert_eq!(Rc::strong_count(&mapper_token), 1);
}

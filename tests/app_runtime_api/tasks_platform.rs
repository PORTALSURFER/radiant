use super::*;

fn take_perform(
    command: radiant::runtime::Command<DemoMessage>,
) -> (
    &'static str,
    radiant::prelude::TaskPriority,
    Box<dyn FnOnce() -> DemoMessage + Send + 'static>,
) {
    match command {
        radiant::runtime::Command::Perform {
            name,
            priority,
            is_cancelled: _,
            work,
        } => (name, priority, work),
        other => panic!("expected one business perform command, got {other:?}"),
    }
}

#[test]
fn business_work_context_is_explicit_runtime_api_not_prelude_app_api() {
    type WorkerContext = radiant::runtime::BusinessWorkContext;
    let _accepts_worker_context = WorkerContext::is_cancelled as fn(&WorkerContext) -> bool;
}

#[test]
fn latest_task_tracks_current_ticket_and_tags_spawned_completion() {
    let mut latest = radiant::prelude::LatestTask::new();
    let first = latest.begin();
    let second = latest.begin();

    assert!(!latest.is_active(first));
    assert!(latest.is_active(second));
    assert!(!latest.finish(first));
    assert!(latest.finish(second));
    assert_eq!(latest.active(), None);

    let mut latest = radiant::prelude::LatestTask::new();
    let mut context: radiant::prelude::UiUpdateContext<DemoMessage> =
        radiant::prelude::UiUpdateContext::default();
    context
        .business()
        .background("latest-task-test")
        .latest(&mut latest)
        .run(
            |_| 7_u32,
            |completion| {
                assert_eq!(completion.task_id(), 1);
                DemoMessage::Increment
            },
        );

    assert_eq!(latest.active().map(|ticket| ticket.id()), Some(1));
}

#[test]
fn business_runtime_builds_named_priority_lanes() {
    type SubmitBusinessWork = fn(&mut radiant::prelude::UiUpdateContext<DemoMessage>);
    let cases: [(&str, radiant::prelude::TaskPriority, SubmitBusinessWork); 4] = [
        (
            "interactive-work",
            radiant::prelude::TaskPriority::Interactive,
            |context: &mut radiant::prelude::UiUpdateContext<DemoMessage>| {
                context
                    .business()
                    .interactive("interactive-work")
                    .run(|_| DemoMessage::Increment, |message| message);
            },
        ),
        (
            "background-work",
            radiant::prelude::TaskPriority::Background,
            |context: &mut radiant::prelude::UiUpdateContext<DemoMessage>| {
                context
                    .business()
                    .background("background-work")
                    .run(|_| DemoMessage::Increment, |message| message);
            },
        ),
        (
            "idle-work",
            radiant::prelude::TaskPriority::Idle,
            |context: &mut radiant::prelude::UiUpdateContext<DemoMessage>| {
                context
                    .business()
                    .idle("idle-work")
                    .run(|_| DemoMessage::Increment, |message| message);
            },
        ),
        (
            "blocking-io-work",
            radiant::prelude::TaskPriority::BlockingIo,
            |context: &mut radiant::prelude::UiUpdateContext<DemoMessage>| {
                context
                    .business()
                    .blocking_io("blocking-io-work")
                    .run(|_| DemoMessage::Increment, |message| message);
            },
        ),
    ];
    for (expected_name, expected_priority, submit) in cases {
        let mut context = radiant::prelude::UiUpdateContext::default();
        submit(&mut context);

        let (name, priority, work) = take_perform(context.into_command());

        assert_eq!(name, expected_name);
        assert_eq!(priority, expected_priority);
        assert_eq!(work(), DemoMessage::Increment);
    }
}

#[test]
fn business_runtime_priority_helper_uses_host_selected_lane() {
    let mut context: radiant::prelude::UiUpdateContext<DemoMessage> =
        radiant::prelude::UiUpdateContext::default();
    context
        .business()
        .priority(
            "host-selected-work",
            radiant::prelude::TaskPriority::BlockingIo,
        )
        .run(|_| DemoMessage::Increment, |message| message);

    let (name, priority, work) = take_perform(context.into_command());

    assert_eq!(name, "host-selected-work");
    assert_eq!(priority, radiant::prelude::TaskPriority::BlockingIo);
    assert_eq!(work(), DemoMessage::Increment);
}

#[test]
fn business_runtime_keyed_latest_tags_completion_with_key_and_ticket() {
    let mut keyed = radiant::prelude::KeyedLatestTasks::new();
    let key = String::from("row-1");
    let mut context = radiant::prelude::UiUpdateContext::default();
    context
        .business()
        .interactive("keyed-preview")
        .latest_for(&mut keyed, key.clone())
        .run(
            |_| 42_u8,
            |completion| {
                assert_eq!(completion.key, "row-1");
                assert_eq!(completion.task_id(), 1);
                assert_eq!(completion.output, 42);
                DemoMessage::Increment
            },
        );

    assert_eq!(keyed.active(&key).map(|ticket| ticket.id()), Some(1));
    let (_name, _priority, work) = take_perform(context.into_command());
    assert_eq!(work(), DemoMessage::Increment);
}

#[test]
fn business_runtime_priority_helper_composes_with_resource_policies() {
    let mut resources = radiant::prelude::ResourceTasks::default();
    let key = radiant::prelude::ResourceKey::scoped("preview", "kick");
    let mut context = radiant::prelude::UiUpdateContext::default();
    context
        .business()
        .priority(
            "resource-preview",
            radiant::prelude::TaskPriority::Interactive,
        )
        .latest_for_resource(&mut resources, key.clone())
        .run(
            |_| 42_u8,
            |completion| {
                assert_eq!(
                    completion.key,
                    radiant::prelude::ResourceKey::scoped("preview", "kick")
                );
                assert_eq!(completion.task_id(), 1);
                assert_eq!(completion.output, 42);
                DemoMessage::Increment
            },
        );

    assert_eq!(resources.active(&key).map(|ticket| ticket.id()), Some(1));
    let (name, priority, work) = take_perform(context.into_command());
    assert_eq!(name, "resource-preview");
    assert_eq!(priority, radiant::prelude::TaskPriority::Interactive);
    assert_eq!(work(), DemoMessage::Increment);

    let exclusive_key = radiant::prelude::ResourceKey::scoped("preview", "snare");
    let mut exclusive_context: radiant::prelude::UiUpdateContext<DemoMessage> =
        radiant::prelude::UiUpdateContext::default();
    let first = exclusive_context
        .business()
        .priority(
            "exclusive-preview",
            radiant::prelude::TaskPriority::Background,
        )
        .exclusive_for(&mut resources, exclusive_key.clone());
    assert!(
        first.is_some(),
        "first exclusive resource work should start"
    );

    let mut duplicate_context: radiant::prelude::UiUpdateContext<DemoMessage> =
        radiant::prelude::UiUpdateContext::default();
    let first = duplicate_context
        .business()
        .priority(
            "exclusive-preview",
            radiant::prelude::TaskPriority::Background,
        )
        .exclusive_for(&mut resources, exclusive_key);
    assert!(
        first.is_none(),
        "active resource should reject duplicate work"
    );
}

#[test]
fn business_runtime_resource_request_returns_typed_completion() {
    let mut resource = radiant::prelude::ResourceSlot::<String>::new("preview");
    let mut context = radiant::prelude::UiUpdateContext::default();
    context
        .business()
        .background("load-preview")
        .resource(&mut resource)
        .run(
            |_| Ok(String::from("ready")),
            |completion| {
                assert_eq!(completion.key().as_str(), "preview");
                assert_eq!(completion.generation(), 1);
                DemoMessage::Increment
            },
        );

    assert!(resource.is_loading());
    let (_name, _priority, work) = take_perform(context.into_command());
    assert_eq!(work(), DemoMessage::Increment);
}

#[test]
fn business_runtime_cancellable_work_exposes_worker_context() {
    let mut context = radiant::prelude::UiUpdateContext::default();
    let request = context
        .business()
        .background("cancel-visible")
        .cancellable();
    let token = request.token();
    token.cancel();
    request.run(
        |worker| worker.is_cancelled(),
        |cancelled| {
            assert!(cancelled);
            DemoMessage::Increment
        },
    );

    let (_name, _priority, work) = take_perform(context.into_command());
    assert_eq!(work(), DemoMessage::Increment);
}

#[test]
fn ui_update_context_schedules_delayed_latest_messages() {
    let mut latest = radiant::prelude::LatestTask::new();
    let mut context = radiant::prelude::UiUpdateContext::default();
    context.after_latest(
        &mut latest,
        std::time::Duration::from_millis(25),
        |ticket| {
            assert_eq!(ticket.id(), 1);
            DemoMessage::Increment
        },
    );

    assert_eq!(latest.active().map(|ticket| ticket.id()), Some(1));
    assert!(latest.finish(latest.active().expect("latest scheduled")));
}

#[test]
fn ui_update_context_exposes_platform_service_helpers() {
    let mut context = radiant::prelude::UiUpdateContext::default();
    context.pick_folder(
        radiant::runtime::FileDialogRequest::new().title("Choose library"),
        |_| DemoMessage::Increment,
    );
    context.pick_file(
        radiant::runtime::FileDialogRequest::new().filter("Wave", vec![String::from("wav")]),
        |_| DemoMessage::Increment,
    );
    context.save_file(
        radiant::runtime::FileDialogRequest::new().filename("export.wav"),
        |_| DemoMessage::Increment,
    );
    context.open_path(std::path::PathBuf::from(r"C:\samples"), |_| {
        DemoMessage::Increment
    });
    context.reveal_path(std::path::PathBuf::from(r"C:\samples\kick.wav"), |_| {
        DemoMessage::Increment
    });
    context.open_url("https://example.invalid", |_| DemoMessage::Increment);
    context.copy_text("C:/samples/kick.wav", |_| DemoMessage::Increment);
    context.copy_file_paths(
        vec![std::path::PathBuf::from(r"C:\samples\kick.wav")],
        |_| DemoMessage::Increment,
    );
    context.read_text(|_| DemoMessage::Increment);
    context.read_file_paths(|_| DemoMessage::Increment);
    context.confirm(
        radiant::runtime::ConfirmDialogRequest::new("Delete sample", "Delete selected sample?"),
        |_| DemoMessage::Increment,
    );
}

#[test]
fn platform_response_helpers_cover_common_request_outcomes() {
    let path = std::path::PathBuf::from(r"C:\samples\kick.wav");
    let response = radiant::prelude::PlatformResponse::Path(path.clone());

    assert_eq!(response.path(), Some(path.as_path()));
    assert_eq!(response.clone().into_path(), Some(path.clone()));
    assert_eq!(response.into_path_or_canceled(), Ok(Some(path)));

    assert_eq!(
        radiant::prelude::PlatformResponse::Canceled.into_path_or_canceled(),
        Ok(None)
    );
    assert!(radiant::prelude::PlatformResponse::Canceled.is_canceled());

    assert!(radiant::prelude::PlatformResponse::Completed.is_completed());
    assert_eq!(
        radiant::prelude::PlatformResponse::Completed.into_completed(),
        Ok(())
    );

    let confirmation = radiant::prelude::PlatformResponse::Confirmation(
        radiant::prelude::ConfirmationResponse::Accepted,
    );
    assert_eq!(
        confirmation.confirmation(),
        Some(radiant::prelude::ConfirmationResponse::Accepted)
    );
    assert_eq!(
        confirmation.into_confirmation(),
        Some(radiant::prelude::ConfirmationResponse::Accepted)
    );

    let text = radiant::prelude::PlatformResponse::Text(String::from("C:/samples/kick.wav"));
    assert_eq!(text.into_text(), Some(String::from("C:/samples/kick.wav")));

    let paths = vec![std::path::PathBuf::from(r"C:\samples\kick.wav")];
    let file_paths = radiant::prelude::PlatformResponse::FilePaths(paths.clone());
    assert_eq!(file_paths.into_file_paths(), Some(paths));
}

#[test]
fn ui_update_context_exposes_drag_session_cleanup_helper() {
    let mut context: radiant::prelude::UiUpdateContext<DemoMessage> =
        radiant::prelude::UiUpdateContext::default();
    context.end_drag_session();
}

#[test]
fn confirm_dialog_supports_named_parts_construction() {
    let request =
        radiant::prelude::ConfirmDialogRequest::from_parts(radiant::prelude::ConfirmDialogParts {
            title: "Overwrite file".to_owned(),
            message: "Replace the existing export?".to_owned(),
            level: radiant::prelude::ConfirmationLevel::Warning,
            buttons: radiant::prelude::ConfirmationButtons::YesNo,
        });

    assert_eq!(request.title, "Overwrite file");
    assert_eq!(request.message, "Replace the existing export?");
    assert_eq!(request.level, radiant::prelude::ConfirmationLevel::Warning);
    assert_eq!(
        request.buttons,
        radiant::prelude::ConfirmationButtons::YesNo
    );
}

#[test]
fn business_runtime_can_submit_cancellable_work() {
    let mut context = radiant::prelude::UiUpdateContext::default();
    let request = context.business().background("cancel-test").cancellable();
    let token = request.token();
    token.cancel();
    request.run(
        |worker| worker.is_cancelled(),
        |cancelled| {
            assert!(cancelled);
            DemoMessage::Increment
        },
    );
}

#[test]
fn business_runtime_can_submit_cancellable_latest_work() {
    let mut latest = radiant::prelude::LatestTask::new();
    let mut context = radiant::prelude::UiUpdateContext::default();
    let token = context
        .business()
        .idle("latest-cancel-test")
        .latest(&mut latest)
        .cancellable()
        .run(
            |worker| worker.is_cancelled(),
            |completion| {
                assert_eq!(completion.task_id(), 1);
                DemoMessage::Increment
            },
        );

    assert_eq!(latest.active().map(|ticket| ticket.id()), Some(1));
    assert!(!token.is_cancelled());
    token.cancel();
    assert!(token.is_cancelled());
}

#[test]
fn ui_update_context_accepts_task_priority_hints() {
    let mut context = radiant::prelude::UiUpdateContext::default();
    context
        .business()
        .idle("idle-cancel-test")
        .cancellable()
        .run(|worker| worker.is_cancelled(), |_| DemoMessage::Increment);
    context
        .business()
        .interactive("interactive-test")
        .run(|_| 1_u8, |_| DemoMessage::Increment);
    context
        .business()
        .blocking_io("blocking-io-test")
        .run(|_| 1_u8, |_| DemoMessage::Increment);
}

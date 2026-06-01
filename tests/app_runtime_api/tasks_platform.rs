use super::*;

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
    let mut context = radiant::prelude::UpdateContext::default();
    context.spawn_latest(
        &mut latest,
        "latest-task-test",
        || 7_u32,
        |completion| {
            assert_eq!(completion.task_id(), 1);
            DemoMessage::Increment
        },
    );

    assert_eq!(latest.active().map(|ticket| ticket.id()), Some(1));
}

#[test]
fn update_context_schedules_delayed_latest_messages() {
    let mut latest = radiant::prelude::LatestTask::new();
    let mut context = radiant::prelude::UpdateContext::default();
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
fn update_context_exposes_platform_service_helpers() {
    let mut context = radiant::prelude::UpdateContext::default();
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
    context.open_url("https://example.invalid", |_| DemoMessage::Increment);
    context.confirm(
        radiant::runtime::ConfirmDialogRequest::new("Delete sample", "Delete selected sample?"),
        |_| DemoMessage::Increment,
    );
}

#[test]
fn update_context_exposes_drag_session_cleanup_helper() {
    let mut context: radiant::prelude::UpdateContext<DemoMessage> =
        radiant::prelude::UpdateContext::default();
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
fn update_context_can_spawn_cancellable_work() {
    let token = radiant::prelude::CancellationToken::new();
    let worker_token = token.clone();
    token.cancel();

    let mut context = radiant::prelude::UpdateContext::default();
    context.spawn_cancellable(
        "cancel-test",
        worker_token,
        |token| token.is_cancelled(),
        |cancelled| {
            assert!(cancelled);
            DemoMessage::Increment
        },
    );
}

#[test]
fn update_context_accepts_task_priority_hints() {
    let token = radiant::prelude::CancellationToken::new();
    let mut context = radiant::prelude::UpdateContext::default();
    context.spawn_cancellable_with_priority(
        "idle-cancel-test",
        radiant::prelude::TaskPriority::Idle,
        token,
        |token| token.is_cancelled(),
        |_| DemoMessage::Increment,
    );
    context.spawn_with_priority(
        "interactive-test",
        radiant::prelude::TaskPriority::Interactive,
        || 1_u8,
        |_| DemoMessage::Increment,
    );
}

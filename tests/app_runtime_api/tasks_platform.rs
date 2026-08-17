use super::*;

fn assert_worker_command(
    command: &radiant::runtime::Command<DemoMessage>,
    name: &'static str,
    priority: radiant::prelude::TaskPriority,
) {
    assert!(matches!(
        command,
        radiant::runtime::Command::PerformWorker(_)
    ));
    assert_eq!(command.business_task_priority(name), Some(priority));
}

#[test]
fn business_work_context_is_explicit_runtime_api_not_prelude_app_api() {
    type WorkerContext = radiant::runtime::BusinessWorkContext;
    let _accepts_worker_context = WorkerContext::is_cancelled as fn(&WorkerContext) -> bool;
}

#[test]
fn business_run_accepts_ui_local_mapper_capture() {
    let mapped = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let mapper_state = std::rc::Rc::clone(&mapped);
    let mut context = radiant::prelude::UiUpdateContext::default();
    context.business().background("ui-local-map").run(
        |_| 7_u32,
        move |output| {
            mapper_state.borrow_mut().push(output);
            DemoMessage::Increment
        },
    );

    let command = context.into_command();
    assert_worker_command(
        &command,
        "ui-local-map",
        radiant::prelude::TaskPriority::Background,
    );
    assert!(mapped.borrow().is_empty());
}

#[test]
fn business_admission_receipt_is_pending_until_controller_admission() {
    let mut context = radiant::prelude::UiUpdateContext::default();
    let receipt = context
        .business()
        .background("receipt")
        .run_with_receipt(|_| 7_u32, |_| DemoMessage::Increment);
    assert_eq!(
        receipt.poll(),
        radiant::prelude::BusinessTaskAdmission::Pending
    );
    assert_worker_command(
        &context.into_command(),
        "receipt",
        radiant::prelude::TaskPriority::Background,
    );
}

#[test]
fn latest_admission_receipt_preserves_ticket_ordering() {
    let mut latest = radiant::prelude::LatestTask::new();
    let mut context = radiant::prelude::UiUpdateContext::default();
    let request = context
        .business()
        .background("latest-receipt")
        .latest(&mut latest);
    let ticket = request.ticket();
    let receipt = request.run_with_receipt(
        |_| 9_u32,
        move |completion| {
            assert_eq!(completion.ticket, ticket);
            DemoMessage::Increment
        },
    );
    assert_eq!(ticket.id(), 1);
    assert_eq!(
        receipt.poll(),
        radiant::prelude::BusinessTaskAdmission::Pending
    );
    assert_worker_command(
        &context.into_command(),
        "latest-receipt",
        radiant::prelude::TaskPriority::Background,
    );
}

#[test]
fn owner_latest_business_worker_api_preserves_ticket_and_pending_receipt() {
    let owner = radiant::application::DeclarativeEffectOwner::new();
    let mut latest = radiant::prelude::LatestTask::new();
    let mut context = radiant::prelude::UiUpdateContext::default();
    let request = context
        .business()
        .background("owner-latest-receipt")
        .latest(&mut latest);
    let ticket = request.ticket();
    let receipt = request.run_for_owner_with_receipt(
        owner,
        |_| 9_u32,
        move |completion| {
            assert_eq!(completion.ticket, ticket);
            DemoMessage::Increment
        },
    );

    assert_eq!(
        receipt.poll(),
        radiant::prelude::BusinessTaskAdmission::Pending
    );
    assert_eq!(latest.active(), Some(ticket));
    assert_worker_command(
        &context.into_command(),
        "owner-latest-receipt",
        radiant::prelude::TaskPriority::Background,
    );
}

#[test]
fn owner_keyed_latest_business_api_preserves_key_ticket_receipt_and_ui_mapper() {
    let owner = radiant::application::DeclarativeEffectOwner::new();
    let mut keyed = radiant::prelude::KeyedLatestTasks::new();
    let key = String::from("row-1");
    let expected_key = key.clone();
    let mapped = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let mapped_state = std::rc::Rc::clone(&mapped);
    let mut context = radiant::prelude::UiUpdateContext::default();
    let request = context
        .business()
        .background("owner-keyed-latest-receipt")
        .latest_for(&mut keyed, key.clone());
    let ticket = request.ticket();
    let receipt: radiant::prelude::BusinessTaskAdmissionReceipt = request
        .run_for_owner_with_receipt(
            owner,
            |_| 42_u8,
            move |completion| {
                assert_eq!(completion.key, expected_key);
                assert_eq!(completion.ticket, ticket);
                assert_eq!(completion.output, 42);
                mapped_state.borrow_mut().push((
                    completion.key.clone(),
                    completion.ticket,
                    completion.output,
                ));
                DemoMessage::Increment
            },
        );

    assert_eq!(
        receipt.poll(),
        radiant::prelude::BusinessTaskAdmission::Pending
    );
    assert_eq!(keyed.active(&key), Some(ticket));
    assert_worker_command(
        &context.into_command(),
        "owner-keyed-latest-receipt",
        radiant::prelude::TaskPriority::Background,
    );
    assert!(mapped.borrow().is_empty());
}

#[test]
fn owner_latest_business_stream_api_preserves_ticket_and_pending_receipt() {
    let owner = radiant::application::DeclarativeEffectOwner::new();
    let mut latest = radiant::prelude::LatestTask::new();
    let mapped = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let event_state = std::rc::Rc::clone(&mapped);
    let final_state = std::rc::Rc::clone(&mapped);
    let mut context = radiant::prelude::UiUpdateContext::default();
    let request = context
        .business()
        .background("owner-latest-stream-receipt")
        .latest(&mut latest);
    let ticket = request.ticket();
    let receipt = request.stream_for_owner_with_receipt(
        owner,
        |_, events| {
            assert!(events.emit(1_u32));
            2_u32
        },
        move |completion| {
            assert_eq!(completion.ticket, ticket);
            event_state.borrow_mut().push(completion.ticket);
            DemoMessage::Increment
        },
        move |completion| {
            assert_eq!(completion.ticket, ticket);
            final_state.borrow_mut().push(completion.ticket);
            DemoMessage::Increment
        },
    );

    assert_eq!(
        receipt.poll(),
        radiant::prelude::BusinessTaskAdmission::Pending
    );
    assert_eq!(latest.active(), Some(ticket));
    assert_worker_command(
        &context.into_command(),
        "owner-latest-stream-receipt",
        radiant::prelude::TaskPriority::Background,
    );
    assert!(mapped.borrow().is_empty());
}

#[test]
fn owner_ordered_stream_business_api_accepts_ui_local_mappers() {
    let owner = radiant::application::DeclarativeEffectOwner::new();
    let mapped = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let event_state = std::rc::Rc::clone(&mapped);
    let final_state = std::rc::Rc::clone(&mapped);
    let mut context = radiant::prelude::UiUpdateContext::default();
    let receipt = context
        .business()
        .background("owner-stream-receipt")
        .stream_for_owner_with_receipt(
            owner,
            |worker_context, events| {
                assert!(!worker_context.is_cancelled());
                assert!(events.emit(1_u8));
                2_u8
            },
            move |event| {
                event_state.borrow_mut().push(event);
                DemoMessage::Increment
            },
            move |output| {
                final_state.borrow_mut().push(output);
                DemoMessage::Increment
            },
        );

    assert_eq!(
        receipt.poll(),
        radiant::prelude::BusinessTaskAdmission::Pending
    );
    assert_worker_command(
        &context.into_command(),
        "owner-stream-receipt",
        radiant::prelude::TaskPriority::Background,
    );
    assert!(mapped.borrow().is_empty());
}

#[test]
fn owner_coalesced_stream_business_api_accepts_ui_local_mappers() {
    let owner = radiant::application::DeclarativeEffectOwner::new();
    let mapped = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let event_state = std::rc::Rc::clone(&mapped);
    let final_state = std::rc::Rc::clone(&mapped);
    let mut context = radiant::prelude::UiUpdateContext::default();
    let receipt = context
        .business()
        .background("owner-stream-latest-receipt")
        .stream_latest_for_owner_with_receipt(
            owner,
            |worker_context, events| {
                assert!(!worker_context.is_cancelled());
                assert!(events.emit(1_u8));
                2_u8
            },
            move |event| {
                event_state.borrow_mut().push(event);
                DemoMessage::Increment
            },
            move |output| {
                final_state.borrow_mut().push(output);
                DemoMessage::Increment
            },
        );

    assert_eq!(
        receipt.poll(),
        radiant::prelude::BusinessTaskAdmission::Pending
    );
    assert_worker_command(
        &context.into_command(),
        "owner-stream-latest-receipt",
        radiant::prelude::TaskPriority::Background,
    );
    assert!(mapped.borrow().is_empty());
}

#[test]
fn owner_latest_coalesced_business_stream_api() {
    let owner = radiant::application::DeclarativeEffectOwner::new();
    let mut latest = radiant::prelude::LatestTask::new();
    let mapped = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let event_state = std::rc::Rc::clone(&mapped);
    let final_state = std::rc::Rc::clone(&mapped);
    let mut context = radiant::prelude::UiUpdateContext::default();
    let request = context
        .business()
        .background("owner-latest-coalesced-stream-receipt")
        .latest(&mut latest);
    let ticket = request.ticket();
    let receipt = request.stream_latest_for_owner_with_receipt(
        owner,
        |worker_context, events| {
            assert!(!worker_context.is_cancelled());
            assert!(events.emit(1_u8));
            2_u8
        },
        move |completion| {
            assert_eq!(completion.ticket, ticket);
            event_state
                .borrow_mut()
                .push((completion.ticket, completion.output));
            DemoMessage::Increment
        },
        move |completion| {
            assert_eq!(completion.ticket, ticket);
            final_state
                .borrow_mut()
                .push((completion.ticket, completion.output));
            DemoMessage::Increment
        },
    );

    assert_eq!(
        receipt.poll(),
        radiant::prelude::BusinessTaskAdmission::Pending
    );
    assert_eq!(latest.active(), Some(ticket));
    assert_worker_command(
        &context.into_command(),
        "owner-latest-coalesced-stream-receipt",
        radiant::prelude::TaskPriority::Background,
    );
    assert!(mapped.borrow().is_empty());
}

#[test]
fn one_shot_business_families_accept_ui_local_mappers() {
    let mut ordinary = radiant::prelude::UiUpdateContext::default();
    let ordinary_state = std::rc::Rc::new(std::cell::RefCell::new(0_u8));
    let ordinary_capture = std::rc::Rc::clone(&ordinary_state);
    ordinary
        .business()
        .background("ui-local-ordinary")
        .run_on_ui(
            |_| 1_u8,
            move |_| {
                *ordinary_capture.borrow_mut() += 1;
                DemoMessage::Increment
            },
        );
    assert_worker_command(
        &ordinary.into_command(),
        "ui-local-ordinary",
        radiant::prelude::TaskPriority::Background,
    );
    assert_eq!(*ordinary_state.borrow(), 0);

    let mut latest_task = radiant::prelude::LatestTask::new();
    let mut latest = radiant::prelude::UiUpdateContext::default();
    let latest_state = std::rc::Rc::new(std::cell::RefCell::new(0_u8));
    let latest_capture = std::rc::Rc::clone(&latest_state);
    latest
        .business()
        .background("ui-local-latest")
        .latest(&mut latest_task)
        .run_on_ui(
            |_| 1_u8,
            move |_| {
                *latest_capture.borrow_mut() += 1;
                DemoMessage::Increment
            },
        );
    assert_worker_command(
        &latest.into_command(),
        "ui-local-latest",
        radiant::prelude::TaskPriority::Background,
    );
    assert_eq!(*latest_state.borrow(), 0);

    let mut cancellable = radiant::prelude::UiUpdateContext::default();
    let cancellable_state = std::rc::Rc::new(std::cell::RefCell::new(0_u8));
    let cancellable_capture = std::rc::Rc::clone(&cancellable_state);
    cancellable
        .business()
        .background("ui-local-cancellable")
        .cancellable()
        .run_on_ui(
            |_| 1_u8,
            move |_| {
                *cancellable_capture.borrow_mut() += 1;
                DemoMessage::Increment
            },
        );
    assert_worker_command(
        &cancellable.into_command(),
        "ui-local-cancellable",
        radiant::prelude::TaskPriority::Background,
    );
    assert_eq!(*cancellable_state.borrow(), 0);

    let mut keyed_tasks = radiant::prelude::KeyedLatestTasks::new();
    let mut keyed = radiant::prelude::UiUpdateContext::default();
    let keyed_state = std::rc::Rc::new(std::cell::RefCell::new(0_u8));
    let keyed_capture = std::rc::Rc::clone(&keyed_state);
    keyed
        .business()
        .background("ui-local-keyed")
        .latest_for(&mut keyed_tasks, "row-1")
        .run(
            |_| 1_u8,
            move |_| {
                *keyed_capture.borrow_mut() += 1;
                DemoMessage::Increment
            },
        );
    assert_worker_command(
        &keyed.into_command(),
        "ui-local-keyed",
        radiant::prelude::TaskPriority::Background,
    );
    assert_eq!(*keyed_state.borrow(), 0);

    let mut resource = radiant::prelude::ResourceSlot::<String>::new("ui-local-resource");
    let mut resource_context = radiant::prelude::UiUpdateContext::default();
    let resource_state = std::rc::Rc::new(std::cell::RefCell::new(0_u8));
    let resource_capture = std::rc::Rc::clone(&resource_state);
    resource_context
        .business()
        .background("ui-local-resource")
        .resource(&mut resource)
        .run(
            |_| Ok(String::from("ready")),
            move |_| {
                *resource_capture.borrow_mut() += 1;
                DemoMessage::Increment
            },
        );
    assert_worker_command(
        &resource_context.into_command(),
        "ui-local-resource",
        radiant::prelude::TaskPriority::Background,
    );
    assert_eq!(*resource_state.borrow(), 0);
}

#[test]
fn streaming_business_families_use_worker_effects_with_ui_local_mappers() {
    let mut ordinary = radiant::prelude::UiUpdateContext::default();
    let ordinary_state = std::rc::Rc::new(std::cell::RefCell::new(0_u8));
    let ordinary_event_state = std::rc::Rc::clone(&ordinary_state);
    let ordinary_final_state = std::rc::Rc::clone(&ordinary_state);
    ordinary.business().background("stream-ordinary").stream(
        |_context, events| {
            assert!(events.emit(1_u8));
            2_u8
        },
        move |_| {
            *ordinary_event_state.borrow_mut() += 1;
            DemoMessage::Increment
        },
        move |_| {
            *ordinary_final_state.borrow_mut() += 1;
            DemoMessage::Increment
        },
    );
    assert_worker_command(
        &ordinary.into_command(),
        "stream-ordinary",
        radiant::prelude::TaskPriority::Background,
    );
    assert_eq!(*ordinary_state.borrow(), 0);

    let mut latest = radiant::prelude::LatestTask::new();
    let mut latest_context = radiant::prelude::UiUpdateContext::default();
    let latest_state = std::rc::Rc::new(std::cell::RefCell::new(0_u8));
    let latest_event_state = std::rc::Rc::clone(&latest_state);
    let latest_final_state = std::rc::Rc::clone(&latest_state);
    latest_context
        .business()
        .background("stream-latest")
        .latest(&mut latest)
        .stream_latest(
            |_context, events| {
                assert!(events.emit(1_u8));
                2_u8
            },
            move |_| {
                *latest_event_state.borrow_mut() += 1;
                DemoMessage::Increment
            },
            move |_| {
                *latest_final_state.borrow_mut() += 1;
                DemoMessage::Increment
            },
        );
    assert_worker_command(
        &latest_context.into_command(),
        "stream-latest",
        radiant::prelude::TaskPriority::Background,
    );
    assert_eq!(*latest_state.borrow(), 0);

    let mut cancellable = radiant::prelude::UiUpdateContext::default();
    let cancellable_state = std::rc::Rc::new(std::cell::RefCell::new(0_u8));
    let cancellable_event_state = std::rc::Rc::clone(&cancellable_state);
    let cancellable_final_state = std::rc::Rc::clone(&cancellable_state);
    cancellable
        .business()
        .background("stream-cancellable")
        .cancellable()
        .stream(
            |_context, events| {
                assert!(events.emit(1_u8));
                2_u8
            },
            move |_| {
                *cancellable_event_state.borrow_mut() += 1;
                DemoMessage::Increment
            },
            move |_| {
                *cancellable_final_state.borrow_mut() += 1;
                DemoMessage::Increment
            },
        );
    assert_worker_command(
        &cancellable.into_command(),
        "stream-cancellable",
        radiant::prelude::TaskPriority::Background,
    );
    assert_eq!(*cancellable_state.borrow(), 0);

    let mut keyed_tasks = radiant::prelude::KeyedLatestTasks::new();
    let mut keyed = radiant::prelude::UiUpdateContext::default();
    let keyed_state = std::rc::Rc::new(std::cell::RefCell::new(0_u8));
    let keyed_event_state = std::rc::Rc::clone(&keyed_state);
    let keyed_final_state = std::rc::Rc::clone(&keyed_state);
    keyed
        .business()
        .background("stream-keyed")
        .latest_for(&mut keyed_tasks, "row-1")
        .stream_latest(
            |_context, events| {
                assert!(events.emit(1_u8));
                2_u8
            },
            move |_| {
                *keyed_event_state.borrow_mut() += 1;
                DemoMessage::Increment
            },
            move |_| {
                *keyed_final_state.borrow_mut() += 1;
                DemoMessage::Increment
            },
        );
    assert_worker_command(
        &keyed.into_command(),
        "stream-keyed",
        radiant::prelude::TaskPriority::Background,
    );
    assert_eq!(*keyed_state.borrow(), 0);

    let mut resources = radiant::prelude::ResourceTasks::default();
    let mut resource = radiant::prelude::UiUpdateContext::default();
    let resource_state = std::rc::Rc::new(std::cell::RefCell::new(0_u8));
    let resource_event_state = std::rc::Rc::clone(&resource_state);
    let resource_final_state = std::rc::Rc::clone(&resource_state);
    resource
        .business()
        .background("stream-resource")
        .latest_for_resource(
            &mut resources,
            radiant::prelude::ResourceKey::scoped("kind", "id"),
        )
        .stream(
            |_context, events| {
                assert!(events.emit(1_u8));
                2_u8
            },
            move |_| {
                *resource_event_state.borrow_mut() += 1;
                DemoMessage::Increment
            },
            move |_| {
                *resource_final_state.borrow_mut() += 1;
                DemoMessage::Increment
            },
        );
    assert_worker_command(
        &resource.into_command(),
        "stream-resource",
        radiant::prelude::TaskPriority::Background,
    );
    assert_eq!(*resource_state.borrow(), 0);
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

        let command = context.into_command();
        assert_worker_command(&command, expected_name, expected_priority);
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

    let command = context.into_command();
    assert_worker_command(
        &command,
        "host-selected-work",
        radiant::prelude::TaskPriority::BlockingIo,
    );
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
    let command = context.into_command();
    assert_worker_command(
        &command,
        "keyed-preview",
        radiant::prelude::TaskPriority::Interactive,
    );
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
    let command = context.into_command();
    assert_worker_command(
        &command,
        "resource-preview",
        radiant::prelude::TaskPriority::Interactive,
    );

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
    let command = context.into_command();
    assert_worker_command(
        &command,
        "load-preview",
        radiant::prelude::TaskPriority::Background,
    );
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

    let command = context.into_command();
    assert_worker_command(
        &command,
        "cancel-visible",
        radiant::prelude::TaskPriority::Background,
    );
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
fn app_platform_completion_is_deferred_and_mapped_once_on_ui_owner() {
    use radiant::prelude as ui;
    use std::{
        cell::RefCell,
        rc::Rc,
        thread,
        time::{Duration, Instant},
    };

    let calls = Rc::new(RefCell::new(0usize));
    let mapper_calls = Rc::clone(&calls);
    let bridge = ui::app(DemoState::default())
        .view(|_| ui::text("Platform"))
        .handle_message(|_, _message: DemoMessage, _context| {})
        .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(180.0, 48.0));
    let outcome = runtime.execute_command(Command::platform_request(
        radiant::runtime::PlatformRequest::CopyFilePaths(Vec::new()),
        move |result| {
            assert!(
                result.is_err(),
                "empty file-path copy should fail deterministically"
            );
            *mapper_calls.borrow_mut() += 1;
            DemoMessage::Increment
        },
    ));

    assert_eq!(outcome.messages_dispatched, 0);
    assert_eq!(*calls.borrow(), 0);
    assert_eq!(Rc::strong_count(&calls), 2);

    let deadline = Instant::now() + Duration::from_secs(1);
    loop {
        let drained = runtime.drain_runtime_messages();
        if drained.messages_dispatched > 0 || Instant::now() >= deadline {
            assert_eq!(drained.messages_dispatched, 1);
            break;
        }
        thread::sleep(Duration::from_millis(1));
    }
    assert_eq!(*calls.borrow(), 1);
    assert_eq!(Rc::strong_count(&calls), 1);
    assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 0);
}

#[test]
fn platform_helpers_accept_ui_local_completion_capture_and_message() {
    #[derive(Clone)]
    struct UiOnlyMessage(std::rc::Rc<std::cell::RefCell<usize>>);

    let state = std::rc::Rc::new(std::cell::RefCell::new(0usize));
    let captured = std::rc::Rc::clone(&state);
    let mut context = radiant::prelude::UiUpdateContext::default();
    context.read_text(move |_| {
        *captured.borrow_mut() += 1;
        UiOnlyMessage(std::rc::Rc::clone(&captured))
    });

    let command = context.into_command();
    let radiant::runtime::Command::PlatformRequest { on_completed, .. } = command else {
        panic!("read_text should queue a platform request");
    };
    let message = on_completed(Ok(radiant::runtime::PlatformResponse::Text(String::new())));

    assert!(std::rc::Rc::ptr_eq(&message.0, &state));
    assert_eq!(*state.borrow(), 1);
}

#[test]
fn platform_response_helpers_cover_common_request_outcomes() {
    let path = std::path::PathBuf::from(r"C:\samples\kick.wav");
    let response = radiant::runtime::PlatformResponse::Path(path.clone());

    assert_eq!(response.path(), Some(path.as_path()));
    assert_eq!(response.clone().into_path(), Some(path.clone()));
    assert_eq!(response.into_path_or_canceled(), Ok(Some(path)));

    assert_eq!(
        radiant::runtime::PlatformResponse::Canceled.into_path_or_canceled(),
        Ok(None)
    );
    assert!(radiant::runtime::PlatformResponse::Canceled.is_canceled());

    assert!(radiant::runtime::PlatformResponse::Completed.is_completed());
    assert_eq!(
        radiant::runtime::PlatformResponse::Completed.into_completed(),
        Ok(())
    );

    let confirmation = radiant::runtime::PlatformResponse::Confirmation(
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

    let text = radiant::runtime::PlatformResponse::Text(String::from("C:/samples/kick.wav"));
    assert_eq!(text.into_text(), Some(String::from("C:/samples/kick.wav")));

    let paths = vec![std::path::PathBuf::from(r"C:\samples\kick.wav")];
    let file_paths = radiant::runtime::PlatformResponse::FilePaths(paths.clone());
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
        radiant::prelude::ConfirmDialogRequest::from_parts(radiant::runtime::ConfirmDialogParts {
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

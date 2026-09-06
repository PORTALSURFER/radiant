use radiant::{
    application::{
        IntoView, Resource, ResourceInterest, ResourceInterestError, ResourceInterestKind,
        SharedResourceTaskMode, SharedResourceTasks, column, resource, text,
    },
    gui::types::Vector2,
    runtime::{
        Command, Effect, EffectOwner, RuntimeBridge, TaskPriority, UiSurface,
        testing::DeterministicHost,
    },
};
use std::{cell::Cell, rc::Rc, sync::Arc};

struct ViewState {
    tasks: SharedResourceTasks,
    key: &'static str,
    first_visible: Rc<Cell<bool>>,
    second_visible: Rc<Cell<bool>>,
    kind: Rc<Cell<ResourceInterestKind>>,
}

struct ViewBridge {
    state: ViewState,
}

impl ViewBridge {
    fn resource_view(&self, interest_id: u64) -> radiant::application::ViewNode<()> {
        resource(Resource::<u8, &'static str>::new(self.state.key).snapshot())
            .idle(text("idle resource"))
            .interest(&self.state.tasks, interest_id, self.state.kind.get())
            .into_view()
            .key(format!("consumer-{interest_id}"))
    }
}

impl RuntimeBridge<()> for ViewBridge {
    #[allow(clippy::arc_with_non_send_sync)]
    fn project_surface(&mut self) -> Arc<UiSurface<()>> {
        let mut views = Vec::new();
        if self.state.first_visible.get() {
            views.push(self.resource_view(1));
        }
        if self.state.second_visible.get() {
            views.push(self.resource_view(2));
        }
        Arc::new(column(views).into_surface())
    }
}

fn host(
    tasks: SharedResourceTasks,
    key: &'static str,
    first_visible: Rc<Cell<bool>>,
    second_visible: Rc<Cell<bool>>,
    kind: Rc<Cell<ResourceInterestKind>>,
) -> DeterministicHost<ViewBridge, ()> {
    DeterministicHost::with_default_config(
        ViewBridge {
            state: ViewState {
                tasks,
                key,
                first_visible,
                second_visible,
                kind,
            },
        },
        Vector2::new(160.0, 80.0),
    )
    .expect("deterministic host")
}

fn refresh(host: &mut DeterministicHost<ViewBridge, ()>) {
    host.refresh().expect("resource view refresh");
}

#[test]
fn accepted_projection_admits_only_visible_resource_views_and_retires_them_on_refresh() {
    let tasks = SharedResourceTasks::new();
    let first_visible = Rc::new(Cell::new(true));
    let second_visible = Rc::new(Cell::new(false));
    let kind = Rc::new(Cell::new(ResourceInterestKind::Visible));
    let mut host = host(
        tasks.clone(),
        "projected-resource",
        Rc::clone(&first_visible),
        Rc::clone(&second_visible),
        Rc::clone(&kind),
    );

    assert_eq!(host.runtime().resource_view_interest_status().active, 1);
    assert_eq!(tasks.interest_count(), 1);
    refresh(&mut host);
    assert_eq!(host.runtime().resource_view_interest_status().active, 1);
    assert_eq!(tasks.interest_count(), 1);

    // Building an unprojected branch is pure metadata construction.
    drop(
        resource(Resource::<u8, &'static str>::new("discarded").snapshot())
            .idle(text::<()>("discarded"))
            .interest(&tasks, 99, ResourceInterestKind::Persistent)
            .into_view(),
    );
    assert_eq!(tasks.interest_count(), 1);

    first_visible.set(false);
    refresh(&mut host);
    assert_eq!(host.runtime().resource_view_interest_status().active, 0);
    assert_eq!(tasks.interest_count(), 0);

    drop(host);
    assert_eq!(tasks.interest_count(), 0);
}

#[test]
fn same_key_views_keep_one_operation_alive_across_removal_and_kind_transition() {
    let tasks = SharedResourceTasks::new();
    let first_visible = Rc::new(Cell::new(true));
    let second_visible = Rc::new(Cell::new(true));
    let kind = Rc::new(Cell::new(ResourceInterestKind::Visible));
    let mut host = host(
        tasks.clone(),
        "shared-projected-resource",
        Rc::clone(&first_visible),
        Rc::clone(&second_visible),
        Rc::clone(&kind),
    );
    assert_eq!(host.runtime().resource_view_interest_status().active, 2);
    refresh(&mut host);
    assert_eq!(host.runtime().resource_view_interest_status().active, 2);
    assert_eq!(tasks.interest_count(), 2);

    let effect = Effect::resource_worker(
        &tasks,
        "shared-projected-resource",
        SharedResourceTaskMode::Join,
        "resource-view-test",
        TaskPriority::Background,
        || (),
        |_| (),
    )
    .expect("operation admission")
    .expect("first worker");
    host.execute_command(Command::effect(effect))
        .expect("worker admission");
    let stale_worker = host.pending_worker_tasks().first().expect("old worker").id;
    let operation = tasks
        .operation(&"shared-projected-resource".into())
        .expect("current operation");
    assert!(
        Effect::resource_worker(
            &tasks,
            "shared-projected-resource",
            SharedResourceTaskMode::Join,
            "resource-view-join",
            TaskPriority::Background,
            || (),
            |_| (),
        )
        .expect("join admission")
        .is_none()
    );

    first_visible.set(false);
    refresh(&mut host);
    assert_eq!(host.runtime().resource_view_interest_status().active, 1);
    assert_eq!(tasks.interest_count(), 1);
    assert_eq!(
        tasks
            .operation(&"shared-projected-resource".into())
            .expect("operation kept by second view")
            .operation_id(),
        operation.operation_id()
    );

    kind.set(ResourceInterestKind::Prefetch);
    refresh(&mut host);
    assert_eq!(host.runtime().resource_view_interest_status().active, 1);
    assert_eq!(
        tasks
            .operation(&"shared-projected-resource".into())
            .expect("kind transition preserves operation")
            .operation_id(),
        operation.operation_id()
    );

    second_visible.set(false);
    refresh(&mut host);
    assert_eq!(tasks.interest_count(), 0);
    assert!(
        tasks
            .operation(&"shared-projected-resource".into())
            .is_none()
    );

    second_visible.set(true);
    refresh(&mut host);
    let replacement = Effect::resource_worker(
        &tasks,
        "shared-projected-resource",
        SharedResourceTaskMode::Join,
        "resource-view-reinsert",
        TaskPriority::Background,
        || (),
        |_| (),
    )
    .expect("replacement admission")
    .expect("replacement worker");
    let replacement_operation = tasks
        .operation(&"shared-projected-resource".into())
        .expect("replacement operation");
    assert_ne!(
        replacement_operation.operation_id(),
        operation.operation_id()
    );
    host.complete_worker(stale_worker)
        .expect("late stale worker completion");
    host.turn().expect("late completion turn");
    assert_eq!(
        tasks
            .operation(&"shared-projected-resource".into())
            .expect("stale completion cannot replace reinserted work")
            .operation_id(),
        replacement_operation.operation_id()
    );
    drop(replacement);
}

enum AppInterestMessage {
    Admitted(Result<ResourceInterest, ResourceInterestError>),
}

struct AppInterestBridge {
    tasks: SharedResourceTasks,
    received: Vec<ResourceInterest>,
}

impl RuntimeBridge<AppInterestMessage> for AppInterestBridge {
    #[allow(clippy::arc_with_non_send_sync)]
    fn project_surface(&mut self) -> Arc<UiSurface<AppInterestMessage>> {
        Arc::new(
            resource(Resource::<u8, &'static str>::new("app-namespace").snapshot())
                .idle(text("projected"))
                .interest(&self.tasks, 7, ResourceInterestKind::Visible)
                .into_view()
                .into_surface(),
        )
    }

    fn update(&mut self, message: AppInterestMessage) -> Command<AppInterestMessage> {
        match message {
            AppInterestMessage::Admitted(Ok(interest)) => self.received.push(interest),
            AppInterestMessage::Admitted(Err(error)) => {
                panic!("unexpected explicit application interest rejection: {error:?}");
            }
        }
        Command::none()
    }
}

#[test]
fn shutdown_releases_projected_and_explicit_application_interests_in_separate_namespaces() {
    let tasks = SharedResourceTasks::new();
    let mut host = DeterministicHost::with_default_config(
        AppInterestBridge {
            tasks: tasks.clone(),
            received: Vec::new(),
        },
        Vector2::new(160.0, 80.0),
    )
    .expect("deterministic host");
    assert_eq!(tasks.interest_count(), 1);

    host.execute_command(tasks.interest(
        "app-namespace",
        EffectOwner::Application,
        7,
        ResourceInterestKind::Persistent,
        AppInterestMessage::Admitted,
    ))
    .expect("explicit application admission");
    assert_eq!(host.bridge().received.len(), 1);
    assert_eq!(tasks.interest_count(), 2);

    drop(host);
    assert_eq!(tasks.interest_count(), 0);
}

struct DemandCountBridge {
    tasks: SharedResourceTasks,
    count: Rc<Cell<usize>>,
}

impl RuntimeBridge<()> for DemandCountBridge {
    #[allow(clippy::arc_with_non_send_sync)]
    fn project_surface(&mut self) -> Arc<UiSurface<()>> {
        let views = (0..self.count.get())
            .map(|index| {
                resource(Resource::<u8, &'static str>::new("overflow-source").snapshot())
                    .idle(text("resource"))
                    .interest(&self.tasks, index as u64, ResourceInterestKind::Visible)
                    .into_view()
                    .key(format!("overflow-consumer-{index}"))
            })
            .collect::<Vec<_>>();
        Arc::new(column(views).into_surface())
    }
}

#[test]
fn excessive_projected_resource_demands_clear_existing_leases_before_admission() {
    let tasks = SharedResourceTasks::new();
    let count = Rc::new(Cell::new(1));
    let mut host = DeterministicHost::with_default_config(
        DemandCountBridge {
            tasks: tasks.clone(),
            count: Rc::clone(&count),
        },
        Vector2::new(160.0, 80.0),
    )
    .expect("deterministic host");
    assert_eq!(host.runtime().resource_view_interest_status().active, 1);
    assert_eq!(tasks.interest_count(), 1);

    count.set(1025);
    host.refresh().expect("overflow refresh");
    let status = host.runtime().resource_view_interest_status();
    assert!(status.source_invalid);
    assert_eq!(status.active, 0);
    assert_eq!(tasks.interest_count(), 0);
}

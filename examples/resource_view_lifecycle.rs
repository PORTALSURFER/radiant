//! Headless resource-view lifecycle fixture with application-owned typed state.
use radiant::{
    application::{
        IntoView, Resource, ResourceCancelIntent, ResourceInterestKind, ResourceRefreshPolicy,
        ResourceRetryIntent, SharedResourceCompletion, SharedResourceTaskMode, SharedResourceTasks,
        column, resource, text,
    },
    gui::types::Vector2,
    runtime::{
        Command, Effect, RuntimeBridge, TaskPriority, UiSurface, testing::DeterministicHost,
    },
};
use std::sync::Arc;

enum Message {
    Load(Result<u32, &'static str>),
    Loaded(SharedResourceCompletion<Result<u32, &'static str>>),
    HideFirst,
    HideAll,
    Cancel(ResourceCancelIntent),
    Retry(ResourceRetryIntent),
}
struct Model {
    tasks: SharedResourceTasks,
    resource: Resource<u32, &'static str>,
    first: bool,
    second: bool,
}
impl RuntimeBridge<Message> for Model {
    #[allow(clippy::arc_with_non_send_sync)]
    fn project_surface(&mut self) -> Arc<UiSurface<Message>> {
        let mut consumers = Vec::new();
        for (id, visible) in [(1, self.first), (2, self.second)] {
            if visible {
                consumers.push(
                    resource(self.resource.snapshot())
                        .idle(text("Choose Load"))
                        .pending(text("Loading"))
                        .ready(|value| text(format!("Value: {value}")))
                        .refreshing(|previous| text(format!("Refreshing value: {previous}")))
                        .failed_with_ready(|error, previous| {
                            text(format!("{error}; previous: {previous:?}"))
                        })
                        .cancelled(text("Cancelled"))
                        .interest(&self.tasks, id, ResourceInterestKind::Visible)
                        .into_view()
                        .key(format!("consumer-{id}")),
                );
            }
        }
        Arc::new(column(consumers).into_surface())
    }
    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::Load(result) => {
                let effect = Effect::resource_worker(
                    &self.tasks,
                    "example",
                    SharedResourceTaskMode::Refresh,
                    "resource-view-fixture",
                    TaskPriority::Background,
                    move || result,
                    Message::Loaded,
                )
                .expect("resource reservation")
                .expect("explicit refresh");
                let operation = self
                    .tasks
                    .operation(&"example".into())
                    .expect("reserved operation");
                self.resource
                    .begin(operation, ResourceRefreshPolicy::RetainReady)
                    .expect("resource state");
                return effect.into();
            }
            Message::Loaded(completion) => {
                self.resource.apply_completion(&self.tasks, completion);
            }
            Message::HideFirst => self.first = false,
            Message::HideAll => {
                self.first = false;
                self.second = false;
            }
            Message::Cancel(intent) => {
                self.resource.cancel_intent(&self.tasks, &intent);
            }
            Message::Retry(intent) => {
                if self.resource.take_retry(&intent) {
                    return self.update(Message::Load(Ok(43)));
                }
            }
        }
        Command::none()
    }
}
fn complete(host: &mut DeterministicHost<Model, Message>) {
    let id = host.pending_worker_tasks()[0].id;
    host.complete_worker(id).expect("worker terminal");
    host.turn().expect("completion turn");
}
fn main() {
    let tasks = SharedResourceTasks::new();
    let mut host = DeterministicHost::with_default_config(
        Model {
            tasks: tasks.clone(),
            resource: Resource::new("example"),
            first: true,
            second: true,
        },
        Vector2::new(240.0, 80.0),
    )
    .expect("deterministic host");
    assert_eq!(tasks.interest_count(), 2);
    host.execute_command(Command::message(Message::Load(Ok(42))))
        .expect("load");
    host.execute_command(Command::message(Message::HideFirst))
        .expect("hide first");
    complete(&mut host);
    assert_eq!(
        host.bridge().resource.value().map(|value| **value),
        Some(42)
    );
    println!("{{\"phase\":\"ready\",\"interests\":1,\"value\":42}}");
    host.execute_command(Command::message(Message::Load(Err("refresh failed"))))
        .expect("refresh");
    complete(&mut host);
    assert_eq!(
        host.bridge().resource.value().map(|value| **value),
        Some(42)
    );
    println!("{{\"phase\":\"failed\",\"retained_value\":42}}");
    let retry = host
        .bridge()
        .resource
        .snapshot()
        .retry_intent()
        .expect("retry intent");
    host.execute_command(Command::message(Message::Retry(retry)))
        .expect("retry");
    let cancel = host
        .bridge()
        .resource
        .snapshot()
        .cancel_intent()
        .expect("cancel intent");
    host.execute_command(Command::message(Message::Cancel(cancel)))
        .expect("cancel");
    complete(&mut host);
    assert_eq!(
        host.bridge().resource.value().map(|value| **value),
        Some(42)
    );
    println!("{{\"phase\":\"cancelled_refresh\",\"late_value_applied\":false}}");
    host.execute_command(Command::message(Message::HideAll))
        .expect("hide all");
    assert_eq!(tasks.interest_count(), 0);
    println!("{{\"phase\":\"released\",\"interests\":0}}");
}

//! Deterministic shared-resource lifecycle fixture; no native window or IO.

use radiant::{
    application::{
        ResourceInterest, ResourceInterestError, ResourceInterestKind, SharedResourceCompletion,
        SharedResourceTaskMode, SharedResourceTasks,
    },
    gui::types::Vector2,
    runtime::{
        Command, Effect, EffectOwner, RuntimeBridge, SurfaceNode, TaskPriority, UiSurface,
        testing::DeterministicHost,
    },
};
use std::sync::Arc;

enum Message {
    Interest(Result<ResourceInterest, ResourceInterestError>),
    Loaded(SharedResourceCompletion<u32>),
}

struct Resources {
    tasks: SharedResourceTasks,
    interests: Vec<ResourceInterest>,
    value: Option<u32>,
}

impl RuntimeBridge<Message> for Resources {
    #[allow(clippy::arc_with_non_send_sync)]
    fn project_surface(&mut self) -> Arc<UiSurface<Message>> {
        Arc::new(UiSurface::new(SurfaceNode::column(1, 0.0, Vec::new())))
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::Interest(result) => self.interests.push(result.expect("accepted interest")),
            Message::Loaded(completion) => {
                if let Some(value) = self.tasks.finish_ready(completion) {
                    self.value = Some(value);
                }
            }
        }
        Command::none()
    }
}

fn load(
    tasks: &SharedResourceTasks,
    mode: SharedResourceTaskMode,
    value: u32,
) -> Option<Effect<Message>> {
    Effect::resource_worker(
        tasks,
        "fixture",
        mode,
        "resource-fixture",
        TaskPriority::Background,
        move || value,
        Message::Loaded,
    )
    .expect("bounded operation admission")
}

fn main() {
    let tasks = SharedResourceTasks::new();
    let mut host = DeterministicHost::with_default_config(
        Resources {
            tasks: tasks.clone(),
            interests: Vec::new(),
            value: None,
        },
        Vector2::new(160.0, 80.0),
    )
    .expect("deterministic host");
    for id in [1, 2] {
        host.execute_command(tasks.interest(
            "fixture",
            EffectOwner::Application,
            id,
            ResourceInterestKind::Visible,
            Message::Interest,
        ))
        .expect("interest command");
    }
    let effect = load(&tasks, SharedResourceTaskMode::Join, 42).expect("first work");
    host.execute_command(effect.into())
        .expect("worker admission");
    assert!(load(&tasks, SharedResourceTaskMode::Join, 99).is_none());
    assert_eq!(host.pending_worker_tasks().len(), 1);
    println!(
        "{{\"phase\":\"shared\",\"interests\":{},\"workers\":1}}",
        tasks.interest_count()
    );

    host.bridge().interests[0].release();
    host.complete_worker(host.pending_worker_tasks()[0].id)
        .expect("worker completion");
    host.turn().expect("UI completion turn");
    assert_eq!(host.bridge().value, Some(42));
    println!(
        "{{\"phase\":\"starter_released\",\"interests\":{},\"value\":42}}",
        tasks.interest_count()
    );

    tasks
        .retain_ready("fixture", true)
        .expect("ready retention");
    host.bridge().interests[1].release();
    host.execute_command(tasks.interest(
        "fixture",
        EffectOwner::Application,
        3,
        ResourceInterestKind::Prefetch,
        Message::Interest,
    ))
    .expect("reacquisition");
    assert!(load(&tasks, SharedResourceTaskMode::Join, 99).is_none());
    println!("{{\"phase\":\"ready_reused\",\"new_workers\":0}}");

    let refresh = load(&tasks, SharedResourceTaskMode::Refresh, 43).expect("explicit refresh");
    host.execute_command(refresh.into())
        .expect("refresh admission");
    host.bridge().interests[2].release();
    host.complete_worker(host.pending_worker_tasks()[0].id)
        .expect("stale worker drain");
    host.turn().expect("stale result turn");
    assert_eq!(host.bridge().value, Some(42));
    println!("{{\"phase\":\"final_release\",\"interests\":0,\"late_value_applied\":false}}");
    tasks.shutdown();
}

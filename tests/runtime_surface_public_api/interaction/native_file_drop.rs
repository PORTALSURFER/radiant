use super::super::*;
use radiant::runtime::{NativeFileDrop, NativeFileDropPhase};
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq)]
enum DropMessage {
    Target {
        phase: NativeFileDropPhase,
        path: Option<PathBuf>,
        target: Option<u64>,
    },
    Fallback {
        target: Option<u64>,
    },
}

#[derive(Default)]
struct NativeDropBridge {
    messages: Arc<Mutex<Vec<DropMessage>>>,
}

impl NativeDropBridge {
    fn messages(&self) -> Arc<Mutex<Vec<DropMessage>>> {
        Arc::clone(&self.messages)
    }
}

impl RuntimeBridge<DropMessage> for NativeDropBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<DropMessage>> {
        Arc::new(UiSurface::new(
            ui::text("Drop target")
                .id(10)
                .size(100.0, 40.0)
                .on_native_file_drop(target_drop_message)
                .into_node(),
        ))
    }

    fn update(&mut self, message: DropMessage) -> Command<DropMessage> {
        self.messages.lock().expect("drop messages").push(message);
        Command::none()
    }
}

#[test]
fn native_file_drop_routes_to_declarative_view_target() {
    let bridge = NativeDropBridge::default();
    let messages = bridge.messages();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(120.0, 60.0));
    let path = PathBuf::from("sample.wav");

    runtime.dispatch_native_file_drop(NativeFileDrop::dropped(
        path.clone(),
        Some(Point::new(8.0, 8.0)),
        None,
    ));

    assert_eq!(
        messages.lock().expect("drop messages").as_slice(),
        &[DropMessage::Target {
            phase: NativeFileDropPhase::Drop,
            path: Some(path),
            target: Some(10),
        }]
    );
}

#[test]
fn native_file_drop_prefers_topmost_declarative_target() {
    let messages = Arc::new(Mutex::new(Vec::new()));
    let events = Arc::clone(&messages);
    let bridge = declarative_runtime_bridge(
        (),
        |_state: &mut ()| {
            Arc::new(UiSurface::new(
                ui::stack([
                    ui::text("Bottom")
                        .id(10)
                        .size(100.0, 40.0)
                        .on_native_file_drop(target_drop_message),
                    ui::text("Top")
                        .id(20)
                        .size(100.0, 40.0)
                        .on_native_file_drop(target_drop_message),
                ])
                .into_node(),
            ))
        },
        move |_state: &mut (), message| {
            events.lock().expect("drop messages").push(message);
        },
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(120.0, 60.0));

    runtime.dispatch_native_file_drop(NativeFileDrop::hover(
        PathBuf::from("sample.wav"),
        Some(Point::new(8.0, 8.0)),
        None,
    ));

    assert_eq!(
        messages.lock().expect("drop messages").as_slice(),
        &[DropMessage::Target {
            phase: NativeFileDropPhase::Hover,
            path: Some(PathBuf::from("sample.wav")),
            target: Some(20),
        }]
    );
}

#[test]
fn native_file_drop_falls_back_to_app_hook_without_declarative_target() {
    #[derive(Default)]
    struct FallbackBridge {
        messages: Arc<Mutex<Vec<DropMessage>>>,
    }

    impl RuntimeBridge<DropMessage> for FallbackBridge {
        fn project_surface(&mut self) -> Arc<UiSurface<DropMessage>> {
            Arc::new(UiSurface::new(
                ui::button("Passive fallback target")
                    .message(DropMessage::Fallback { target: None })
                    .id(10)
                    .size(100.0, 40.0)
                    .into_node(),
            ))
        }

        fn native_file_drop(&mut self, drop: NativeFileDrop) -> Command<DropMessage> {
            Command::message(DropMessage::Fallback {
                target: drop.target_widget,
            })
        }

        fn update(&mut self, message: DropMessage) -> Command<DropMessage> {
            self.messages.lock().expect("drop messages").push(message);
            Command::none()
        }
    }

    let bridge = FallbackBridge::default();
    let messages = Arc::clone(&bridge.messages);
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(120.0, 60.0));

    runtime.dispatch_native_file_drop(NativeFileDrop::dropped(
        PathBuf::from("sample.wav"),
        Some(Point::new(8.0, 8.0)),
        None,
    ));

    assert_eq!(
        messages.lock().expect("drop messages").as_slice(),
        &[DropMessage::Fallback { target: Some(10) }]
    );
}

#[test]
fn native_file_drop_accepting_view_without_mapper_falls_back_with_declarative_target() {
    #[derive(Default)]
    struct FallbackBridge {
        messages: Arc<Mutex<Vec<DropMessage>>>,
    }

    impl RuntimeBridge<DropMessage> for FallbackBridge {
        fn project_surface(&mut self) -> Arc<UiSurface<DropMessage>> {
            Arc::new(UiSurface::new(
                ui::text("Declarative fallback target")
                    .id(10)
                    .size(100.0, 40.0)
                    .accepts_native_file_drop()
                    .into_node(),
            ))
        }

        fn native_file_drop(&mut self, drop: NativeFileDrop) -> Command<DropMessage> {
            Command::message(DropMessage::Fallback {
                target: drop.target_widget,
            })
        }

        fn update(&mut self, message: DropMessage) -> Command<DropMessage> {
            self.messages.lock().expect("drop messages").push(message);
            Command::none()
        }
    }

    let bridge = FallbackBridge::default();
    let messages = Arc::clone(&bridge.messages);
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(120.0, 60.0));

    runtime.dispatch_native_file_drop(NativeFileDrop::dropped(
        PathBuf::from("sample.wav"),
        Some(Point::new(8.0, 8.0)),
        None,
    ));

    assert_eq!(
        messages.lock().expect("drop messages").as_slice(),
        &[DropMessage::Fallback { target: Some(10) }]
    );
}

#[test]
fn native_file_drop_without_position_routes_to_topmost_declarative_target() {
    let bridge = NativeDropBridge::default();
    let messages = bridge.messages();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(120.0, 60.0));

    runtime.dispatch_native_file_drop(NativeFileDrop::dropped(
        PathBuf::from("sample.wav"),
        None,
        None,
    ));

    assert_eq!(
        messages.lock().expect("drop messages").as_slice(),
        &[DropMessage::Target {
            phase: NativeFileDropPhase::Drop,
            path: Some(PathBuf::from("sample.wav")),
            target: Some(10),
        }]
    );
}

#[test]
fn native_file_drop_target_does_not_become_pointer_hit_target() {
    let bridge = NativeDropBridge::default();
    let runtime = SurfaceRuntime::new(bridge, Vector2::new(120.0, 60.0));

    assert_eq!(runtime.widget_at(Point::new(8.0, 8.0)), None);
}

fn target_drop_message(drop: NativeFileDrop) -> DropMessage {
    DropMessage::Target {
        phase: drop.phase,
        path: drop.path,
        target: drop.target_widget,
    }
}

use super::*;
use radiant::runtime::{
    Command, DeclarativeCommandRuntimeBridge, DeclarativeCommandRuntimeBridgeParts,
    DeclarativeOwnedCommandRuntimeBridge, DeclarativeOwnedCommandRuntimeBridgeParts,
    SurfaceRuntime, declarative_command_runtime_bridge, declarative_owned_command_runtime_bridge,
};

#[test]
fn declarative_command_bridge_supports_command_update_flow() {
    let bridge = declarative_command_runtime_bridge(
        DemoState::default(),
        project_command_surface,
        |state: &mut DemoState, message| match message {
            CommandDemoMessage::Start => Command::batch([
                Command::message(CommandDemoMessage::Rename(String::from("Closure"))),
                Command::message(CommandDemoMessage::Increment),
                Command::request_repaint(),
            ]),
            CommandDemoMessage::Increment => {
                state.count += 1;
                Command::none()
            }
            CommandDemoMessage::Rename(name) => {
                state.name = name;
                Command::none()
            }
            CommandDemoMessage::ShowListAndReveal => {
                state.show_list = true;
                Command::scroll_fixed_row_into_view(31, 18, 24.0, 2, 2, 1)
            }
            CommandDemoMessage::ScrollReported(offset_y) => {
                state.scroll_reports.push(offset_y);
                Command::request_paint_only()
            }
        },
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(420.0, 32.0));

    let outcome = runtime.dispatch_message(CommandDemoMessage::Start);

    assert_eq!(outcome.messages_dispatched, 3);
    assert!(outcome.repaint_requested);
    assert_eq!(runtime.bridge().state().count, 1);
    assert_eq!(runtime.bridge().state().name, "Closure");
}

#[test]
fn command_update_flow_scrolls_newly_projected_surfaces() {
    let bridge = declarative_owned_command_runtime_bridge(
        DemoState::default(),
        project_owned_command_surface,
        |state: &mut DemoState, message| match message {
            CommandDemoMessage::Start => Command::none(),
            CommandDemoMessage::Increment => {
                state.count += 1;
                Command::none()
            }
            CommandDemoMessage::Rename(name) => {
                state.name = name;
                Command::none()
            }
            CommandDemoMessage::ShowListAndReveal => {
                state.show_list = true;
                Command::scroll_fixed_row_into_view(31, 18, 24.0, 2, 2, 1)
            }
            CommandDemoMessage::ScrollReported(offset_y) => {
                state.scroll_reports.push(offset_y);
                Command::request_paint_only()
            }
        },
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(420.0, 72.0));

    let outcome = runtime.dispatch_message(CommandDemoMessage::ShowListAndReveal);

    assert!(outcome.surface_refresh_requested);
    assert!(
        runtime
            .bridge()
            .state()
            .scroll_reports
            .last()
            .is_some_and(|offset| *offset > 0.0),
        "reveal command should run after the list-bearing surface is projected"
    );
}

#[test]
fn owned_command_bridge_supports_runtime_command_flow() {
    let bridge = declarative_owned_command_runtime_bridge(
        DemoState::default(),
        project_owned_command_surface,
        |state: &mut DemoState, message| match message {
            CommandDemoMessage::Start => Command::batch([
                Command::message(CommandDemoMessage::Rename(String::from("Owned"))),
                Command::message(CommandDemoMessage::Increment),
                Command::request_repaint(),
            ]),
            CommandDemoMessage::Increment => {
                state.count += 1;
                Command::none()
            }
            CommandDemoMessage::Rename(name) => {
                state.name = name;
                Command::none()
            }
            CommandDemoMessage::ShowListAndReveal => {
                state.show_list = true;
                Command::scroll_fixed_row_into_view(31, 18, 24.0, 2, 2, 1)
            }
            CommandDemoMessage::ScrollReported(offset_y) => {
                state.scroll_reports.push(offset_y);
                Command::request_paint_only()
            }
        },
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(420.0, 32.0));

    let outcome = runtime.dispatch_message(CommandDemoMessage::Start);

    assert_eq!(outcome.messages_dispatched, 3);
    assert!(outcome.repaint_requested);
    assert_eq!(runtime.bridge().state().count, 1);
    assert_eq!(runtime.bridge().state().name, "Owned");
}

#[test]
fn declarative_command_bridges_support_named_parts_construction() {
    let mut bridge =
        DeclarativeCommandRuntimeBridge::from_parts(DeclarativeCommandRuntimeBridgeParts {
            state: DemoState::default(),
            project: project_command_surface,
            update: |state: &mut DemoState, message| match message {
                CommandDemoMessage::Start => Command::request_repaint(),
                CommandDemoMessage::Increment => {
                    state.count += 1;
                    Command::none()
                }
                CommandDemoMessage::Rename(name) => {
                    state.name = name;
                    Command::none()
                }
                CommandDemoMessage::ShowListAndReveal => {
                    state.show_list = true;
                    Command::scroll_fixed_row_into_view(31, 18, 24.0, 2, 2, 1)
                }
                CommandDemoMessage::ScrollReported(offset_y) => {
                    state.scroll_reports.push(offset_y);
                    Command::request_paint_only()
                }
            },
        });
    assert!(bridge.update(CommandDemoMessage::Start).requests_repaint());
    bridge.reduce_message(CommandDemoMessage::Increment);
    assert_eq!(bridge.state().count, 1);

    let mut owned_bridge = DeclarativeOwnedCommandRuntimeBridge::from_parts(
        DeclarativeOwnedCommandRuntimeBridgeParts {
            state: DemoState::default(),
            project: project_owned_command_surface,
            update: |state: &mut DemoState, message| match message {
                CommandDemoMessage::Start => Command::none(),
                CommandDemoMessage::Increment => {
                    state.count += 1;
                    Command::none()
                }
                CommandDemoMessage::Rename(name) => {
                    state.name = name;
                    Command::none()
                }
                CommandDemoMessage::ShowListAndReveal => {
                    state.show_list = true;
                    Command::scroll_fixed_row_into_view(31, 18, 24.0, 2, 2, 1)
                }
                CommandDemoMessage::ScrollReported(offset_y) => {
                    state.scroll_reports.push(offset_y);
                    Command::request_paint_only()
                }
            },
        },
    );
    owned_bridge.reduce_message(CommandDemoMessage::Rename(String::from("Parts")));
    assert_eq!(owned_bridge.state().name, "Parts");
    assert!(owned_bridge.pull_surface().find_widget(10).is_some());
}

#[test]
fn generic_trait_reduction_updates_shared_and_owned_command_bridges() {
    let mut shared = declarative_command_runtime_bridge(
        DemoState::default(),
        project_command_surface,
        |state: &mut DemoState, message| match message {
            CommandDemoMessage::Increment => {
                state.count += 1;
                Command::request_repaint()
            }
            _ => Command::none(),
        },
    );
    reduce_through_runtime_bridge(&mut shared, CommandDemoMessage::Increment);
    assert_eq!(shared.state().count, 1);

    let mut owned = declarative_owned_command_runtime_bridge(
        DemoState::default(),
        project_owned_command_surface,
        |state: &mut DemoState, message| match message {
            CommandDemoMessage::Rename(name) => {
                state.name = name;
                Command::request_repaint()
            }
            _ => Command::none(),
        },
    );
    reduce_through_runtime_bridge(
        &mut owned,
        CommandDemoMessage::Rename(String::from("Generic")),
    );
    assert_eq!(owned.state().name, "Generic");
}

enum CommandDemoMessage {
    Start,
    Increment,
    Rename(String),
    ShowListAndReveal,
    ScrollReported(f32),
}

fn project_command_surface(state: &mut DemoState) -> Arc<UiSurface<CommandDemoMessage>> {
    if state.show_list {
        let rows = (0..40)
            .map(|index| {
                SurfaceChild::new(
                    SlotParams {
                        size_main: SizeModeMain::Intrinsic,
                        size_cross: SizeModeCross::Fill,
                        constraints: Constraints::unconstrained(),
                        margin: Default::default(),
                        align_cross_override: None,
                        allow_fixed_compress: false,
                    },
                    SurfaceNode::text(
                        100 + index,
                        format!("Row {index}"),
                        WidgetSizing::fixed(Vector2::new(180.0, 24.0)),
                    ),
                )
            })
            .collect();
        let scroll = SurfaceNode::scroll_area(31, SurfaceNode::column(32, 0.0, rows))
            .with_scroll_message(Arc::new(|update| {
                Some(CommandDemoMessage::ScrollReported(update.offset.y))
            }));
        return Arc::new(UiSurface::new(SurfaceNode::column(
            30,
            0.0,
            vec![SurfaceChild::new(
                SlotParams {
                    size_main: SizeModeMain::Fixed(72.0),
                    size_cross: SizeModeCross::Fixed(220.0),
                    constraints: Constraints::unconstrained(),
                    margin: Default::default(),
                    align_cross_override: None,
                    allow_fixed_compress: false,
                },
                scroll,
            )],
        )));
    }
    let title = TextWidget::new(
        10,
        format!("{} ({})", display_name(state), state.count),
        WidgetSizing::fixed(Vector2::new(140.0, 20.0)).with_baseline(14.0),
    );
    let button = ButtonWidget::new(11, "Run", WidgetSizing::fixed(Vector2::new(96.0, 28.0)));
    let input = TextInputWidget::new(
        12,
        state.name.clone(),
        WidgetSizing::new(Vector2::new(120.0, 28.0), Vector2::new(180.0, 28.0)),
    );
    let children = vec![
        SurfaceChild::fill(SurfaceNode::static_widget(title)),
        SurfaceChild::fill(SurfaceNode::widget(
            button,
            WidgetMessageMapper::button(|_| CommandDemoMessage::Start),
        )),
        SurfaceChild::fill(SurfaceNode::widget(input, WidgetMessageMapper::none())),
    ];

    Arc::new(UiSurface::new(SurfaceNode::row(1, 8.0, children)))
}

fn project_owned_command_surface(state: &mut DemoState) -> UiSurface<CommandDemoMessage> {
    Arc::unwrap_or_clone(project_command_surface(state))
}

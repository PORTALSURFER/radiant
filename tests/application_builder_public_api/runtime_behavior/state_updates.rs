use super::*;
use radiant::widgets::Widget;

#[derive(Clone, Debug, PartialEq, Eq)]
enum ContinuityMessage {
    Reorder,
    Replace,
    Hide,
    Show,
}

#[derive(Default)]
struct ContinuityState {
    reordered: bool,
    incompatible: bool,
    visible: bool,
}

fn continuity_view(state: &ContinuityState) -> radiant::prelude::View<ContinuityMessage> {
    use radiant::prelude as ui;

    let tracked: ui::View<ContinuityMessage> = if state.incompatible {
        ui::text("Replacement")
    } else {
        ui::button("Tracked").message(ContinuityMessage::Reorder)
    };
    let tracked = ui::preserve_state(ui::ContinuityKey::from("tracked"), tracked);
    if !state.visible {
        return ui::column([]);
    }
    if state.reordered {
        ui::column([ui::text("Inserted"), tracked])
    } else {
        ui::column([tracked, ui::text("Sibling")])
    }
}

fn continuity_bridge() -> impl radiant::runtime::RuntimeBridge<ContinuityMessage> {
    use radiant::prelude as ui;

    ui::app(ContinuityState {
        visible: true,
        ..ContinuityState::default()
    })
    .view(continuity_view)
    .handle_message(|state, message, _context| match message {
        ContinuityMessage::Reorder => state.reordered = true,
        ContinuityMessage::Replace => state.incompatible = true,
        ContinuityMessage::Hide => state.visible = false,
        ContinuityMessage::Show => state.visible = true,
    })
    .into_bridge()
}

fn first_child_id<Message>(surface: &radiant::runtime::UiSurface<Message>) -> u64 {
    let radiant::layout::LayoutNode::Container(container) = surface.layout_node() else {
        panic!("continuity view should lower to a column");
    };
    container.children[0].child.id()
}

#[test]
fn stateful_app_builder_projects_updates_and_preserves_context_requests() {
    use radiant::prelude as ui;

    let mut bridge = ui::app(DemoState::default())
        .title("Counter")
        .size(320, 120)
        .view(|state| {
            ui::column([
                ui::text(format!("Count: {}", state.count)),
                ui::button("Increment").message(DemoMessage::Increment),
            ])
        })
        .handle_message(|state, message, context| match message {
            DemoMessage::Increment => {
                state.count += 1;
                context.request_repaint();
            }
        })
        .into_bridge();

    let before = bridge.project_surface();
    let radiant::layout::LayoutNode::Container(container) = before.layout_node() else {
        panic!("counter view should lower to a container");
    };
    let text_id = container.children[0].child.id();
    let button_id = container.children[1].child.id();
    let increment = before
        .dispatch_widget_output(
            button_id,
            radiant::widgets::WidgetOutput::typed(crate::programmatic_button_message()),
        )
        .expect("generated button should route through the same surface mapper");

    let command = bridge.update(increment);

    assert!(command.requests_repaint());
    let after = bridge.project_surface();
    assert_eq!(
        widget_ref::<TextWidget, _>(&after, text_id, "text").text,
        "Count: 1"
    );
}

#[test]
fn preserve_state_keeps_compatible_button_state_through_sibling_reorder() {
    use radiant::{
        gui::types::Point,
        runtime::SurfaceRuntime,
        widgets::{ButtonWidget, PointerButton, PointerModifiers, WidgetInput},
    };

    let mut runtime = SurfaceRuntime::new(continuity_bridge(), Vector2::new(180.0, 64.0));
    let tracked_id = first_child_id(runtime.surface());
    assert!(runtime.dispatch_input(
        tracked_id,
        WidgetInput::PointerPress {
            position: Point::new(12.0, 12.0),
            button: PointerButton::Primary,
            modifiers: PointerModifiers::default(),
            timestamp: None,
        }
    ));
    assert!(
        widget_ref::<ButtonWidget, _>(runtime.surface(), tracked_id, "tracked button")
            .common()
            .state
            .pressed
    );

    runtime.dispatch_message(ContinuityMessage::Reorder);

    let after_id = {
        let radiant::layout::LayoutNode::Container(container) = runtime.surface().layout_node()
        else {
            panic!("continuity view should lower to a column");
        };
        container.children[1].child.id()
    };
    assert_eq!(after_id, tracked_id);
    assert!(
        widget_ref::<ButtonWidget, _>(runtime.surface(), tracked_id, "tracked button")
            .common()
            .state
            .pressed
    );
}

#[test]
fn preserve_state_cleans_incompatible_replacement_before_strict_audit() {
    use radiant::{
        runtime::{IdentityAudit, SurfaceRuntime},
        widgets::TextWidget,
    };

    let mut runtime = SurfaceRuntime::new(continuity_bridge(), Vector2::new(180.0, 64.0));
    let tracked_id = first_child_id(runtime.surface());
    assert!(runtime.focus_widget(tracked_id));
    runtime.set_identity_audit(IdentityAudit::strict());

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        runtime.dispatch_message(ContinuityMessage::Replace);
    }));
    assert!(result.is_err());
    assert_eq!(runtime.focused_widget(), None);
    assert_eq!(
        runtime
            .last_refresh_diagnostics()
            .identity
            .replacement_count,
        1
    );
    assert!(
        widget_ref::<TextWidget, _>(runtime.surface(), tracked_id, "replacement text")
            .text
            .contains("Replacement")
    );
}

#[test]
fn preserve_state_reappearance_starts_widget_state_fresh_after_disappearance() {
    use radiant::{
        gui::types::Point,
        runtime::SurfaceRuntime,
        widgets::{ButtonWidget, PointerButton, PointerModifiers, WidgetInput},
    };

    let mut runtime = SurfaceRuntime::new(continuity_bridge(), Vector2::new(180.0, 64.0));
    let tracked_id = first_child_id(runtime.surface());
    assert!(runtime.dispatch_input(
        tracked_id,
        WidgetInput::PointerPress {
            position: Point::new(12.0, 12.0),
            button: PointerButton::Primary,
            modifiers: PointerModifiers::default(),
            timestamp: None,
        }
    ));
    assert!(
        widget_ref::<ButtonWidget, _>(runtime.surface(), tracked_id, "tracked button")
            .common()
            .state
            .pressed
    );

    runtime.dispatch_message(ContinuityMessage::Hide);
    assert!(runtime.surface().find_widget(tracked_id).is_none());
    runtime.dispatch_message(ContinuityMessage::Show);

    let reappeared_id = first_child_id(runtime.surface());
    assert_eq!(reappeared_id, tracked_id);
    assert!(
        !widget_ref::<ButtonWidget, _>(runtime.surface(), tracked_id, "tracked button")
            .common()
            .state
            .pressed
    );
}

#[test]
fn stateful_runtime_keeps_ordinary_messages_and_worker_mappers_ui_local() {
    use radiant::prelude as ui;
    use std::{cell::RefCell, rc::Rc};

    #[derive(Clone)]
    enum UiOnlyMessage {
        Load(Rc<RefCell<Vec<&'static str>>>),
        Loaded(Rc<RefCell<Vec<&'static str>>>),
    }

    let events = Rc::new(RefCell::new(Vec::new()));
    let view_events = Rc::clone(&events);
    let bridge = ui::app(DemoState::default())
        .view(move |state| {
            ui::column([
                ui::text(format!("Count: {}", state.count)).id(10),
                ui::button("Load")
                    .message(UiOnlyMessage::Load(Rc::clone(&view_events)))
                    .id(11),
            ])
        })
        .handle_message(|state, message, context| match message {
            UiOnlyMessage::Load(events) => {
                events.borrow_mut().push("load");
                context
                    .business()
                    .background("ui-only-message")
                    .run(|_| (), move |()| UiOnlyMessage::Loaded(events));
            }
            UiOnlyMessage::Loaded(events) => {
                events.borrow_mut().push("loaded");
                state.count += 1;
            }
        })
        .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(180.0, 64.0));
    let message = runtime
        .surface()
        .dispatch_widget_output(
            11,
            radiant::widgets::WidgetOutput::typed(crate::programmatic_button_message()),
        )
        .expect("button should emit its UI-only message");

    runtime.dispatch_message(message);
    let finished = wait_for_runtime_message(&mut runtime);

    assert_eq!(finished.messages_dispatched, 1);
    assert_eq!(&*events.borrow(), &["load", "loaded"]);
    assert_eq!(
        widget_ref::<TextWidget, _>(runtime.surface(), 10, "text").text,
        "Count: 1"
    );
}

#[test]
fn handle_message_exposes_ui_update_context_with_clear_app_api_name() {
    use radiant::prelude as ui;

    let mut bridge = ui::app(DemoState::default())
        .view(|state| ui::text(format!("Count: {}", state.count)))
        .handle_message(|state, message, context| match message {
            DemoMessage::Increment => {
                state.count += 1;
                context.request_repaint();
            }
        })
        .into_bridge();

    let command = bridge.update(DemoMessage::Increment);

    assert!(command.requests_repaint());
    let after = bridge.project_surface();
    let text_id = after.root().id();
    assert_eq!(
        widget_ref::<TextWidget, _>(&after, text_id, "text").text,
        "Count: 1"
    );
}

#[test]
fn handle_message_is_the_only_context_aware_app_handler_name() {
    use radiant::prelude as ui;

    let mut bridge = ui::app(DemoState::default())
        .view(|state| ui::text(format!("Count: {}", state.count)))
        .handle_message(|state, message, context| match message {
            DemoMessage::Increment => {
                state.count += 1;
                context.request_repaint();
            }
        })
        .into_bridge();

    let command = bridge.update(DemoMessage::Increment);

    assert!(command.requests_repaint());
    let after = bridge.project_surface();
    let text_id = after.root().id();
    assert_eq!(
        widget_ref::<TextWidget, _>(&after, text_id, "text").text,
        "Count: 1"
    );
}

#[test]
fn ordinary_handler_without_repaint_command_requests_surface_repaint_by_default() {
    use radiant::prelude as ui;
    use radiant::runtime::RepaintScope;

    let mut bridge = ui::app(DemoState::default())
        .view(|state| ui::text(format!("Count: {}", state.count)))
        .handle_message(|state, message, _context| match message {
            DemoMessage::Increment => state.count += 1,
        })
        .into_bridge();

    let command = bridge.update(DemoMessage::Increment);

    assert_eq!(command.repaint_scope(), Some(RepaintScope::Surface));
}

#[test]
fn ordinary_handler_explicit_paint_only_is_not_upgraded_to_surface_repaint() {
    use radiant::prelude as ui;
    use radiant::runtime::RepaintScope;

    let mut bridge = ui::app(DemoState::default())
        .view(|state| ui::text(format!("Count: {}", state.count)))
        .handle_message(|state, message, context| match message {
            DemoMessage::Increment => {
                state.count += 1;
                context.request_paint_only();
            }
        })
        .into_bridge();

    let command = bridge.update(DemoMessage::Increment);

    assert_eq!(command.repaint_scope(), Some(RepaintScope::PaintOnly));
}

#[test]
fn ordinary_handler_explicit_surface_repaint_is_preserved() {
    use radiant::prelude as ui;
    use radiant::runtime::RepaintScope;

    let mut bridge = ui::app(DemoState::default())
        .view(|state| ui::text(format!("Count: {}", state.count)))
        .handle_message(|state, message, context| match message {
            DemoMessage::Increment => {
                state.count += 1;
                context.request_repaint();
            }
        })
        .into_bridge();

    let command = bridge.update(DemoMessage::Increment);

    assert_eq!(command.repaint_scope(), Some(RepaintScope::Surface));
}

#[test]
fn repaint_policy_none_disables_ordinary_message_automatic_repaint() {
    use radiant::prelude as ui;

    let mut bridge = ui::app(DemoState::default())
        .view(|state| ui::text(format!("Count: {}", state.count)))
        .handle_message(|state, message, _context| match message {
            DemoMessage::Increment => state.count += 1,
        })
        .repaint_policy(ui::RepaintPolicy::none())
        .into_bridge();

    let command = bridge.update(DemoMessage::Increment);

    assert!(!command.requests_repaint());
}

#[test]
fn repaint_policy_can_skip_frame_messages() {
    use radiant::prelude as ui;

    #[derive(Clone, Debug, PartialEq)]
    enum Message {
        Frame,
        User,
    }

    let mut bridge = ui::app(DemoState::default())
        .view(|state| ui::text(format!("Count: {}", state.count)))
        .handle_message(|state, message, _context| match message {
            Message::Frame => state.count += 1,
            Message::User => state.count += 10,
        })
        .repaint_policy(ui::RepaintPolicy::after_messages_except_value(
            Message::Frame,
        ))
        .into_bridge();

    let frame_command = bridge.update(Message::Frame);
    let user_command = bridge.update(Message::User);

    assert!(!frame_command.requests_repaint());
    assert!(user_command.requests_repaint());
    let after = bridge.project_surface();
    let text_id = after.root().id();
    assert_eq!(
        widget_ref::<TextWidget, _>(&after, text_id, "text").text,
        "Count: 11"
    );
}

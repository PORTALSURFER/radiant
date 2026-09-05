use super::{MenuMessage, click};
use crate::{
    application::{IntoView, MenuCommand, message_menu},
    gui::types::{Point, Rect},
    layout::Vector2,
    runtime::{DeclarativeOwnedRuntimeBridge, PaintPrimitive, SurfaceRuntime, UiSurface},
    widgets::WidgetTone,
};

#[test]
fn message_menu_emits_host_messages() {
    let bridge = DeclarativeOwnedRuntimeBridge::new(
        Vec::<MenuMessage>::new(),
        |_| {
            UiSurface::new(
                message_menu(
                    "Actions",
                    [
                        MenuCommand::new("Open", MenuMessage::Open),
                        MenuCommand::new("Delete", MenuMessage::Delete).danger(),
                    ],
                )
                .into_node(),
            )
        },
        |messages: &mut Vec<MenuMessage>, message| messages.push(message),
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(240.0, 120.0));

    click(&mut runtime, Point::new(20.0, 70.0));

    assert_eq!(runtime.bridge().state(), &[MenuMessage::Delete]);
}

#[test]
fn message_menu_applies_command_styles() {
    let frame = UiSurface::new(
        message_menu(
            "Actions",
            [MenuCommand::new("Delete", MenuMessage::Delete).danger()],
        )
        .into_node(),
    )
    .frame(
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(240.0, 120.0)),
        &Default::default(),
    );

    assert!(
        frame.paint_plan.primitives.iter().any(|primitive| {
            matches!(
                primitive,
                PaintPrimitive::FillRect(fill) if fill.color.r > fill.color.g
            )
        }),
        "danger commands should apply the danger-toned button style"
    );
}

#[test]
fn menu_command_style_helpers_are_generic() {
    let command = MenuCommand::new("Open", MenuMessage::Open).primary();
    assert_eq!(command.style.tone, WidgetTone::Accent);
}

#[test]
fn menu_command_keeps_capture_and_updates_accessible_name_across_locale_refresh() {
    use crate::{
        application::{ApplicationEnvironment, LocaleId, TextScale, WritingDirection},
        runtime::Event,
        widgets::{PointerButton, PointerModifiers},
    };
    struct State {
        environment: ApplicationEnvironment,
        label: &'static str,
        messages: Vec<MenuMessage>,
    }
    let bridge = DeclarativeOwnedRuntimeBridge::new(
        State {
            environment: ApplicationEnvironment::new(LocaleId::english()),
            label: "Open",
            messages: Vec::new(),
        },
        |state: &mut State| {
            UiSurface::new(
                message_menu(
                    "Actions",
                    [MenuCommand::new(state.label, MenuMessage::Open).hotkey_hint("Ctrl-O")],
                )
                .into_node(),
            )
            .with_application_environment(state.environment.clone())
        },
        |state: &mut State, message| state.messages.push(message),
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(400.0, 300.0));
    let plan = runtime
        .surface()
        .paint_plan(runtime.layout(), &Default::default());
    let id = plan.first_text_run("Open").unwrap().widget_id;
    let bounds = runtime.layout().rects[&id];
    runtime.dispatch_event(Event::PointerPress {
        position: Point::new(bounds.min.x + 10.0, bounds.min.y + 10.0),
        button: PointerButton::Primary,
        modifiers: PointerModifiers::default(),
        timestamp: None,
    });
    assert_eq!(runtime.focused_widget(), Some(id));
    runtime.bridge_mut().state_mut().label = "فتح";
    runtime.bridge_mut().state_mut().environment =
        ApplicationEnvironment::new(LocaleId::new("ar").unwrap())
            .with_text_scale(TextScale::new(2.0).unwrap())
            .with_writing_direction(WritingDirection::Rtl);
    runtime.refresh();
    let plan = runtime
        .surface()
        .paint_plan(runtime.layout(), &Default::default());
    assert_eq!(plan.first_text_run("فتح").unwrap().widget_id, id);
    assert_eq!(runtime.focused_widget(), Some(id));
    let widget = runtime.surface().find_widget(id).unwrap().widget();
    assert_eq!(widget.automation_semantics().label.as_deref(), Some("فتح"));
    assert!(widget.common().state.pressed);
    let bounds = runtime.layout().rects[&id];
    assert_eq!(bounds.height(), 56.0);
    runtime.dispatch_event(Event::PointerRelease {
        position: Point::new(bounds.min.x + 10.0, bounds.min.y + 10.0),
        button: PointerButton::Primary,
        modifiers: PointerModifiers::default(),
        timestamp: None,
    });
    assert_eq!(runtime.bridge().state().messages, [MenuMessage::Open]);
    assert!(
        !runtime
            .surface()
            .find_widget(id)
            .unwrap()
            .widget()
            .common()
            .state
            .pressed
    );
}

use super::super::*;
use radiant::runtime::SurfaceRuntime;
use radiant::widgets::{
    IconButtonWidget, InteractiveRowWidget, PointerShieldMessage, PointerShieldWidget,
    ProgressBarMessage, ProgressBarWidget, SliderWidget, TextInputMessage, TextInputWidget,
};

#[test]
fn application_builders_expose_interactive_row_scrollbar_icon_button_and_compact_slider() {
    use radiant::prelude::{self as ui, IntoView};

    let icon = ui::SvgIcon::from_svg(
        r##"<svg viewBox="0 0 4 4" xmlns="http://www.w3.org/2000/svg"><path d="M1 0 L4 2 L1 4 Z"/></svg>"##,
    )
    .expect("icon");
    let surface: UiSurface<&'static str> = ui::column([
        ui::interactive_row()
            .draggable()
            .drop_target_mode(true, true)
            .pointer_motion_during_interaction()
            .pointer_motion_active(true)
            .mapped(|message| match message {
                ui::InteractiveRowMessage::Activate { .. } => "row",
                ui::InteractiveRowMessage::ActivateWithModifiers { .. } => "row-modifiers",
                ui::InteractiveRowMessage::DoubleActivate { .. } => "double-row",
                ui::InteractiveRowMessage::Hover { .. } => "hover",
                ui::InteractiveRowMessage::Drag(_) => "drag",
                ui::InteractiveRowMessage::SecondaryActivate { .. } => "secondary",
                ui::InteractiveRowMessage::Drop => "drop",
                ui::InteractiveRowMessage::HoverDropTarget { .. } => "hover-drop",
                ui::InteractiveRowMessage::ClearDropTarget { .. } => "clear-drop",
            })
            .id(20),
        ui::interactive_row()
            .drop_target_mode(true, false)
            .mapped(|message| match message {
                ui::InteractiveRowMessage::Drop => "drop-only",
                _ => "drop-only-other",
            })
            .id(24),
        ui::interactive_row()
            .filter_mapped(|message| {
                message
                    .is_single_activation()
                    .then_some("filtered-row-activate")
            })
            .id(30),
        ui::scrollbar(ui::ScrollbarAxis::Horizontal)
            .viewport_fraction(0.25)
            .offset_fraction(0.5)
            .mapped(|_| "scroll")
            .id(21),
        ui::icon_button(icon).active(true).message("icon").id(22),
        ui::slider(0.25).compact().message(|_| "slider").id(23),
        ui::determinate_progress_bar(0.4)
            .colors(ui::Rgba8::new(1, 2, 3, 4), ui::Rgba8::new(5, 6, 7, 8))
            .max_track_height(5.0)
            .activatable()
            .message("progress")
            .id(25),
        ui::pointer_drop_shield(true)
            .mapped(|_| "pointer-drop")
            .id(26),
        ui::pointer_drop_shield(true).on_drop("drop-message").id(27),
        ui::pointer_move_shield(true)
            .on_pointer_move(|position| {
                if position.x > 3.0 {
                    "move-right"
                } else {
                    "move-left"
                }
            })
            .id(28),
        ui::pointer_shield(true).view().id(29),
    ])
    .into_surface();

    let row = widget_ref::<InteractiveRowWidget, _>(&surface, 20, "interactive row");
    assert!(row.props.draggable);
    assert!(row.props.droppable);
    assert!(row.props.drop_hover);
    assert!(row.props.drag_active);
    assert_eq!(
        row.props.pointer_motion,
        ui::InteractiveRowPointerMotion::DuringInteraction
    );
    assert!(row.props.pointer_motion_active);
    assert_eq!(
        surface.dispatch_widget_output(
            20,
            radiant::widgets::WidgetOutput::typed(ui::InteractiveRowMessage::Activate {
                provenance: ui::InteractionProvenance::Programmatic,
            }),
        ),
        Some("row")
    );
    let drop_only = widget_ref::<InteractiveRowWidget, _>(&surface, 24, "drop-only row");
    assert!(drop_only.props.droppable);
    assert!(!drop_only.props.drop_hover);
    assert_eq!(
        surface.dispatch_widget_output(
            24,
            radiant::widgets::WidgetOutput::typed(ui::InteractiveRowMessage::Drop),
        ),
        Some("drop-only")
    );
    assert_eq!(
        surface.dispatch_widget_output(
            30,
            radiant::widgets::WidgetOutput::typed(ui::InteractiveRowMessage::Activate {
                provenance: ui::InteractionProvenance::Programmatic,
            }),
        ),
        Some("filtered-row-activate")
    );
    assert_eq!(
        surface.dispatch_widget_output(
            30,
            radiant::widgets::WidgetOutput::typed(ui::InteractiveRowMessage::Drop),
        ),
        None
    );

    let scrollbar = widget_ref::<radiant::widgets::ScrollbarWidget, _>(&surface, 21, "scrollbar");
    assert_eq!(scrollbar.props.viewport_fraction, 0.25);
    assert_eq!(scrollbar.state.offset_fraction, 0.5);
    assert_eq!(
        surface.dispatch_widget_output(
            21,
            radiant::widgets::WidgetOutput::typed(
                radiant::widgets::ScrollbarMessage::OffsetChanged {
                    offset_fraction: 0.75,
                }
            ),
        ),
        Some("scroll")
    );

    let icon_button = widget_ref::<IconButtonWidget, _>(&surface, 22, "icon button");
    assert!(icon_button.common.state.active);
    assert_eq!(
        surface.dispatch_widget_output(
            22,
            radiant::widgets::WidgetOutput::typed(crate::programmatic_button_message()),
        ),
        Some("icon")
    );

    let slider = widget_ref::<SliderWidget, _>(&surface, 23, "slider");
    assert_eq!(slider.common.sizing.preferred, Vector2::new(92.0, 20.0));
    let progress = widget_ref::<ProgressBarWidget, _>(&surface, 25, "progress bar");
    assert_eq!(
        progress.props.mode,
        radiant::widgets::ProgressBarMode::Determinate(0.4)
    );
    assert_eq!(progress.props.max_track_height, 5.0);
    assert!(progress.props.interactive);
    assert_eq!(
        surface.dispatch_widget_output(
            25,
            radiant::widgets::WidgetOutput::typed(ProgressBarMessage::Activate),
        ),
        Some("progress")
    );
    assert_eq!(
        surface.dispatch_widget_output(
            25,
            radiant::widgets::WidgetOutput::typed(crate::programmatic_button_message()),
        ),
        None
    );

    let shield = widget_ref::<PointerShieldWidget, _>(&surface, 26, "pointer shield");
    assert!(shield.props.active);
    assert!(!shield.props.pointer_move);
    assert!(!shield.props.pointer_press);
    assert!(!shield.props.pointer_release);
    assert!(shield.props.pointer_drop);
    assert_eq!(
        surface.dispatch_widget_output(
            26,
            radiant::widgets::WidgetOutput::typed(PointerShieldMessage::PointerDrop {
                position: ui::Point::new(1.0, 2.0),
                button: ui::PointerButton::Primary,
                modifiers: Default::default(),
                timestamp: None,
            }),
        ),
        Some("pointer-drop")
    );
    assert_eq!(
        surface.dispatch_widget_output(
            27,
            radiant::widgets::WidgetOutput::typed(PointerShieldMessage::PointerDrop {
                position: ui::Point::new(1.0, 2.0),
                button: ui::PointerButton::Primary,
                modifiers: Default::default(),
                timestamp: None,
            }),
        ),
        Some("drop-message")
    );
    assert_eq!(
        surface.dispatch_widget_output(
            27,
            radiant::widgets::WidgetOutput::typed(PointerShieldMessage::PointerMove {
                position: ui::Point::new(1.0, 2.0),
                timestamp: None,
                sequence_range: None,
            }),
        ),
        None
    );
    assert_eq!(
        surface.dispatch_widget_output(
            28,
            radiant::widgets::WidgetOutput::typed(PointerShieldMessage::PointerMove {
                position: ui::Point::new(4.0, 2.0),
                timestamp: None,
                sequence_range: None,
            }),
        ),
        Some("move-right")
    );
    assert_eq!(
        surface.dispatch_widget_output(
            29,
            radiant::widgets::WidgetOutput::typed(PointerShieldMessage::PointerMove {
                position: ui::Point::new(4.0, 2.0),
                timestamp: None,
                sequence_range: None,
            }),
        ),
        None
    );
}

#[derive(Default)]
struct RuntimeIconButtonState {
    active: bool,
    enabled: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum RuntimeIconButtonMessage {
    ToggleActive,
    ToggleEnabled,
}

#[test]
fn keyed_runtime_icon_buttons_reproject_active_and_disabled_markers() {
    use radiant::prelude as ui;

    let icon = ui::SvgIcon::from_svg(
        r##"<svg viewBox="0 0 4 4" xmlns="http://www.w3.org/2000/svg"><path d="M1 0 L4 2 L1 4 Z"/></svg>"##,
    )
    .expect("icon");
    let bridge = ui::app(RuntimeIconButtonState {
        active: false,
        enabled: true,
    })
    .view(move |state| {
        ui::column([
            ui::icon_button(icon.clone())
                .active(state.active)
                .enabled(state.enabled)
                .message(RuntimeIconButtonMessage::ToggleActive)
                .id(501),
            ui::icon_button(icon.clone())
                .active(state.active)
                .enabled(state.enabled)
                .bare()
                .message(RuntimeIconButtonMessage::ToggleEnabled)
                .id(502),
        ])
    })
    .update(|state, message| match message {
        RuntimeIconButtonMessage::ToggleActive => state.active = !state.active,
        RuntimeIconButtonMessage::ToggleEnabled => state.enabled = !state.enabled,
    })
    .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, ui::Vector2::new(180.0, 64.0));

    let marker_count = |runtime: &SurfaceRuntime<_, RuntimeIconButtonMessage>, id| {
        runtime
            .frame_with_default_theme()
            .paint_plan
            .stroke_polylines()
            .filter(|marker| marker.widget_id == id)
            .count()
    };
    let dispatch = |runtime: &mut SurfaceRuntime<_, RuntimeIconButtonMessage>, id, message| {
        let message = runtime
            .surface()
            .dispatch_widget_output(id, radiant::widgets::WidgetOutput::typed(message))
            .expect("icon button should map activation");
        runtime.dispatch_message(message);
    };

    for id in [501, 502] {
        let button = widget_ref::<IconButtonWidget, _>(runtime.surface(), id, "icon button");
        assert!(!button.common.state.active);
        assert!(!button.common.state.disabled);
        assert_eq!(marker_count(&runtime, id), 0);
    }

    dispatch(&mut runtime, 501, crate::programmatic_button_message());
    for id in [501, 502] {
        let button = widget_ref::<IconButtonWidget, _>(runtime.surface(), id, "icon button");
        assert!(button.common.state.active);
        assert!(!button.common.state.disabled);
        assert_eq!(marker_count(&runtime, id), 1);
    }

    dispatch(&mut runtime, 501, crate::programmatic_button_message());
    for id in [501, 502] {
        let button = widget_ref::<IconButtonWidget, _>(runtime.surface(), id, "icon button");
        assert!(!button.common.state.active);
        assert!(!button.common.state.disabled);
        assert_eq!(marker_count(&runtime, id), 0);
    }

    dispatch(&mut runtime, 501, crate::programmatic_button_message());
    dispatch(&mut runtime, 502, crate::programmatic_button_message());
    for id in [501, 502] {
        let button = widget_ref::<IconButtonWidget, _>(runtime.surface(), id, "icon button");
        assert!(button.common.state.active);
        assert!(button.common.state.disabled);
        assert_eq!(marker_count(&runtime, id), 0);
    }
}

#[test]
fn text_input_builder_can_seed_selection_and_route_full_input_events() {
    use radiant::prelude::{self as ui, IntoView};

    let surface: UiSurface<TextInputMessage> = ui::text_input("Rename me")
        .select_all()
        .completion_suffix(".wav")
        .message_event(|message| message)
        .id(10)
        .into_surface();

    let input = widget_ref::<TextInputWidget, _>(&surface, 10, "text input");
    assert_eq!(input.state.selection_anchor, 0);
    assert_eq!(input.state.caret, "Rename me".chars().count());
    assert_eq!(input.props.completion_suffix.as_deref(), Some(".wav"));
    assert_eq!(
        surface.dispatch_widget_output(
            10,
            radiant::widgets::WidgetOutput::typed(TextInputMessage::Submitted {
                value: String::from("Renamed"),
            }),
        ),
        Some(TextInputMessage::Submitted {
            value: String::from("Renamed"),
        })
    );
}

#[test]
fn text_input_clear_button_builder_keeps_clear_slot_stable_and_routes_messages() {
    use radiant::prelude::{self as ui, IntoView};

    #[derive(Clone, Debug, PartialEq, Eq)]
    enum Message {
        Rename(String),
        Clear,
    }

    let surface = ui::text_input("kick")
        .placeholder("Any")
        .clear_button(Message::Clear)
        .id(31)
        .message_event(|message| Message::Rename(message.into_value()))
        .into_surface();
    let clear_button_id = ui::text_input_clear_button_id(31);
    let input = widget_ref::<TextInputWidget, _>(&surface, 31, "clearable input");

    assert_eq!(input.props.placeholder.as_deref(), Some("Any"));
    assert_eq!(
        surface.dispatch_widget_output(
            31,
            radiant::widgets::WidgetOutput::typed(TextInputMessage::Changed {
                value: String::from("snare"),
            }),
        ),
        Some(Message::Rename(String::from("snare")))
    );
    assert_eq!(
        surface.dispatch_widget_output(
            clear_button_id,
            radiant::widgets::WidgetOutput::typed(crate::programmatic_button_message()),
        ),
        Some(Message::Clear)
    );

    let empty_surface = ui::text_input("")
        .clear_button(Message::Clear)
        .id(41)
        .message_event(|message| Message::Rename(message.into_value()))
        .into_surface();
    let empty_clear_button_id = ui::text_input_clear_button_id(41);

    assert!(empty_surface.find_widget(empty_clear_button_id).is_none());
    assert_eq!(
        empty_surface.dispatch_widget_output(
            empty_clear_button_id,
            radiant::widgets::WidgetOutput::typed(crate::programmatic_button_message()),
        ),
        None
    );
}

#[test]
fn text_input_clear_button_builder_supports_mapped_clear_messages() {
    use radiant::prelude::{self as ui, IntoView};

    #[derive(Clone, Debug, PartialEq, Eq)]
    enum Message {
        Rename(String),
        Clear(String),
    }

    let surface = ui::text_input("kick")
        .placeholder("Any")
        .clear_button_mapped(|| Message::Clear(String::from("name-filter")))
        .id(31)
        .message_event(|message| Message::Rename(message.into_value()))
        .into_surface();
    let clear_button_id = ui::text_input_clear_button_id(31);

    let input = widget_ref::<TextInputWidget, _>(&surface, 31, "clearable input");
    assert_eq!(input.props.placeholder.as_deref(), Some("Any"));
    assert_eq!(
        surface.dispatch_widget_output(
            clear_button_id,
            radiant::widgets::WidgetOutput::typed(crate::programmatic_button_message()),
        ),
        Some(Message::Clear(String::from("name-filter")))
    );
}

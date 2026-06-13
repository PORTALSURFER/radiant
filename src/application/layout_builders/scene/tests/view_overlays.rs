use crate::{
    application::{IntoView, Layer, button, overlays, row, scene, text},
    gui::types::Point,
    layout::Vector2,
    runtime::{DeclarativeOwnedRuntimeBridge, Event, SurfaceRuntime},
    widgets::{ButtonMessage, PointerButton, PointerModifiers},
};

#[test]
fn view_overlays_project_from_owner_subtree() {
    let labels = scene::<()>(
        text("Owner").overlays(
            overlays()
                .layer(Layer::floating(text("Floating")))
                .layer(Layer::modal(text("Modal"))),
        ),
    )
    .into_view()
    .view_frame_at_size_with_default_theme(Vector2::new(320.0, 180.0))
    .paint_plan
    .text_label_strings();

    assert_eq!(labels, ["Owner", "Floating", "Modal"]);
}

#[test]
fn view_overlays_omit_none_layers() {
    let labels = scene::<()>(
        text("Owner").overlays(
            overlays()
                .layer_opt(None)
                .layer_opt(Some(Layer::context_menu(text("Menu")))),
        ),
    )
    .into_view()
    .view_frame_at_size_with_default_theme(Vector2::new(320.0, 180.0))
    .paint_plan
    .text_label_strings();

    assert_eq!(labels, ["Owner", "Menu"]);
}

#[test]
fn view_overlays_support_typed_optional_layer_helpers() {
    let labels = scene::<()>(
        text("Owner").overlays(
            overlays()
                .floating_opt(Some(text("Floating")))
                .popover_opt(None)
                .modal_opt(Some(text("Modal")))
                .context_menu_opt(Some(text("Menu")))
                .tooltip_opt(None)
                .drag_preview_opt(Some(text("Drag"))),
        ),
    )
    .into_view()
    .view_frame_at_size_with_default_theme(Vector2::new(320.0, 180.0))
    .paint_plan
    .text_label_strings();

    assert_eq!(labels, ["Owner", "Floating", "Modal", "Menu", "Drag"]);
}

#[test]
fn view_overlays_dismissible_context_menu_routes_outside_click() {
    #[derive(Clone, Debug, PartialEq)]
    enum Message {
        Base,
        Dismiss,
    }

    let bridge = DeclarativeOwnedRuntimeBridge::new(
        Vec::<Message>::new(),
        |_| {
            scene::<Message>(
                button("Base")
                    .message(Message::Base)
                    .fill()
                    .overlays(overlays().dismissible_context_menu(text("Menu"), Message::Dismiss)),
            )
            .into_view()
            .fill()
            .into_surface()
        },
        |state, message| state.push(message),
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(240.0, 160.0));

    runtime.dispatch_event(Event::primary_press(Point::new(220.0, 140.0)));

    assert_eq!(runtime.bridge().state(), &[Message::Dismiss]);
}

#[test]
fn view_overlays_context_menu_survives_release_from_opening_secondary_press() {
    #[derive(Clone, Debug, PartialEq)]
    enum Message {
        Open,
        Dismiss,
    }

    let bridge = DeclarativeOwnedRuntimeBridge::new(
        false,
        |open| {
            let base = button("Target")
                .secondary_clicks()
                .filter_mapped(|message| match message {
                    ButtonMessage::SecondaryActivate { .. } => Some(Message::Open),
                    _ => None,
                })
                .fill();
            let overlays = if *open {
                overlays().dismissible_context_menu(text("Menu"), Message::Dismiss)
            } else {
                overlays()
            };
            scene(base.overlays(overlays))
                .into_view()
                .fill()
                .into_surface()
        },
        |open, message| match message {
            Message::Open => *open = true,
            Message::Dismiss => *open = false,
        },
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(240.0, 160.0));
    let target = Point::new(24.0, 18.0);

    runtime.dispatch_event(Event::PointerPress {
        position: target,
        button: PointerButton::Secondary,
        modifiers: PointerModifiers::default(),
    });
    runtime.dispatch_event(Event::PointerRelease {
        position: target,
        button: PointerButton::Secondary,
        modifiers: PointerModifiers::default(),
    });

    assert!(
        *runtime.bridge().state(),
        "releasing the secondary button that opened a context menu should not dismiss it"
    );
}

#[test]
fn view_overlays_blocking_modal_consumes_base_input() {
    #[derive(Clone, Debug, PartialEq)]
    enum Message {
        Base,
    }

    let bridge = DeclarativeOwnedRuntimeBridge::new(
        Vec::<Message>::new(),
        |_| {
            scene::<Message>(
                button("Base")
                    .message(Message::Base)
                    .fill()
                    .overlays(overlays().blocking_modal(text("Modal"))),
            )
            .into_view()
            .fill()
            .into_surface()
        },
        |state, message| state.push(message),
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(240.0, 160.0));

    runtime.dispatch_event(Event::primary_press(Point::new(220.0, 140.0)));

    assert!(runtime.bridge().state().is_empty());
}

#[test]
fn scene_overlay_layers_preserve_declaration_order_within_kind() {
    let labels = scene::<()>(row([
        text("Left").overlays(overlays().modal(text("Left modal"))),
        text("Right").overlays(overlays().modal(text("Right modal"))),
    ]))
    .into_view()
    .view_frame_at_size_with_default_theme(Vector2::new(320.0, 180.0))
    .paint_plan
    .text_label_strings();

    assert_eq!(labels, ["Left", "Right", "Left modal", "Right modal"]);
}

#[test]
fn scene_overlay_layers_compose_before_explicit_root_layers() {
    let labels = scene::<()>(text("Base").overlays(overlays().modal(text("Component modal"))))
        .layer(Layer::modal(text("Root modal")))
        .into_view()
        .view_frame_at_size_with_default_theme(Vector2::new(320.0, 180.0))
        .paint_plan
        .text_label_strings();

    assert_eq!(labels, ["Base", "Component modal", "Root modal"]);
}

#[test]
fn scene_overlay_dismiss_policy_routes_above_base() {
    #[derive(Clone, Debug, PartialEq)]
    enum Message {
        Base,
        Dismiss,
    }

    let bridge = DeclarativeOwnedRuntimeBridge::new(
        Vec::<Message>::new(),
        |_| {
            scene::<Message>(
                button("Base")
                    .message(Message::Base)
                    .fill()
                    .overlays(overlays().dismissible_context_menu(text("Menu"), Message::Dismiss)),
            )
            .into_view()
            .fill()
            .into_surface()
        },
        |state, message| state.push(message),
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(240.0, 160.0));

    runtime.dispatch_event(Event::primary_press(Point::new(220.0, 140.0)));

    assert_eq!(runtime.bridge().state(), &[Message::Dismiss]);
}

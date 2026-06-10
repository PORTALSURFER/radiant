use crate::{
    application::{IntoView, tree_row},
    gui::types::{Point, Rect, Vector2},
    widgets::{InteractiveRowActions, PointerButton, PointerModifiers, WidgetInput},
};

#[derive(Clone, Debug, PartialEq)]
enum TreeRowMessage {
    Activate,
    Toggle,
}

#[test]
fn tree_row_routes_interactive_actions() {
    let view = tree_row("Folder")
        .input_id(91)
        .interactive_actions(InteractiveRowActions::new().activate(|| TreeRowMessage::Activate));
    let mut surface = view.into_surface();
    let bounds = Rect::from_size(160.0, 22.0);
    let position = Point::new(8.0, 10.0);

    surface.dispatch_widget_input(
        91,
        bounds,
        WidgetInput::PointerPress {
            position,
            button: PointerButton::Primary,
            modifiers: PointerModifiers::default(),
        },
    );
    let output = surface.dispatch_widget_input(
        91,
        bounds,
        WidgetInput::PointerRelease {
            position,
            button: PointerButton::Primary,
            modifiers: PointerModifiers::default(),
        },
    );

    assert_eq!(
        output.and_then(|output| output.typed_cloned::<TreeRowMessage>()),
        Some(TreeRowMessage::Activate)
    );
}

#[test]
fn tree_row_with_toggle_projects_label() {
    let view = tree_row("Folder")
        .has_children(true)
        .expanded(false)
        .on_toggle(|| TreeRowMessage::Toggle)
        .interactive_actions(InteractiveRowActions::new().activate(|| TreeRowMessage::Activate));

    assert!(
        view.view_frame_at_size_with_default_theme(Vector2::new(160.0, 22.0))
            .paint_plan
            .contains_text("Folder")
    );
}

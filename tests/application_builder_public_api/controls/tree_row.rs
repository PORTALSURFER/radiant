use radiant::prelude::{self as ui, IntoView};

#[derive(Clone, Debug, PartialEq)]
enum TreeMessage {
    Activate,
}

#[test]
fn tree_row_builder_is_available_from_prelude() {
    let view = ui::tree_row("Folder")
        .depth(2)
        .expanded(true)
        .has_children(true)
        .selected(true)
        .drag_drop_state(ui::TreeRowDragDropState {
            drop_target: true,
            ..ui::TreeRowDragDropState::new()
        })
        .input_id(55)
        .interactive_actions(ui::InteractiveRowActions::new().activate(|| TreeMessage::Activate));

    let mut surface = view.into_surface();
    let bounds = ui::Rect::from_size(180.0, 22.0);
    let position = ui::Point::new(20.0, 10.0);

    surface.dispatch_widget_input(
        55,
        bounds,
        ui::WidgetInput::PointerPress {
            position,
            button: ui::PointerButton::Primary,
            modifiers: Default::default(),
        },
    );
    let output = surface.dispatch_widget_input(
        55,
        bounds,
        ui::WidgetInput::PointerRelease {
            position,
            button: ui::PointerButton::Primary,
            modifiers: Default::default(),
        },
    );

    assert_eq!(
        output.and_then(|output| output.typed_cloned::<TreeMessage>()),
        Some(TreeMessage::Activate)
    );
}

use radiant::prelude::{self as ui, IntoView};

#[derive(Clone, Debug, PartialEq)]
enum TreeMessage {
    Activate,
}

#[test]
fn tree_row_builder_is_available_from_prelude() {
    let accent = ui::Rgba8::new(80, 160, 220, 255);
    let _: ui::TreeGuideStyle = ui::TreeGuideMetrics::new(12.0, 22.0).with_color(accent);
    let view = ui::tree_row("Folder")
        .depth(2)
        .expanded(true)
        .has_children(true)
        .selected(true)
        .guide_style(ui::StyledTreeGuideStyle::new(
            12.0,
            22.0,
            ui::WidgetStyle::subtle(ui::WidgetTone::Accent),
        ))
        .palette(ui::DenseRowPalette::new().selected(accent.with_alpha(96)))
        .drop_target_outline(ui::DenseRowOutlineStyle::new(0.5, accent, 1.5))
        .selected_hover_marker(ui::DenseRowMarkerStyle::new(
            ui::DenseRowMarkerParts::leading(2.0),
            accent,
        ))
        .drag_drop_state(ui::TreeRowDragDropState {
            drop_target: true,
            ..ui::TreeRowDragDropState::new()
        })
        .stable_row_identity(55, "folder-row")
        .interactive_actions(ui::InteractiveRowActions::new().activate(|| TreeMessage::Activate));
    let input_id = ui::stable_widget_id(55, "folder-row");

    let mut surface = view.into_surface();
    let bounds = ui::Rect::from_size(180.0, 22.0);
    let position = ui::Point::new(20.0, 10.0);

    surface.dispatch_widget_input(
        input_id,
        bounds,
        ui::WidgetInput::PointerPress {
            position,
            button: ui::PointerButton::Primary,
            modifiers: Default::default(),
        },
    );
    let output = surface.dispatch_widget_input(
        input_id,
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

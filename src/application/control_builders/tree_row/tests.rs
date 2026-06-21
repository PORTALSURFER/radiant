use super::hit_target::{TreeRowHitTarget, TreeRowHitTargetParts};
use crate::{
    application::{IntoView, tree_row},
    gui::{
        list::{DenseRowMarkerParts, DenseRowMarkerStyle, DenseRowPalette},
        types::{Point, Rect, Rgba8, Vector2},
    },
    widgets::{InteractiveRowActions, PointerButton, PointerModifiers, Widget, WidgetInput},
};

#[derive(Clone, Debug, PartialEq)]
enum TreeRowMessage {
    Activate,
    ActivateWithModifiers(PointerModifiers),
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
fn tree_row_routes_modifier_aware_activation() {
    let view = tree_row("Folder").input_id(92).interactive_actions(
        InteractiveRowActions::new().primary_with_modifiers(TreeRowMessage::ActivateWithModifiers),
    );
    let mut surface = view.into_surface();
    let bounds = Rect::from_size(160.0, 22.0);
    let position = Point::new(8.0, 10.0);
    let modifiers = PointerModifiers {
        command: true,
        ..PointerModifiers::default()
    };

    surface.dispatch_widget_input(
        92,
        bounds,
        WidgetInput::PointerPress {
            position,
            button: PointerButton::Primary,
            modifiers,
        },
    );
    let output = surface.dispatch_widget_input(
        92,
        bounds,
        WidgetInput::PointerRelease {
            position,
            button: PointerButton::Primary,
            modifiers,
        },
    );

    assert_eq!(
        output.and_then(|output| output.typed_cloned::<TreeRowMessage>()),
        Some(TreeRowMessage::ActivateWithModifiers(modifiers))
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

#[test]
fn selected_hover_tree_row_paints_configured_fill_and_marker() {
    let selected_hover = Rgba8::new(20, 40, 60, 180);
    let marker = Rgba8::new(220, 80, 40, 245);
    let mut target = TreeRowHitTarget::new(TreeRowHitTargetParts {
        label: "Folder".into(),
        selected: true,
        focused: false,
        drag_drop: Default::default(),
        palette: DenseRowPalette::new().selected_hovered(selected_hover),
        drop_target_outline: crate::gui::list::DenseRowOutlineStyle::new(
            0.5,
            Rgba8::new(0, 0, 0, 0),
            1.0,
        ),
        selected_hover_marker: Some(DenseRowMarkerStyle::new(
            DenseRowMarkerParts::leading(3.0),
            marker,
        )),
        normal_label_color: None,
        highlighted_label_color: Rgba8::new(255, 255, 255, 255),
        actions: InteractiveRowActions::new().activate(|| TreeRowMessage::Activate),
    });
    let bounds = Rect::from_size(160.0, 22.0);

    target.handle_input(bounds, WidgetInput::pointer_move(Point::new(8.0, 10.0)));
    let plan = target.paint_plan_with_defaults(bounds);

    assert!(
        plan.fill_rects()
            .any(|fill| fill.rect == bounds && fill.color == selected_hover),
        "selected+hovered tree row should use the configured selected-hover fill"
    );
    assert!(
        plan.fill_rects()
            .any(|fill| fill.rect.width() == 3.0 && fill.color == marker),
        "selected+hovered tree row should paint the configured leading marker"
    );
}

#[test]
fn selected_idle_tree_row_keeps_normal_label_color() {
    let normal = Rgba8::new(120, 130, 140, 255);
    let highlighted = Rgba8::new(255, 255, 255, 255);
    let target = TreeRowHitTarget::new(TreeRowHitTargetParts {
        label: "Folder".into(),
        selected: true,
        focused: true,
        drag_drop: Default::default(),
        palette: DenseRowPalette::new().selected(Rgba8::new(20, 40, 60, 180)),
        drop_target_outline: crate::gui::list::DenseRowOutlineStyle::new(
            0.5,
            Rgba8::new(0, 0, 0, 0),
            1.0,
        ),
        selected_hover_marker: None,
        normal_label_color: Some(normal),
        highlighted_label_color: highlighted,
        actions: InteractiveRowActions::new().activate(|| TreeRowMessage::Activate),
    });
    let bounds = Rect::from_size(160.0, 22.0);

    let plan = target.paint_plan_with_defaults(bounds);

    assert_eq!(plan.first_text_color("Folder"), Some(normal));
}

#[test]
fn selected_hovered_tree_row_uses_highlighted_label_color() {
    let normal = Rgba8::new(120, 130, 140, 255);
    let highlighted = Rgba8::new(255, 255, 255, 255);
    let mut target = TreeRowHitTarget::new(TreeRowHitTargetParts {
        label: "Folder".into(),
        selected: true,
        focused: true,
        drag_drop: Default::default(),
        palette: DenseRowPalette::new().selected(Rgba8::new(20, 40, 60, 180)),
        drop_target_outline: crate::gui::list::DenseRowOutlineStyle::new(
            0.5,
            Rgba8::new(0, 0, 0, 0),
            1.0,
        ),
        selected_hover_marker: None,
        normal_label_color: Some(normal),
        highlighted_label_color: highlighted,
        actions: InteractiveRowActions::new().activate(|| TreeRowMessage::Activate),
    });
    let bounds = Rect::from_size(160.0, 22.0);

    target.handle_input(bounds, WidgetInput::pointer_move(Point::new(8.0, 10.0)));
    let plan = target.paint_plan_with_defaults(bounds);

    assert_eq!(plan.first_text_color("Folder"), Some(highlighted));
}

#[test]
fn focused_tree_row_paints_selected_fill() {
    let selected = Rgba8::new(20, 40, 60, 180);
    let target = TreeRowHitTarget::new(TreeRowHitTargetParts {
        label: "Folder".into(),
        selected: false,
        focused: true,
        drag_drop: Default::default(),
        palette: DenseRowPalette::new().selected(selected),
        drop_target_outline: crate::gui::list::DenseRowOutlineStyle::new(
            0.5,
            Rgba8::new(0, 0, 0, 0),
            1.0,
        ),
        selected_hover_marker: None,
        normal_label_color: None,
        highlighted_label_color: Rgba8::new(255, 255, 255, 255),
        actions: InteractiveRowActions::new().activate(|| TreeRowMessage::Activate),
    });
    let bounds = Rect::from_size(160.0, 22.0);

    let plan = target.paint_plan_with_defaults(bounds);

    assert!(
        plan.fill_rects()
            .any(|fill| fill.rect == bounds && fill.color == selected),
        "focused tree row should use the selected fill without pointer-hover emphasis"
    );
}

#[test]
fn selected_focused_tree_row_paints_selected_fill_without_marker() {
    let selected = Rgba8::new(12, 24, 36, 140);
    let selected_hover = Rgba8::new(20, 40, 60, 180);
    let marker = Rgba8::new(220, 80, 40, 245);
    let target = TreeRowHitTarget::new(TreeRowHitTargetParts {
        label: "Folder".into(),
        selected: true,
        focused: true,
        drag_drop: Default::default(),
        palette: DenseRowPalette::new()
            .selected(selected)
            .selected_hovered(selected_hover),
        drop_target_outline: crate::gui::list::DenseRowOutlineStyle::new(
            0.5,
            Rgba8::new(0, 0, 0, 0),
            1.0,
        ),
        selected_hover_marker: Some(DenseRowMarkerStyle::new(
            DenseRowMarkerParts::leading(3.0),
            marker,
        )),
        normal_label_color: None,
        highlighted_label_color: Rgba8::new(255, 255, 255, 255),
        actions: InteractiveRowActions::new().activate(|| TreeRowMessage::Activate),
    });
    let bounds = Rect::from_size(160.0, 22.0);

    let plan = target.paint_plan_with_defaults(bounds);

    assert!(
        plan.fill_rects()
            .any(|fill| fill.rect == bounds && fill.color == selected),
        "selected+focused tree row should use the base selected fill"
    );
    assert!(
        !plan
            .fill_rects()
            .any(|fill| fill.rect.width() == 3.0 && fill.color == marker),
        "selected+focused tree row should not paint pointer-hover marker"
    );
}

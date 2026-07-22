use super::hit_target::{TreeRowHitTarget, TreeRowHitTargetParts};
use crate::{
    application::{IntoView, tree_row},
    gui::{
        list::{DenseRowMarkerParts, DenseRowMarkerStyle, DenseRowPalette},
        types::{Point, Rect, Rgba8, Vector2},
    },
    theme::ThemeTokens,
    widgets::{
        InteractiveRowActions, PointerButton, PointerModifiers, Widget, WidgetInput, WidgetStyle,
        WidgetTone, stable_widget_id,
    },
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
fn tree_row_stable_row_identity_keys_row_and_hit_target() {
    let row_key = "folder-row-source-a";
    fn keyed_row(row_key: &'static str) -> crate::application::ViewNode<TreeRowMessage> {
        tree_row("Folder")
            .stable_row_identity(42, row_key)
            .interactive_actions(InteractiveRowActions::new().activate(|| TreeRowMessage::Activate))
    }
    let input_id = stable_widget_id(42, row_key);
    let mut surface = keyed_row(row_key).into_surface();
    let bounds = Rect::from_size(160.0, 22.0);
    let position = Point::new(8.0, 10.0);

    surface.dispatch_widget_input(
        input_id,
        bounds,
        WidgetInput::PointerPress {
            position,
            button: PointerButton::Primary,
            modifiers: PointerModifiers::default(),
        },
    );
    let output = surface.dispatch_widget_input(
        input_id,
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
    let layout = keyed_row(row_key).view_layout_at_size(Vector2::new(160.0, 22.0));
    let root_id = crate::application::scoped_key_id(crate::application::ROOT_KEY_SCOPE, row_key);

    assert!(
        layout.rects.contains_key(&root_id),
        "stable tree row identity should key the composed row subtree"
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
fn styled_tree_row_resolves_dense_chrome_from_frame_theme() {
    let theme = ThemeTokens {
        accent_mint: Rgba8::new(12, 180, 220, 255),
        ..ThemeTokens::default()
    };
    let view = tree_row("Folder")
        .selected(true)
        .style(WidgetStyle::subtle(WidgetTone::Accent))
        .interactive_actions(InteractiveRowActions::new().activate(|| TreeRowMessage::Activate));

    let frame = view
        .into_surface()
        .frame(Rect::from_size(160.0, 22.0), &theme);

    assert!(
        frame
            .paint_plan
            .fill_rects()
            .any(|fill| fill.color == theme.accent_mint.with_alpha(120)),
        "styled tree rows should resolve selected chrome from the active theme"
    );
}

#[test]
fn selected_tree_row_paints_persistent_leading_marker() {
    let marker = Rgba8::new(240, 80, 60, 255);
    let view = tree_row("Folder")
        .selected(true)
        .selected_marker(DenseRowMarkerStyle::new(
            DenseRowMarkerParts::leading(2.0),
            marker,
        ))
        .interactive_actions(InteractiveRowActions::new().activate(|| TreeRowMessage::Activate));

    let frame = view.view_frame_at_size_with_default_theme(Vector2::new(160.0, 22.0));

    assert!(
        frame
            .paint_plan
            .fill_rects()
            .any(|fill| fill.color == marker && fill.rect.width() == 2.0)
    );
}

#[test]
fn selected_hover_tree_row_paints_configured_fill_and_marker() {
    let selected_hover = Rgba8::new(20, 40, 60, 180);
    let marker = Rgba8::new(220, 80, 40, 245);
    let mut target = TreeRowHitTarget::new(TreeRowHitTargetParts {
        label: "Folder".into(),
        label_inset_x: 4.0,
        selected: true,
        focused: false,
        drag_drop: Default::default(),
        style: None,
        palette: Some(DenseRowPalette::new().selected_hovered(selected_hover)),
        drop_target_outline: Some(crate::gui::list::DenseRowOutlineStyle::new(
            0.5,
            Rgba8::new(0, 0, 0, 0),
            1.0,
        )),
        selected_marker: None,
        selected_trailing_marker: None,
        hover_trailing_marker: None,
        focus_outline: None,
        selected_hover_marker: Some(DenseRowMarkerStyle::new(
            DenseRowMarkerParts::leading(3.0),
            marker,
        )),
        normal_label_color: None,
        highlighted_label_color: Rgba8::new(255, 255, 255, 255),
        trailing_icon: None,
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
        label_inset_x: 4.0,
        selected: true,
        focused: true,
        drag_drop: Default::default(),
        style: None,
        palette: Some(DenseRowPalette::new().selected(Rgba8::new(20, 40, 60, 180))),
        drop_target_outline: Some(crate::gui::list::DenseRowOutlineStyle::new(
            0.5,
            Rgba8::new(0, 0, 0, 0),
            1.0,
        )),
        selected_marker: None,
        selected_trailing_marker: None,
        hover_trailing_marker: None,
        focus_outline: None,
        selected_hover_marker: None,
        normal_label_color: Some(normal),
        highlighted_label_color: highlighted,
        trailing_icon: None,
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
        label_inset_x: 4.0,
        selected: true,
        focused: true,
        drag_drop: Default::default(),
        style: None,
        palette: Some(DenseRowPalette::new().selected(Rgba8::new(20, 40, 60, 180))),
        drop_target_outline: Some(crate::gui::list::DenseRowOutlineStyle::new(
            0.5,
            Rgba8::new(0, 0, 0, 0),
            1.0,
        )),
        selected_marker: None,
        selected_trailing_marker: None,
        hover_trailing_marker: None,
        focus_outline: None,
        selected_hover_marker: None,
        normal_label_color: Some(normal),
        highlighted_label_color: highlighted,
        trailing_icon: None,
        actions: InteractiveRowActions::new().activate(|| TreeRowMessage::Activate),
    });
    let bounds = Rect::from_size(160.0, 22.0);

    target.handle_input(bounds, WidgetInput::pointer_move(Point::new(8.0, 10.0)));
    let plan = target.paint_plan_with_defaults(bounds);

    assert_eq!(plan.first_text_color("Folder"), Some(highlighted));
}

#[test]
fn focused_tree_row_paints_focus_outline_without_selected_fill() {
    let selected = Rgba8::new(20, 40, 60, 180);
    let focus = Rgba8::new(220, 220, 216, 255);
    let target = TreeRowHitTarget::new(TreeRowHitTargetParts {
        label: "Folder".into(),
        label_inset_x: 4.0,
        selected: false,
        focused: true,
        drag_drop: Default::default(),
        style: None,
        palette: Some(DenseRowPalette::new().selected(selected)),
        drop_target_outline: Some(crate::gui::list::DenseRowOutlineStyle::new(
            0.5,
            Rgba8::new(0, 0, 0, 0),
            1.0,
        )),
        selected_marker: None,
        selected_trailing_marker: None,
        hover_trailing_marker: None,
        focus_outline: Some(crate::gui::list::DenseRowOutlineStyle::new(0.5, focus, 1.0)),
        selected_hover_marker: None,
        normal_label_color: None,
        highlighted_label_color: Rgba8::new(255, 255, 255, 255),
        trailing_icon: None,
        actions: InteractiveRowActions::new().activate(|| TreeRowMessage::Activate),
    });
    let bounds = Rect::from_size(160.0, 22.0);

    let plan = target.paint_plan_with_defaults(bounds);

    assert!(!plan.fill_rects().any(|fill| fill.color == selected));
    assert!(plan.stroke_rects().any(|stroke| stroke.color == focus));
}

#[test]
fn selected_focused_tree_row_paints_selected_fill_without_marker() {
    let selected = Rgba8::new(12, 24, 36, 140);
    let selected_hover = Rgba8::new(20, 40, 60, 180);
    let marker = Rgba8::new(220, 80, 40, 245);
    let target = TreeRowHitTarget::new(TreeRowHitTargetParts {
        label: "Folder".into(),
        label_inset_x: 4.0,
        selected: true,
        focused: true,
        drag_drop: Default::default(),
        style: None,
        palette: Some(
            DenseRowPalette::new()
                .selected(selected)
                .selected_hovered(selected_hover),
        ),
        drop_target_outline: Some(crate::gui::list::DenseRowOutlineStyle::new(
            0.5,
            Rgba8::new(0, 0, 0, 0),
            1.0,
        )),
        selected_marker: None,
        selected_trailing_marker: None,
        hover_trailing_marker: None,
        focus_outline: None,
        selected_hover_marker: Some(DenseRowMarkerStyle::new(
            DenseRowMarkerParts::leading(3.0),
            marker,
        )),
        normal_label_color: None,
        highlighted_label_color: Rgba8::new(255, 255, 255, 255),
        trailing_icon: None,
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

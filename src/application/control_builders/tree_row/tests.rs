use super::hit_target::{TreeRowHitTarget, TreeRowHitTargetParts};
use crate::{
    application::{
        ApplicationEnvironment, IntoView, LocaleId, TextScale, WritingDirection, tree_row,
    },
    gui::{
        list::{DenseRowMarkerParts, DenseRowMarkerStyle, DenseRowPalette},
        types::{Point, Rect, Rgba8, Vector2},
    },
    runtime::{PaintPrimitive, ResolvedEnvironment},
    theme::ThemeTokens,
    widgets::{
        InteractiveRowActions, InteractiveRowLocalActions, PointerButton, PointerModifiers, Widget,
        WidgetInput, WidgetPaintContext, WidgetStyle, WidgetTone, stable_widget_id,
    },
};

#[derive(Clone, Debug, PartialEq)]
enum TreeRowMessage {
    Activate,
    ActivateWithModifiers(PointerModifiers),
    Toggle,
}

fn text_environment(scale: f32) -> ResolvedEnvironment {
    ResolvedEnvironment::from_snapshots(
        crate::runtime::WindowEnvironment::new(
            crate::theme::DpiScale::new(2.0),
            None,
            false,
            false,
        ),
        std::sync::Arc::new(
            ApplicationEnvironment::new(LocaleId::english())
                .with_text_scale(TextScale::new(scale).expect("valid text scale")),
        ),
    )
}

fn row_text(plan: &crate::runtime::SurfacePaintPlan) -> &crate::runtime::PaintTextRun {
    plan.text_runs().next().expect("tree row text run")
}

#[test]
fn tree_row_declared_metrics_follow_scale_without_scaling_physical_row_geometry() {
    for (scale, expected_font, expected_inset) in
        [(1.0, 13.0, 4.0), (1.5, 19.5, 6.0), (2.0, 26.0, 8.0)]
    {
        let input_id = 801;
        let surface = tree_row("Folder")
            .input_id(input_id)
            .interactive_actions(InteractiveRowActions::new().activate(|| TreeRowMessage::Activate))
            .into_surface()
            .with_application_environment(
                ApplicationEnvironment::new(LocaleId::english())
                    .with_text_scale(TextScale::new(scale).expect("valid text scale")),
            );
        let frame = surface.frame_at_size_with_default_theme(Vector2::new(160.0, 22.0));
        let hit_bounds = frame
            .layout
            .rects
            .get(&input_id)
            .expect("tree hit target bounds");
        assert_eq!(hit_bounds.height(), 22.0);
        let text = row_text(&frame.paint_plan);
        assert_eq!(text.font_size, expected_font);
        assert_eq!(text.rect.min.x, hit_bounds.min.x + expected_inset);
    }
}

#[test]
fn tree_row_explicit_height_selects_nominal_font_but_stays_physical() {
    let input_id = 802;
    let surface = tree_row("Folder")
        .row_height(38.0)
        .input_id(input_id)
        .interactive_actions(InteractiveRowActions::new().activate(|| TreeRowMessage::Activate))
        .into_surface()
        .with_application_environment(
            ApplicationEnvironment::new(LocaleId::english())
                .with_text_scale(TextScale::new(1.5).expect("valid text scale")),
        );
    let frame = surface.frame_at_size_with_default_theme(Vector2::new(220.0, 38.0));
    let hit_bounds = frame
        .layout
        .rects
        .get(&input_id)
        .expect("tree hit target bounds");
    assert_eq!(hit_bounds.height(), 38.0);
    assert_eq!(row_text(&frame.paint_plan).font_size, 27.0);
}

#[test]
fn tree_row_assigned_bounds_do_not_derive_declared_font_or_inset() {
    let input_id = 803;
    let surface = tree_row("Folder")
        .input_id(input_id)
        .interactive_actions(InteractiveRowActions::new().activate(|| TreeRowMessage::Activate))
        .into_surface();
    let widget = surface
        .find_widget(input_id)
        .expect("tree hit target")
        .widget();
    let environment = text_environment(2.0);
    let layout = crate::layout::LayoutOutput::default();
    let theme = ThemeTokens::default();
    for height in [11.0, 60.0] {
        let bounds = Rect::from_size(120.0, height);
        let mut primitives = Vec::new();
        let mut context =
            WidgetPaintContext::new(&mut primitives, bounds, &layout, &theme, &environment);
        widget.append_paint_with_context(&mut context);
        let PaintPrimitive::Text(text) = primitives
            .iter()
            .find(|primitive| matches!(primitive, PaintPrimitive::Text(_)))
            .expect("tree label")
        else {
            panic!("expected tree text");
        };
        assert_eq!(text.font_size, 26.0);
        assert_eq!(text.rect.min.x, 8.0);
    }
}

#[test]
fn tree_row_optional_semantics_use_host_selection_and_runtime_focus() {
    let input_id = 804;
    let surface = tree_row("Folder")
        .selected(true)
        .input_id(input_id)
        .interactive_actions(InteractiveRowActions::new().activate(|| TreeRowMessage::Activate))
        .into_surface();
    let widget = surface
        .find_widget(input_id)
        .expect("tree hit target")
        .widget();
    let semantics = widget.automation_semantics();
    assert_eq!(semantics.role, crate::gui::automation::AutomationRole::Row);
    assert_eq!(semantics.label.as_deref(), Some("Folder"));
    assert!(semantics.selected);
    assert!(!semantics.focused);

    let mut focused = surface;
    focused
        .find_widget_mut(input_id)
        .expect("tree hit target")
        .widget_mut()
        .handle_input(
            Rect::from_size(120.0, 22.0),
            WidgetInput::FocusChanged(true),
        );
    assert!(
        focused
            .find_widget(input_id)
            .expect("focused tree hit target")
            .widget()
            .automation_semantics()
            .focused
    );
}

#[test]
fn tree_row_keeps_guide_expander_icon_and_hit_geometry_physical() {
    let input_id = 805;
    let view = tree_row("Folder")
        .depth(2)
        .has_children(true)
        .expanded(true)
        .row_height(38.0)
        .trailing_icon(crate::gui::svg::IconName::ChevronDown.icon())
        .input_id(input_id)
        .interactive_actions(InteractiveRowActions::new().activate(|| TreeRowMessage::Activate));
    let frame = view.view_frame_at_size_with_default_theme(Vector2::new(240.0, 38.0));
    let hit_bounds = frame
        .layout
        .rects
        .get(&input_id)
        .expect("tree hit target bounds");
    assert_eq!(hit_bounds.height(), 38.0);
    assert!(
        frame
            .layout
            .rects
            .values()
            .any(|rect| rect.width() == 24.0 && rect.height() == 38.0)
    );
    assert!(
        frame
            .layout
            .rects
            .values()
            .any(|rect| rect.width() == 28.0 && rect.height() == 38.0)
    );
    let icon = frame
        .paint_plan
        .svgs_for_widget(input_id)
        .next()
        .expect("tree trailing icon");
    assert_eq!(icon.rect.width(), 11.0);
    assert_eq!(icon.rect.height(), 11.0);
    assert_eq!(icon.rect.min.x, hit_bounds.max.x - 16.0);
    assert_eq!(icon.rect.min.y, hit_bounds.min.y + 13.5);
}

#[test]
fn tree_row_ui_local_construction_preserves_configured_hit_height() {
    let input_id = 806;
    let view = tree_row("Folder")
        .row_height(38.0)
        .input_id(input_id)
        .interactive_actions_local(
            InteractiveRowLocalActions::new().activate(|| TreeRowMessage::Activate),
        );
    let frame = view.view_frame_at_size_with_default_theme(Vector2::new(200.0, 38.0));
    assert_eq!(
        frame
            .layout
            .rects
            .get(&input_id)
            .expect("local tree hit target bounds")
            .height(),
        38.0
    );
    assert_eq!(row_text(&frame.paint_plan).font_size, 18.0);
}

#[test]
fn tree_row_ltr_and_rtl_mirror_container_geometry_but_keep_label_metrics() {
    let input_id = 807;
    let make = |direction| {
        tree_row("Folder")
            .depth(2)
            .input_id(input_id)
            .interactive_actions(InteractiveRowActions::new().activate(|| TreeRowMessage::Activate))
            .into_surface()
            .with_application_environment(
                ApplicationEnvironment::new(LocaleId::english())
                    .with_writing_direction(direction)
                    .with_text_scale(TextScale::new(1.5).expect("valid text scale")),
            )
            .frame_at_size_with_default_theme(Vector2::new(240.0, 22.0))
    };
    let ltr = make(WritingDirection::Ltr);
    let rtl = make(WritingDirection::Rtl);
    let ltr_hit = ltr.layout.rects.get(&input_id).expect("ltr hit target");
    let rtl_hit = rtl.layout.rects.get(&input_id).expect("rtl hit target");
    assert_eq!(ltr_hit.min.x, 240.0 - rtl_hit.max.x);
    assert_eq!(ltr_hit.max.x, 240.0 - rtl_hit.min.x);
    let ltr_text = row_text(&ltr.paint_plan);
    let rtl_text = row_text(&rtl.paint_plan);
    assert_eq!(ltr_text.text, rtl_text.text);
    assert_eq!(ltr_text.font_size, 19.5);
    assert_eq!(rtl_text.font_size, 19.5);
    assert_eq!(ltr_text.rect.width(), rtl_text.rect.width());
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
            timestamp: None,
        },
    );
    let output = surface.dispatch_widget_input(
        91,
        bounds,
        WidgetInput::PointerRelease {
            position,
            button: PointerButton::Primary,
            modifiers: PointerModifiers::default(),
            timestamp: None,
        },
    );

    assert_eq!(
        output.and_then(|output| surface.dispatch_widget_output(91, output)),
        Some(TreeRowMessage::Activate)
    );
}

#[test]
fn tree_row_pointer_focus_uses_full_marker_only_until_release() {
    let faded = Rgba8::new(230, 230, 226, 0);
    let pressed = Rgba8::new(230, 230, 226, 255);
    let view = tree_row("Folder")
        .focus_marker(DenseRowMarkerStyle::new(
            DenseRowMarkerParts::leading(6.0),
            faded,
        ))
        .pressed_focus_marker(DenseRowMarkerStyle::new(
            DenseRowMarkerParts::leading(6.0),
            pressed,
        ))
        .input_id(92)
        .interactive_actions(InteractiveRowActions::new().activate(|| TreeRowMessage::Activate));
    let mut surface = view.into_surface();
    let bounds = Rect::from_size(160.0, 22.0);
    let position = Point::new(12.0, 10.0);
    let paints_pressed_marker = |surface: &crate::runtime::UiSurface<TreeRowMessage>| {
        surface
            .find_widget(92)
            .expect("tree row hit target")
            .widget()
            .paint_plan_with_defaults(bounds)
            .fill_rects()
            .any(|fill| fill.color == pressed && fill.rect.width() == 6.0)
    };

    assert!(!paints_pressed_marker(&surface));
    surface.dispatch_widget_input(92, bounds, WidgetInput::primary_press(position));
    assert!(paints_pressed_marker(&surface));
    surface.dispatch_widget_input(92, bounds, WidgetInput::primary_release(position));
    assert!(!paints_pressed_marker(&surface));
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
            timestamp: None,
        },
    );
    let output = surface.dispatch_widget_input(
        input_id,
        bounds,
        WidgetInput::PointerRelease {
            position,
            button: PointerButton::Primary,
            modifiers: PointerModifiers::default(),
            timestamp: None,
        },
    );

    assert_eq!(
        output.and_then(|output| surface.dispatch_widget_output(input_id, output)),
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
            timestamp: None,
        },
    );
    let output = surface.dispatch_widget_input(
        92,
        bounds,
        WidgetInput::PointerRelease {
            position,
            button: PointerButton::Primary,
            modifiers,
            timestamp: None,
        },
    );

    assert_eq!(
        output.and_then(|output| surface.dispatch_widget_output(92, output)),
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
        row_height: 22.0,
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
        focus_marker: None,
        pressed_focus_marker: None,
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
        row_height: 22.0,
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
        focus_marker: None,
        pressed_focus_marker: None,
        selected_trailing_marker: None,
        hover_trailing_marker: None,
        focus_outline: None,
        selected_hover_marker: None,
        normal_label_color: Some(normal),
        highlighted_label_color: highlighted,
        trailing_icon: None,
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
        row_height: 22.0,
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
        focus_marker: None,
        pressed_focus_marker: None,
        selected_trailing_marker: None,
        hover_trailing_marker: None,
        focus_outline: None,
        selected_hover_marker: None,
        normal_label_color: Some(normal),
        highlighted_label_color: highlighted,
        trailing_icon: None,
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
        row_height: 22.0,
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
        focus_marker: None,
        pressed_focus_marker: None,
        selected_trailing_marker: None,
        hover_trailing_marker: None,
        focus_outline: Some(crate::gui::list::DenseRowOutlineStyle::new(0.5, focus, 1.0)),
        selected_hover_marker: None,
        normal_label_color: None,
        highlighted_label_color: Rgba8::new(255, 255, 255, 255),
        trailing_icon: None,
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
        row_height: 22.0,
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
        focus_marker: None,
        pressed_focus_marker: None,
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

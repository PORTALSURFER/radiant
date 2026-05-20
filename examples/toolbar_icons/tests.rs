use super::*;
use crate::icon_button::IconToggleButton;
use crate::icons::ToolbarIcons;
use crate::model::{ToolId, ToolMessage, ToolbarState};
use crate::view::project_surface;
use radiant::runtime::SurfaceRuntime;

#[test]
fn svg_toolbar_icons_parse_active_and_inactive_vector_icons() {
    let theme = ThemeTokens::default();
    let icons = ToolbarIcons::new(&theme);

    assert!(Arc::ptr_eq(
        &icons.select.active_glyph,
        &icons.select.active_glyph
    ));
    assert!(!Arc::ptr_eq(
        &icons.select.active_glyph,
        &icons.select.inactive_glyph
    ));
    assert!(!Arc::ptr_eq(
        &icons.brush.active_glyph,
        &icons.brush.inactive_glyph
    ));
}

#[test]
fn toolbar_button_routes_toggle_through_runtime() {
    let bridge = radiant::app(ToolbarState::default())
        .view(project_surface)
        .update(|state, message| match message {
            ToolMessage::Toggle(tool) => state.toggle(tool),
        })
        .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(360.0, 150.0));

    assert!(runtime.dispatch_input(
        11,
        WidgetInput::PointerPress {
            position: Point::new(70.0, 60.0),
            button: PointerButton::Primary,
            modifiers: Default::default(),
        },
    ));
    assert!(runtime.dispatch_input(
        11,
        WidgetInput::PointerRelease {
            position: Point::new(70.0, 60.0),
            button: PointerButton::Primary,
            modifiers: Default::default(),
        },
    ));

    let brush_active = runtime
        .surface()
        .find_widget(11)
        .and_then(|widget| {
            widget
                .widget_object()
                .as_any()
                .downcast_ref::<IconToggleButton>()
        })
        .map(|button| button.active);
    assert_eq!(brush_active, Some(true));
}

#[test]
fn toolbar_button_paints_svg_icon_and_active_marker() {
    let theme = ThemeTokens::default();
    let button = IconToggleButton::new(ToolId::Select, ToolbarIcons::new(&theme).select, true);
    let mut primitives = Vec::new();
    button.append_paint(
        &mut primitives,
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(42.0, 36.0)),
        &LayoutOutput::default(),
        &theme,
    );

    assert!(
        primitives
            .iter()
            .any(|primitive| matches!(primitive, PaintPrimitive::Svg(_)))
    );
    let PaintPrimitive::FillRect(background) = primitives[0] else {
        panic!("active toolbar button should paint a background first");
    };
    assert_ne!(background.color, theme.accent_warning);
    assert!(primitives.len() >= 4);
}

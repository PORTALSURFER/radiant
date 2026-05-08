//! Public API coverage for surface hover and container chrome behavior.

use radiant::{
    gui::types::Rgba8,
    layout::{Point, Vector2},
    runtime::{
        Event, PaintPrimitive, SurfacePaintPlan, SurfaceRuntime, UiSurface,
        declarative_runtime_bridge,
    },
    theme::ThemeTokens,
    widgets::WidgetStyle,
};
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq)]
enum DemoMessage {
    Increment,
    Rename(String),
    SetActive(bool),
}

fn widget_fill_color(plan: &SurfacePaintPlan, widget_id: u64) -> Option<Rgba8> {
    plan.primitives
        .iter()
        .find_map(|primitive| match primitive {
            PaintPrimitive::FillRect(fill) if fill.widget_id == widget_id => Some(fill.color),
            PaintPrimitive::FillPolygon(fill) if fill.widget_id == widget_id => Some(fill.color),
            _ => None,
        })
}

fn widget_stroke_color(plan: &SurfacePaintPlan, widget_id: u64) -> Option<Rgba8> {
    plan.primitives
        .iter()
        .find_map(|primitive| match primitive {
            PaintPrimitive::StrokeRect(stroke) if stroke.widget_id == widget_id => {
                Some(stroke.color)
            }
            PaintPrimitive::StrokePolygon(stroke) if stroke.widget_id == widget_id => {
                Some(stroke.color)
            }
            _ => None,
        })
}

#[test]
fn checkbox_hover_changes_paint_chrome() {
    use radiant::prelude::{self as ui, IntoView};

    let surface: UiSurface<DemoMessage> = ui::checkbox(false)
        .message(|_| DemoMessage::SetActive(false))
        .id(10)
        .into_surface();
    let bridge =
        declarative_runtime_bridge(Arc::new(surface), |surface| Arc::clone(surface), |_, _| {});
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(64.0, 64.0));
    let theme = ThemeTokens::default();
    let before = widget_fill_color(&runtime.paint_plan(&theme), 10);

    runtime.dispatch_event(Event::PointerMove {
        position: Point::new(12.0, 12.0),
    });
    let after = widget_fill_color(&runtime.paint_plan(&theme), 10);

    assert_ne!(before, after);
}

#[test]
fn styled_list_row_hover_changes_container_chrome() {
    use radiant::prelude::{self as ui, IntoView};

    let surface: UiSurface<DemoMessage> =
        ui::list_row("row", [ui::text("Hover row").id(20).fill_width()])
            .id(10)
            .into_surface();
    let bridge =
        declarative_runtime_bridge(Arc::new(surface), |surface| Arc::clone(surface), |_, _| {});
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(240.0, 52.0));
    let theme = ThemeTokens::default();
    let before_plan = runtime.paint_plan(&theme);
    let before = widget_fill_color(&before_plan, 10);
    let border_before = widget_stroke_color(&before_plan, 10);

    runtime.dispatch_event(Event::PointerMove {
        position: Point::new(12.0, 12.0),
    });
    let after_plan = runtime.paint_plan(&theme);
    let after = widget_fill_color(&after_plan, 10);
    let border_after = widget_stroke_color(&after_plan, 10);
    let hover_markers: Vec<_> = after_plan
        .primitives
        .iter()
        .filter_map(|primitive| match primitive {
            PaintPrimitive::FillRect(fill)
                if fill.widget_id == 10
                    && fill.color == theme.accent_danger
                    && fill.rect.width() <= 3.0 =>
            {
                Some(fill)
            }
            _ => None,
        })
        .collect();

    assert_eq!(runtime.hovered_container(), Some(10));
    assert_ne!(before, after);
    assert_eq!(border_before, border_after);
    assert_eq!(hover_markers.len(), 1);
}

#[test]
fn static_styled_container_does_not_hover_without_opt_in() {
    use radiant::prelude::{self as ui, IntoView};

    let surface: UiSurface<DemoMessage> = ui::row([ui::text("Header").id(20)])
        .id(10)
        .style(WidgetStyle::default())
        .padding(8.0)
        .into_surface();
    let bridge =
        declarative_runtime_bridge(Arc::new(surface), |surface| Arc::clone(surface), |_, _| {});
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(240.0, 52.0));
    let theme = ThemeTokens::default();
    let before = widget_fill_color(&runtime.paint_plan(&theme), 10);

    runtime.dispatch_event(Event::PointerMove {
        position: Point::new(12.0, 12.0),
    });
    let after = widget_fill_color(&runtime.paint_plan(&theme), 10);

    assert_eq!(runtime.hovered_container(), None);
    assert_eq!(before, after);
}

#[test]
fn control_hover_suppresses_surrounding_container_hover_chrome() {
    use radiant::prelude::{self as ui, IntoView};

    let surface: UiSurface<DemoMessage> = ui::column([ui::text_input("")
        .placeholder("Type")
        .message(DemoMessage::Rename)
        .id(20)
        .size(180.0, 32.0)])
    .id(10)
    .style(WidgetStyle::default())
    .padding(8.0)
    .into_surface();
    let bridge =
        declarative_runtime_bridge(Arc::new(surface), |surface| Arc::clone(surface), |_, _| {});
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(240.0, 64.0));
    let theme = ThemeTokens::default();
    let before = runtime.paint_plan(&theme);
    let body_before = widget_fill_color(&before, 10);
    let input_before = widget_fill_color(&before, 20);

    runtime.dispatch_event(Event::PointerMove {
        position: Point::new(12.0, 12.0),
    });
    let after = runtime.paint_plan(&theme);

    assert_eq!(runtime.hovered_widget(), Some(20));
    assert_eq!(runtime.hovered_container(), None);
    assert_eq!(body_before, widget_fill_color(&after, 10));
    assert_ne!(input_before, widget_fill_color(&after, 20));
}

#[test]
fn input_only_controls_do_not_suppress_surrounding_container_hover_chrome() {
    use radiant::prelude::{self as ui, IntoView};

    let surface: UiSurface<DemoMessage> = ui::list_row(
        "row",
        [ui::button("Cell")
            .message(DemoMessage::Increment)
            .id(20)
            .input_only()
            .size(120.0, 24.0)],
    )
    .id(10)
    .into_surface();
    let bridge =
        declarative_runtime_bridge(Arc::new(surface), |surface| Arc::clone(surface), |_, _| {});
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(240.0, 52.0));
    let theme = ThemeTokens::default();
    let before = runtime.paint_plan(&theme);
    let body_before = widget_fill_color(&before, 10);

    runtime.dispatch_event(Event::PointerMove {
        position: Point::new(24.0, 20.0),
    });
    let after = runtime.paint_plan(&theme);

    assert_eq!(runtime.hovered_widget(), Some(20));
    assert_eq!(runtime.hovered_container(), Some(10));
    assert_ne!(body_before, widget_fill_color(&after, 10));
}

use super::support::{CustomStatusWidget, DemoMessage, widget_fill_color};
use radiant::{
    layout::{Point, Vector2},
    prelude::IntoView,
    runtime::{Event, SurfaceRuntime, UiSurface, declarative_runtime_bridge},
    theme::ThemeTokens,
};
use std::sync::Arc;

#[test]
fn custom_widget_contract_can_suppress_surrounding_container_hover_chrome() {
    use radiant::prelude as ui;

    let mut custom = CustomStatusWidget::new(20);
    custom.common.focus = radiant::widgets::FocusBehavior::None;
    custom.common.paint.suppresses_container_hover = true;

    let surface: UiSurface<DemoMessage> = ui::list_row(
        "row",
        [ui::custom_widget(custom, |_| None).id(20).size(120.0, 24.0)],
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
    assert_eq!(runtime.hovered_container(), None);
    assert_eq!(body_before, widget_fill_color(&after, 10));
}

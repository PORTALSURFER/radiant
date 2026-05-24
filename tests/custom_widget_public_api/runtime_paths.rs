use super::support::{CustomStatusWidget, CustomWidgetMessage, DemoMessage, DemoState};
use radiant::{
    layout::{Point, Rect, Vector2, layout_tree},
    runtime::{PaintPrimitive, SurfaceNode, SurfaceRuntime, UiSurface, WidgetMessageMapper},
    theme::ThemeTokens,
    widgets::{PointerButton, WidgetInput},
};

#[test]
fn runtime_lets_custom_widgets_reconcile_retained_state_after_refresh() {
    use radiant::prelude as ui;

    let bridge = ui::app(DemoState::default())
        .view(|state| {
            ui::column([
                ui::custom_widget_mapped(
                    CustomStatusWidget::new(1),
                    |message: CustomWidgetMessage| DemoMessage::Rename(format!("{message:?}")),
                )
                .id(30),
                ui::text(state.name.clone()).id(31),
            ])
        })
        .update(|state, message| {
            if let DemoMessage::Rename(name) = message {
                state.name = name;
            }
        })
        .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(160.0, 48.0));

    assert!(runtime.dispatch_input(
        30,
        WidgetInput::PointerRelease {
            position: Point::new(12.0, 12.0),
            button: PointerButton::Primary,
            modifiers: Default::default(),
        },
    ));

    let custom = runtime
        .surface()
        .find_widget(30)
        .and_then(|widget| {
            widget
                .widget_object()
                .as_any()
                .downcast_ref::<CustomStatusWidget>()
        })
        .expect("custom widget should remain projected");

    assert_eq!(custom.activation_count, 1);
}

#[test]
fn custom_widget_travels_through_runtime_input_message_and_paint_paths() {
    let surface: UiSurface<DemoMessage> = UiSurface::new(SurfaceNode::custom_widget(
        CustomStatusWidget::new(91),
        WidgetMessageMapper::dynamic(|output| {
            output
                .custom_ref::<CustomWidgetMessage>()
                .map(|message| DemoMessage::Rename(format!("{message:?}")))
        }),
    ));
    let layout = layout_tree(
        &surface.layout_node(),
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 28.0)),
    );
    let plan = surface.paint_plan(&layout, &ThemeTokens::default());

    assert!(matches!(
        plan.primitives.first(),
        Some(PaintPrimitive::FillRect(fill)) if fill.widget_id == 91
    ));

    let mut interactive = surface.clone();
    let output = interactive
        .dispatch_widget_input(
            91,
            layout.rects[&91],
            WidgetInput::PointerRelease {
                position: Point::new(12.0, 12.0),
                button: PointerButton::Primary,
                modifiers: Default::default(),
            },
        )
        .expect("custom widget should emit output");
    let message = surface
        .dispatch_widget_output(91, output)
        .expect("custom output should map to a host message");

    assert_eq!(message, DemoMessage::Rename("Activated".to_owned()));
}

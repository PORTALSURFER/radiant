use super::support::{CustomStatusWidget, CustomWidgetMessage, DemoMessage, DemoState};
use radiant::{
    application as app,
    layout::Vector2,
    prelude::IntoView,
    runtime::{SurfaceRuntime, UiSurface, WidgetMessageMapper},
    widgets::{PointerButton, TextWidget, WidgetInput, WidgetOutput},
};
use std::sync::Arc;

#[test]
fn application_builder_accepts_custom_widgets_with_generated_and_explicit_ids() {
    use radiant::prelude as ui;

    let surface: UiSurface<DemoMessage> = ui::column([
        ui::custom_widget_mapped(
            CustomStatusWidget::new(3),
            |message: CustomWidgetMessage| DemoMessage::Rename(format!("{message:?}")),
        )
        .key("typed-custom"),
        ui::custom_widget(CustomStatusWidget::new(1), |output| {
            output
                .custom_ref::<CustomWidgetMessage>()
                .map(|_| DemoMessage::SetActive(true))
        })
        .key("generated-custom"),
        ui::custom_widget(CustomStatusWidget::new(2), |output| {
            output
                .custom_ref::<CustomWidgetMessage>()
                .map(|_| DemoMessage::SetActive(false))
        })
        .id(77),
    ])
    .id(10)
    .into_surface();

    assert!(surface.find_widget(77).is_some());
    assert_eq!(
        surface.find_widget(77).unwrap().widget_object().common().id,
        77
    );
    assert_eq!(surface.keyboard_focus_order().len(), 3);
}

#[test]
fn application_builder_custom_widget_views_support_named_parts_construction() {
    use radiant::prelude as ui;

    let surface: UiSurface<DemoMessage> = ui::row([
        ui::widget(ui::MappedWidget::from_parts(app::MappedWidgetParts {
            widget: CustomStatusWidget::new(1),
            messages: WidgetMessageMapper::dynamic(|output| {
                output
                    .custom_ref::<CustomWidgetMessage>()
                    .map(|message| DemoMessage::Rename(format!("{message:?}")))
            }),
        }))
        .id(41),
        ui::widget(ui::DynamicWidget::from_parts(app::DynamicWidgetParts {
            widget: Box::new(CustomStatusWidget::new(2)),
            map: Arc::new(|output| {
                output
                    .custom_ref::<CustomWidgetMessage>()
                    .map(|_| DemoMessage::SetActive(true))
            }),
        }))
        .id(42),
    ])
    .into_surface();

    assert!(surface.find_widget(41).is_some());
    assert!(surface.find_widget(42).is_some());
    assert_eq!(surface.keyboard_focus_order(), vec![41, 42]);

    let message = surface
        .dispatch_widget_output(42, WidgetOutput::custom(CustomWidgetMessage::Activated))
        .expect("dynamic widget named-parts mapper should emit a message");
    assert_eq!(message, DemoMessage::SetActive(true));
}

#[test]
fn application_builder_routes_typed_custom_widget_output() {
    use radiant::layout::Point;
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

    let handled = runtime.dispatch_input(
        30,
        WidgetInput::PointerRelease {
            position: Point::new(12.0, 12.0),
            button: PointerButton::Primary,
            modifiers: Default::default(),
        },
    );
    let surface = runtime.surface();
    let text = surface
        .find_widget(31)
        .and_then(|widget| widget.widget_object().as_any().downcast_ref::<TextWidget>())
        .map(|widget| widget.text.as_str());

    assert!(handled);
    assert_eq!(text, Some("Activated"));
}

#[test]
fn application_builder_routes_direct_custom_widget_messages() {
    use radiant::prelude as ui;

    let message = ui::custom_widget_direct(DirectDemoWidget::new())
        .id(30)
        .view_dispatch_widget_output(30, WidgetOutput::typed(DemoMessage::SetActive(true)));

    assert_eq!(message, Some(DemoMessage::SetActive(true)));
}

#[derive(Clone)]
struct DirectDemoWidget {
    common: radiant::widgets::WidgetCommon,
}

impl DirectDemoWidget {
    fn new() -> Self {
        Self {
            common: radiant::widgets::WidgetCommon::new(
                30,
                radiant::widgets::WidgetSizing::fixed(Vector2::new(80.0, 20.0)),
            ),
        }
    }
}

impl radiant::widgets::Widget for DirectDemoWidget {
    fn common(&self) -> &radiant::widgets::WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut radiant::widgets::WidgetCommon {
        &mut self.common
    }

    fn handle_input(
        &mut self,
        _bounds: radiant::prelude::Rect,
        _input: WidgetInput,
    ) -> Option<WidgetOutput> {
        None
    }

    fn append_paint(
        &self,
        _primitives: &mut Vec<radiant::prelude::PaintPrimitive>,
        _bounds: radiant::prelude::Rect,
        _layout: &radiant::prelude::LayoutOutput,
        _theme: &radiant::prelude::ThemeTokens,
    ) {
    }
}

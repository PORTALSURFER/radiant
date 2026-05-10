//! Public API coverage for user-defined Radiant widgets.

use radiant::prelude::IntoView;
use radiant::{
    gui::types::Rgba8,
    layout::{Point, Rect, Vector2, layout_tree},
    runtime::{
        Event, PaintPrimitive, SurfaceNode, SurfaceRuntime, UiSurface, WidgetMessageMapper,
        declarative_runtime_bridge,
    },
    theme::ThemeTokens,
    widgets::{
        PointerButton, TextWidget, Widget, WidgetCommon, WidgetInput, WidgetKey, WidgetOutput,
        WidgetSizing,
    },
};
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq)]
enum DemoMessage {
    Rename(String),
    SetActive(bool),
}

#[derive(Default)]
struct DemoState {
    name: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CustomWidgetMessage {
    Activated,
}

#[derive(Clone)]
struct CustomStatusWidget {
    common: WidgetCommon,
    label: &'static str,
    activation_count: usize,
}

impl CustomStatusWidget {
    fn new(id: u64) -> Self {
        let mut common = WidgetCommon::new(id, WidgetSizing::fixed(Vector2::new(120.0, 28.0)));
        common.focus = radiant::widgets::FocusBehavior::Keyboard;
        Self {
            common,
            label: "custom",
            activation_count: 0,
        }
    }
}

impl Widget for CustomStatusWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        match input {
            WidgetInput::PointerMove { position } => {
                self.common.state.hovered = bounds.contains(position);
                None
            }
            WidgetInput::PointerRelease {
                position,
                button: PointerButton::Primary,
            } if bounds.contains(position) => {
                self.activation_count += 1;
                Some(WidgetOutput::custom(CustomWidgetMessage::Activated))
            }
            WidgetInput::KeyPress(WidgetKey::Enter) if self.common.state.focused => {
                self.activation_count += 1;
                Some(WidgetOutput::custom(CustomWidgetMessage::Activated))
            }
            WidgetInput::FocusChanged(focused) => {
                self.common.state.focused = focused;
                None
            }
            _ => None,
        }
    }

    fn synchronize_from_previous(&mut self, previous: &dyn Widget) {
        if let Some(previous) = previous.as_any().downcast_ref::<CustomStatusWidget>() {
            self.activation_count = previous.activation_count;
        }
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &radiant::layout::LayoutOutput,
        theme: &ThemeTokens,
    ) {
        primitives.push(PaintPrimitive::FillRect(radiant::runtime::PaintFillRect {
            widget_id: self.common.id,
            rect: bounds,
            color: if self.common.state.hovered {
                theme.accent_danger
            } else {
                theme.surface_base
            },
        }));
        primitives.push(PaintPrimitive::Text(radiant::runtime::PaintTextRun {
            widget_id: self.common.id,
            text: self.label.into(),
            rect: bounds,
            font_size: 13.0,
            baseline: Some(18.0),
            color: theme.text_primary,
            align: radiant::runtime::PaintTextAlign::Center,
            wrap: radiant::widgets::TextWrap::None,
        }));
    }
}

fn widget_fill_color(plan: &radiant::runtime::SurfacePaintPlan, widget_id: u64) -> Option<Rgba8> {
    plan.primitives
        .iter()
        .find_map(|primitive| match primitive {
            PaintPrimitive::FillRect(fill) if fill.widget_id == widget_id => Some(fill.color),
            _ => None,
        })
}

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
            },
        )
        .expect("custom widget should emit output");
    let message = surface
        .dispatch_widget_output(91, output)
        .expect("custom output should map to a host message");

    assert_eq!(message, DemoMessage::Rename("Activated".to_owned()));
}

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
fn application_builder_routes_typed_custom_widget_output() {
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

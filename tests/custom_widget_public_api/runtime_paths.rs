use super::support::{CustomStatusWidget, CustomWidgetMessage, DemoMessage, DemoState};
use radiant::{
    layout::{LayoutOutput, Point, Rect, Vector2, layout_tree},
    runtime::{PaintPrimitive, SurfaceNode, SurfaceRuntime, UiSurface, WidgetMessageMapper},
    theme::ThemeTokens,
    widgets::{PointerButton, Widget, WidgetCommon, WidgetInput, WidgetOutput, WidgetSizing},
};

#[derive(Clone)]
struct OverflowPaintWidget {
    common: WidgetCommon,
}

impl OverflowPaintWidget {
    fn new(id: u64) -> Self {
        Self {
            common: WidgetCommon::new(id, WidgetSizing::fixed(Vector2::new(80.0, 40.0))),
        }
    }
}

impl Widget for OverflowPaintWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, _bounds: Rect, _input: WidgetInput) -> Option<WidgetOutput> {
        None
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        theme: &ThemeTokens,
    ) {
        primitives.push(PaintPrimitive::FillRect(radiant::runtime::PaintFillRect {
            widget_id: self.common.id,
            rect: Rect::from_min_max(
                Point::new(bounds.min.x - 24.0, bounds.min.y - 12.0),
                Point::new(bounds.max.x + 24.0, bounds.max.y + 12.0),
            ),
            color: theme.highlight_cyan,
        }));
    }
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

    assert!(
        plan.primitives.iter().any(
            |primitive| matches!(primitive, PaintPrimitive::FillRect(fill) if fill.widget_id == 91)
        ),
        "custom widget paint should still travel through the runtime paint path"
    );

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

#[test]
fn runtime_applies_widget_paint_bounds_clip_to_custom_widget_paint() {
    let surface: UiSurface<DemoMessage> = UiSurface::new(SurfaceNode::custom_widget(
        OverflowPaintWidget::new(710),
        WidgetMessageMapper::dynamic(|_| None),
    ));
    let layout = layout_tree(
        &surface.layout_node(),
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(80.0, 40.0)),
    );

    let plan = surface.paint_plan(&layout, &ThemeTokens::default());
    let clip_start = plan
        .primitives
        .iter()
        .position(
            |primitive| matches!(primitive, PaintPrimitive::ClipStart(clip) if clip.node_id == 710),
        )
        .expect("default widget paint bounds should begin a clip");
    let fill = plan
        .primitives
        .iter()
        .position(
            |primitive| matches!(primitive, PaintPrimitive::FillRect(fill) if fill.widget_id == 710),
        )
        .expect("custom widget fill should be emitted");
    let clip_end = plan
        .primitives
        .iter()
        .position(
            |primitive| matches!(primitive, PaintPrimitive::ClipEnd(clip) if clip.node_id == 710),
        )
        .expect("default widget paint bounds should end the clip");
    let PaintPrimitive::ClipStart(clip) = &plan.primitives[clip_start] else {
        unreachable!("clip_start index was matched above");
    };

    assert_eq!(clip.rect, layout.rects[&710]);
    assert!(clip_start < fill);
    assert!(fill < clip_end);
}

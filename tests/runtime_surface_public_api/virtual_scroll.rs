use super::DemoMessage;
use radiant::{
    layout::{Point, Rect, Vector2, VirtualizationAxis},
    runtime::{
        PaintPrimitive, SurfaceChild, SurfaceNode, SurfaceRuntime, UiSurface, WidgetMessageMapper,
        declarative_runtime_bridge,
    },
    theme::ThemeTokens,
    widgets::{TextWidget, Widget, WidgetCommon, WidgetInput, WidgetSizing},
};
use std::sync::Arc;

#[derive(Clone, Debug)]
struct CustomWheelHitWidget {
    common: WidgetCommon,
}

impl CustomWheelHitWidget {
    fn new(id: u64) -> Self {
        Self {
            common: WidgetCommon::new(
                id,
                radiant::widgets::WidgetSizing::fixed(Vector2::new(120.0, 40.0)),
            ),
        }
    }
}

impl Widget for CustomWheelHitWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(
        &mut self,
        _bounds: Rect,
        input: WidgetInput,
    ) -> Option<radiant::widgets::WidgetOutput> {
        matches!(input, WidgetInput::Wheel { .. })
            .then(|| radiant::widgets::WidgetOutput::typed(DemoMessage::Increment))
    }

    fn accepts_pointer_input(&self, input: &WidgetInput) -> bool {
        input.pointer_position().is_some_and(|point| point.x < 60.0)
    }

    fn accepts_wheel_input(&self) -> bool {
        true
    }

    fn append_paint(
        &self,
        _primitives: &mut Vec<PaintPrimitive>,
        _bounds: Rect,
        _layout: &radiant::layout::LayoutOutput,
        _theme: &ThemeTokens,
    ) {
    }
}

#[test]
fn wheel_routing_honors_custom_pointer_hit_policy() {
    let bridge = declarative_runtime_bridge(
        0_usize,
        |_state: &mut usize| {
            Arc::new(UiSurface::new(SurfaceNode::custom_widget(
                CustomWheelHitWidget::new(1),
                WidgetMessageMapper::typed(|message: DemoMessage| message),
            )))
        },
        |count: &mut usize, message| {
            if message == DemoMessage::Increment {
                *count += 1;
            }
        },
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(120.0, 40.0));

    assert!(runtime.wheel_or_scroll_at(Point::new(30.0, 20.0), Vector2::new(0.0, -40.0)));
    assert_eq!(*runtime.bridge().state(), 1);
    assert!(!runtime.wheel_or_scroll_at(Point::new(90.0, 20.0), Vector2::new(0.0, -40.0)));
    assert_eq!(*runtime.bridge().state(), 1);
}

#[test]
fn surface_runtime_scrolls_virtual_list_with_cached_layout_and_bounded_paint_plan() {
    let bridge = declarative_runtime_bridge(
        (),
        |_state: &mut ()| {
            let rows = (0..10_000_u64)
                .map(|index| {
                    SurfaceChild::fill(SurfaceNode::static_widget(TextWidget::new(
                        index + 10,
                        format!("Row {index:05}"),
                        WidgetSizing::fixed(Vector2::new(160.0, 28.0)).with_baseline(18.0),
                    )))
                })
                .collect::<Vec<_>>();
            Arc::new(UiSurface::new(SurfaceNode::virtual_scroll_area(
                1,
                SurfaceNode::column(2, 4.0, rows),
                VirtualizationAxis::Vertical,
                96.0,
            )))
        },
        |_state: &mut (), _message: DemoMessage| {},
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(220.0, 120.0));

    assert!(runtime.scroll_at(Point::new(24.0, 24.0), Vector2::new(0.0, 3_000.0)));

    let layout = runtime.layout();
    let window = layout
        .virtual_windows
        .get(&1)
        .expect("virtual scroll window should be resolved");
    assert!(window.first_index > 0);
    assert!(window.last_index_exclusive - window.first_index < 128);
    assert!(
        layout.stats.measured_nodes < 64,
        "scroll relayout should reuse virtual metrics instead of measuring the full list"
    );

    let paint = runtime.paint_plan(&ThemeTokens::default());
    assert!(
        paint.primitives.len() < 160,
        "virtual scroll paint should stay bounded to the materialized window"
    );
}

#[test]
fn surface_runtime_skips_non_wheel_widgets_before_virtual_scroll_fallback() {
    let bridge = declarative_runtime_bridge(
        (),
        |_state: &mut ()| {
            let rows = (0..10_000_u64)
                .map(|index| {
                    SurfaceChild::fill(SurfaceNode::custom_widget(
                        PanicOnWheelWidget::new(index + 10),
                        WidgetMessageMapper::typed(|_: DemoMessage| DemoMessage::Increment),
                    ))
                })
                .collect::<Vec<_>>();
            Arc::new(UiSurface::new(SurfaceNode::virtual_scroll_area(
                1,
                SurfaceNode::column(2, 4.0, rows),
                VirtualizationAxis::Vertical,
                96.0,
            )))
        },
        |_state: &mut (), _message: DemoMessage| {},
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(220.0, 120.0));

    assert!(runtime.wheel_or_scroll_at(Point::new(24.0, 24.0), Vector2::new(0.0, 3_000.0)));

    let window = runtime
        .layout()
        .virtual_windows
        .get(&1)
        .expect("virtual scroll window should be resolved");
    assert!(window.first_index > 0);
}

#[derive(Clone, Debug)]
struct PanicOnWheelWidget {
    common: WidgetCommon,
}

impl PanicOnWheelWidget {
    fn new(id: u64) -> Self {
        let mut common = WidgetCommon::new(
            id,
            WidgetSizing::fixed(Vector2::new(160.0, 28.0)).with_baseline(18.0),
        );
        common.focus = radiant::widgets::FocusBehavior::Pointer;
        Self { common }
    }
}

impl Widget for PanicOnWheelWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(
        &mut self,
        _bounds: Rect,
        input: WidgetInput,
    ) -> Option<radiant::widgets::WidgetOutput> {
        if matches!(input, WidgetInput::Wheel { .. }) {
            panic!("wheel input should skip widgets that do not opt into wheel routing");
        }
        None
    }

    fn append_paint(
        &self,
        _primitives: &mut Vec<PaintPrimitive>,
        _bounds: Rect,
        _layout: &radiant::layout::LayoutOutput,
        _theme: &ThemeTokens,
    ) {
    }
}

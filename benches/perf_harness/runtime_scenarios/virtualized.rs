use radiant::{
    layout::{Point, SizeModeCross, SizeModeMain, SlotParams, Vector2, VirtualizationAxis},
    runtime::{
        Event, RuntimeBridge, SurfaceChild, SurfaceNode, SurfaceRuntime, UiSurface,
        WidgetMessageMapper,
    },
    theme::ThemeTokens,
    widgets::{ButtonWidget, WidgetSizing},
};
use std::{hint::black_box, sync::Arc};

pub(super) fn virtualized_list_wheel_10k() -> impl FnMut() {
    let mut virtualized_wheel = StatefulVirtualizedWheelBench::new();
    move || virtualized_wheel.step()
}

pub(super) fn virtualized_list_hover_10k() -> impl FnMut() {
    let mut virtualized_hover = StatefulVirtualizedHoverBench::new();
    move || virtualized_hover.step()
}

pub(super) fn virtualized_list_stable_hover_10k() -> impl FnMut() {
    let mut virtualized_stable_hover = StatefulVirtualizedStableHoverBench::new();
    move || virtualized_stable_hover.step()
}

pub(super) fn virtualized_list_hover_paint_10k() -> impl FnMut() {
    let mut virtualized_hover_paint = StatefulVirtualizedHoverPaintBench::new();
    move || virtualized_hover_paint.step()
}

pub(super) fn virtualized_nested_scroll_hover_10k() -> impl FnMut() {
    let mut virtualized_nested_scroll_hover = StatefulVirtualizedNestedScrollHoverBench::new();
    move || virtualized_nested_scroll_hover.step()
}

struct StatefulVirtualizedWheelBench {
    runtime: SurfaceRuntime<VirtualWheelBridge, ()>,
    offset: f32,
}

impl StatefulVirtualizedWheelBench {
    fn new() -> Self {
        Self {
            runtime: SurfaceRuntime::new(VirtualWheelBridge, Vector2::new(220.0, 120.0)),
            offset: 0.0,
        }
    }

    fn step(&mut self) {
        self.offset = (self.offset + 360.0) % 120_000.0;
        let handled = self
            .runtime
            .wheel_or_scroll_at(Point::new(24.0, 24.0), Vector2::new(0.0, self.offset));
        assert!(handled);
        black_box(self.runtime.layout());
    }
}

struct StatefulVirtualizedHoverBench {
    runtime: SurfaceRuntime<VirtualWheelBridge, ()>,
    offset: f32,
}

impl StatefulVirtualizedHoverBench {
    fn new() -> Self {
        Self {
            runtime: SurfaceRuntime::new(VirtualWheelBridge, Vector2::new(220.0, 120.0)),
            offset: 0.0,
        }
    }

    fn step(&mut self) {
        self.offset = (self.offset + 360.0) % 120_000.0;
        let scrolled = self
            .runtime
            .wheel_or_scroll_at(Point::new(24.0, 24.0), Vector2::new(0.0, self.offset));
        assert!(scrolled);
        let hovered = self.runtime.dispatch_event(Event::PointerMove {
            position: Point::new(24.0, 24.0),
        });
        assert!(hovered.is_some());
        black_box(self.runtime.layout());
    }
}

struct StatefulVirtualizedStableHoverBench {
    runtime: SurfaceRuntime<VirtualWheelBridge, ()>,
    x: f32,
}

impl StatefulVirtualizedStableHoverBench {
    fn new() -> Self {
        let mut runtime = SurfaceRuntime::new(VirtualWheelBridge, Vector2::new(220.0, 120.0));
        let hovered = runtime.dispatch_event(Event::PointerMove {
            position: Point::new(24.0, 24.0),
        });
        assert!(hovered.is_some());
        Self { runtime, x: 24.0 }
    }

    fn step(&mut self) {
        self.x = if self.x < 28.0 { self.x + 1.0 } else { 24.0 };
        let hovered = self.runtime.dispatch_event(Event::PointerMove {
            position: Point::new(self.x, 24.0),
        });
        assert!(hovered.is_some());
        black_box(self.runtime.layout());
    }
}

struct StatefulVirtualizedHoverPaintBench {
    runtime: SurfaceRuntime<VirtualWheelBridge, ()>,
    theme: ThemeTokens,
    offset: f32,
}

impl StatefulVirtualizedHoverPaintBench {
    fn new() -> Self {
        Self {
            runtime: SurfaceRuntime::new(VirtualWheelBridge, Vector2::new(220.0, 120.0)),
            theme: ThemeTokens::default(),
            offset: 0.0,
        }
    }

    fn step(&mut self) {
        self.offset = (self.offset + 360.0) % 120_000.0;
        let scrolled = self
            .runtime
            .wheel_or_scroll_at(Point::new(24.0, 24.0), Vector2::new(0.0, self.offset));
        assert!(scrolled);
        let hovered = self.runtime.dispatch_event(Event::PointerMove {
            position: Point::new(24.0, 24.0),
        });
        assert!(hovered.is_some());
        let plan = self.runtime.paint_plan(&self.theme);
        assert!(
            plan.primitives.len() < 128,
            "virtualized hover paint should stay bounded to the visible window"
        );
        black_box(plan);
    }
}

struct VirtualWheelBridge;

impl RuntimeBridge<()> for VirtualWheelBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<()>> {
        let rows = (0..10_000_u64)
            .map(|index| {
                SurfaceChild::new(
                    SlotParams {
                        size_main: SizeModeMain::Fixed(28.0),
                        size_cross: SizeModeCross::Fill,
                        constraints: radiant::layout::Constraints::unconstrained(),
                        margin: Default::default(),
                        align_cross_override: None,
                        allow_fixed_compress: false,
                    },
                    SurfaceNode::widget(
                        ButtonWidget::new(
                            index + 10,
                            format!("Row {index:05}"),
                            WidgetSizing::fixed(Vector2::new(160.0, 28.0)),
                        ),
                        WidgetMessageMapper::none(),
                    ),
                )
            })
            .collect();
        Arc::new(UiSurface::new(SurfaceNode::virtual_scroll_area(
            1,
            SurfaceNode::column(2, 4.0, rows),
            VirtualizationAxis::Vertical,
            96.0,
        )))
    }
}

struct StatefulVirtualizedNestedScrollHoverBench {
    runtime: SurfaceRuntime<NestedScrollBridge, ()>,
    offset: f32,
}

impl StatefulVirtualizedNestedScrollHoverBench {
    fn new() -> Self {
        Self {
            runtime: SurfaceRuntime::new(NestedScrollBridge, Vector2::new(240.0, 140.0)),
            offset: 0.0,
        }
    }

    fn step(&mut self) {
        self.offset = (self.offset + 280.0) % 120_000.0;
        let scrolled = self
            .runtime
            .wheel_or_scroll_at(Point::new(20.0, 20.0), Vector2::new(0.0, self.offset));
        assert!(scrolled);
        self.runtime.dispatch_event(Event::PointerMove {
            position: Point::new(232.0, 20.0),
        });
        assert!(self.runtime.hovered_scroll_affordance().is_some());
        black_box(self.runtime.layout());
    }
}

struct NestedScrollBridge;

impl RuntimeBridge<()> for NestedScrollBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<()>> {
        let rows = (0..10_000_u64)
            .map(|index| {
                SurfaceChild::new(
                    SlotParams {
                        size_main: SizeModeMain::Fixed(32.0),
                        size_cross: SizeModeCross::Fill,
                        constraints: radiant::layout::Constraints::unconstrained(),
                        margin: Default::default(),
                        align_cross_override: None,
                        allow_fixed_compress: false,
                    },
                    SurfaceNode::scroll_area(
                        20_000 + index,
                        SurfaceNode::column(
                            40_000 + index,
                            0.0,
                            (0..4)
                                .map(|child_index| {
                                    SurfaceChild::new(
                                        SlotParams {
                                            size_main: SizeModeMain::Fixed(18.0),
                                            size_cross: SizeModeCross::Fill,
                                            constraints:
                                                radiant::layout::Constraints::unconstrained(),
                                            margin: Default::default(),
                                            align_cross_override: None,
                                            allow_fixed_compress: false,
                                        },
                                        SurfaceNode::text(
                                            80_000 + index * 4 + child_index,
                                            format!("Nested {index:05}.{child_index}"),
                                            WidgetSizing::fixed(Vector2::new(180.0, 18.0)),
                                        ),
                                    )
                                })
                                .collect(),
                        ),
                    ),
                )
            })
            .collect();
        Arc::new(UiSurface::new(SurfaceNode::virtual_scroll_area(
            1,
            SurfaceNode::column(2, 2.0, rows),
            VirtualizationAxis::Vertical,
            96.0,
        )))
    }
}

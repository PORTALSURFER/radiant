#[path = "virtualized/bridges.rs"]
mod bridges;

use crate::runner::ScenarioCounters;
use radiant::{
    layout::{Point, Vector2},
    runtime::{Event, SurfaceRuntime},
    theme::ThemeTokens,
};
use std::hint::black_box;

use bridges::{NestedScrollBridge, VirtualWheelBridge};

pub(super) fn virtualized_list_wheel_10k() -> impl FnMut() -> ScenarioCounters {
    let mut virtualized_wheel = StatefulVirtualizedWheelBench::new();
    move || virtualized_wheel.step()
}

pub(super) fn virtualized_list_hover_10k() -> impl FnMut() -> ScenarioCounters {
    let mut virtualized_hover = StatefulVirtualizedHoverBench::new();
    move || virtualized_hover.step()
}

pub(super) fn virtualized_list_stable_hover_10k() -> impl FnMut() -> ScenarioCounters {
    let mut virtualized_stable_hover = StatefulVirtualizedStableHoverBench::new();
    move || virtualized_stable_hover.step()
}

pub(super) fn virtualized_list_hover_paint_10k() -> impl FnMut() -> ScenarioCounters {
    let mut virtualized_hover_paint = StatefulVirtualizedHoverPaintBench::new();
    move || virtualized_hover_paint.step()
}

pub(super) fn virtualized_nested_scroll_hover_10k() -> impl FnMut() -> ScenarioCounters {
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

    fn step(&mut self) -> ScenarioCounters {
        self.offset = (self.offset + 360.0) % 120_000.0;
        let handled = self
            .runtime
            .wheel_or_scroll_at(Point::new(24.0, 24.0), Vector2::new(0.0, self.offset));
        assert!(handled);
        black_box(self.runtime.layout());
        ScenarioCounters::default()
            .with_relayout_count(1)
            .with_allocation_sensitive_work_count(1)
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

    fn step(&mut self) -> ScenarioCounters {
        self.offset = (self.offset + 360.0) % 120_000.0;
        let scrolled = self
            .runtime
            .wheel_or_scroll_at(Point::new(24.0, 24.0), Vector2::new(0.0, self.offset));
        assert!(scrolled);
        let hovered = self
            .runtime
            .dispatch_pointer_move_with_outcome(Point::new(24.0, 24.0));
        assert!(hovered.routed());
        black_box(self.runtime.layout());
        ScenarioCounters::default()
            .with_scene_rebuild_count(bool_counter(hovered.needs_scene_rebuild()))
            .with_paint_only_count(bool_counter(hovered.paint_only_requested))
            .with_relayout_count(1)
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

    fn step(&mut self) -> ScenarioCounters {
        self.x = if self.x < 28.0 { self.x + 1.0 } else { 24.0 };
        let hovered = self
            .runtime
            .dispatch_pointer_move_with_outcome(Point::new(self.x, 24.0));
        assert!(hovered.routed());
        black_box(self.runtime.layout());
        ScenarioCounters::default()
            .with_scene_rebuild_count(bool_counter(hovered.needs_scene_rebuild()))
            .with_paint_only_count(bool_counter(hovered.paint_only_requested))
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

    fn step(&mut self) -> ScenarioCounters {
        self.offset = (self.offset + 360.0) % 120_000.0;
        let scrolled = self
            .runtime
            .wheel_or_scroll_at(Point::new(24.0, 24.0), Vector2::new(0.0, self.offset));
        assert!(scrolled);
        let hovered = self
            .runtime
            .dispatch_pointer_move_with_outcome(Point::new(24.0, 24.0));
        assert!(hovered.routed());
        let plan = self.runtime.paint_plan(&self.theme);
        assert!(
            plan.primitives.len() < 128,
            "virtualized hover paint should stay bounded to the visible window"
        );
        let primitive_count = plan.primitives.len() as u64;
        black_box(plan);
        ScenarioCounters::default()
            .with_scene_rebuild_count(bool_counter(hovered.needs_scene_rebuild()))
            .with_paint_only_count(bool_counter(hovered.paint_only_requested))
            .with_relayout_count(1)
            .with_paint_primitive_count(primitive_count)
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

    fn step(&mut self) -> ScenarioCounters {
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
        ScenarioCounters::default()
            .with_relayout_count(1)
            .with_allocation_sensitive_work_count(1)
    }
}

fn bool_counter(value: bool) -> u64 {
    if value { 1 } else { 0 }
}

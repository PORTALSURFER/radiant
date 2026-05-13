//! Runtime surface performance scenarios.

use crate::viewport;
use radiant::{
    layout::{
        ContainerKind, ContainerPolicy, LayoutNode, LayoutOutput, Point, SizeModeCross,
        SizeModeMain, SlotParams, Vector2, VirtualizationAxis, layout_tree,
    },
    runtime::{
        Command, Event, RuntimeBridge, SurfaceChild, SurfaceNode, SurfaceRuntime, UiSurface,
        WidgetMessageMapper,
    },
    theme::ThemeTokens,
    widgets::{ButtonWidget, TextWidget, WidgetSizing},
};
use std::{hint::black_box, sync::Arc};

pub(super) fn surface_large_tree() -> impl FnMut() {
    let runtime_surface = StatefulRuntimeSurfaceBench::new();
    move || runtime_surface.step()
}

pub(super) fn text_paint_plan_1k() -> impl FnMut() {
    let text_surface = StatefulTextPaintPlanBench::new();
    move || text_surface.step()
}

pub(super) fn horizontal_scroll_paint_1k() -> impl FnMut() {
    let horizontal_scroll_surface = StatefulHorizontalScrollPaintBench::new();
    move || horizontal_scroll_surface.step()
}

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

pub(super) fn refresh_large_tree() -> impl FnMut() {
    let mut refresh_large_tree = StatefulRefreshBench::new();
    move || refresh_large_tree.step()
}

pub(super) fn resize_large_tree() -> impl FnMut() {
    let mut resize_large_tree = StatefulResizeBench::new();
    move || resize_large_tree.step()
}

pub(super) fn command_flattening_512() -> impl FnMut() {
    bench_command_flattening_512
}

struct StatefulRuntimeSurfaceBench {
    surface: UiSurface<()>,
    layout_node: LayoutNode,
    theme: ThemeTokens,
}

impl StatefulRuntimeSurfaceBench {
    fn new() -> Self {
        let surface = UiSurface::<()>::new(runtime_surface_node(250));
        let layout_node = surface.layout_node();
        Self {
            surface,
            layout_node,
            theme: ThemeTokens::default(),
        }
    }

    fn step(&self) {
        let output = layout_tree(&self.layout_node, viewport(960.0, 720.0));
        let plan = self.surface.paint_plan(&output, &self.theme);
        assert!(output.rects.len() >= 250);
        assert!(!plan.primitives.is_empty());
        black_box((output, plan));
    }
}

struct StatefulTextPaintPlanBench {
    surface: UiSurface<()>,
    layout_node: LayoutNode,
    theme: ThemeTokens,
}

impl StatefulTextPaintPlanBench {
    fn new() -> Self {
        let surface = UiSurface::<()>::new(text_paint_surface_node(1_000));
        let layout_node = surface.layout_node();
        Self {
            surface,
            layout_node,
            theme: ThemeTokens::default(),
        }
    }

    fn step(&self) {
        let output = layout_tree(&self.layout_node, viewport(960.0, 720.0));
        let plan = self.surface.paint_plan(&output, &self.theme);
        let stats = plan.stats();
        assert!(
            stats.text > 0 && stats.text < 64,
            "text-heavy scroll paint should stay bounded to the visible clip"
        );
        assert!(
            stats.total >= stats.text,
            "text-heavy paint plan should retain all text primitives"
        );
        black_box((output, plan));
    }
}

struct StatefulHorizontalScrollPaintBench {
    surface: UiSurface<()>,
    layout: LayoutOutput,
    theme: ThemeTokens,
}

impl StatefulHorizontalScrollPaintBench {
    fn new() -> Self {
        let surface = UiSurface::<()>::new(horizontal_scroll_surface_node(1_000));
        let layout_node = surface.layout_node();
        let layout = layout_tree(&layout_node, viewport(320.0, 80.0));
        Self {
            surface,
            layout,
            theme: ThemeTokens::default(),
        }
    }

    fn step(&self) {
        let plan = self.surface.paint_plan(&self.layout, &self.theme);
        let stats = plan.stats();
        assert!(
            stats.text > 0 && stats.text < 16,
            "horizontal clipped row paint should stay bounded to the visible clip"
        );
        black_box(plan);
    }
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

struct StatefulRefreshBench {
    runtime: SurfaceRuntime<RefreshBridge, ()>,
}

impl StatefulRefreshBench {
    fn new() -> Self {
        Self {
            runtime: SurfaceRuntime::new(RefreshBridge { revision: 0 }, Vector2::new(960.0, 720.0)),
        }
    }

    fn step(&mut self) {
        self.runtime.refresh();
        black_box(self.runtime.layout());
    }
}

struct StatefulResizeBench {
    runtime: SurfaceRuntime<RefreshBridge, ()>,
    wide: bool,
}

impl StatefulResizeBench {
    fn new() -> Self {
        Self {
            runtime: SurfaceRuntime::new(RefreshBridge { revision: 0 }, Vector2::new(960.0, 720.0)),
            wide: false,
        }
    }

    fn step(&mut self) {
        self.wide = !self.wide;
        let viewport = if self.wide {
            Vector2::new(960.0, 720.0)
        } else {
            Vector2::new(720.0, 540.0)
        };
        self.runtime.set_viewport(viewport);
        black_box(self.runtime.layout());
    }
}

struct RefreshBridge {
    revision: u64,
}

impl RuntimeBridge<()> for RefreshBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<()>> {
        self.revision = self.revision.wrapping_add(1);
        Arc::new(UiSurface::new(runtime_surface_node(1_000)))
    }
}

fn runtime_surface_node(count: u64) -> SurfaceNode<()> {
    let rows = (0..count)
        .map(|index| {
            SurfaceChild::new(
                SlotParams {
                    size_main: SizeModeMain::Intrinsic,
                    size_cross: SizeModeCross::Fill,
                    constraints: radiant::layout::Constraints::unconstrained(),
                    margin: Default::default(),
                    align_cross_override: None,
                    allow_fixed_compress: false,
                },
                SurfaceNode::static_widget(TextWidget::new(
                    10_000 + index,
                    format!("Row {index}"),
                    WidgetSizing::fixed(Vector2::new(180.0, 24.0)),
                )),
            )
        })
        .collect();
    SurfaceNode::scroll_area(
        1,
        SurfaceNode::container(
            2,
            ContainerPolicy {
                kind: ContainerKind::Column,
                spacing: 2.0,
                ..ContainerPolicy::default()
            },
            rows,
        ),
    )
}

fn text_paint_surface_node(count: u64) -> SurfaceNode<()> {
    let rows = (0..count)
        .map(|index| {
            SurfaceChild::new(
                SlotParams {
                    size_main: SizeModeMain::Fixed(22.0),
                    size_cross: SizeModeCross::Fill,
                    constraints: radiant::layout::Constraints::unconstrained(),
                    margin: Default::default(),
                    align_cross_override: None,
                    allow_fixed_compress: false,
                },
                SurfaceNode::static_widget(TextWidget::new(
                    40_000 + index,
                    format!(
                        "Track {index:04}  position {index:04}.{:02}  cached text row",
                        index % 97
                    ),
                    WidgetSizing::fixed(Vector2::new(520.0, 22.0)),
                )),
            )
        })
        .collect();
    SurfaceNode::scroll_area(
        30_000,
        SurfaceNode::container(
            30_001,
            ContainerPolicy {
                kind: ContainerKind::Column,
                spacing: 1.0,
                ..ContainerPolicy::default()
            },
            rows,
        ),
    )
}

fn horizontal_scroll_surface_node(count: u64) -> SurfaceNode<()> {
    let items = (0..count)
        .map(|index| {
            SurfaceChild::new(
                SlotParams {
                    size_main: SizeModeMain::Fixed(88.0),
                    size_cross: SizeModeCross::Fill,
                    constraints: radiant::layout::Constraints::unconstrained(),
                    margin: Default::default(),
                    align_cross_override: None,
                    allow_fixed_compress: false,
                },
                SurfaceNode::static_widget(TextWidget::new(
                    90_000 + index,
                    format!("Clip {index:04}"),
                    WidgetSizing::fixed(Vector2::new(88.0, 24.0)),
                )),
            )
        })
        .collect();
    SurfaceNode::scroll_area(
        80_000,
        SurfaceNode::container(
            80_001,
            ContainerPolicy {
                kind: ContainerKind::Row,
                spacing: 2.0,
                ..ContainerPolicy::default()
            },
            items,
        ),
    )
}

fn bench_command_flattening_512() {
    let command = Command::batch((0..512).map(|index| {
        if index % 8 == 0 {
            Command::batch([
                Command::message(index),
                Command::request_repaint(),
                Command::message(index + 10_000),
            ])
        } else if index % 5 == 0 {
            Command::request_paint_only()
        } else {
            Command::message(index)
        }
    }));
    let messages = command.into_messages();
    assert_eq!(messages.len(), 486);
    black_box(messages);
}

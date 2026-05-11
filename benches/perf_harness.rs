//! Standalone performance harness for Radiant layout, runtime, and GPU data paths.

use radiant::{
    gui::types::ImageRgba,
    layout::{
        ContainerKind, ContainerPolicy, LayoutDebugOptions, LayoutEngine, LayoutNode, LayoutOutput,
        LayoutState, Point, Rect, SizeModeCross, SizeModeMain, SlotChild, SlotParams, Vector2,
        VirtualizationAxis, VirtualizationPolicy, layout_tree, layout_tree_with_state,
    },
    prelude::{
        IntoView, VirtualListWindowRequest, button, list_row_id, resolve_virtual_list_window,
        selectable, virtual_list, virtual_list_window,
    },
    runtime::{
        Command, Event, GpuSignalSummary, GpuSurfaceCapabilities, GpuSurfaceContent,
        PaintPrimitive, RuntimeBridge, SurfaceChild, SurfaceNode, SurfaceRuntime, UiSurface,
        WidgetMessageMapper,
    },
    theme::ThemeTokens,
    widgets::{ButtonWidget, GpuSurfaceWidget, TextWidget, WidgetSizing},
};
use std::{
    env,
    hint::black_box,
    sync::Arc,
    time::{Duration, Instant},
};

const LAYOUT_ITERATIONS: usize = 120;
const RUNTIME_ITERATIONS: usize = 100;
const GPU_ITERATIONS: usize = 60;
const RUN_ALL_IN_DEBUG_ENV: &str = "RADIANT_PERF_RUN_ALL_IN_DEBUG";

fn main() {
    let filters = scenario_filters_from_args(env::args());
    if should_skip_unfiltered_debug_run(&filters) {
        println!(
            "radiant_perf skipped unfiltered debug run; pass a scenario filter or set {RUN_ALL_IN_DEBUG_ENV}=1"
        );
        return;
    }

    let mut runner = ScenarioRunner::new(filters);
    runner.run_scenario("layout_deep_nesting", LAYOUT_ITERATIONS, || {
        bench_deep_nesting
    });
    runner.run_scenario("layout_wrap_1k", LAYOUT_ITERATIONS, || bench_wrap_1k);
    runner.run_scenario("layout_virtualized_10k", LAYOUT_ITERATIONS, || {
        bench_virtualized_10k
    });
    runner.run_scenario("layout_virtualized_fixed_10k", LAYOUT_ITERATIONS, || {
        bench_virtualized_fixed_10k
    });
    runner.run_scenario(
        "layout_virtualized_fixed_scroll_10k",
        LAYOUT_ITERATIONS,
        || {
            let mut fixed_scroll = StatefulVirtualizedScrollBench::new();
            move || fixed_scroll.step()
        },
    );
    runner.run_scenario("layout_mark_dirty_subtree_10k", LAYOUT_ITERATIONS, || {
        let mut dirty_subtree = StatefulDirtySubtreeBench::new();
        move || dirty_subtree.step()
    });
    runner.run_scenario(
        "app_virtual_list_projection_10k",
        RUNTIME_ITERATIONS,
        || bench_app_virtual_list_projection_10k,
    );
    runner.run_scenario(
        "app_virtual_list_projection_generated_child_ids_10k",
        RUNTIME_ITERATIONS,
        || bench_app_virtual_list_projection_generated_child_ids_10k,
    );
    runner.run_scenario(
        "app_virtual_selectable_list_projection_10k",
        RUNTIME_ITERATIONS,
        || bench_app_virtual_selectable_list_projection_10k,
    );
    runner.run_scenario(
        "app_virtual_list_window_projection_10k",
        RUNTIME_ITERATIONS,
        || bench_app_virtual_list_window_projection_10k,
    );
    runner.run_scenario("runtime_surface_large_tree", RUNTIME_ITERATIONS, || {
        let runtime_surface = StatefulRuntimeSurfaceBench::new();
        move || runtime_surface.step()
    });
    runner.run_scenario("runtime_text_paint_plan_1k", RUNTIME_ITERATIONS, || {
        let text_surface = StatefulTextPaintPlanBench::new();
        move || text_surface.step()
    });
    runner.run_scenario(
        "runtime_horizontal_scroll_paint_1k",
        RUNTIME_ITERATIONS,
        || {
            let horizontal_scroll_surface = StatefulHorizontalScrollPaintBench::new();
            move || horizontal_scroll_surface.step()
        },
    );
    runner.run_scenario(
        "runtime_virtualized_list_wheel_10k",
        RUNTIME_ITERATIONS,
        || {
            let mut virtualized_wheel = StatefulVirtualizedWheelBench::new();
            move || virtualized_wheel.step()
        },
    );
    runner.run_scenario(
        "runtime_virtualized_list_hover_10k",
        RUNTIME_ITERATIONS,
        || {
            let mut virtualized_hover = StatefulVirtualizedHoverBench::new();
            move || virtualized_hover.step()
        },
    );
    runner.run_scenario(
        "runtime_virtualized_list_stable_hover_10k",
        RUNTIME_ITERATIONS,
        || {
            let mut virtualized_stable_hover = StatefulVirtualizedStableHoverBench::new();
            move || virtualized_stable_hover.step()
        },
    );
    runner.run_scenario(
        "runtime_virtualized_list_hover_paint_10k",
        RUNTIME_ITERATIONS,
        || {
            let mut virtualized_hover_paint = StatefulVirtualizedHoverPaintBench::new();
            move || virtualized_hover_paint.step()
        },
    );
    runner.run_scenario(
        "runtime_virtualized_nested_scroll_hover_10k",
        RUNTIME_ITERATIONS,
        || {
            let mut virtualized_nested_scroll_hover =
                StatefulVirtualizedNestedScrollHoverBench::new();
            move || virtualized_nested_scroll_hover.step()
        },
    );
    runner.run_scenario("runtime_refresh_large_tree", RUNTIME_ITERATIONS, || {
        let mut refresh_large_tree = StatefulRefreshBench::new();
        move || refresh_large_tree.step()
    });
    runner.run_scenario("runtime_resize_large_tree", RUNTIME_ITERATIONS, || {
        let mut resize_large_tree = StatefulResizeBench::new();
        move || resize_large_tree.step()
    });
    runner.run_scenario("runtime_command_flattening_512", RUNTIME_ITERATIONS, || {
        bench_command_flattening_512
    });
    runner.run_scenario("gpu_signal_summary", GPU_ITERATIONS, || {
        bench_gpu_signal_summary
    });
    runner.run_scenario("gpu_surface_projection", GPU_ITERATIONS, || {
        bench_gpu_surface_projection
    });
    runner.finish();
}

struct ScenarioRunner {
    filters: Vec<String>,
    matched: usize,
}

impl ScenarioRunner {
    fn new(filters: Vec<String>) -> Self {
        Self {
            filters,
            matched: 0,
        }
    }

    fn run_scenario<Build, Bench>(&mut self, name: &str, iterations: usize, build: Build)
    where
        Build: FnOnce() -> Bench,
        Bench: FnMut(),
    {
        if !scenario_matches_filters(name, &self.filters) {
            return;
        }
        self.matched += 1;
        run_scenario(name, iterations, build());
    }

    fn finish(self) {
        if self.matched == 0 && !self.filters.is_empty() {
            eprintln!(
                "no radiant_perf scenarios matched filters: {:?}",
                self.filters
            );
            std::process::exit(2);
        }
    }
}

fn scenario_filters_from_args(args: impl IntoIterator<Item = String>) -> Vec<String> {
    args.into_iter()
        .skip(1)
        .filter(|arg| !arg.starts_with('-') && !arg.is_empty())
        .collect()
}

fn scenario_matches_filters(name: &str, filters: &[String]) -> bool {
    filters.is_empty() || filters.iter().any(|filter| name.contains(filter))
}

fn should_skip_unfiltered_debug_run(filters: &[String]) -> bool {
    cfg!(debug_assertions) && filters.is_empty() && env::var_os(RUN_ALL_IN_DEBUG_ENV).is_none()
}

fn run_scenario(name: &str, iterations: usize, mut bench: impl FnMut()) {
    bench();
    let started = Instant::now();
    for _ in 0..iterations {
        bench();
    }
    print_metric(name, iterations, started.elapsed());
}

fn print_metric(name: &str, iterations: usize, elapsed: Duration) {
    let total_us = elapsed.as_micros();
    let avg_us = total_us as f64 / iterations.max(1) as f64;
    println!(
        "radiant_perf scenario={name} iterations={iterations} total_us={total_us} avg_us={avg_us:.3}"
    );
}

fn bench_deep_nesting() {
    let node = deep_nesting_tree();
    let output = layout_tree(&node, viewport(640.0, 360.0));
    assert!(output.rects.len() >= 301);
    black_box(output);
}

fn deep_nesting_tree() -> LayoutNode {
    let mut node = LayoutNode::widget(9_999, Vector2::new(8.0, 8.0));
    for id in (1..=300).rev() {
        node = LayoutNode::container(
            id,
            ContainerPolicy {
                kind: ContainerKind::PaddingBox,
                padding: radiant::layout::Insets::all(1.0),
                ..ContainerPolicy::default()
            },
            vec![SlotChild::new(SlotParams::fill(), node)],
        );
    }
    node
}

fn bench_wrap_1k() {
    let root = wrap_tree(1_000);
    let output = layout_tree(&root, viewport(1024.0, 768.0));
    assert_eq!(output.rects.len(), 1_001);
    black_box(output);
}

fn wrap_tree(count: u64) -> LayoutNode {
    let children = (0..count)
        .map(|index| {
            SlotChild::new(
                SlotParams::fill(),
                LayoutNode::widget(index + 2, Vector2::new(12.0, 8.0)),
            )
        })
        .collect();
    LayoutNode::container(
        1,
        ContainerPolicy {
            kind: ContainerKind::Wrap,
            wrap: radiant::layout::WrapPolicy {
                item_gap: 1.0,
                line_gap: 1.0,
            },
            ..ContainerPolicy::default()
        },
        children,
    )
}

fn bench_virtualized_10k() {
    let root = virtualized_scroll_tree(10_000, SizeModeMain::Intrinsic);
    let mut state = LayoutState::default();
    state.scroll_offsets.insert(1, Vector2::new(0.0, 20_000.0));
    let output = layout_tree_with_state(
        &root,
        viewport(300.0, 140.0),
        &state,
        LayoutDebugOptions::default(),
    );
    let window = output
        .virtual_windows
        .get(&1)
        .expect("virtual window metadata");
    assert_eq!(window.total_children, 10_000);
    assert!(output.stats.materialized_nodes < 256);
    black_box(output);
}

fn bench_virtualized_fixed_10k() {
    let root = virtualized_scroll_tree(10_000, SizeModeMain::Fixed(28.0));
    let mut state = LayoutState::default();
    state.scroll_offsets.insert(1, Vector2::new(0.0, 20_000.0));
    let output = layout_tree_with_state(
        &root,
        viewport(300.0, 140.0),
        &state,
        LayoutDebugOptions::default(),
    );
    let window = output
        .virtual_windows
        .get(&1)
        .expect("virtual window metadata");
    assert_eq!(window.total_children, 10_000);
    assert!(output.stats.materialized_nodes < 256);
    assert!(output.stats.measured_nodes < 64);
    black_box(output);
}

struct StatefulVirtualizedScrollBench {
    root: LayoutNode,
    engine: LayoutEngine,
    state: LayoutState,
    offset: f32,
}

impl StatefulVirtualizedScrollBench {
    fn new() -> Self {
        let root = virtualized_scroll_tree(10_000, SizeModeMain::Fixed(28.0));
        let mut engine = LayoutEngine::default();
        let state = LayoutState::default();
        let output = engine.layout_with_state(
            &root,
            viewport(300.0, 140.0),
            &state,
            LayoutDebugOptions::default(),
        );
        assert!(output.virtual_windows.contains_key(&1));
        Self {
            root,
            engine,
            state,
            offset: 0.0,
        }
    }

    fn step(&mut self) {
        self.offset = (self.offset + 360.0) % 120_000.0;
        self.state
            .scroll_offsets
            .insert(1, Vector2::new(0.0, self.offset));
        let output = self.engine.layout_with_state(
            &self.root,
            viewport(300.0, 140.0),
            &self.state,
            LayoutDebugOptions::default(),
        );
        let window = output
            .virtual_windows
            .get(&1)
            .expect("virtual window metadata");
        assert_eq!(window.total_children, 10_000);
        assert!(output.stats.materialized_nodes < 256);
        assert!(output.stats.measured_nodes < 64);
        black_box(output);
    }
}

struct StatefulDirtySubtreeBench {
    root: LayoutNode,
    engine: LayoutEngine,
}

impl StatefulDirtySubtreeBench {
    fn new() -> Self {
        Self {
            root: virtualized_scroll_tree(10_000, SizeModeMain::Fixed(28.0)),
            engine: LayoutEngine::default(),
        }
    }

    fn step(&mut self) {
        self.engine.mark_measure_dirty_subtree(&self.root, 2);
        self.engine.clear_dirty();
        black_box(&self.engine);
    }
}

fn virtualized_scroll_tree(count: u64, size_main: SizeModeMain) -> LayoutNode {
    let items = (0..count)
        .map(|index| {
            SlotChild::new(
                SlotParams {
                    size_main,
                    size_cross: SizeModeCross::Fill,
                    constraints: radiant::layout::Constraints::unconstrained(),
                    margin: Default::default(),
                    align_cross_override: None,
                    allow_fixed_compress: false,
                },
                LayoutNode::widget(index + 10, Vector2::new(120.0, 10.0)),
            )
        })
        .collect();

    LayoutNode::container(
        1,
        ContainerPolicy {
            kind: ContainerKind::ScrollView,
            overflow: radiant::layout::OverflowPolicy::Scroll,
            virtualization: Some(VirtualizationPolicy {
                enabled: true,
                axis: VirtualizationAxis::Vertical,
                overscan_px: 16.0,
            }),
            ..ContainerPolicy::default()
        },
        vec![SlotChild::new(
            SlotParams::fill(),
            LayoutNode::container(
                2,
                ContainerPolicy {
                    kind: ContainerKind::Column,
                    spacing: 1.0,
                    ..ContainerPolicy::default()
                },
                items,
            ),
        )],
    )
}

fn bench_app_virtual_list_projection_10k() {
    let surface = virtual_list(
        0..10_000_u64,
        |index| {
            list_row_id(
                index + 10_000,
                [button(format!("Row {index:05}"))
                    .message(())
                    .id(index + 20_000)
                    .fill_width()
                    .height(28.0)],
            )
            .height(32.0)
        },
        96.0,
    )
    .into_surface();
    let layout = surface.layout_node();
    assert_eq!(layout.id(), 1);
    black_box((surface, layout));
}

fn bench_app_virtual_list_projection_generated_child_ids_10k() {
    let surface = virtual_list(
        0..10_000_u64,
        |index| {
            list_row_id(
                index + 10_000,
                [button(format!("Row {index:05}"))
                    .message(())
                    .fill_width()
                    .height(28.0)],
            )
            .height(32.0)
        },
        96.0,
    )
    .into_surface();
    let layout = surface.layout_node();
    assert_eq!(layout.id(), 1);
    black_box((surface, layout));
}

fn bench_app_virtual_selectable_list_projection_10k() {
    let surface = virtual_list(
        0..10_000_u64,
        |index| {
            selectable(format!("Row {index:05}"), false)
                .message(move |_| ())
                .id(index + 10_000)
                .fill_width()
                .height(32.0)
        },
        96.0,
    )
    .into_surface();
    let layout = surface.layout_node();
    assert_eq!(layout.id(), 1);
    black_box((surface, layout));
}

fn bench_app_virtual_list_window_projection_10k() {
    let window = resolve_virtual_list_window(VirtualListWindowRequest {
        total_items: 10_000,
        viewport_len: 18,
        requested_start: 4_000,
        overscan: 4,
        ..VirtualListWindowRequest::default()
    });
    let surface = virtual_list_window(
        window,
        32.0,
        |index| {
            list_row_id(
                index as u64 + 10_000,
                [button(format!("Row {index:05}"))
                    .message(())
                    .id(index as u64 + 20_000)
                    .fill_width()
                    .height(28.0)],
            )
        },
        96.0,
    )
    .into_surface();
    let layout = surface.layout_node();
    assert_eq!(layout.id(), 1);
    black_box((surface, layout));
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

fn bench_gpu_signal_summary() {
    let samples = signal_samples(65_536, 2);
    let summary = GpuSignalSummary::from_interleaved_samples(&samples, 65_536, 2);
    assert!(!summary.levels.is_empty());
    black_box(summary);
}

fn bench_gpu_surface_projection() {
    let image = Arc::new(ImageRgba::new(512, 64, vec![128; 512 * 64 * 4]).expect("valid image"));
    let content = GpuSurfaceContent::RgbaAtlas {
        source_rect: Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(512.0, 64.0)),
        atlas: image,
    };
    let surface = UiSurface::<()>::new(SurfaceNode::static_widget(
        GpuSurfaceWidget::new(
            41,
            WidgetSizing::fixed(Vector2::new(512.0, 64.0)),
            7,
            1,
            content,
        )
        .with_capabilities(GpuSurfaceCapabilities {
            fast_pointer_move: true,
            coalesce_vertical_wheel: true,
            native_hover_cursor: None,
        }),
    ));
    let output = layout_tree(&surface.layout_node(), viewport(512.0, 64.0));
    let plan = surface.paint_plan(&output, &ThemeTokens::default());
    assert!(
        plan.primitives
            .iter()
            .any(|primitive| matches!(primitive, PaintPrimitive::GpuSurface(_)))
    );
    black_box(plan);
}

fn signal_samples(frames: usize, bands: usize) -> Vec<f32> {
    (0..frames.saturating_mul(bands))
        .map(|index| {
            let phase = (index % 1024) as f32 / 1024.0;
            (phase * 2.0 - 1.0) * if index % 2 == 0 { 0.75 } else { -0.5 }
        })
        .collect()
}

fn viewport(width: f32, height: f32) -> Rect {
    Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(width, height))
}

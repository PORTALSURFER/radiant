//! Standalone performance harness for Radiant layout, runtime, and GPU data paths.

use radiant::{
    gui::types::ImageRgba,
    layout::{
        ContainerKind, ContainerPolicy, LayoutDebugOptions, LayoutNode, LayoutState, Point, Rect,
        SizeModeCross, SizeModeMain, SlotChild, SlotParams, Vector2, VirtualizationAxis,
        VirtualizationPolicy, layout_tree, layout_tree_with_state,
    },
    prelude::{IntoView, button, list_row, virtual_list},
    runtime::{
        Event, GpuSignalSummary, GpuSurfaceCapabilities, GpuSurfaceContent, PaintPrimitive,
        RuntimeBridge, SurfaceChild, SurfaceNode, SurfaceRuntime, UiSurface, WidgetMessageMapper,
    },
    theme::ThemeTokens,
    widgets::{ButtonWidget, GpuSurfaceWidget, TextWidget, WidgetSizing},
};
use std::{
    hint::black_box,
    sync::Arc,
    time::{Duration, Instant},
};

const LAYOUT_ITERATIONS: usize = 120;
const RUNTIME_ITERATIONS: usize = 100;
const GPU_ITERATIONS: usize = 60;

fn main() {
    run_scenario("layout_deep_nesting", LAYOUT_ITERATIONS, bench_deep_nesting);
    run_scenario("layout_wrap_1k", LAYOUT_ITERATIONS, bench_wrap_1k);
    run_scenario(
        "layout_virtualized_10k",
        LAYOUT_ITERATIONS,
        bench_virtualized_10k,
    );
    run_scenario(
        "layout_virtualized_fixed_10k",
        LAYOUT_ITERATIONS,
        bench_virtualized_fixed_10k,
    );
    let mut fixed_scroll = StatefulVirtualizedScrollBench::new();
    run_scenario(
        "layout_virtualized_fixed_scroll_10k",
        LAYOUT_ITERATIONS,
        move || fixed_scroll.step(),
    );
    run_scenario(
        "app_virtual_list_projection_10k",
        RUNTIME_ITERATIONS,
        bench_app_virtual_list_projection_10k,
    );
    let runtime_surface = StatefulRuntimeSurfaceBench::new();
    run_scenario(
        "runtime_surface_large_tree",
        RUNTIME_ITERATIONS,
        move || runtime_surface.step(),
    );
    let mut virtualized_wheel = StatefulVirtualizedWheelBench::new();
    run_scenario(
        "runtime_virtualized_list_wheel_10k",
        RUNTIME_ITERATIONS,
        move || virtualized_wheel.step(),
    );
    let mut virtualized_hover = StatefulVirtualizedHoverBench::new();
    run_scenario(
        "runtime_virtualized_list_hover_10k",
        RUNTIME_ITERATIONS,
        move || virtualized_hover.step(),
    );
    let mut refresh_large_tree = StatefulRefreshBench::new();
    run_scenario(
        "runtime_refresh_large_tree",
        RUNTIME_ITERATIONS,
        move || refresh_large_tree.step(),
    );
    run_scenario(
        "gpu_signal_summary",
        GPU_ITERATIONS,
        bench_gpu_signal_summary,
    );
    run_scenario(
        "gpu_surface_projection",
        GPU_ITERATIONS,
        bench_gpu_surface_projection,
    );
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
    engine: radiant::layout::LayoutEngine,
    state: LayoutState,
    offset: f32,
}

impl StatefulVirtualizedScrollBench {
    fn new() -> Self {
        let root = virtualized_scroll_tree(10_000, SizeModeMain::Fixed(28.0));
        let mut engine = radiant::layout::LayoutEngine::default();
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
            list_row(
                index,
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

//! Standalone performance harness for Radiant layout, runtime, and GPU data paths.

#[path = "perf_harness/app_projection.rs"]
mod app_projection;
#[path = "perf_harness/command_drain.rs"]
mod command_drain;
#[path = "perf_harness/layout_scenarios.rs"]
mod layout_scenarios;
#[path = "perf_harness/runtime_scenarios.rs"]
mod runtime_scenarios;

use radiant::{
    gui::types::ImageRgba,
    layout::{Point, Rect, Vector2, layout_tree},
    runtime::{
        GpuSignalSummary, GpuSurfaceCapabilities, GpuSurfaceContent, GpuSurfaceRuntimeOverlays,
        PaintPrimitive, SurfaceNode, UiSurface,
    },
    theme::ThemeTokens,
    widgets::{GpuSurfaceWidget, WidgetSizing},
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
    runner.run_scenario(
        "layout_deep_nesting",
        LAYOUT_ITERATIONS,
        layout_scenarios::deep_nesting,
    );
    runner.run_scenario(
        "layout_wrap_1k",
        LAYOUT_ITERATIONS,
        layout_scenarios::wrap_1k,
    );
    runner.run_scenario(
        "layout_virtualized_10k",
        LAYOUT_ITERATIONS,
        layout_scenarios::virtualized_10k,
    );
    runner.run_scenario(
        "layout_virtualized_fixed_10k",
        LAYOUT_ITERATIONS,
        layout_scenarios::virtualized_fixed_10k,
    );
    runner.run_scenario(
        "layout_virtualized_fixed_scroll_10k",
        LAYOUT_ITERATIONS,
        layout_scenarios::virtualized_fixed_scroll_10k,
    );
    runner.run_scenario(
        "layout_mark_dirty_subtree_10k",
        LAYOUT_ITERATIONS,
        layout_scenarios::mark_dirty_subtree_10k,
    );
    runner.run_scenario(
        "app_virtual_list_projection_10k",
        RUNTIME_ITERATIONS,
        app_projection::virtual_list_projection_10k,
    );
    runner.run_scenario(
        "app_virtual_list_projection_generated_child_ids_10k",
        RUNTIME_ITERATIONS,
        app_projection::virtual_list_projection_generated_child_ids_10k,
    );
    runner.run_scenario(
        "app_virtual_selectable_list_projection_10k",
        RUNTIME_ITERATIONS,
        app_projection::virtual_selectable_list_projection_10k,
    );
    runner.run_scenario(
        "app_virtual_list_window_projection_10k",
        RUNTIME_ITERATIONS,
        app_projection::virtual_list_window_projection_10k,
    );
    runner.run_scenario(
        "runtime_surface_large_tree",
        RUNTIME_ITERATIONS,
        runtime_scenarios::surface_large_tree,
    );
    runner.run_scenario(
        "runtime_text_paint_plan_1k",
        RUNTIME_ITERATIONS,
        runtime_scenarios::text_paint_plan_1k,
    );
    runner.run_scenario(
        "runtime_horizontal_scroll_paint_1k",
        RUNTIME_ITERATIONS,
        runtime_scenarios::horizontal_scroll_paint_1k,
    );
    runner.run_scenario(
        "runtime_virtualized_list_wheel_10k",
        RUNTIME_ITERATIONS,
        runtime_scenarios::virtualized_list_wheel_10k,
    );
    runner.run_scenario(
        "runtime_virtualized_list_hover_10k",
        RUNTIME_ITERATIONS,
        runtime_scenarios::virtualized_list_hover_10k,
    );
    runner.run_scenario(
        "runtime_virtualized_list_stable_hover_10k",
        RUNTIME_ITERATIONS,
        runtime_scenarios::virtualized_list_stable_hover_10k,
    );
    runner.run_scenario(
        "runtime_virtualized_list_hover_paint_10k",
        RUNTIME_ITERATIONS,
        runtime_scenarios::virtualized_list_hover_paint_10k,
    );
    runner.run_scenario(
        "runtime_pointer_overlay_paint_10k",
        RUNTIME_ITERATIONS,
        runtime_scenarios::pointer_overlay_paint_10k,
    );
    runner.run_scenario(
        "runtime_virtualized_nested_scroll_hover_10k",
        RUNTIME_ITERATIONS,
        runtime_scenarios::virtualized_nested_scroll_hover_10k,
    );
    runner.run_scenario(
        "runtime_refresh_large_tree",
        RUNTIME_ITERATIONS,
        runtime_scenarios::refresh_large_tree,
    );
    runner.run_scenario(
        "runtime_resize_large_tree",
        RUNTIME_ITERATIONS,
        runtime_scenarios::resize_large_tree,
    );
    runner.run_scenario(
        "runtime_command_flattening_512",
        RUNTIME_ITERATIONS,
        runtime_scenarios::command_flattening_512,
    );
    runner.run_scenario(
        "runtime_command_drain_1k",
        RUNTIME_ITERATIONS,
        command_drain::flat_command_drain,
    );
    runner.run_scenario(
        "runtime_nested_command_drain_1k",
        RUNTIME_ITERATIONS,
        command_drain::nested_command_drain,
    );
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
            runtime_overlays: GpuSurfaceRuntimeOverlays::default(),
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

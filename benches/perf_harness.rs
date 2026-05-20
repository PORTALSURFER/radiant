//! Standalone performance harness for Radiant layout, runtime, and GPU data paths.

#[path = "perf_harness/app_projection.rs"]
mod app_projection;
#[path = "perf_harness/catalog.rs"]
mod catalog;
#[path = "perf_harness/command_drain.rs"]
mod command_drain;
#[path = "perf_harness/layout_scenarios.rs"]
mod layout_scenarios;
#[path = "perf_harness/resource_scenarios.rs"]
mod resource_scenarios;
#[path = "perf_harness/runner.rs"]
mod runner;
#[path = "perf_harness/runtime_scenarios.rs"]
mod runtime_scenarios;
#[path = "perf_harness/text_scenarios.rs"]
mod text_scenarios;

use radiant::{
    gui::types::ImageRgba,
    layout::{Point, Rect, Vector2, layout_tree},
    runtime::{
        GpuShaderSurfaceDescriptor, GpuSignalSummary, GpuSurfaceCapabilities, GpuSurfaceContent,
        GpuSurfaceRuntimeOverlays, PaintPrimitive, SurfaceNode, UiSurface,
    },
    theme::ThemeTokens,
    widgets::{GpuSurfaceWidget, WidgetSizing},
};
use std::{env, hint::black_box, sync::Arc};

const GPU_ITERATIONS: usize = 60;

fn main() {
    let args = env::args().collect::<Vec<_>>();
    if runner::scenario_list_requested(&args) {
        runner::print_scenario_list(catalog::PERF_SCENARIOS);
        return;
    }

    let filters = runner::scenario_filters_from_args(&args);
    let category_filters = runner::category_filters_from_args(&args);
    if runner::should_skip_unfiltered_debug_run(&filters, &category_filters) {
        runner::print_unfiltered_debug_skip();
        return;
    }

    let output_format = runner::output_format_from_args(&args);
    let baseline = runner::baseline_from_args(&args);
    let baseline_output = runner::baseline_output_from_args(&args);
    let fail_on_baseline_regression = runner::fail_on_baseline_regression_from_args(&args);
    let fail_on_missing_baseline = runner::fail_on_missing_baseline_from_args(&args);
    let mut runner = runner::ScenarioRunner::new(
        filters,
        category_filters,
        output_format,
        baseline,
        baseline_output,
        fail_on_baseline_regression,
        fail_on_missing_baseline,
    );
    catalog::run_registered_scenarios(&mut runner);
    runner.finish();
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

fn bench_gpu_custom_shader_projection() {
    let descriptor = GpuShaderSurfaceDescriptor::new("perf/custom-shader")
        .wgsl_source("@vertex fn vertex_main() -> @builtin(position) vec4<f32> { return vec4<f32>(); }\n@fragment fn fragment_main() -> @location(0) vec4<f32> { return vec4<f32>(1.0); }")
        .entry_point("vertex_main")
        .fragment_entry_point("fragment_main")
        .uniform_bytes([1, 3, 5, 7, 9, 11, 13, 15])
        .storage_bytes([2; 256])
        .vertex_count(6);
    let surface = UiSurface::<()>::new(SurfaceNode::static_widget(
        GpuSurfaceWidget::new(
            42,
            WidgetSizing::fixed(Vector2::new(320.0, 120.0)),
            91,
            4,
            GpuSurfaceContent::CustomShader {
                descriptor: Arc::new(descriptor),
            },
        )
        .with_capabilities(GpuSurfaceCapabilities {
            fast_pointer_move: true,
            coalesce_vertical_wheel: false,
            runtime_overlays: GpuSurfaceRuntimeOverlays::default(),
        }),
    ));
    let output = layout_tree(&surface.layout_node(), viewport(320.0, 120.0));
    let plan = surface.paint_plan(&output, &ThemeTokens::default());
    let Some(PaintPrimitive::GpuSurface(gpu_surface)) = plan.primitives.first() else {
        panic!("expected custom shader GPU surface primitive");
    };
    assert!(matches!(
        gpu_surface.content,
        GpuSurfaceContent::CustomShader { .. }
    ));
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

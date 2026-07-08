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
    layout::{
        Constraints, ContainerKind, ContainerPolicy, Point, Rect, SizeModeCross, SizeModeMain,
        SlotParams, Vector2, layout_tree,
    },
    runtime::{
        GpuShaderSurfaceDescriptor, GpuSignalSummary, GpuSurfaceCapabilities, GpuSurfaceContent,
        GpuSurfaceRuntimeOverlays, PaintPrimitive, SurfaceChild, SurfaceNode, UiSurface,
    },
    theme::ThemeTokens,
    widgets::{GpuSurfaceWidget, WidgetSizing},
};
use runner::ScenarioCounters;
use std::{env, hint::black_box, sync::Arc};

const GPU_ITERATIONS: usize = 60;

fn main() {
    let args = env::args().collect::<Vec<_>>();
    let filters = runner::scenario_filters_from_args(&args);
    let category_filters = runner::category_filters_from_args(&args);
    let group_filters = runner::group_filters_from_args(&args);
    if runner::scenario_list_requested(&args) {
        runner::print_scenario_list(catalog::PERF_SCENARIOS);
        return;
    }

    if runner::should_skip_unfiltered_debug_run(&filters, &category_filters, &group_filters) {
        runner::print_unfiltered_debug_skip();
        return;
    }

    let output_format = runner::output_format_from_args(&args);
    let baseline = runner::baseline_from_args(&args);
    let baseline_output = runner::baseline_output_from_args(&args);
    let fail_on_baseline_regression = runner::fail_on_baseline_regression_from_args(&args);
    let fail_on_missing_baseline = runner::fail_on_missing_baseline_from_args(&args);
    let mut runner = runner::ScenarioRunner::new(runner::ScenarioRunnerConfig {
        filters,
        category_filters,
        group_filters,
        output_format,
        baseline,
        baseline_output,
        fail_on_baseline_regression,
        fail_on_missing_baseline,
    });
    catalog::run_registered_scenarios(&mut runner);
    runner.finish();
}

fn bench_gpu_signal_summary() -> ScenarioCounters {
    let samples = signal_samples(65_536, 2);
    let summary = GpuSignalSummary::from_interleaved_samples(&samples, 65_536, 2);
    assert!(!summary.levels.is_empty());
    let level_count = summary.levels.len() as u64;
    black_box(summary);
    ScenarioCounters::default().with_allocation_sensitive_work_count(level_count)
}

fn bench_gpu_surface_projection() -> ScenarioCounters {
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
    let gpu_surface_count = plan
        .primitives
        .iter()
        .filter(|primitive| matches!(primitive, PaintPrimitive::GpuSurface(_)))
        .count() as u64;
    let primitive_count = plan.primitives.len() as u64;
    black_box(plan);
    ScenarioCounters::default()
        .with_gpu_surface_count(gpu_surface_count)
        .with_retained_surface_cache_hit_count(0)
        .with_paint_primitive_count(primitive_count)
}

fn bench_gpu_surface_stack_projection_128() -> ScenarioCounters {
    let image = Arc::new(ImageRgba::new(640, 24, vec![96; 640 * 24 * 4]).expect("valid image"));
    let rows = (0..128)
        .map(|index| {
            let content = GpuSurfaceContent::RgbaAtlas {
                source_rect: Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(640.0, 24.0)),
                atlas: Arc::clone(&image),
            };
            SurfaceChild::new(
                SlotParams {
                    size_main: SizeModeMain::Fixed(24.0),
                    size_cross: SizeModeCross::Fill,
                    constraints: Constraints::unconstrained(),
                    margin: Default::default(),
                    align_cross_override: None,
                    allow_fixed_compress: false,
                },
                SurfaceNode::static_widget(
                    GpuSurfaceWidget::new(
                        10_000 + index,
                        WidgetSizing::fixed(Vector2::new(640.0, 24.0)),
                        20_000 + index,
                        index,
                        content,
                    )
                    .with_capabilities(GpuSurfaceCapabilities {
                        fast_pointer_move: index % 2 == 0,
                        coalesce_vertical_wheel: true,
                        runtime_overlays: GpuSurfaceRuntimeOverlays::default(),
                    }),
                ),
            )
        })
        .collect();
    let surface = UiSurface::<()>::new(SurfaceNode::container(
        9_000,
        ContainerPolicy {
            kind: ContainerKind::Column,
            spacing: 0.0,
            ..ContainerPolicy::default()
        },
        rows,
    ));
    let output = layout_tree(&surface.layout_node(), viewport(640.0, 720.0));
    let plan = surface.paint_plan(&output, &ThemeTokens::default());
    let gpu_surface_count = plan
        .primitives
        .iter()
        .filter(|primitive| matches!(primitive, PaintPrimitive::GpuSurface(_)))
        .count();
    assert_eq!(
        gpu_surface_count, 30,
        "GPU surface stack projection should stay bounded to visible rows"
    );
    let primitive_count = plan.primitives.len() as u64;
    black_box(plan);
    ScenarioCounters::default()
        .with_gpu_surface_count(gpu_surface_count as u64)
        .with_retained_surface_cache_hit_count(0)
        .with_paint_primitive_count(primitive_count)
}

fn bench_gpu_custom_shader_projection() -> ScenarioCounters {
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
    let Some(gpu_surface) = plan
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            PaintPrimitive::GpuSurface(gpu_surface) => Some(gpu_surface),
            _ => None,
        })
    else {
        panic!("expected custom shader GPU surface primitive");
    };
    assert!(matches!(
        gpu_surface.content,
        GpuSurfaceContent::CustomShader { .. }
    ));
    let primitive_count = plan.primitives.len() as u64;
    black_box(plan);
    ScenarioCounters::default()
        .with_gpu_surface_count(1)
        .with_retained_surface_cache_hit_count(0)
        .with_paint_primitive_count(primitive_count)
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

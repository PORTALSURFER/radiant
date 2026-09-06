//! Opt-in, offscreen measurements of the synchronous custom-shader builder.
//!
//! The fixture calls the same module, layout, pipeline, and validation-scope
//! builder functions as production. It deliberately does not render, wait for
//! GPU work, or make a foreground timing claim.

use super::*;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use vello::wgpu;

const VALID_SHADER: &str = r#"
@vertex fn vertex_main(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0), vec2<f32>(3.0, -1.0), vec2<f32>(-1.0, 3.0));
    return vec4<f32>(positions[index], 0.0, 1.0);
}
@fragment fn fragment_main() -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, 0.0, 0.0, 1.0);
}
"#;

const INVALID_SHADER: &str = "this is not WGSL";
const WARM_REPETITIONS: usize = 3;
const DISTINCT_VARIANTS: usize = 4;

#[derive(Clone, Copy)]
enum PreparationOutcome {
    Ready,
    ShaderModuleValidationFailure,
    PipelineValidationFailure,
}

impl PreparationOutcome {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::ShaderModuleValidationFailure => "shader_module_validation_failure",
            Self::PipelineValidationFailure => "pipeline_validation_failure",
        }
    }
}

struct PreparationSample {
    workload: &'static str,
    variant: usize,
    cache_hit: bool,
    outcome: PreparationOutcome,
    shader_module_create: Duration,
    shader_module_validation_pop: Duration,
    layout_create: Duration,
    render_pipeline_create: Duration,
    pipeline_validation_pop: Duration,
    total: Duration,
    source_hash: String,
}

impl PreparationSample {
    fn json(&self) -> serde_json::Value {
        serde_json::json!({
            "workload": self.workload,
            "variant": self.variant,
            "cache_hit": self.cache_hit,
            "outcome": self.outcome.as_str(),
            "source_hash": self.source_hash,
            "shader_module_create_ns": duration_ns(self.shader_module_create),
            "shader_module_validation_pop_ns": duration_ns(self.shader_module_validation_pop),
            "layout_create_ns": duration_ns(self.layout_create),
            "render_pipeline_create_ns": duration_ns(self.render_pipeline_create),
            "pipeline_validation_pop_ns": duration_ns(self.pipeline_validation_pop),
            "total_ns": duration_ns(self.total),
        })
    }
}

struct PreparationCache {
    pipelines: HashMap<CustomShaderPipelineKey, CreatedCustomShaderPipeline>,
}

impl PreparationCache {
    fn prepare(
        &mut self,
        device: &wgpu::Device,
        key: CustomShaderPipelineKey,
        workload: &'static str,
        variant: usize,
    ) -> PreparationSample {
        if self.pipelines.contains_key(&key) {
            return PreparationSample {
                workload,
                variant,
                cache_hit: true,
                outcome: PreparationOutcome::Ready,
                shader_module_create: Duration::ZERO,
                shader_module_validation_pop: Duration::ZERO,
                layout_create: Duration::ZERO,
                render_pipeline_create: Duration::ZERO,
                pipeline_validation_pop: Duration::ZERO,
                total: Duration::ZERO,
                source_hash: fnv1a64(&key.wgsl_source),
            };
        }

        let (sample, created) = measure_pipeline_preparation(device, &key, workload, variant);
        if let Some(created) = created {
            assert!(self.pipelines.insert(key, created).is_none());
        }
        sample
    }
}

#[test]
#[ignore = "requires a native adapter and RADIANT_SHADER_PREPARATION_OUTPUT_DIR"]
fn records_sync_custom_shader_preparation_phases() {
    let output_dir = required_env("RADIANT_SHADER_PREPARATION_OUTPUT_DIR");
    let label = required_env("RADIANT_SHADER_PREPARATION_LABEL");
    let source_revision = required_env("RADIANT_SHADER_PREPARATION_SOURCE_REVISION");
    let (adapter_info, device, device_setup) = native_device();
    let mut cache = PreparationCache {
        pipelines: HashMap::new(),
    };
    let mut samples = Vec::with_capacity(2 + WARM_REPETITIONS + DISTINCT_VARIANTS);

    let cold_key = pipeline_key("cold", VALID_SHADER);
    samples.push(cache.prepare(&device, cold_key.clone(), "cold_device", 0));
    for repetition in 0..WARM_REPETITIONS {
        samples.push(cache.prepare(&device, cold_key.clone(), "warm_identical_key", repetition));
    }
    for variant in 0..DISTINCT_VARIANTS {
        let source = format!("{VALID_SHADER}\n// bounded distinct variant {variant}\n");
        samples.push(cache.prepare(
            &device,
            pipeline_key(&format!("distinct-{variant}"), &source),
            "bounded_distinct_key",
            variant,
        ));
    }
    samples.push(
        measure_pipeline_preparation(
            &device,
            &pipeline_key("invalid", INVALID_SHADER),
            "invalid_wgsl",
            0,
        )
        .0,
    );

    assert!(matches!(samples[0].outcome, PreparationOutcome::Ready));
    assert!(
        samples[1..=WARM_REPETITIONS]
            .iter()
            .all(|sample| sample.cache_hit && matches!(sample.outcome, PreparationOutcome::Ready))
    );
    assert!(
        samples[WARM_REPETITIONS + 1..WARM_REPETITIONS + 1 + DISTINCT_VARIANTS]
            .iter()
            .all(|sample| !sample.cache_hit && matches!(sample.outcome, PreparationOutcome::Ready))
    );
    assert!(matches!(
        samples.last().expect("invalid sample").outcome,
        PreparationOutcome::ShaderModuleValidationFailure
    ));

    let backend = format!("{:?}", adapter_info.backend);
    let output = serde_json::json!({
        "schema_version": 1,
        "fixture": "opt-1457-sync-custom-shader-preparation",
        "label": label,
        "source_revision": source_revision,
        "thread_id": format!("{:?}", std::thread::current().id()),
        "adapter": {
            "name": adapter_info.name,
            "vendor": adapter_info.vendor,
            "device": adapter_info.device,
            "device_type": format!("{:?}", adapter_info.device_type),
            "driver": adapter_info.driver,
            "driver_info": adapter_info.driver_info,
        },
        "backend": backend,
        "target_format": "Rgba8Unorm",
        "cold_device_setup_ns": duration_ns(device_setup),
        "source_hash_algorithm": "fnv1a64",
        "samples": samples.iter().map(PreparationSample::json).collect::<Vec<_>>(),
    });
    fs::create_dir_all(&output_dir).expect("create preparation measurement output directory");
    let output_path = PathBuf::from(output_dir).join("shader-preparation-samples.json");
    fs::write(
        &output_path,
        serde_json::to_vec_pretty(&output).expect("serialize preparation samples"),
    )
    .expect("write preparation samples");
    eprintln!(
        "RADIANT_SHADER_PREPARATION_OUTPUT={}",
        output_path.display()
    );
}

fn measure_pipeline_preparation(
    device: &wgpu::Device,
    key: &CustomShaderPipelineKey,
    workload: &'static str,
    variant: usize,
) -> (PreparationSample, Option<CreatedCustomShaderPipeline>) {
    let total_start = Instant::now();
    let source_hash = fnv1a64(&key.wgsl_source);
    let module_scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let module_start = Instant::now();
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("radiant_opt_1457_shader_module"),
        source: wgpu::ShaderSource::Wgsl(key.wgsl_source.as_ref().into()),
    });
    let shader_module_create = module_start.elapsed();
    let module_pop_start = Instant::now();
    let module_error = pollster::block_on(module_scope.pop());
    let shader_module_validation_pop = module_pop_start.elapsed();
    if module_error.is_some() {
        return (
            PreparationSample {
                workload,
                variant,
                cache_hit: false,
                outcome: PreparationOutcome::ShaderModuleValidationFailure,
                shader_module_create,
                shader_module_validation_pop,
                layout_create: Duration::ZERO,
                render_pipeline_create: Duration::ZERO,
                pipeline_validation_pop: Duration::ZERO,
                total: total_start.elapsed(),
                source_hash,
            },
            None,
        );
    }

    let layout_start = Instant::now();
    let bind_group_layout = layout::create_custom_shader_bind_group_layout(&request(device, key));
    let pipeline_layout = layout::create_custom_shader_pipeline_layout(device, &bind_group_layout);
    let layout_create = layout_start.elapsed();
    let pipeline_scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let pipeline_start = Instant::now();
    let _pipeline =
        create_custom_shader_render_pipeline(&request(device, key), &shader, &pipeline_layout);
    let render_pipeline_create = pipeline_start.elapsed();
    let pipeline_pop_start = Instant::now();
    let pipeline_error = pollster::block_on(pipeline_scope.pop());
    let pipeline_validation_pop = pipeline_pop_start.elapsed();

    let outcome = if pipeline_error.is_some() {
        PreparationOutcome::PipelineValidationFailure
    } else {
        PreparationOutcome::Ready
    };
    let created =
        matches!(outcome, PreparationOutcome::Ready).then_some(CreatedCustomShaderPipeline {
            bind_group_layout,
            pipeline: _pipeline,
        });
    (
        PreparationSample {
            workload,
            variant,
            cache_hit: false,
            outcome,
            shader_module_create,
            shader_module_validation_pop,
            layout_create,
            render_pipeline_create,
            pipeline_validation_pop,
            total: total_start.elapsed(),
            source_hash,
        },
        created,
    )
}

fn request<'a>(
    device: &'a wgpu::Device,
    key: &'a CustomShaderPipelineKey,
) -> CustomShaderPipelineRequest<'a> {
    CustomShaderPipelineRequest {
        surface_key: 1,
        device,
        target_format: wgpu::TextureFormat::Rgba8Unorm,
        key: key.clone(),
    }
}

fn pipeline_key(shader_key: &str, source: &str) -> CustomShaderPipelineKey {
    CustomShaderPipelineKey {
        shader_key: Arc::from(shader_key),
        wgsl_source: Arc::from(source),
        vertex_entry_point: Arc::from("vertex_main"),
        fragment_entry_point: Arc::from("fragment_main"),
        has_uniform_payload: false,
        has_storage_payload: false,
        has_presentation_uniform_payload: false,
    }
}

fn native_device() -> (wgpu::AdapterInfo, wgpu::Device, Duration) {
    let setup_start = Instant::now();
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: None,
        force_fallback_adapter: false,
    }))
    .expect("native adapter available for shader preparation measurement");
    let adapter_info = adapter.get_info();
    let (device, _queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("radiant_opt_1457_shader_preparation_device"),
        required_features: wgpu::Features::empty(),
        required_limits: wgpu::Limits::downlevel_defaults(),
        experimental_features: wgpu::ExperimentalFeatures::disabled(),
        memory_hints: wgpu::MemoryHints::Performance,
        trace: wgpu::Trace::Off,
    }))
    .expect("native device available for shader preparation measurement");
    (adapter_info, device, setup_start.elapsed())
}

fn required_env(name: &str) -> String {
    env::var(name).unwrap_or_else(|_| panic!("{name} must be set for this fixture"))
}

fn duration_ns(duration: Duration) -> u64 {
    duration
        .as_nanos()
        .try_into()
        .expect("bounded phase timing must fit u64 nanoseconds")
}

fn fnv1a64(input: &str) -> String {
    let hash = input
        .as_bytes()
        .iter()
        .fold(0xcbf29ce484222325_u64, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
        });
    format!("{hash:016x}")
}

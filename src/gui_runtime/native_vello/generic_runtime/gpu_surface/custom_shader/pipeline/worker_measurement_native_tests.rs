//! Opt-in native measurements of actual worker-side shader preparation.
//!
//! Worker elapsed time surrounds only `prepare_custom_shader_pipeline`; spawn
//! and join are reported separately. This fixture makes no foreground, input,
//! GPU-duration, or cache-hit claim.

use super::measurement_native_tests::{
    INVALID_SHADER, VALID_SHADER, duration_ns, fnv1a64, native_device, pipeline_key,
};
use super::*;
use std::{
    env, fs,
    io::Write,
    path::PathBuf,
    sync::mpsc,
    time::{Duration, Instant},
};
use vello::wgpu;

const WARM_REPETITIONS: usize = 3;
const DISTINCT_VARIANTS: usize = 4;

#[derive(Clone, Copy)]
enum WorkerOutcome {
    Ready,
    Cancelled,
    HostRejected,
    ShaderModule,
    Pipeline,
    Panicked,
}

impl WorkerOutcome {
    fn from_result(result: Result<CustomShaderPipeline, CustomShaderPreparationFailure>) -> Self {
        match result {
            Ok(_) => Self::Ready,
            Err(CustomShaderPreparationFailure::Cancelled) => Self::Cancelled,
            Err(CustomShaderPreparationFailure::HostRejected) => Self::HostRejected,
            Err(CustomShaderPreparationFailure::ShaderModule) => Self::ShaderModule,
            Err(CustomShaderPreparationFailure::Pipeline) => Self::Pipeline,
            Err(CustomShaderPreparationFailure::Panicked) => Self::Panicked,
        }
    }
    const fn as_str(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Cancelled => "cancelled",
            Self::HostRejected => "host_rejected",
            Self::ShaderModule => "shader_module",
            Self::Pipeline => "pipeline",
            Self::Panicked => "panicked",
        }
    }
}

struct WorkerSample {
    workload: &'static str,
    variant: usize,
    source_hash: String,
    outcome: WorkerOutcome,
    submitting_thread_id: String,
    worker_thread_id: String,
    worker_elapsed: Duration,
    spawn_join_elapsed: Duration,
}
impl WorkerSample {
    fn json(&self) -> serde_json::Value {
        serde_json::json!({
            "workload": self.workload, "variant": self.variant,
            "source_hash": self.source_hash, "outcome": self.outcome.as_str(),
            "submitting_thread_id": self.submitting_thread_id,
            "worker_thread_id": self.worker_thread_id,
            "worker_elapsed_ns": duration_ns(self.worker_elapsed),
            "spawn_join_elapsed_ns": duration_ns(self.spawn_join_elapsed),
        })
    }
}

#[test]
#[ignore = "requires a native adapter and RADIANT_SHADER_WORKER_PREPARATION_OUTPUT_DIR"]
fn records_worker_custom_shader_preparation() {
    let output_dir = required_env("RADIANT_SHADER_WORKER_PREPARATION_OUTPUT_DIR");
    let label = required_env("RADIANT_SHADER_WORKER_PREPARATION_LABEL");
    let source_revision = required_env("RADIANT_SHADER_WORKER_PREPARATION_SOURCE_REVISION");
    let (adapter_info, device, device_setup) = native_device();
    // Capture identity from the submitting/UI handle before cloning it into requests.
    let device_identity = wgpu_device_id(&device);
    let mut samples = Vec::with_capacity(2 + WARM_REPETITIONS + DISTINCT_VARIANTS);
    let cold_key = pipeline_key("cold", VALID_SHADER);
    samples.push(measure(
        &device,
        device_identity,
        &cold_key,
        "cold_device",
        0,
    ));
    for repetition in 0..WARM_REPETITIONS {
        samples.push(measure(
            &device,
            device_identity,
            &cold_key,
            "warm_device_identical_full_builder",
            repetition,
        ));
    }
    for variant in 0..DISTINCT_VARIANTS {
        let source = format!("{VALID_SHADER}\n// bounded distinct variant {variant}\n");
        let key = pipeline_key(&format!("distinct-{variant}"), &source);
        samples.push(measure(
            &device,
            device_identity,
            &key,
            "bounded_distinct_key",
            variant,
        ));
    }
    let invalid_key = pipeline_key("invalid", INVALID_SHADER);
    samples.push(measure(
        &device,
        device_identity,
        &invalid_key,
        "invalid_wgsl",
        0,
    ));
    assert!(matches!(samples[0].outcome, WorkerOutcome::Ready));
    assert!(
        samples[1..=WARM_REPETITIONS]
            .iter()
            .all(|sample| matches!(sample.outcome, WorkerOutcome::Ready))
    );
    assert!(
        samples[WARM_REPETITIONS + 1..WARM_REPETITIONS + 1 + DISTINCT_VARIANTS]
            .iter()
            .all(|sample| matches!(sample.outcome, WorkerOutcome::Ready))
    );
    assert!(matches!(
        samples.last().expect("invalid sample").outcome,
        WorkerOutcome::ShaderModule
    ));
    assert!(
        samples
            .iter()
            .all(|sample| sample.submitting_thread_id != sample.worker_thread_id)
    );

    let output = serde_json::json!({
        "schema_version": 1, "fixture": "opt-1457-worker-custom-shader-preparation",
        "label": label, "source_revision": source_revision, "worker_measurement_only": true,
        "ui_captured_device_identity": format!("{device_identity:#x}"),
        "adapter": { "name": adapter_info.name, "vendor": adapter_info.vendor,
            "device": adapter_info.device, "device_type": format!("{:?}", adapter_info.device_type),
            "driver": adapter_info.driver, "driver_info": adapter_info.driver_info,
            "backend": format!("{:?}", adapter_info.backend) },
        "target_format": "Rgba8Unorm", "device_setup_ns": duration_ns(device_setup),
        "source_hash_algorithm": "fnv1a64",
        "samples": samples.iter().map(WorkerSample::json).collect::<Vec<_>>(),
    });
    fs::create_dir_all(&output_dir)
        .expect("create worker preparation measurement output directory");
    let output_path = PathBuf::from(output_dir).join("shader-preparation-worker-samples.json");
    let mut output_file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&output_path)
        .expect("exclusively create worker preparation samples");
    output_file
        .write_all(
            &serde_json::to_vec_pretty(&output).expect("serialize worker preparation samples"),
        )
        .expect("write worker preparation samples");
    eprintln!(
        "RADIANT_SHADER_WORKER_PREPARATION_OUTPUT={}",
        output_path.display()
    );
}

fn measure(
    device: &wgpu::Device,
    device_identity: usize,
    key: &CustomShaderPipelineKey,
    workload: &'static str,
    variant: usize,
) -> WorkerSample {
    let request = OwnedCustomShaderPipelineRequest {
        device: device.clone(),
        device_identity,
        target_format: wgpu::TextureFormat::Rgba8Unorm,
        key: key.clone(),
    };
    let submitting_thread_id = format!("{:?}", std::thread::current().id());
    let source_hash = fnv1a64(&key.wgsl_source);
    let (sender, receiver) = mpsc::sync_channel(1);
    let spawn_join_start = Instant::now();
    let worker = std::thread::Builder::new()
        .name("radiant-opt-1457-measurement".into())
        .spawn(move || {
            let worker_thread_id = format!("{:?}", std::thread::current().id());
            let worker_start = Instant::now();
            let result = prepare_custom_shader_pipeline(request, || false);
            let worker_elapsed = worker_start.elapsed();
            let outcome = WorkerOutcome::from_result(result);
            let _ = sender.send((worker_thread_id, worker_elapsed, outcome));
        })
        .expect("spawn worker measurement thread");
    let (worker_thread_id, worker_elapsed, outcome) = receiver.recv().expect("worker result");
    worker.join().expect("worker measurement does not panic");
    WorkerSample {
        workload,
        variant,
        source_hash,
        outcome,
        submitting_thread_id,
        worker_thread_id,
        worker_elapsed,
        spawn_join_elapsed: spawn_join_start.elapsed(),
    }
}

fn required_env(name: &str) -> String {
    env::var(name).unwrap_or_else(|_| panic!("{name} must be set for this fixture"))
}

//! Bounded native before-change workload recorder. Writes raw observations on exit.

use radiant::prelude::*;
use radiant::runtime::{
    FrameProfile, GpuSignalGainPreview, GpuSignalSummary, ProfilingOptions, RenderCanvasContent,
    RenderCanvasShaderSurfaceDescriptor, render_canvas,
};
use std::{cell::RefCell, rc::Rc, sync::Arc};

#[path = "rendering_baseline/artifacts.rs"]
mod artifacts;
#[path = "rendering_baseline/upload_trace.rs"]
mod upload_trace;

struct State {
    mode: String,
    tick: u64,
    primary_identity: Option<u64>,
    window_environment: Option<radiant::runtime::WindowEnvironment>,
    samples: Arc<[f32]>,
    summary: Option<Arc<GpuSignalSummary>>,
    observations: Rc<RefCell<Vec<serde_json::Value>>>,
}

#[derive(Clone)]
enum Message {
    Tick,
    Stop,
}

const FRAMES: usize = 600 * 48_000;

impl State {
    fn observe(&mut self, profile: FrameProfile) {
        if self.primary_identity.is_none() {
            self.primary_identity = profile.window_identity;
        }
        if self.observations.borrow().len() < 4096 {
            self.observations.borrow_mut().push(serde_json::json!({
                "type": "native_frame", "workload": self.mode, "tick": self.tick,
                "window": profile.window_identity, "sequence": profile.frame_sequence,
                "role": if profile.window_identity == self.primary_identity { "primary" } else { "auxiliary" },
                "cpu_prepare_us": profile.timings.frame_work.total()
                    .saturating_sub(profile.timings.frame_work.render_to_texture)
                    .saturating_sub(profile.timings.frame_work.full_screen_blit).as_secs_f64() * 1e6,
                "cpu_composite_us": profile.timings.composited_base.refresh.as_secs_f64() * 1e6,
                "cpu_overlay_us": profile.timings.transient_overlay.paint.as_secs_f64() * 1e6,
                "cpu_blit_encode_us": profile.timings.frame_work.full_screen_blit.as_secs_f64() * 1e6,
                "cpu_submit_present_us": profile.timings.submit_present.as_secs_f64() * 1e6,
                "cpu_total_us": profile.timings.cpu_envelope_total().as_secs_f64() * 1e6,
                "present_interval_us": profile.timings.since_last_present.as_secs_f64() * 1e6,
                "scene_rebuild_count": u8::from(profile.scene_rebuild),
                "signal_summary_build_count": profile.counters.gpu_surfaces.signal.summary_builds,
                "signal_body_render_count": profile.counters.gpu_surfaces.signal.body_renders,
                "shader_pipeline_rebuild_count": profile.counters.gpu_surfaces.custom_shader.pipeline_rebuilds,
                "shader_static_write_bytes": profile.counters.gpu_surfaces.custom_shader.static_write_bytes,
                "cpu_render_encode_us": profile.timings.frame_work.render_to_texture.as_secs_f64() * 1e6,
            }));
        }
    }
}

fn view(state: &State) -> View<Message> {
    let moving = (state.tick % 120) as f32;
    let offset = if state.mode == "pan" {
        moving * 32.0
    } else if state.mode == "crossing" {
        moving * 48_000.0
    } else {
        0.0
    };
    let signal = || {
        let range = [offset, offset + 48_000.0];
        let content = if let Some(summary) = &state.summary {
            RenderCanvasContent::SignalSummaryBands {
                frames: FRAMES,
                band_count: 2,
                frame_range: range,
                summary: Arc::clone(summary),
                gain_preview: Some(GpuSignalGainPreview {
                    start: 0.0,
                    end: 1.0,
                    gain: 0.5 + moving / 120.0,
                    fade_in_length: 0.1,
                    fade_out_length: 0.1,
                    ..GpuSignalGainPreview::default()
                }),
                sample_slide_frame_offset: 0,
            }
        } else {
            RenderCanvasContent::SignalBands {
                frames: FRAMES,
                band_count: 2,
                frame_range: range,
                samples: Arc::clone(&state.samples),
            }
        };
        render_canvas(100, 0, content).id(100).size(720.0, 240.0)
    };
    let body = if state.mode == "shaders" {
        column(
            (0..16)
                .map(|index| {
                    render_canvas(
                        200 + index,
                        0,
                        RenderCanvasContent::CustomShader {
                            descriptor: Arc::new(
                                RenderCanvasShaderSurfaceDescriptor::new("baseline/equivalent")
                                    .wgsl_source(DEMO_SHADER_WGSL)
                                    .entry_point("vertex_main")
                                    .fragment_entry_point("fragment_main")
                                    .vertex_count(6),
                            ),
                        },
                    )
                    .id(200 + index)
                    .size(720.0, 20.0)
                })
                .collect::<Vec<_>>(),
        )
    } else if matches!(state.mode.as_str(), "local" | "two_windows" | "idle") {
        column(
            (0..100)
                .map(|index| {
                    text(if index == 0 {
                        format!("Local edit {}", state.tick)
                    } else {
                        format!("Unchanged sibling {index}")
                    })
                    .id(300 + index)
                })
                .collect::<Vec<_>>(),
        )
    } else {
        signal()
    };
    column([text(format!("Native baseline: {}", state.mode)), body]).padding(12.0)
}

#[allow(clippy::arc_with_non_send_sync)]
fn main() -> radiant::Result {
    let mut args: Vec<_> = std::env::args().collect();
    if args.len() == 1
        && let (Ok(mode), Ok(output)) = (
            std::env::var("RADIANT_BASELINE_MODE"),
            std::env::var("RADIANT_BASELINE_OUTPUT"),
        )
    {
        args.extend([mode, output]);
    }
    if args.len() != 3
        || ![
            "cold",
            "pan",
            "crossing",
            "gain",
            "shaders",
            "local",
            "two_windows",
            "idle",
        ]
        .contains(&args[1].as_str())
    {
        return Err("usage: rendering_baseline cold|pan|crossing|gain|shaders|local|two_windows|idle output.jsonl".into());
    }
    let upload_trace = upload_trace::UploadTrace::from_environment()?;
    let output = artifacts::ArtifactOutput::create(&args[2])?;
    let mode = args[1].clone();
    let preparation_started = std::time::Instant::now();
    let samples: Arc<[f32]> = if matches!(mode.as_str(), "cold" | "pan" | "crossing" | "gain") {
        (0..FRAMES * 2)
            .map(|index| ((index % 2048) as f32 / 1024.0) - 1.0)
            .collect()
    } else {
        Arc::from([])
    };
    let summary = (mode == "gain").then(|| {
        Arc::new(GpuSignalSummary::from_interleaved_samples(
            &samples, FRAMES, 2,
        ))
    });
    let observations = Rc::new(RefCell::new(Vec::with_capacity(4096)));
    observations.borrow_mut().push(serde_json::json!({
        "type": "native_fixture", "workload": mode,
        "source_frames": samples.len() / 2, "band_count": 2,
        "sample_rate_hz": 48_000, "sample_bytes": std::mem::size_of_val(samples.as_ref()),
        "source_prepare_us": preparation_started.elapsed().as_secs_f64() * 1e6,
        "summary_precomputed": summary.is_some(), "frame_limit": 240,
    }));
    let run_started = std::time::Instant::now();
    let state = State {
        mode,
        tick: 0,
        primary_identity: None,
        window_environment: None,
        samples,
        summary,
        observations: Rc::clone(&observations),
    };
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        upload_trace.run(|| {
        radiant::app(state)
        .title("Radiant rendering baseline")
        .size(760, 520)
        .profiling(ProfilingOptions::frame())
        .view(view)
        .animation(|state| state.mode != "idle" && state.tick < 240)
        .on_frame(|| Message::Tick)
        .on_startup(|_, context| context.after(std::time::Duration::from_secs(20), Message::Stop))
        .on_frame_profile(State::observe)
        .on_frame_gpu_timing(|state, sample| {
            if state.observations.borrow().len() < 4096 {
                state
                    .observations
                    .borrow_mut()
                    .push(serde_json::json!({"type": "native_gpu",
                    "window": sample.window_identity, "sequence": sample.frame_sequence,
                    "gpu_us": sample.outcome.duration().map(|d| d.as_secs_f64() * 1e6),
                    "outcome": format!("{:?}", sample.outcome)}));
            }
        })
        .auxiliary_windows(|state| {
            if state.mode != "two_windows" || state.primary_identity.is_none() {
                return Vec::new();
            }
            let mut window = AuxiliaryWindow::utility(
                "baseline-aux",
                "Baseline auxiliary",
                420.0,
                260.0,
                Arc::new(view(state).into_surface()),
            );
            window.options.frame.profiling = ProfilingOptions::frame();
            vec![window]
        })
        .handle_message(|state, message, context| {
            let environment = context.window_environment();
            if state.window_environment != Some(environment) {
                state.window_environment = Some(environment);
                state.observations.borrow_mut().push(serde_json::json!({
                    "type": "native_environment", "role": "primary", "tick": state.tick,
                    "scale": environment.display_scale().factor(),
                    "color_scheme": format!("{:?}", environment.color_scheme()),
                    "contrast": environment.contrast(), "reduced_motion": environment.reduced_motion(),
                }));
            }
            if matches!(message, Message::Stop) {
                context.exit();
                return;
            }
            state.tick += 1;
            if state.tick == 240 {
                context.after(std::time::Duration::from_millis(250), Message::Stop);
            } else if state.mode != "idle" {
                context.request_repaint();
            }
        })
        .run_with_artifacts()
    })
    }));
    // This executable never reuses the native runtime after an unwind. Preserve
    // already observed rows for diagnosis, but mark the entire run as failed.
    let (startup, result) = match result {
        Ok(report) => (
            report.artifacts.startup_timing,
            report.result.map_err(|error| error.to_string()),
        ),
        Err(_) => (
            None,
            Err("native runtime panicked; see process stderr".to_owned()),
        ),
    };
    observations.borrow_mut().push(serde_json::json!({
        "type": "native_run", "elapsed_us": run_started.elapsed().as_secs_f64() * 1e6,
        "startup_status": startup.as_ref().map(|artifact| artifact.status.as_str()),
        "startup_failure": startup.as_ref().and_then(|artifact| artifact.failure_reason.as_deref()),
        "first_present_ms": startup.as_ref().and_then(|artifact| artifact.first_present_ms),
        "startup_timing": startup,
        "run_error": result.as_ref().err(),
    }));
    upload_trace.append_artifacts(&mut observations.borrow_mut());
    output.finish(&observations.borrow())?;
    result?;
    if !observations
        .borrow()
        .iter()
        .any(|row| row["type"] == "native_frame")
    {
        return Err("no native profiles published".into());
    }
    Ok(())
}
const DEMO_SHADER_WGSL: &str = r#"
struct Params {
    dest: vec4<f32>,
    source: vec4<f32>,
    target_size: vec2<f32>,
    overlay_ratios: array<vec4<f32>, 2>,
    overlay_widths: array<vec4<f32>, 2>,
    overlay_colors: array<vec4<f32>, 8>,
};

@group(0) @binding(0)
var<uniform> params: Params;

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) local: vec2<f32>,
};

@vertex
fn vertex_main(@builtin(vertex_index) vertex_index: u32) -> VertexOut {
    var corners = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(1.0, 1.0),
    );
    let local = corners[vertex_index];
    let pixel = params.dest.xy + local * params.dest.zw;
    let clip = vec2<f32>(
        pixel.x / params.target_size.x * 2.0 - 1.0,
        1.0 - pixel.y / params.target_size.y * 2.0,
    );
    var out: VertexOut;
    out.position = vec4<f32>(clip, 0.0, 1.0);
    out.local = local;
    return out;
}

@fragment
fn fragment_main(in: VertexOut) -> @location(0) vec4<f32> {
    return vec4<f32>(0.16 + in.local.x * 0.28, 0.72, 0.82 - in.local.y * 0.24, 1.0);
}
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use radiant::runtime::{PaintPrimitive, SurfaceRuntime};

    fn state(mode: &str) -> State {
        State {
            mode: mode.to_owned(),
            tick: 0,
            primary_identity: None,
            window_environment: None,
            samples: Arc::from([]),
            summary: None,
            observations: Rc::new(RefCell::new(Vec::new())),
        }
    }

    #[test]
    fn equivalent_shader_fixture_contains_sixteen_distinct_surface_keys() {
        let runtime = SurfaceRuntime::new(
            radiant::app(state("shaders"))
                .view(view)
                .update(|_, _| {})
                .into_bridge(),
            Vector2::new(760.0, 520.0),
        );
        let plan = runtime.paint_plan(&Default::default());
        let surfaces: Vec<_> = plan
            .primitives
            .iter()
            .filter_map(|primitive| {
                if let PaintPrimitive::GpuSurface(surface) = primitive {
                    Some(surface)
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(surfaces.len(), 16);
        for (index, surface) in surfaces.iter().enumerate() {
            assert_eq!(surface.key, 200 + index as u64);
            assert_eq!(surface.content, surfaces[0].content);
        }
    }

    #[test]
    fn profiles_preserve_window_identity_and_distinct_cpu_stages() {
        let mut state = state("two_windows");
        for identity in [7, 9, 7] {
            state.observe(FrameProfile {
                window_identity: Some(identity),
                frame_sequence: Some(1),
                timings: radiant::runtime::FrameProfileTimings {
                    frame_work: radiant::runtime::FrameProfileWorkTimings {
                        refresh_surface: std::time::Duration::from_micros(10),
                        render_to_texture: std::time::Duration::from_micros(20),
                        full_screen_blit: std::time::Duration::from_micros(3),
                        ..Default::default()
                    },
                    submit_present: std::time::Duration::from_micros(4),
                    ..Default::default()
                },
                ..FrameProfile::default()
            });
        }
        let rows = state.observations.borrow();
        assert_eq!(rows[0]["role"], "primary");
        assert_eq!(rows[1]["role"], "auxiliary");
        assert_eq!(rows[2]["role"], "primary");
        for key in [
            "cpu_prepare_us",
            "cpu_render_encode_us",
            "cpu_submit_present_us",
            "present_interval_us",
        ] {
            assert!(rows[0][key].is_number());
        }
        assert_eq!(rows[0]["cpu_prepare_us"], 10.0);
        assert_eq!(rows[0]["cpu_render_encode_us"], 20.0);
        assert_eq!(rows[0]["cpu_blit_encode_us"], 3.0);
        assert_eq!(rows[0]["cpu_submit_present_us"], 4.0);
        assert_eq!(rows[0]["cpu_total_us"], 37.0);
        assert!(rows[0].get("gpu_us").is_none());
    }
}

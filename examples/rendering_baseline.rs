//! Bounded native before-change workload recorder. Writes raw observations on exit.

use radiant::prelude::*;
use radiant::runtime::{
    FrameProfile, GpuSignalGainPreview, GpuSignalSummary, ProfilingOptions, RenderCanvasContent,
    RenderCanvasShaderSurfaceDescriptor, render_canvas,
};
use std::{cell::RefCell, fs::File, io::Write, rc::Rc, sync::Arc};

struct State {
    mode: String,
    tick: u64,
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
        if self.observations.borrow().len() < 4096 {
            self.observations.borrow_mut().push(serde_json::json!({
                "type": "native_frame", "workload": self.mode, "tick": self.tick,
                "window": profile.window_identity, "sequence": profile.frame_sequence,
                "cpu_prepare_us": profile.timings.frame_work.total().as_secs_f64() * 1e6,
                "cpu_submit_present_us": profile.timings.submit_present.as_secs_f64() * 1e6,
                "cpu_total_us": profile.timings.cpu_envelope_total().as_secs_f64() * 1e6,
                "present_interval_us": profile.timings.since_last_present.as_secs_f64() * 1e6,
                "scene_rebuild": profile.scene_rebuild,
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
    let args: Vec<_> = std::env::args().collect();
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
    let mut output = File::create_new(&args[2]).map_err(|error| error.to_string())?;
    let mode = args[1].clone();
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
    let state = State {
        mode,
        tick: 0,
        samples,
        summary,
        observations: Rc::clone(&observations),
    };
    let result = radiant::app(state)
        .title("Radiant rendering baseline")
        .size(760, 520)
        .profiling(ProfilingOptions::frame())
        .view(view)
        .animation(|state| state.mode != "idle")
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
            if state.mode != "two_windows" {
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
            if matches!(message, Message::Stop) {
                context.exit();
                return;
            }
            state.tick += 1;
            if state.tick >= 240 {
                context.exit();
            } else if state.mode != "idle" {
                context.request_repaint();
            }
        })
        .run_with_artifacts();
    eprintln!("native run artifacts: {:?}", result.artifacts);
    let result = result.result.map_err(|error| error.to_string());
    for row in observations.borrow().iter() {
        writeln!(output, "{row}").map_err(|error| error.to_string())?;
    }
    output.flush().map_err(|error| error.to_string())?;
    if observations.borrow().is_empty() {
        return Err("no native profiles published".into());
    }
    result
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

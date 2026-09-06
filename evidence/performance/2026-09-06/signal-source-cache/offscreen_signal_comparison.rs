//! Native-GPU comparison fixture for retained signal source-cache residency.

use super::*;
use crate::gui::types::{Point, Rect, Vector2};
use crate::runtime::{
    GpuSignalGainPreview, GpuSignalSummary, GpuSurfaceCapabilities, PaintGpuSurface, PaintPrimitive,
};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use vello::wgpu;

const TARGET_SIZE: u32 = 64;
const FRAMES: usize = 65_536;
const BAND_COUNT: usize = 4;
const SIGNAL_KEY: u64 = 145_300;
const SIGNAL_WIDGET_ID: u64 = 1453;
const INITIAL_RANGE: [f32; 2] = [8193.0, 12289.0];
const NEARBY_RANGE: [f32; 2] = [8201.0, 12297.0];
const INTERVAL_RANGE: [f32; 2] = [8321.0, 12417.0];
const LOD_RANGE: [f32; 2] = [8193.0, 16385.0];

#[test]
#[ignore = "requires a native GPU and comparison output environment"]
fn offscreen_signal_source_cache_comparison() {
    let expected_source_cache = expected_source_cache();
    let label = required_safe_env("RADIANT_COMPARISON_LABEL");
    let output_dir = std::env::var("RADIANT_COMPARISON_OUTPUT_DIR")
        .expect("RADIANT_COMPARISON_OUTPUT_DIR is required");
    fs::create_dir_all(&output_dir).expect("create comparison output directory");

    let (device, queue, adapter) = native_device();
    write_adapter(&output_dir, &label, &adapter);
    let samples = deterministic_samples(1.0);
    let replacement_samples = deterministic_samples(0.42);
    let precomputed_summary = Arc::new(GpuSignalSummary::from_interleaved_samples(
        &samples, FRAMES, BAND_COUNT,
    ));

    let mut raw_renderer = GpuSurfaceRenderer::default();
    let initial = render_case(
        &mut raw_renderer,
        &device,
        &queue,
        raw_surface(Arc::clone(&samples), INITIAL_RANGE, 1),
    );
    assert_raw_initial(&initial);
    write_case(&output_dir, &label, "raw_initial", &initial, None);

    let nearby = render_case(
        &mut raw_renderer,
        &device,
        &queue,
        raw_surface(Arc::clone(&samples), NEARBY_RANGE, 1),
    );
    assert_raw_presentation(&nearby, expected_source_cache, "nearby pan");
    assert_pixels_changed(&initial.pixels, &nearby.pixels, "nearby pan");
    write_case(
        &output_dir,
        &label,
        "raw_nearby_pan",
        &nearby,
        Some(&initial.pixels),
    );

    let interval = render_case(
        &mut raw_renderer,
        &device,
        &queue,
        raw_surface(Arc::clone(&samples), INTERVAL_RANGE, 1),
    );
    assert_raw_uploading_presentation(&interval, expected_source_cache, "interval crossing");
    assert_pixels_changed(&nearby.pixels, &interval.pixels, "interval crossing");
    write_case(
        &output_dir,
        &label,
        "raw_interval_crossing",
        &interval,
        Some(&nearby.pixels),
    );

    let lod = render_case(
        &mut raw_renderer,
        &device,
        &queue,
        raw_surface(Arc::clone(&samples), LOD_RANGE, 1),
    );
    assert_raw_uploading_presentation(&lod, expected_source_cache, "LOD span");
    assert_pixels_changed(&interval.pixels, &lod.pixels, "LOD span");
    write_case(
        &output_dir,
        &label,
        "raw_lod_span",
        &lod,
        Some(&interval.pixels),
    );

    let replacement = render_case(
        &mut raw_renderer,
        &device,
        &queue,
        raw_surface(replacement_samples, INITIAL_RANGE, 1),
    );
    assert_raw_rebuild(&replacement, "source replacement");
    assert_pixels_changed(&lod.pixels, &replacement.pixels, "source replacement");
    write_case(
        &output_dir,
        &label,
        "raw_source_replacement",
        &replacement,
        Some(&lod.pixels),
    );

    let revision = render_case(
        &mut raw_renderer,
        &device,
        &queue,
        raw_surface(Arc::clone(&samples), INITIAL_RANGE, 2),
    );
    assert_raw_rebuild(&revision, "revision");
    assert_eq!(
        revision.pixels, initial.pixels,
        "revision-only update changed rendered pixels"
    );
    write_case(
        &output_dir,
        &label,
        "raw_revision",
        &revision,
        Some(&replacement.pixels),
    );

    let mut summary_renderer = GpuSurfaceRenderer::default();
    let summary_initial = render_case(
        &mut summary_renderer,
        &device,
        &queue,
        summary_surface(Arc::clone(&precomputed_summary), INITIAL_RANGE, None, 0, 1),
    );
    assert_summary_initial(&summary_initial);
    write_case(
        &output_dir,
        &label,
        "summary_initial",
        &summary_initial,
        None,
    );

    let gain_preview = render_case(
        &mut summary_renderer,
        &device,
        &queue,
        summary_surface(
            Arc::clone(&precomputed_summary),
            INITIAL_RANGE,
            Some(GpuSignalGainPreview {
                start: 0.18,
                end: 0.76,
                gain: 0.28,
                ..GpuSignalGainPreview::default()
            }),
            0,
            1,
        ),
    );
    assert_summary_presentation(&gain_preview, expected_source_cache, "gain preview");
    assert_pixels_changed(
        &summary_initial.pixels,
        &gain_preview.pixels,
        "gain preview",
    );
    write_case(
        &output_dir,
        &label,
        "summary_gain_preview",
        &gain_preview,
        Some(&summary_initial.pixels),
    );

    let fade = render_case(
        &mut summary_renderer,
        &device,
        &queue,
        summary_surface(
            Arc::clone(&precomputed_summary),
            INITIAL_RANGE,
            Some(GpuSignalGainPreview {
                start: 0.18,
                end: 0.76,
                gain: 0.7,
                fade_in_length: 0.2,
                fade_in_curve: 0.4,
                fade_in_mute: 0.08,
                fade_in_outer_gain: 0.0,
                fade_out_length: 0.24,
                fade_out_curve: -0.35,
                fade_out_mute: 0.08,
                fade_out_outer_gain: 0.0,
            }),
            0,
            1,
        ),
    );
    assert_summary_presentation(&fade, expected_source_cache, "fade preview");
    assert_pixels_changed(&gain_preview.pixels, &fade.pixels, "fade preview");
    write_case(
        &output_dir,
        &label,
        "summary_fade_preview",
        &fade,
        Some(&gain_preview.pixels),
    );

    let summary_nearby = render_case(
        &mut summary_renderer,
        &device,
        &queue,
        summary_surface(
            precomputed_summary,
            NEARBY_RANGE,
            Some(GpuSignalGainPreview {
                start: 0.18,
                end: 0.76,
                gain: 0.7,
                fade_in_length: 0.2,
                fade_in_curve: 0.4,
                fade_in_mute: 0.08,
                fade_in_outer_gain: 0.0,
                fade_out_length: 0.24,
                fade_out_curve: -0.35,
                fade_out_mute: 0.08,
                fade_out_outer_gain: 0.0,
            }),
            8,
            1,
        ),
    );
    assert_summary_presentation(&summary_nearby, expected_source_cache, "nearby pan");
    assert_pixels_changed(&fade.pixels, &summary_nearby.pixels, "summary nearby pan");
    write_case(
        &output_dir,
        &label,
        "summary_nearby_pan",
        &summary_nearby,
        Some(&fade.pixels),
    );
}

struct RenderedCase {
    stats: GpuSurfaceRenderStats,
    pixels: Vec<u8>,
}

fn expected_source_cache() -> bool {
    match std::env::var("RADIANT_SIGNAL_EXPECT_SOURCE_CACHE")
        .expect("set RADIANT_SIGNAL_EXPECT_SOURCE_CACHE to 0 for baseline or 1 for candidate")
        .as_str()
    {
        "0" => false,
        "1" => true,
        value => panic!("RADIANT_SIGNAL_EXPECT_SOURCE_CACHE must be 0 or 1, got {value:?}"),
    }
}

fn required_safe_env(name: &str) -> String {
    let value = std::env::var(name).unwrap_or_else(|_| panic!("{name} is required"));
    assert!(
        !value.is_empty()
            && value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_')),
        "{name} must contain only ASCII letters, digits, '-' or '_'"
    );
    value
}

fn deterministic_samples(amplitude: f32) -> Arc<[f32]> {
    let mut samples = Vec::with_capacity(FRAMES * BAND_COUNT);
    for frame in 0..FRAMES {
        for band in 0..BAND_COUNT {
            let phase = ((frame * (band * 2 + 3) + band * 29) % 257) as f32 / 256.0;
            let saw = phase * 2.0 - 1.0;
            let spike = if (frame + band * 17) % 193 < 4 {
                if (frame / 193) % 2 == 0 { 1.0 } else { -1.0 }
            } else {
                0.0
            };
            samples.push((saw * 0.64 + spike * 0.36) * amplitude);
        }
    }
    samples.into()
}

fn raw_surface(samples: Arc<[f32]>, frame_range: [f32; 2], revision: u64) -> PaintGpuSurface {
    signal_surface(
        GpuSurfaceContent::SignalBands {
            frames: FRAMES,
            band_count: BAND_COUNT,
            frame_range,
            samples,
        },
        revision,
    )
}

fn summary_surface(
    summary: Arc<GpuSignalSummary>,
    frame_range: [f32; 2],
    gain_preview: Option<GpuSignalGainPreview>,
    sample_slide_frame_offset: i64,
    revision: u64,
) -> PaintGpuSurface {
    signal_surface(
        GpuSurfaceContent::SignalSummaryBands {
            frames: FRAMES,
            band_count: BAND_COUNT,
            frame_range,
            summary,
            gain_preview,
            sample_slide_frame_offset,
        },
        revision,
    )
}

fn signal_surface(content: GpuSurfaceContent, revision: u64) -> PaintGpuSurface {
    PaintGpuSurface {
        widget_id: SIGNAL_WIDGET_ID,
        key: SIGNAL_KEY,
        revision,
        rect: Rect::from_min_size(
            Point::new(0.0, 0.0),
            Vector2::new(TARGET_SIZE as f32, TARGET_SIZE as f32),
        ),
        content,
        capabilities: GpuSurfaceCapabilities::default(),
        overlays: Vec::new(),
    }
}

fn render_case(
    renderer: &mut GpuSurfaceRenderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    surface: PaintGpuSurface,
) -> RenderedCase {
    let primitives = [PaintPrimitive::GpuSurface(surface)];
    let (texture, view) = render_target(device);
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("radiant_offscreen_signal_comparison"),
    });
    let stats = {
        let mut target = GpuSurfaceRenderTarget {
            device,
            queue,
            encoder: &mut encoder,
            target_view: &view,
            format: wgpu::TextureFormat::Rgba8Unorm,
            size: Vector2::new(TARGET_SIZE as f32, TARGET_SIZE as f32),
            dpi_scale: crate::theme::DpiScale::ONE,
            upload_plan_context: None,
            upload_plan: None,
            collect_upload_plan: false,
        };
        let mut occlusion = SurfaceOcclusionPlan::default();
        occlusion.preprocess(&primitives);
        renderer.render(&mut target, &primitives, &occlusion, &[])
    };
    renderer.finish_presentation_staging_belt();
    queue.submit(std::iter::once(encoder.finish()));
    let pixels = readback_rgba(device, queue, &texture);
    renderer.recall_presentation_staging_belt();
    assert!(
        stats.render_canvas_upload_plan.is_none(),
        "legacy render must not report an upload plan"
    );
    assert_upload_accounting(&stats);
    RenderedCase { stats, pixels }
}

fn render_target(device: &wgpu::Device) -> (wgpu::Texture, wgpu::TextureView) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("radiant_offscreen_signal_comparison_target"),
        size: wgpu::Extent3d {
            width: TARGET_SIZE,
            height: TARGET_SIZE,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    (texture, view)
}

fn readback_rgba(device: &wgpu::Device, queue: &wgpu::Queue, texture: &wgpu::Texture) -> Vec<u8> {
    let row_bytes = TARGET_SIZE * 4;
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("radiant_offscreen_signal_comparison_readback"),
        size: u64::from(row_bytes) * u64::from(TARGET_SIZE),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(row_bytes),
                rows_per_image: Some(TARGET_SIZE),
            },
        },
        wgpu::Extent3d {
            width: TARGET_SIZE,
            height: TARGET_SIZE,
            depth_or_array_layers: 1,
        },
    );
    queue.submit(std::iter::once(encoder.finish()));
    let slice = buffer.slice(..);
    let (sender, receiver) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = sender.send(result);
    });
    device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(Duration::from_secs(5)),
        })
        .expect("bounded offscreen comparison readback poll");
    receiver
        .recv_timeout(Duration::from_millis(100))
        .expect("offscreen comparison readback callback")
        .expect("offscreen comparison readback result");
    let pixels = slice.get_mapped_range().to_vec();
    buffer.unmap();
    assert_eq!(pixels.len(), (TARGET_SIZE * TARGET_SIZE * 4) as usize);
    pixels
}

fn assert_upload_accounting(stats: &GpuSurfaceRenderStats) {
    assert!(
        stats
            .render_canvas_uploads
            .immutable_payload
            .operations
            .is_some()
    );
    assert!(
        stats
            .render_canvas_uploads
            .immutable_payload
            .logical_bytes
            .is_some()
    );
    assert!(
        stats
            .render_canvas_uploads
            .renderer_parameter
            .operations
            .is_some()
    );
    assert!(
        stats
            .render_canvas_uploads
            .renderer_parameter
            .logical_bytes
            .is_some()
    );
}

fn immutable_operations(case: &RenderedCase) -> usize {
    case.stats
        .render_canvas_uploads
        .immutable_payload
        .operations
        .expect("immutable upload accounting is available")
}

fn assert_raw_initial(case: &RenderedCase) {
    assert_eq!(case.stats.signal.summary_builds, 1);
    assert!(immutable_operations(case) > 0);
}

fn assert_raw_presentation(case: &RenderedCase, expected_source_cache: bool, name: &str) {
    assert_eq!(
        case.stats.signal.summary_builds,
        usize::from(!expected_source_cache),
        "{name} summary-build expectation"
    );
    if expected_source_cache {
        assert_eq!(immutable_operations(case), 0, "{name} immutable uploads");
    } else {
        assert!(immutable_operations(case) > 0, "{name} immutable uploads");
    }
}

fn assert_raw_uploading_presentation(case: &RenderedCase, expected_source_cache: bool, name: &str) {
    assert_eq!(
        case.stats.signal.summary_builds,
        usize::from(!expected_source_cache),
        "{name} summary-build expectation"
    );
    assert!(immutable_operations(case) > 0, "{name} immutable uploads");
}

fn assert_raw_rebuild(case: &RenderedCase, name: &str) {
    assert_eq!(
        case.stats.signal.summary_builds, 1,
        "{name} summary rebuild"
    );
    assert!(immutable_operations(case) > 0, "{name} immutable upload");
}

fn assert_summary_initial(case: &RenderedCase) {
    assert_eq!(case.stats.signal.summary_builds, 0);
    assert!(immutable_operations(case) > 0);
}

fn assert_summary_presentation(case: &RenderedCase, expected_source_cache: bool, name: &str) {
    assert_eq!(case.stats.signal.summary_builds, 0, "{name} summary build");
    if expected_source_cache {
        assert_eq!(immutable_operations(case), 0, "{name} immutable uploads");
    } else {
        assert!(immutable_operations(case) > 0, "{name} immutable uploads");
    }
}

fn assert_pixels_changed(before: &[u8], after: &[u8], name: &str) {
    assert_eq!(before.len(), after.len(), "{name} pixel length");
    assert_ne!(before, after, "{name} did not change full RGBA output");
}

struct AdapterObservation {
    name: String,
    backend: String,
    vendor: u32,
    device: u32,
    device_type: String,
    driver: String,
    driver_info: String,
}

impl AdapterObservation {
    fn json(&self) -> String {
        format!(
            "{{\"name\":{},\"backend\":{},\"vendor\":{},\"device\":{},\"device_type\":{},\"driver\":{},\"driver_info\":{}}}",
            json_string(&self.name),
            json_string(&self.backend),
            self.vendor,
            self.device,
            json_string(&self.device_type),
            json_string(&self.driver),
            json_string(&self.driver_info),
        )
    }
}

fn native_device() -> (wgpu::Device, wgpu::Queue, AdapterObservation) {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::PRIMARY,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        compatible_surface: None,
        ..Default::default()
    }))
    .expect("offscreen comparison requires a WGPU adapter");
    let info = adapter.get_info();
    let observation = AdapterObservation {
        name: info.name,
        backend: format!("{:?}", info.backend),
        vendor: info.vendor,
        device: info.device,
        device_type: format!("{:?}", info.device_type),
        driver: info.driver,
        driver_info: info.driver_info,
    };
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("radiant_offscreen_signal_comparison"),
        required_features: wgpu::Features::empty(),
        required_limits: wgpu::Limits::default(),
        ..Default::default()
    }))
    .expect("offscreen comparison requires a WGPU device");
    (device, queue, observation)
}

fn write_adapter(output_dir: &str, label: &str, adapter: &AdapterObservation) {
    let file_name = format!("{label}-signal-adapter.json");
    write_exclusive(
        Path::new(output_dir).join(file_name),
        adapter.json().as_bytes(),
    );
}

fn write_case(
    output_dir: &str,
    label: &str,
    case: &str,
    rendered: &RenderedCase,
    before: Option<&[u8]>,
) {
    let rgba_name = format!("{label}-signal-{case}.rgba");
    let json_name = format!("{label}-signal-{case}.json");
    write_exclusive(Path::new(output_dir).join(&rgba_name), &rendered.pixels);
    let changed = before
        .map(|pixels| pixels != rendered.pixels)
        .unwrap_or(false);
    let metadata = format!(
        "{{\"label\":{},\"case\":{},\"width\":{},\"height\":{},\"rgba_file\":{},\"pixels_changed_from_previous\":{},\"stats\":{}}}",
        json_string(label),
        json_string(case),
        TARGET_SIZE,
        TARGET_SIZE,
        json_string(&rgba_name),
        changed,
        stats_json(&rendered.stats),
    );
    write_exclusive(Path::new(output_dir).join(json_name), metadata.as_bytes());
    println!("{metadata}");
}

fn write_exclusive(path: impl AsRef<Path>, bytes: &[u8]) {
    let path = path.as_ref();
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .unwrap_or_else(|error| panic!("exclusive comparison output {}: {error}", path.display()));
    file.write_all(bytes)
        .unwrap_or_else(|error| panic!("write comparison output {}: {error}", path.display()));
}

fn stats_json(stats: &GpuSurfaceRenderStats) -> String {
    let immutable = stats.render_canvas_uploads.immutable_payload;
    format!(
        "{{\"immutable_payload\":{{\"operations\":{},\"logical_bytes\":{}}},\"signal\":{{\"summary_builds\":{},\"summary_cache_hits\":{},\"body_renders\":{},\"body_cache_hits\":{}}}}}",
        option_usize_json(immutable.operations),
        option_u64_json(immutable.logical_bytes),
        stats.signal.summary_builds,
        stats.signal.summary_cache_hits,
        stats.signal.body_renders,
        stats.signal.body_cache_hits,
    )
}

fn option_usize_json(value: Option<usize>) -> String {
    value.map_or_else(|| "null".to_owned(), |value| value.to_string())
}

fn option_u64_json(value: Option<u64>) -> String {
    value.map_or_else(|| "null".to_owned(), |value| value.to_string())
}

fn json_string(value: &str) -> String {
    let mut output = String::from("\"");
    for character in value.chars() {
        match character {
            '\"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            character if character.is_control() => {
                output.push_str(&format!("\\u{:04x}", character as u32))
            }
            character => output.push(character),
        }
    }
    output.push('\"');
    output
}

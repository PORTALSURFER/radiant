//! Portable native-GPU comparison fixture for custom-shader pipeline residency.

use super::*;
use crate::gui::types::{Point, Rect, Vector2};
use crate::runtime::{
    GpuShaderPresentationUniformUpdate, GpuShaderSurfaceDescriptor, GpuSurfaceCapabilities,
    GpuSurfaceContent, PaintGpuSurface,
};
use std::sync::Arc;
use std::time::Duration;
use vello::wgpu;

const TARGET_SIZE: u32 = 64;
const TILE_COUNT: usize = 16;
const TILE_GRID_WIDTH: usize = 4;
const TILE_SIZE: u32 = TARGET_SIZE / TILE_GRID_WIDTH as u32;
const TILE_KEY_BASE: u64 = 300_000;
const TILE_WIDGET_ID: u64 = 44;
const UPDATED_TILE_INDEX: usize = 6;

const TILE_SHADER: &str = r#"
@group(0) @binding(1) var<uniform> base: vec4<f32>;
@group(0) @binding(3) var<uniform> presentation: vec4<f32>;
@vertex fn vertex_main(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0), vec2<f32>(3.0, -1.0), vec2<f32>(-1.0, 3.0));
    return vec4<f32>(positions[index], 0.0, 1.0);
}
@fragment fn fragment_main() -> @location(0) vec4<f32> { return base + presentation; }
"#;

#[test]
#[ignore = "requires native GPU adapter, offscreen WGPU rendering, and RADIANT_EXPECTED_SHADER_PIPELINES"]
fn offscreen_16_tile_shader_pipeline_comparison() {
    let expected_pipelines = expected_pipeline_count();
    let label = std::env::var("RADIANT_COMPARISON_LABEL").ok();
    let (device, queue, adapter) = native_device();
    let mut renderer = GpuSurfaceRenderer::default();
    let first_primitives = tile_primitives(None);

    let (cold, first_texture) =
        render_tile_frame(&mut renderer, &device, &queue, &first_primitives, &[]);
    assert_eq!(cold.custom_shader.surfaces_rendered, TILE_COUNT);
    assert_eq!(cold.custom_shader.pipeline_rebuilds, expected_pipelines);
    assert_eq!(cold.custom_shader.binding_rebuilds, TILE_COUNT);
    let first_pixels = readback_rgba(&device, &queue, &first_texture);
    assert_tile_pixels(&first_pixels, None);
    renderer.recall_presentation_staging_belt();

    let (warm, _) = render_tile_frame(&mut renderer, &device, &queue, &first_primitives, &[]);
    assert_eq!(warm.custom_shader.surfaces_rendered, TILE_COUNT);
    assert_eq!(warm.custom_shader.pipeline_rebuilds, 0);
    assert_eq!(warm.custom_shader.binding_rebuilds, 0);
    assert_eq!(warm.custom_shader.static_writes, 0);
    assert_eq!(warm.custom_shader.presentation_writes, 0);
    renderer.recall_presentation_staging_belt();

    let updated_primitives = tile_primitives(Some(UPDATED_TILE_INDEX));
    let update = GpuShaderPresentationUniformUpdate::try_new(
        TILE_WIDGET_ID,
        tile_key(UPDATED_TILE_INDEX),
        91,
        2,
        2,
        rgba_uniform_bytes([0.0, 1.0, 0.0, 0.0]),
    )
    .expect("aligned presentation update");
    let (updated, updated_texture) = render_tile_frame(
        &mut renderer,
        &device,
        &queue,
        &updated_primitives,
        &[update],
    );
    assert_eq!(updated.custom_shader.surfaces_rendered, TILE_COUNT);
    assert_eq!(updated.custom_shader.pipeline_rebuilds, 0);
    assert_eq!(updated.custom_shader.binding_rebuilds, 0);
    assert_eq!(updated.custom_shader.static_writes, 1);
    assert_eq!(updated.custom_shader.presentation_writes, 2);
    let residency = renderer.custom_shader_residency_snapshot();
    assert_eq!(residency.pipeline_resident_count, expected_pipelines);
    assert_eq!(residency.binding_resident_count, TILE_COUNT);
    let updated_pixels = readback_rgba(&device, &queue, &updated_texture);
    assert_tile_pixels(&updated_pixels, Some(UPDATED_TILE_INDEX));

    println!(
        "{{\"adapter\":{},\"source_env_label\":{},\"expected_shader_pipelines\":{},\"cold\":{},\"warm\":{},\"updated\":{},\"residency\":{{\"pipeline_resident_count\":{},\"binding_resident_count\":{}}},\"tile_center_rgba\":{}}}",
        adapter.json(),
        optional_json_string(label.as_deref()),
        expected_pipelines,
        counters_json(cold),
        counters_json(warm),
        counters_json(updated),
        residency.pipeline_resident_count,
        residency.binding_resident_count,
        tile_centers_json(&updated_pixels),
    );
    renderer.recall_presentation_staging_belt();
}

fn expected_pipeline_count() -> usize {
    let value = std::env::var("RADIANT_EXPECTED_SHADER_PIPELINES")
        .expect("set RADIANT_EXPECTED_SHADER_PIPELINES explicitly to 1 or 16");
    let count = value
        .parse::<usize>()
        .expect("RADIANT_EXPECTED_SHADER_PIPELINES must be an integer");
    assert!(
        matches!(count, 1 | TILE_COUNT),
        "expected pipeline count must be 1 or {TILE_COUNT}"
    );
    count
}

fn tile_primitives(updated_tile: Option<usize>) -> Vec<PaintPrimitive> {
    (0..TILE_COUNT)
        .map(|index| {
            let updated = updated_tile == Some(index);
            PaintPrimitive::GpuSurface(tile_surface(
                index,
                tile_descriptor(
                    if updated {
                        [0.0, 0.0, 0.0, 1.0]
                    } else {
                        tile_base_color(index)
                    },
                    if updated { 2 } else { 1 },
                ),
            ))
        })
        .collect()
}

fn tile_descriptor(base: [f32; 4], storage_revision: u64) -> GpuShaderSurfaceDescriptor {
    GpuShaderSurfaceDescriptor::new("offscreen-comparison-shared-tile")
        .wgsl_source(Arc::<str>::from(TILE_SHADER))
        .entry_point("vertex_main")
        .fragment_entry_point("fragment_main")
        .uniform_bytes(rgba_uniform_bytes(base))
        .presentation_uniform(rgba_uniform_bytes([0.0; 4]), 1)
        .storage_identity(91)
        .storage_revision(storage_revision)
}

fn tile_surface(index: usize, descriptor: GpuShaderSurfaceDescriptor) -> PaintGpuSurface {
    let x = (index % TILE_GRID_WIDTH) as f32 * TILE_SIZE as f32;
    let y = (index / TILE_GRID_WIDTH) as f32 * TILE_SIZE as f32;
    PaintGpuSurface {
        widget_id: TILE_WIDGET_ID,
        key: tile_key(index),
        revision: 1,
        rect: Rect::from_min_size(
            Point::new(x, y),
            Vector2::new(TILE_SIZE as f32, TILE_SIZE as f32),
        ),
        content: GpuSurfaceContent::CustomShader {
            descriptor: Arc::new(descriptor),
        },
        capabilities: GpuSurfaceCapabilities::default(),
        overlays: Vec::new(),
    }
}

fn tile_key(index: usize) -> u64 {
    TILE_KEY_BASE + index as u64
}
fn tile_base_color(index: usize) -> [f32; 4] {
    [
        0.2 + 0.1 * (index % TILE_GRID_WIDTH) as f32,
        0.2 + 0.1 * (index / TILE_GRID_WIDTH) as f32,
        0.4,
        1.0,
    ]
}
fn rgba_uniform_bytes(rgba: [f32; 4]) -> [u8; 16] {
    let mut bytes = [0; 16];
    for (index, value) in rgba.into_iter().enumerate() {
        bytes[index * 4..(index + 1) * 4].copy_from_slice(&value.to_ne_bytes());
    }
    bytes
}

fn render_tile_frame(
    renderer: &mut GpuSurfaceRenderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    primitives: &[PaintPrimitive],
    presentation_updates: &[GpuShaderPresentationUniformUpdate],
) -> (GpuSurfaceRenderStats, wgpu::Texture) {
    let (texture, view) = render_target(device);
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("radiant_offscreen_tile_comparison"),
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
        occlusion.preprocess(primitives);
        renderer.render(&mut target, primitives, &occlusion, presentation_updates)
    };
    renderer.finish_presentation_staging_belt();
    queue.submit(std::iter::once(encoder.finish()));
    (stats, texture)
}

fn render_target(device: &wgpu::Device) -> (wgpu::Texture, wgpu::TextureView) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("radiant_offscreen_tile_comparison_target"),
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
        label: Some("radiant_offscreen_tile_comparison"),
        required_features: wgpu::Features::empty(),
        required_limits: wgpu::Limits::default(),
        ..Default::default()
    }))
    .expect("offscreen comparison requires a WGPU device");
    (device, queue, observation)
}

fn readback_rgba(device: &wgpu::Device, queue: &wgpu::Queue, texture: &wgpu::Texture) -> Vec<u8> {
    let row_bytes = 256;
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("radiant_offscreen_tile_comparison_readback"),
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
    let mapped = slice.get_mapped_range();
    let pixels = mapped.to_vec();
    drop(mapped);
    buffer.unmap();
    pixels
}

fn assert_tile_pixels(pixels: &[u8], updated_tile: Option<usize>) {
    for index in 0..TILE_COUNT {
        let expected = if updated_tile == Some(index) {
            [0.0, 1.0, 0.0, 1.0]
        } else {
            tile_base_color(index)
        };
        assert_color(&tile_center(pixels, index), expected);
    }
}
fn tile_centers_json(pixels: &[u8]) -> String {
    let values = (0..TILE_COUNT)
        .map(|index| {
            let [r, g, b, a] = tile_center(pixels, index);
            format!("[{r},{g},{b},{a}]")
        })
        .collect::<Vec<_>>();
    format!("[{}]", values.join(","))
}
fn tile_center(pixels: &[u8], index: usize) -> [u8; 4] {
    let x = (index % TILE_GRID_WIDTH) as u32 * TILE_SIZE + TILE_SIZE / 2;
    let y = (index / TILE_GRID_WIDTH) as u32 * TILE_SIZE + TILE_SIZE / 2;
    let offset = y as usize * 256 + x as usize * 4;
    [
        pixels[offset],
        pixels[offset + 1],
        pixels[offset + 2],
        pixels[offset + 3],
    ]
}
fn assert_color(actual: &[u8; 4], expected: [f32; 4]) {
    let expected = expected.map(|component| (component * 255.0).round() as i16);
    assert!(
        actual
            .iter()
            .copied()
            .map(i16::from)
            .zip(expected)
            .all(|(actual, expected)| (actual - expected).abs() <= 8),
        "expected {expected:?}, got {actual:?}"
    );
}
fn counters_json(stats: GpuSurfaceRenderStats) -> String {
    let custom = stats.custom_shader;
    format!(
        "{{\"surfaces_rendered\":{},\"pipeline_rebuilds\":{},\"binding_rebuilds\":{},\"static_writes\":{},\"static_write_bytes\":{},\"presentation_writes\":{},\"presentation_write_bytes\":{}}}",
        custom.surfaces_rendered,
        custom.pipeline_rebuilds,
        custom.binding_rebuilds,
        custom.static_writes,
        custom.static_write_bytes,
        custom.presentation_writes,
        custom.presentation_write_bytes,
    )
}
fn optional_json_string(value: Option<&str>) -> String {
    value.map(json_string).unwrap_or_else(|| "null".to_owned())
}
fn json_string(value: &str) -> String {
    let mut output = String::from("\"");
    for character in value.chars() {
        match character {
            '"' => output.push_str("\\\""),
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
    output.push('"');
    output
}

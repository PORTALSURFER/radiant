use vello::wgpu;
use std::time::Duration;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let use_vello = std::env::args().any(|arg| arg == "--vello");
    println!("vello_requested={use_vello}");
    let timestamps = std::env::args().any(|arg| arg == "--timestamps");
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor { backends: wgpu::Backends::METAL, ..wgpu::InstanceDescriptor::new_without_display_handle() });
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))?;
    println!("adapter={:?}", adapter.get_info());
    let timing_features = wgpu::Features::TIMESTAMP_QUERY | wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS;
    println!("timestamps_requested={timestamps} advertised={}", adapter.features().contains(timing_features));
    if timestamps && !adapter.features().contains(timing_features) { return Err("timestamp features unavailable".into()); }
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor { label: Some("Radiant minimal Metal diagnostic"), required_features: if timestamps { timing_features } else { wgpu::Features::empty() }, ..Default::default() }))?;
    let size = wgpu::Extent3d { width: 64, height: 64, depth_or_array_layers: 1 };
    let texture = device.create_texture(&wgpu::TextureDescriptor { label: Some("64x64 diagnostic"), size, mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2, format: wgpu::TextureFormat::Rgba8Unorm, usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::STORAGE_BINDING, view_formats: &[] });
    let view = texture.create_view(&Default::default());
    let queries = timestamps.then(|| device.create_query_set(&wgpu::QuerySetDescriptor { label: Some("probe timestamps"), ty: wgpu::QueryType::Timestamp, count: 2 }));
    let resolve = timestamps.then(|| device.create_buffer(&wgpu::BufferDescriptor { label: Some("probe query resolve"), size: 16, usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC, mapped_at_creation: false }));
    let query_readback = timestamps.then(|| device.create_buffer(&wgpu::BufferDescriptor { label: Some("probe query readback"), size: 16, usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ, mapped_at_creation: false }));
    let mut renderer = if use_vello { Some(vello::Renderer::new(&device, vello::RendererOptions { antialiasing_support: vello::AaSupport::area_only(), ..Default::default() })?) } else { None };
    let mut scene = vello::Scene::new();
    scene.fill(vello::peniko::Fill::NonZero, vello::kurbo::Affine::IDENTITY, vello::peniko::Color::from_rgba8(0, 255, 0, 255), None, &vello::kurbo::Rect::new(0.0, 0.0, 64.0, 64.0));
    for _ in 0..32 {
        let mut start_encoder = device.create_command_encoder(&Default::default());
        if let Some(queries) = &queries { start_encoder.write_timestamp(queries, 0); }
        queue.submit([start_encoder.finish()]);
        if let Some(renderer) = &mut renderer {
            renderer.render_to_texture(&device, &queue, &scene, &view, &vello::RenderParams { base_color: vello::peniko::Color::TRANSPARENT, width: 64, height: 64, antialiasing_method: vello::AaConfig::Area })?;
        } else {
        let mut encoder = device.create_command_encoder(&Default::default());
        { let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("minimal clear"), color_attachments: &[Some(wgpu::RenderPassColorAttachment { view: &view, depth_slice: None, resolve_target: None, ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.25, g: 0.5, b: 0.75, a: 1.0 }), store: wgpu::StoreOp::Store } })], depth_stencil_attachment: None, timestamp_writes: None, occlusion_query_set: None, multiview_mask: None,
        }); }
        queue.submit([encoder.finish()]);
        }
        let mut end_encoder = device.create_command_encoder(&Default::default());
        if let (Some(queries), Some(resolve), Some(readback)) = (&queries, &resolve, &query_readback) {
            end_encoder.write_timestamp(queries, 1);
            end_encoder.resolve_query_set(queries, 0..2, resolve, 0);
            end_encoder.copy_buffer_to_buffer(resolve, 0, readback, 0, 16);
        }
        queue.submit([end_encoder.finish()]);
    }
    let buffer = device.create_buffer(&wgpu::BufferDescriptor { label: Some("diagnostic readback"), size: 64*64*4, usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ, mapped_at_creation: false });
    let mut encoder = device.create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(wgpu::TexelCopyTextureInfo { texture: &texture, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All }, wgpu::TexelCopyBufferInfo { buffer: &buffer, layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(256), rows_per_image: Some(64) } }, size);
    let submitted = queue.submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    buffer.slice(..).map_async(wgpu::MapMode::Read, move |result| { let _ = tx.send(result); });
    device.poll(wgpu::PollType::Wait { submission_index: Some(submitted), timeout: Some(Duration::from_secs(5)) })?;
    rx.recv_timeout(Duration::from_secs(1))??;
    let bytes = buffer.slice(..).get_mapped_range();
    let expected = if use_vello { [0, 255, 0, 255] } else { [64, 128, 191, 255] };
    let matches = bytes.chunks_exact(4).all(|pixel| pixel == expected);
    println!("submitted_draw_passes=32 pixels={} first={:?} all_expected={}", bytes.len()/4, &bytes[..4], matches);
    drop(bytes);
    buffer.unmap();
    if !matches { return Err("GPU readback mismatch".into()); }
    if let Some(query_readback) = &query_readback {
        let (tx, rx) = std::sync::mpsc::channel();
        query_readback.slice(..).map_async(wgpu::MapMode::Read, move |result| { let _ = tx.send(result); });
        device.poll(wgpu::PollType::Wait { submission_index: None, timeout: Some(Duration::from_secs(5)) })?;
        rx.recv_timeout(Duration::from_secs(1))??;
        let data = query_readback.slice(..).get_mapped_range();
        let start = u64::from_le_bytes(data[0..8].try_into()?);
        let end = u64::from_le_bytes(data[8..16].try_into()?);
        println!("timestamp_start={start} timestamp_end={end} period={} nonzero_delta={}", queue.get_timestamp_period(), start != end);
        drop(data);
        query_readback.unmap();
    }
    println!("diagnostic=PASS; this is not a native presentation or performance baseline");
    Ok(())
}

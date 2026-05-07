//! Scene encoding for generic runtime paint plans.

use super::GenericSharedPixelBytes;
use crate::gui_runtime::native_vello::*;

#[derive(Clone, Debug, Default)]
pub(in crate::gui_runtime::native_vello) struct RetainedSurfaceFrameCache {
    entry: Option<RetainedSurfaceFrameCacheEntry>,
}

#[derive(Clone, Debug)]
struct RetainedSurfaceFrameCacheEntry {
    descriptor: RetainedSurfaceDescriptor,
    rect: UiRect,
    viewport: Vector2,
    frame: PaintFrame,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(in crate::gui_runtime::native_vello) struct RetainedSurfaceEncodeStats {
    pub bridge_calls: u32,
    pub cache_hits: u32,
    pub primitive_count: usize,
    pub text_run_count: usize,
}

impl RetainedSurfaceFrameCache {
    fn cached_frame(
        &self,
        descriptor: RetainedSurfaceDescriptor,
        rect: UiRect,
        viewport: Vector2,
    ) -> Option<&PaintFrame> {
        if descriptor.volatile || descriptor.dirty_mask != 0 {
            return None;
        }
        let entry = self.entry.as_ref()?;
        (entry.descriptor.key == descriptor.key
            && entry.descriptor.revision == descriptor.revision
            && entry.descriptor.dirty_mask == 0
            && !entry.descriptor.volatile
            && entry.rect == rect
            && entry.viewport == viewport)
            .then_some(&entry.frame)
    }

    fn store(
        &mut self,
        descriptor: RetainedSurfaceDescriptor,
        rect: UiRect,
        viewport: Vector2,
        frame: PaintFrame,
    ) {
        if descriptor.volatile || descriptor.dirty_mask != 0 {
            return;
        }
        self.entry = Some(RetainedSurfaceFrameCacheEntry {
            descriptor,
            rect,
            viewport,
            frame,
        });
    }
}

pub(in crate::gui_runtime::native_vello) fn encode_surface_paint_plan_to_scene<Bridge, Message>(
    plan: &crate::runtime::SurfacePaintPlan,
    scene: &mut Scene,
    text_renderer: &mut NativeTextRenderer,
    bridge: &mut Bridge,
    viewport: Vector2,
    retained_cache: &mut RetainedSurfaceFrameCache,
) -> RetainedSurfaceEncodeStats
where
    Bridge: RuntimeBridge<Message>,
{
    scene.reset();
    let mut stats = RetainedSurfaceEncodeStats::default();
    let mut text_runs = Vec::new();
    for primitive in &plan.primitives {
        match primitive {
            PaintPrimitive::ClipStart(clip) => {
                flush_text_runs(scene, text_renderer, &mut text_runs, &mut stats);
                scene.push_clip_layer(Fill::NonZero, Affine::IDENTITY, &to_kurbo_rect(clip.rect));
            }
            PaintPrimitive::ClipEnd(_) => {
                flush_text_runs(scene, text_renderer, &mut text_runs, &mut stats);
                scene.pop_layer();
            }
            PaintPrimitive::FillRect(fill) => encode_rect(scene, fill.color, fill.rect),
            PaintPrimitive::StrokeRect(stroke) => {
                scene.stroke(
                    &vello::kurbo::Stroke::new(stroke.width as f64),
                    Affine::IDENTITY,
                    color_from_rgba(stroke.color),
                    None,
                    &to_kurbo_rect(stroke.rect),
                );
            }
            PaintPrimitive::FillPolygon(fill) => {
                encode_polygon_fill(scene, fill.color, &fill.points);
            }
            PaintPrimitive::StrokePolygon(stroke) => {
                encode_polygon_stroke(scene, stroke.color, stroke.width, &stroke.points);
            }
            PaintPrimitive::StrokePolyline(stroke) => {
                encode_polyline_stroke(scene, stroke.color, stroke.width, &stroke.points);
            }
            PaintPrimitive::Text(text) => {
                let align = match text.align {
                    PaintTextAlign::Left => TextAlign::Left,
                    PaintTextAlign::Center => TextAlign::Center,
                    PaintTextAlign::Right => TextAlign::Right,
                };
                let baseline_offset = text.baseline.unwrap_or(text.font_size);
                text_runs.push(TextRun {
                    text: text.text.clone(),
                    position: Point::new(
                        text.rect.min.x,
                        text.rect.min.y + baseline_offset - text.font_size,
                    ),
                    font_size: text.font_size,
                    color: text.color,
                    max_width: Some(text.rect.width().max(0.0)),
                    align,
                });
            }
            PaintPrimitive::Image(draw) => {
                encode_image(
                    scene,
                    Arc::clone(&draw.image.pixels),
                    draw.image.width,
                    draw.image.height,
                    draw.rect,
                );
            }
            PaintPrimitive::CustomSurface(custom) => {
                if let Some(retained) = custom.retained {
                    if let Some(frame) =
                        retained_cache.cached_frame(retained, custom.rect, viewport)
                    {
                        stats.cache_hits = stats.cache_hits.saturating_add(1);
                        stats.primitive_count =
                            stats.primitive_count.saturating_add(frame.primitives.len());
                        stats.text_run_count =
                            stats.text_run_count.saturating_add(frame.text_runs.len());
                        encode_paint_frame_to_scene(frame, scene, text_renderer);
                        continue;
                    }
                    stats.bridge_calls = stats.bridge_calls.saturating_add(1);
                    if let Some(frame) =
                        bridge.render_retained_surface(retained, custom.rect, viewport)
                    {
                        stats.primitive_count =
                            stats.primitive_count.saturating_add(frame.primitives.len());
                        stats.text_run_count =
                            stats.text_run_count.saturating_add(frame.text_runs.len());
                        encode_paint_frame_to_scene(&frame, scene, text_renderer);
                        retained_cache.store(retained, custom.rect, viewport, frame);
                        continue;
                    }
                }
                scene.stroke(
                    &vello::kurbo::Stroke::new(1.0),
                    Affine::IDENTITY,
                    color_from_rgba(Rgba8 {
                        r: 96,
                        g: 96,
                        b: 96,
                        a: 255,
                    }),
                    None,
                    &to_kurbo_rect(custom.rect),
                );
            }
        }
    }
    flush_text_runs(scene, text_renderer, &mut text_runs, &mut stats);
    stats
}

fn flush_text_runs(
    scene: &mut Scene,
    text_renderer: &mut NativeTextRenderer,
    text_runs: &mut Vec<TextRun>,
    stats: &mut RetainedSurfaceEncodeStats,
) {
    if text_runs.is_empty() {
        return;
    }
    stats.text_run_count = stats.text_run_count.saturating_add(text_runs.len());
    text_renderer.draw_text_runs(scene, text_runs);
    text_runs.clear();
}

fn encode_paint_frame_to_scene(
    frame: &PaintFrame,
    scene: &mut Scene,
    text_renderer: &mut NativeTextRenderer,
) {
    for primitive in frame.primitives.iter() {
        match primitive {
            Primitive::Rect(fill) => encode_rect(scene, fill.color, fill.rect),
            Primitive::Circle(fill) => {
                scene.fill(
                    Fill::NonZero,
                    Affine::IDENTITY,
                    color_from_rgba(fill.color),
                    None,
                    &Circle::new(
                        (fill.center.x as f64, fill.center.y as f64),
                        fill.radius as f64,
                    ),
                );
            }
            Primitive::LinearGradient(fill) => {
                let mut gradient = Gradient::new_linear(
                    KurboPoint::new(fill.start.x as f64, fill.start.y as f64),
                    KurboPoint::new(fill.end.x as f64, fill.end.y as f64),
                );
                gradient
                    .stops
                    .push((0.0, color_from_rgba(fill.start_color)).into());
                gradient
                    .stops
                    .push((1.0, color_from_rgba(fill.end_color)).into());
                scene.fill(
                    Fill::NonZero,
                    Affine::IDENTITY,
                    &gradient,
                    None,
                    &to_kurbo_rect(fill.rect),
                );
            }
            Primitive::Image(draw) => {
                encode_image(
                    scene,
                    Arc::clone(&draw.image.pixels),
                    draw.image.width,
                    draw.image.height,
                    draw.rect,
                );
            }
        }
    }
    text_renderer.draw_text_runs(scene, &frame.text_runs);
}

fn encode_rect(scene: &mut Scene, color: Rgba8, rect: UiRect) {
    scene.fill(
        Fill::NonZero,
        Affine::IDENTITY,
        color_from_rgba(color),
        None,
        &to_kurbo_rect(rect),
    );
}

fn encode_polygon_fill(scene: &mut Scene, color: Rgba8, points: &[Point]) {
    if let Some(path) = polygon_path(points) {
        scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            color_from_rgba(color),
            None,
            &path,
        );
    }
}

fn encode_polygon_stroke(scene: &mut Scene, color: Rgba8, width: f32, points: &[Point]) {
    if let Some(path) = polygon_path(points) {
        scene.stroke(
            &vello::kurbo::Stroke::new(width as f64),
            Affine::IDENTITY,
            color_from_rgba(color),
            None,
            &path,
        );
    }
}

fn encode_polyline_stroke(scene: &mut Scene, color: Rgba8, width: f32, points: &[Point]) {
    if let Some(path) = polyline_path(points) {
        scene.stroke(
            &vello::kurbo::Stroke::new(width as f64),
            Affine::IDENTITY,
            color_from_rgba(color),
            None,
            &path,
        );
    }
}

fn polygon_path(points: &[Point]) -> Option<BezPath> {
    let first = points.first()?;
    let mut path = BezPath::new();
    path.move_to(KurboPoint::new(first.x as f64, first.y as f64));
    for point in &points[1..] {
        path.line_to(KurboPoint::new(point.x as f64, point.y as f64));
    }
    path.close_path();
    Some(path)
}

fn polyline_path(points: &[Point]) -> Option<BezPath> {
    let first = points.first()?;
    let mut path = BezPath::new();
    path.move_to(KurboPoint::new(first.x as f64, first.y as f64));
    for point in &points[1..] {
        path.line_to(KurboPoint::new(point.x as f64, point.y as f64));
    }
    Some(path)
}

fn encode_image(
    scene: &mut Scene,
    pixels: Arc<[u8]>,
    image_width: usize,
    image_height: usize,
    rect: UiRect,
) {
    let (Ok(width), Ok(height)) = (u32::try_from(image_width), u32::try_from(image_height)) else {
        return;
    };
    if width == 0 || height == 0 || rect.width() <= 0.0 || rect.height() <= 0.0 {
        return;
    }
    let image_data = ImageData {
        data: Blob::new(Arc::new(GenericSharedPixelBytes(pixels))),
        format: ImageFormat::Rgba8,
        alpha_type: ImageAlphaType::Alpha,
        width,
        height,
    };
    let transform = Affine::translate((rect.min.x as f64, rect.min.y as f64))
        * Affine::scale_non_uniform(
            rect.width() as f64 / f64::from(width),
            rect.height() as f64 / f64::from(height),
        );
    scene.draw_image(&image_data, transform);
}

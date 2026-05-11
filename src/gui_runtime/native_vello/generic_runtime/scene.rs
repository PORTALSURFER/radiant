//! Scene encoding for generic runtime paint plans.

use crate::gui_runtime::native_vello::*;
use crate::layout::Rect;

mod cache;
mod frame;
mod image;
mod shape;
mod text_input;
mod text_input_selection;
pub(in crate::gui_runtime::native_vello) use cache::{
    RetainedSurfaceEncodeStats, RetainedSurfaceFrameCache,
};
use frame::{encode_paint_frame_to_scene, flush_text_runs};
use image::encode_image;
use shape::{encode_polygon_fill, encode_polygon_stroke, encode_polyline_stroke, encode_rect};
use text_input::encode_text_input;

pub(in crate::gui_runtime::native_vello) fn encode_surface_paint_plan_to_scene<
    'plan,
    Bridge,
    Message,
>(
    plan: &'plan crate::runtime::SurfacePaintPlan,
    context: SurfaceSceneEncodeContext<'_, 'plan, Bridge>,
) -> RetainedSurfaceEncodeStats
where
    Bridge: RuntimeBridge<Message>,
{
    let SurfaceSceneEncodeContext {
        scene,
        text_renderer,
        bridge,
        viewport,
        retained_cache,
        text_runs,
        fast_pointer_move_gpu_surface_hit_rects,
        animation_time,
    } = context;
    scene.reset();
    text_runs.clear();
    fast_pointer_move_gpu_surface_hit_rects.clear();
    let mut stats = RetainedSurfaceEncodeStats {
        paint_plan_primitives: plan.primitives.len(),
        ..RetainedSurfaceEncodeStats::default()
    };
    for primitive in &plan.primitives {
        match primitive {
            PaintPrimitive::ClipStart(clip) => {
                stats.clip_layer_count = stats.clip_layer_count.saturating_add(1);
                flush_text_runs(scene, text_renderer, text_runs, &mut stats);
                scene.push_clip_layer(Fill::NonZero, Affine::IDENTITY, &to_kurbo_rect(clip.rect));
            }
            PaintPrimitive::ClipEnd(_) => {
                flush_text_runs(scene, text_renderer, text_runs, &mut stats);
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
                stats.text_primitive_count = stats.text_primitive_count.saturating_add(1);
                let align = match text.align {
                    PaintTextAlign::Left => TextAlign::Left,
                    PaintTextAlign::Center => TextAlign::Center,
                    PaintTextAlign::Right => TextAlign::Right,
                };
                let baseline_offset = text.baseline.unwrap_or(text.font_size);
                text_runs.push(SceneTextRun {
                    text: text.text.as_ref(),
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
            PaintPrimitive::OverlayPanel(panel) => {
                encode_rect(
                    scene,
                    Rgba8 {
                        r: 48,
                        g: 48,
                        b: 48,
                        a: 255,
                    },
                    panel.rect,
                );
            }
            PaintPrimitive::TextInput(input) => {
                stats.text_input_count = stats.text_input_count.saturating_add(1);
                flush_text_runs(scene, text_renderer, text_runs, &mut stats);
                encode_text_input(scene, text_renderer, input, animation_time);
                stats.record_text_runs(1);
            }
            PaintPrimitive::Image(draw) => {
                stats.image_count = stats.image_count.saturating_add(1);
                encode_image(
                    scene,
                    Arc::clone(&draw.image.pixels),
                    draw.image.width,
                    draw.image.height,
                    draw.source_rect,
                    draw.rect,
                );
            }
            PaintPrimitive::GpuSurface(surface) => {
                stats.gpu_surface_count = stats.gpu_surface_count.saturating_add(1);
                if surface.capabilities.fast_pointer_move {
                    fast_pointer_move_gpu_surface_hit_rects.push(surface.rect);
                }
            }
            PaintPrimitive::CustomSurface(custom) => {
                stats.custom_surface_count = stats.custom_surface_count.saturating_add(1);
                if let Some(retained) = custom.retained {
                    if let Some(frame) =
                        retained_cache.cached_frame(retained, custom.rect, viewport)
                    {
                        stats.cache_hits = stats.cache_hits.saturating_add(1);
                        stats.record_retained_frame(frame);
                        encode_paint_frame_to_scene(frame, scene, text_renderer);
                        continue;
                    }
                    stats.bridge_calls = stats.bridge_calls.saturating_add(1);
                    if let Some(frame) =
                        bridge.render_retained_surface(retained, custom.rect, viewport)
                    {
                        stats.record_retained_frame(&frame);
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
    flush_text_runs(scene, text_renderer, text_runs, &mut stats);
    stats
}

pub(in crate::gui_runtime::native_vello) struct SurfaceSceneEncodeContext<'a, 'plan, Bridge> {
    pub scene: &'a mut Scene,
    pub text_renderer: &'a mut NativeTextRenderer,
    pub bridge: &'a mut Bridge,
    pub viewport: Vector2,
    pub retained_cache: &'a mut RetainedSurfaceFrameCache,
    pub text_runs: &'a mut Vec<SceneTextRun<'plan>>,
    pub fast_pointer_move_gpu_surface_hit_rects: &'a mut Vec<Rect>,
    pub animation_time: Duration,
}

//! Scene encoding for generic runtime paint plans.

use super::runtime_helpers::GpuSurfaceInteractionRegion;
use crate::{
    gui::types::{Rgba8, Vector2},
    gui_runtime::native_vello::{NativeTextRenderer, to_kurbo_rect},
    runtime::{PaintPrimitive, RuntimeBridge},
};
use std::{sync::Arc, time::Duration};
use vello::{Scene, kurbo::Affine, peniko::Fill};

mod cache;
mod clip;
mod custom_surface;
mod frame;
mod image;
mod shape;
mod svg;
mod text;
mod text_input;
mod text_input_selection;
mod text_runs;
pub(in crate::gui_runtime::native_vello) use cache::{
    RetainedSurfaceEncodeStats, RetainedSurfaceFrameCache,
};
use clip::SceneClipState;
use custom_surface::encode_custom_surface;
use image::encode_image;
use shape::{
    encode_path_fill, encode_polygon_fill, encode_polygon_stroke, encode_polyline_stroke,
    encode_rect, encode_rect_stroke,
};
use svg::encode_svg;
use text::encode_text;
use text_input::encode_text_input;
pub(in crate::gui_runtime::native_vello) use text_runs::SceneTextRunBuffer;
use text_runs::flush_text_runs;

pub(in crate::gui_runtime::native_vello) fn encode_surface_paint_plan_to_scene<Bridge, Message>(
    plan: &crate::runtime::SurfacePaintPlan,
    context: SurfaceSceneEncodeContext<'_, Bridge>,
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
        gpu_surface_interaction_regions,
        animation_time,
    } = context;
    scene.reset();
    text_runs.clear();
    gpu_surface_interaction_regions.clear();
    let mut stats = RetainedSurfaceEncodeStats {
        paint_plan_primitives: plan.primitives.len(),
        ..RetainedSurfaceEncodeStats::default()
    };
    let mut clip_state = SceneClipState::default();
    for primitive in &plan.primitives {
        match primitive {
            PaintPrimitive::ClipStart(clip) => {
                flush_text_runs(scene, text_renderer, text_runs, &mut stats);
                if clip_state.begin(clip.rect).pushes_layer() {
                    stats.clip_layer_count = stats.clip_layer_count.saturating_add(1);
                    scene.push_clip_layer(
                        Fill::NonZero,
                        Affine::IDENTITY,
                        &to_kurbo_rect(clip.rect),
                    );
                }
                continue;
            }
            PaintPrimitive::ClipEnd(_) => {
                if clip_state.end().pops_layer() {
                    flush_text_runs(scene, text_renderer, text_runs, &mut stats);
                    scene.pop_layer();
                }
                continue;
            }
            _ if clip_state.is_suppressed() => continue,
            _ => {}
        }
        if flushes_pending_text_before_encoding(primitive) {
            flush_text_runs(scene, text_renderer, text_runs, &mut stats);
        }
        match primitive {
            PaintPrimitive::ClipStart(_) | PaintPrimitive::ClipEnd(_) => {}
            PaintPrimitive::FillRect(fill) => encode_rect(scene, fill.color, fill.rect),
            PaintPrimitive::FillPath(fill) => {
                encode_path_fill(
                    scene,
                    fill.color,
                    fill.transform,
                    fill.fill_rule,
                    &fill.path,
                );
            }
            PaintPrimitive::Svg(svg) => {
                stats.svg_document_count = stats.svg_document_count.saturating_add(1);
                encode_svg(scene, svg);
            }
            PaintPrimitive::StrokeRect(stroke) => {
                encode_rect_stroke(scene, stroke.color, stroke.width, stroke.rect);
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
                encode_text(text_runs, text);
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
                if let Some(region) = GpuSurfaceInteractionRegion::from_gpu_surface(surface) {
                    gpu_surface_interaction_regions.push(region);
                }
            }
            PaintPrimitive::CustomSurface(custom) => {
                encode_custom_surface(
                    scene,
                    text_renderer,
                    bridge,
                    viewport,
                    retained_cache,
                    custom,
                    &mut stats,
                );
            }
        }
    }
    flush_text_runs(scene, text_renderer, text_runs, &mut stats);
    stats
}

pub(super) fn flushes_pending_text_before_encoding(primitive: &PaintPrimitive) -> bool {
    !matches!(
        primitive,
        PaintPrimitive::Text(_) | PaintPrimitive::ClipStart(_) | PaintPrimitive::ClipEnd(_)
    )
}

pub(in crate::gui_runtime::native_vello) struct SurfaceSceneEncodeContext<'a, Bridge> {
    pub scene: &'a mut Scene,
    pub text_renderer: &'a mut NativeTextRenderer,
    pub bridge: &'a mut Bridge,
    pub viewport: Vector2,
    pub retained_cache: &'a mut RetainedSurfaceFrameCache,
    pub text_runs: &'a mut SceneTextRunBuffer,
    pub gpu_surface_interaction_regions: &'a mut Vec<GpuSurfaceInteractionRegion>,
    pub animation_time: Duration,
}

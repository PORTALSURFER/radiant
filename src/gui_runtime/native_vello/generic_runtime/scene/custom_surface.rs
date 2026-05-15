use super::{
    RetainedSurfaceEncodeStats, RetainedSurfaceFrameCache, frame::encode_paint_frame_to_scene,
    shape::encode_rect_stroke,
};
use crate::{gui_runtime::native_vello::*, runtime::PaintCustomSurface};

pub(in crate::gui_runtime::native_vello::generic_runtime::scene) fn encode_custom_surface<
    Bridge,
    Message,
>(
    scene: &mut Scene,
    text_renderer: &mut NativeTextRenderer,
    bridge: &mut Bridge,
    viewport: Vector2,
    retained_cache: &mut RetainedSurfaceFrameCache,
    custom: &PaintCustomSurface,
    stats: &mut RetainedSurfaceEncodeStats,
) where
    Bridge: RuntimeBridge<Message>,
{
    stats.custom_surface_count = stats.custom_surface_count.saturating_add(1);
    if encode_retained_custom_surface(
        scene,
        text_renderer,
        bridge,
        viewport,
        retained_cache,
        custom,
        stats,
    ) {
        return;
    }
    stats.custom_surface_fallback_count = stats.custom_surface_fallback_count.saturating_add(1);
    encode_custom_surface_fallback(scene, custom);
}

fn encode_retained_custom_surface<Bridge, Message>(
    scene: &mut Scene,
    text_renderer: &mut NativeTextRenderer,
    bridge: &mut Bridge,
    viewport: Vector2,
    retained_cache: &mut RetainedSurfaceFrameCache,
    custom: &PaintCustomSurface,
    stats: &mut RetainedSurfaceEncodeStats,
) -> bool
where
    Bridge: RuntimeBridge<Message>,
{
    let Some(retained) = custom.retained else {
        return false;
    };
    if let Some(frame) = retained_cache.cached_frame(retained, custom.rect, viewport) {
        stats.cache_hits = stats.cache_hits.saturating_add(1);
        stats.record_retained_frame(frame);
        encode_paint_frame_to_scene(frame, scene, text_renderer);
        return true;
    }

    stats.bridge_calls = stats.bridge_calls.saturating_add(1);
    let Some(frame) = bridge.render_retained_surface(retained, custom.rect, viewport) else {
        stats.retained_surface_miss_count = stats.retained_surface_miss_count.saturating_add(1);
        return false;
    };
    stats.record_retained_frame(&frame);
    encode_paint_frame_to_scene(&frame, scene, text_renderer);
    retained_cache.store(retained, custom.rect, viewport, frame);
    true
}

fn encode_custom_surface_fallback(scene: &mut Scene, custom: &PaintCustomSurface) {
    encode_rect_stroke(
        scene,
        Rgba8 {
            r: 96,
            g: 96,
            b: 96,
            a: 255,
        },
        1.0,
        custom.rect,
    );
}

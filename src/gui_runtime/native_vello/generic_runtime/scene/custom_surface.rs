use super::{
    RetainedSurfaceEncodeStats, RetainedSurfaceFrameCache, frame::encode_paint_frame_to_scene,
    shape::encode_rect_stroke,
};
use crate::{
    gui::types::{Rgba8, Vector2},
    gui_runtime::native_vello::NativeTextRenderer,
    runtime::{PaintCustomSurface, RuntimeBridge, RuntimeRetainedSurfaceCapability},
};
use vello::Scene;

pub(in crate::gui_runtime::native_vello::generic_runtime::scene) struct CustomSurfaceEncodeContext<
    'a,
    Bridge,
> {
    pub scene: &'a mut Scene,
    pub text_renderer: &'a mut NativeTextRenderer,
    pub bridge: &'a mut Bridge,
    pub retained_surface: Option<RuntimeRetainedSurfaceCapability<Bridge>>,
    pub viewport: Vector2,
    pub retained_cache: &'a mut RetainedSurfaceFrameCache,
    pub stats: &'a mut RetainedSurfaceEncodeStats,
}

pub(in crate::gui_runtime::native_vello::generic_runtime::scene) fn encode_custom_surface<
    Bridge,
    Message,
>(
    mut context: CustomSurfaceEncodeContext<'_, Bridge>,
    custom: &PaintCustomSurface,
) where
    Bridge: RuntimeBridge<Message>,
{
    context.stats.custom_surface_count = context.stats.custom_surface_count.saturating_add(1);
    if encode_retained_custom_surface(&mut context, custom) {
        return;
    }
    context.stats.custom_surface_fallback_count = context
        .stats
        .custom_surface_fallback_count
        .saturating_add(1);
    encode_custom_surface_fallback(context.scene, custom);
}

fn encode_retained_custom_surface<Bridge, Message>(
    context: &mut CustomSurfaceEncodeContext<'_, Bridge>,
    custom: &PaintCustomSurface,
) -> bool
where
    Bridge: RuntimeBridge<Message>,
{
    let Some(retained) = custom.retained else {
        return false;
    };
    if let Some(frame) =
        context
            .retained_cache
            .cached_frame(retained, custom.rect, context.viewport)
    {
        context.stats.cache_hits = context.stats.cache_hits.saturating_add(1);
        context.stats.record_retained_frame(frame);
        encode_paint_frame_to_scene(frame, context.scene, context.text_renderer);
        return true;
    }

    context.stats.bridge_calls = context.stats.bridge_calls.saturating_add(1);
    let Some(capability) = context.retained_surface else {
        return false;
    };
    let Some(frame) = capability.render(context.bridge, retained, custom.rect, context.viewport)
    else {
        context.stats.retained_surface_miss_count =
            context.stats.retained_surface_miss_count.saturating_add(1);
        return false;
    };
    context.stats.record_retained_frame(&frame);
    encode_paint_frame_to_scene(&frame, context.scene, context.text_renderer);
    context
        .retained_cache
        .store(retained, custom.rect, context.viewport, frame);
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

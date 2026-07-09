//! Per-frame model refresh and transient overlay preparation.

use super::{FrameWork, FrameWorkReason, GenericNativeVelloRunner, RenderFrameProfile};
use crate::runtime::RuntimeBridge;

impl<Bridge, Message> GenericNativeVelloRunner<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(super) fn refresh_deferred_surface_if_needed(&mut self, profile: &mut RenderFrameProfile) {
        if !self.timing.deferred_surface_refresh || self.timing.deferred_scene_rebuild {
            return;
        }

        let (_, elapsed) = profile.measure(|| self.core.refresh_surface());
        self.timing.deferred_surface_refresh = false;
        profile.refresh_surface = elapsed;

        let (_, elapsed) = profile.measure(|| {
            self.core.paint_plan_into(&mut self.frame.last_paint_plan);
        });
        profile.paint_plan = elapsed;

        self.frame.mark_scene_texture_dirty();
        self.frame.refresh_gpu_surface_interaction_regions();
        self.frame.refresh_post_gpu_overlay_cache();
        self.export_automation_targets();
        self.record_frame_work(FrameWork::RefreshSurface {
            reason: FrameWorkReason::DeferredSurfaceRefresh,
        });
        self.timing
            .startup_timing
            .mark_deferred_model_refresh_done();
    }

    pub(super) fn rebuild_deferred_scene_if_needed(&mut self, profile: &mut RenderFrameProfile) {
        if !self.timing.deferred_scene_rebuild {
            return;
        }

        let mut skipped_rebuild = false;
        let (_, elapsed) = profile.measure(|| {
            let requires_encode = self.timing.deferred_scene_rebuild_requires_encode;
            let refreshed_surface = self.timing.deferred_surface_refresh;
            if refreshed_surface {
                self.core.refresh_surface();
                self.timing.deferred_surface_refresh = false;
            }
            let viewport_relayout = self
                .apply_pending_viewport_resize_if_needed()
                .unwrap_or(false);
            if !requires_encode && !refreshed_surface && !viewport_relayout {
                self.timing.deferred_scene_rebuild = false;
                self.frame.mark_scene_texture_dirty();
                skipped_rebuild = true;
                return;
            }
            self.rebuild_scene_for_interactive_route_now();
        });
        if skipped_rebuild {
            return;
        }
        profile.deferred_scene_rebuild = elapsed;
    }

    pub(super) fn paint_transient_overlays(&mut self, profile: &mut RenderFrameProfile) {
        self.frame.transient_overlay_primitives.clear();
        let has_app_overlay = self.core.has_transient_overlay_painter();
        let has_runtime_overlay = self.core.has_runtime_overlay_paint();
        if !has_app_overlay && !has_runtime_overlay {
            profile.transient_overlay_primitives = 0;
            return;
        }
        let (_, elapsed) = profile.measure(|| {
            if has_app_overlay {
                self.core.paint_transient_overlay(
                    &self.frame.last_paint_plan,
                    &mut self.frame.transient_overlay_primitives,
                    self.timing.animation_origin.elapsed(),
                );
            }
            if has_runtime_overlay {
                self.core
                    .paint_runtime_overlay(&mut self.frame.transient_overlay_primitives);
            }
        });
        profile.transient_overlay_paint = elapsed;
        profile.transient_overlay_primitives = self.frame.transient_overlay_primitives.len();
    }
}

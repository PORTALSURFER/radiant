//! Per-frame model refresh and transient overlay preparation.

use super::{
    GenericNativeVelloRunner, RenderFrameProfile, collect_gpu_surface_interaction_regions,
};
use crate::runtime::RuntimeBridge;
use std::time::Instant;

impl<Bridge, Message> GenericNativeVelloRunner<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(super) fn refresh_deferred_surface_if_needed(&mut self, profile: &mut RenderFrameProfile) {
        if !self.timing.deferred_surface_refresh || self.timing.deferred_scene_rebuild {
            return;
        }

        let started = Instant::now();
        self.core.refresh_surface();
        self.timing.deferred_surface_refresh = false;
        profile.refresh_surface = started.elapsed();

        let started = Instant::now();
        self.core.paint_plan_into(&mut self.frame.last_paint_plan);
        profile.paint_plan = started.elapsed();

        self.frame.mark_scene_texture_dirty();
        collect_gpu_surface_interaction_regions(
            &self.frame.last_paint_plan.primitives,
            &mut self.frame.gpu_surface_interaction_regions,
        );
        self.timing
            .startup_timing
            .mark_deferred_model_refresh_done();
    }

    pub(super) fn rebuild_deferred_scene_if_needed(&mut self, profile: &mut RenderFrameProfile) {
        if !self.timing.deferred_scene_rebuild {
            return;
        }

        let started = Instant::now();
        if self.timing.deferred_surface_refresh {
            self.core.refresh_surface();
            self.timing.deferred_surface_refresh = false;
        }
        self.rebuild_scene_for_interactive_route_now();
        profile.deferred_scene_rebuild = started.elapsed();
    }

    pub(super) fn paint_transient_overlays(&mut self, profile: &mut RenderFrameProfile) {
        self.frame.transient_overlay_primitives.clear();
        let started = Instant::now();
        self.core.paint_transient_overlay(
            &self.frame.last_paint_plan,
            &mut self.frame.transient_overlay_primitives,
            self.timing.animation_origin.elapsed(),
        );
        self.core
            .paint_runtime_overlay(&mut self.frame.transient_overlay_primitives);
        profile.transient_overlay_paint = started.elapsed();
        profile.transient_overlay_primitives = self.frame.transient_overlay_primitives.len();
    }
}

//! Per-frame model refresh and transient overlay preparation.

use super::{
    FrameWork, FrameWorkReason, GenericNativeAdapterOwner, GenericNativeVelloRunner,
    NativeLifecycle, NativeRunnerTimingState, NativeRunnerWindowState,
    PreparedSurfaceRefreshNativeEvidence, RenderFrameProfile, admit_prepared_surface_refresh,
};
use crate::runtime::RuntimeBridge;

impl<Bridge, Message> GenericNativeVelloRunner<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(super) fn refresh_deferred_surface_if_needed(&mut self, profile: &mut RenderFrameProfile) {
        let native_evidence = self.prepared_surface_refresh_native_evidence();
        self.refresh_deferred_surface_if_needed_with_evidence(profile, native_evidence, None);
    }

    fn refresh_deferred_surface_if_needed_with_evidence(
        &mut self,
        profile: &mut RenderFrameProfile,
        native_evidence: PreparedSurfaceRefreshNativeEvidence,
        current_native_evidence: Option<PreparedSurfaceRefreshNativeEvidence>,
    ) {
        if !self.timing.deferred_surface_refresh || self.timing.deferred_scene_rebuild {
            return;
        }

        let scope = self
            .take_deferred_surface_refresh_scope()
            .unwrap_or(crate::runtime::RepaintScope::Surface);
        let mut used_prepared_refresh = false;
        let mut projection_admitted = false;
        let mut prepared_terminal_messages = None;
        let mut projection_completion_mismatch = false;
        let (_, elapsed) = profile.measure(|| {
            if let Some(ticket) =
                admit_prepared_surface_refresh(&mut self.frame_stage_owner, native_evidence)
            {
                projection_admitted = true;
                let adapter = self.adapter.as_ref();
                let window = &self.window;
                let timing = &self.timing;
                let lifecycle = self.native_lifecycle_snapshot();
                let owner = &self.frame_stage_owner;
                let core = &mut self.core;
                let plan = &mut self.frame.last_paint_plan;
                prepared_terminal_messages = core.try_prepared_surface_refresh(scope, plan, || {
                    let current_native_evidence = current_native_evidence.unwrap_or_else(|| {
                        Self::prepared_surface_refresh_native_evidence_from_parts(
                            adapter, window, timing, lifecycle,
                        )
                    });
                    ticket.is_current(owner, current_native_evidence)
                });
                used_prepared_refresh = prepared_terminal_messages.is_some();
                projection_completion_mismatch = !self
                    .frame_stage_owner
                    .complete_projection(ticket.into_stage_ticket());
            }
            // Projection admission is the no-replay boundary. A prepared
            // candidate can veto before publication, but a None result after
            // admission must not re-enter the combined bridge/projection path.
            if !projection_admitted && !projection_completion_mismatch {
                self.core.refresh_surface_with_scope(scope);
            }
        });
        profile.refresh_surface = elapsed;

        if let Some(terminal_messages) = prepared_terminal_messages {
            self.complete_prepared_surface_refresh(terminal_messages);
        }

        let paint_plan_decision = if used_prepared_refresh {
            super::PaintPlanCacheDecision::Rebuilt
        } else {
            let (decision, elapsed) =
                profile.measure(|| self.core.paint_plan_into(&mut self.frame.last_paint_plan));
            profile.paint_plan = elapsed;
            decision
        };
        self.publish_native_ime_cursor_area();

        if !used_prepared_refresh {
            self.frame.mark_scene_texture_dirty();
            if matches!(paint_plan_decision, super::PaintPlanCacheDecision::Rebuilt) {
                self.frame.refresh_gpu_surface_interaction_regions();
                self.frame.refresh_post_gpu_overlay_cache();
            }
            self.export_automation_targets();
        }
        self.record_frame_work(FrameWork::RefreshSurface {
            reason: FrameWorkReason::DeferredSurfaceRefresh,
        });
        self.timing
            .startup_timing
            .mark_deferred_model_refresh_done();
    }

    #[cfg(test)]
    pub(super) fn refresh_deferred_surface_if_needed_for_test(
        &mut self,
        profile: &mut RenderFrameProfile,
        native_evidence: PreparedSurfaceRefreshNativeEvidence,
    ) {
        self.refresh_deferred_surface_if_needed_with_evidence(
            profile,
            native_evidence,
            Some(native_evidence),
        );
    }

    fn prepared_surface_refresh_native_evidence(&self) -> PreparedSurfaceRefreshNativeEvidence {
        Self::prepared_surface_refresh_native_evidence_from_parts(
            self.adapter.as_ref(),
            &self.window,
            &self.timing,
            self.native_lifecycle_snapshot(),
        )
    }

    fn prepared_surface_refresh_native_evidence_from_parts(
        adapter: Option<&GenericNativeAdapterOwner>,
        window: &NativeRunnerWindowState,
        timing: &NativeRunnerTimingState,
        lifecycle: NativeLifecycle,
    ) -> PreparedSurfaceRefreshNativeEvidence {
        PreparedSurfaceRefreshNativeEvidence {
            window_id: window.id,
            adapter_generation: adapter.and_then(|adapter| adapter.capture_generation()),
            target_generation: window.target_generation,
            environment: window.environment,
            native_resources_present: window.native_resources.is_some(),
            target_fenced: window.native_surface_target_fenced,
            pending_viewport_resize: timing.pending_viewport_resize.is_some(),
            pending_surface_resize: timing.pending_surface_resize.is_some(),
            lifecycle,
            newer_visual_request: timing.deferred_scene_rebuild,
        }
    }

    pub(super) fn rebuild_deferred_scene_if_needed(
        &mut self,
        profile: &mut RenderFrameProfile,
    ) -> bool {
        if !self.timing.deferred_scene_rebuild {
            return false;
        }

        let mut skipped_rebuild = false;
        let (_, elapsed) = profile.measure(|| {
            let requires_encode = self.timing.deferred_scene_rebuild_requires_encode;
            let refresh_scope = self.take_deferred_surface_refresh_scope();
            let refreshed_surface = refresh_scope.is_some();
            if let Some(scope) = refresh_scope {
                self.core.refresh_surface_with_scope(scope);
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
            if refreshed_surface {
                self.rebuild_scene_for_interactive_route_now_after_surface_refresh();
            } else {
                self.rebuild_scene_for_interactive_route_now();
            }
        });
        if skipped_rebuild {
            return false;
        }
        profile.deferred_scene_rebuild = elapsed;
        true
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

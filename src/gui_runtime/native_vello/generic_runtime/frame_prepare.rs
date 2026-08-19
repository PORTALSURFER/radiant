//! Per-frame model refresh and transient overlay preparation.

use super::prepared_surface_refresh::{
    admit_prepared_surface_refresh_layout, admit_prepared_surface_refresh_paint_plan,
};
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
        let (_, elapsed) = profile.measure(|| {
            if self.prepared_surface_refresh_is_eligible()
                && let Some(ticket) =
                    admit_prepared_surface_refresh(&mut self.frame_stage_owner, native_evidence)
            {
                projection_admitted = true;
                self.core
                    .record_test_prepared_surface_refresh_phase("projection-admitted");
                let mut projection_ticket = Some(ticket);
                let mut layout_ticket = None;
                let mut paint_plan_ticket = None;
                let adapter = self.adapter.as_ref();
                let window = &self.window;
                let timing = &self.timing;
                let lifecycle = self.native_lifecycle_snapshot();
                let owner = &mut self.frame_stage_owner;
                let core = &mut self.core;
                let plan = &mut self.frame.last_paint_plan;
                let mut prepared = core.prepare_prepared_surface_refresh(scope);
                if prepared.is_some() {
                    core.record_test_prepared_surface_refresh_phase("candidate-held");
                }
                let current_native_evidence = current_native_evidence.unwrap_or_else(|| {
                    Self::prepared_surface_refresh_native_evidence_from_parts(
                        adapter, window, timing, lifecycle,
                    )
                });
                if projection_ticket
                    .as_ref()
                    .is_some_and(|ticket| ticket.is_current(owner, current_native_evidence))
                    && let Some(ticket) = projection_ticket.take()
                    && owner.complete_projection(ticket.into_stage_ticket())
                {
                    core.record_test_prepared_surface_refresh_phase("projection-complete");
                    if prepared.is_some()
                        && let Some(ticket) =
                            admit_prepared_surface_refresh_layout(owner, current_native_evidence)
                    {
                        let current = ticket.is_current(owner, current_native_evidence);
                        layout_ticket = Some(ticket);
                        core.record_test_prepared_surface_refresh_phase("layout-admitted");
                        if current
                            && let Some(ticket) = layout_ticket.take()
                            && owner.complete_layout(ticket.into_stage_ticket())
                        {
                            core.record_test_prepared_surface_refresh_phase("layout-complete");
                            if let Some(ticket) = admit_prepared_surface_refresh_paint_plan(
                                owner,
                                current_native_evidence,
                            ) {
                                let current = ticket.is_current(owner, current_native_evidence);
                                paint_plan_ticket = Some(ticket);
                                core.record_test_prepared_surface_refresh_phase(
                                    "paint-plan-admitted",
                                );
                                if current && let Some(prepared) = prepared.take() {
                                    prepared_terminal_messages =
                                        core.publish_prepared_surface_refresh(plan, prepared);
                                    core.record_test_prepared_surface_refresh_phase("published");
                                }
                            }
                        }
                    }
                }
                if let Some(prepared) = prepared.take() {
                    core.discard_prepared_surface_refresh(prepared);
                }
                used_prepared_refresh = prepared_terminal_messages.is_some();
                if let Some(ticket) = projection_ticket.take() {
                    owner.complete_projection(ticket.into_stage_ticket());
                }
                if let Some(ticket) = layout_ticket.take() {
                    owner.complete_layout(ticket.into_stage_ticket());
                }
                if let Some(ticket) = paint_plan_ticket.take() {
                    owner.complete_paint_plan(ticket.into_stage_ticket());
                    core.record_test_prepared_surface_refresh_phase("paint-plan-complete");
                }
            }
            // Projection admission is the no-replay boundary. A prepared
            // candidate can veto before publication, but a None result after
            // admission must not re-enter the combined bridge/projection path.
            if !projection_admitted {
                self.core.refresh_surface_with_scope(scope);
            }
        });
        profile.refresh_surface = elapsed;

        if let Some(terminal_messages) = prepared_terminal_messages {
            self.complete_prepared_surface_refresh(terminal_messages);
        }

        // Projection admission is the no-replay boundary. A later candidate,
        // Layout, PaintPlan, or currentness veto has already discarded its
        // inert candidate and cleaned every exact ticket; it must not fall
        // through to active paint, IME, scene, automation, or frame-work
        // publication. Only a pre-Projection veto may use the combined path.
        if projection_admitted && !used_prepared_refresh {
            return;
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

    /// Keep unsupported virtualized runtime state on the combined refresh path.
    /// This read must happen before Projection admission because admission is
    /// the no-replay boundary for a prepared candidate.
    fn prepared_surface_refresh_is_eligible(&self) -> bool {
        self.core.runtime.prepared_surface_refresh_is_eligible()
    }

    #[cfg(test)]
    pub(super) fn refresh_deferred_surface_if_needed_for_test(
        &mut self,
        profile: &mut RenderFrameProfile,
        native_evidence: PreparedSurfaceRefreshNativeEvidence,
    ) {
        self.refresh_deferred_surface_if_needed_for_test_with_current_evidence(
            profile,
            native_evidence,
            native_evidence,
        );
    }

    #[cfg(test)]
    pub(super) fn refresh_deferred_surface_if_needed_for_test_with_current_evidence(
        &mut self,
        profile: &mut RenderFrameProfile,
        native_evidence: PreparedSurfaceRefreshNativeEvidence,
        current_native_evidence: PreparedSurfaceRefreshNativeEvidence,
    ) {
        self.refresh_deferred_surface_if_needed_with_evidence(
            profile,
            native_evidence,
            Some(current_native_evidence),
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

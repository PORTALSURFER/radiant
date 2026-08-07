//! Runtime state and event routing for the generic native Vello runner.

use super::{FrameWorkReason, GenericRouteOutcome};
use crate::gui::types::{Point, Vector2};
use crate::runtime::BasePaintPlanContext;
use crate::runtime::{
    CommandOutcome, DevtoolsOverlayOptions, RuntimeAnimationActivity, RuntimeBridge, SurfaceRuntime,
};
use crate::theme::{AppearancePolicy, ResolvedAppearance};
use crate::widgets::{PointerButton, WidgetKey};
use std::time::Instant;

pub(in crate::gui_runtime::native_vello) struct GenericNativeRuntimeCore<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(in crate::gui_runtime::native_vello) runtime: SurfaceRuntime<Bridge, Message>,
    pub(in crate::gui_runtime::native_vello) last_pointer_press: Option<PointerPressStamp>,
    appearance_policy: AppearancePolicy,
    resolved_appearance: ResolvedAppearance,
    base_paint_plan_context: Option<(BasePaintPlanContext, ResolvedAppearance)>,
    paint_segment_observer: crate::runtime::PaintSegmentObserver,
}

/// Result of one backend-neutral base paint-plan preparation pass.
///
/// The native runner uses this private decision to distinguish an exact plan
/// cache hit from a newly materialized plan. Runtime projection, traversal,
/// interaction, and state work still run for both cases.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::gui_runtime::native_vello) enum PaintPlanCacheDecision {
    Rebuilt,
    Reused,
}

#[derive(Clone, Copy, Debug)]
pub(in crate::gui_runtime::native_vello) struct PointerPressStamp {
    pub(in crate::gui_runtime::native_vello) at: Instant,
    pub(in crate::gui_runtime::native_vello) position: Point,
    pub(in crate::gui_runtime::native_vello) button: PointerButton,
}

impl<Bridge, Message> GenericNativeRuntimeCore<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(super) fn timed_repaint_deadline(&self) -> Option<Instant> {
        self.runtime.timed_repaint_deadline()
    }

    pub(super) fn advance_timed_repaints(&mut self, now: Instant) -> bool {
        self.runtime.advance_timed_repaints(now)
    }

    pub(super) fn begin_native_recovery(&mut self) -> bool {
        self.runtime.begin_native_recovery()
    }

    pub(super) fn finish_native_recovery(&mut self) -> bool {
        self.runtime.finish_native_recovery()
    }

    #[cfg(test)]
    pub(in crate::gui_runtime::native_vello) fn new(bridge: Bridge, viewport: Vector2) -> Self {
        Self::new_with_debug_layout(bridge, viewport, false)
    }

    #[cfg(test)]
    pub(in crate::gui_runtime::native_vello) fn new_with_debug_layout(
        bridge: Bridge,
        viewport: Vector2,
        debug_layout: bool,
    ) -> Self {
        Self::new_with_frame_options(
            bridge,
            viewport,
            debug_layout,
            DevtoolsOverlayOptions::default(),
        )
    }

    pub(in crate::gui_runtime::native_vello) fn new_with_frame_options(
        bridge: Bridge,
        viewport: Vector2,
        debug_layout: bool,
        devtools_overlay: DevtoolsOverlayOptions,
    ) -> Self {
        let mut runtime = SurfaceRuntime::new(bridge, viewport);
        if debug_layout {
            runtime.set_layout_debug_options(crate::layout::LayoutDebugOptions::bounds_only());
        }
        runtime.set_devtools_overlay_options(devtools_overlay);
        Self {
            runtime,
            last_pointer_press: None,
            appearance_policy: AppearancePolicy::FollowEnvironment,
            resolved_appearance: ResolvedAppearance::fixed(crate::theme::ThemeTokens::dark()),
            base_paint_plan_context: None,
            paint_segment_observer: crate::runtime::PaintSegmentObserver::new(),
        }
    }

    pub(super) fn set_viewport(&mut self, viewport: Vector2) -> bool {
        self.runtime.set_viewport_and_report_relayout(viewport)
    }

    pub(super) fn set_current_pointer_position(&mut self, position: Option<Point>) {
        self.runtime.set_current_pointer_position(position);
    }

    #[cfg(test)]
    pub(super) fn paint_plan(&self) -> crate::runtime::SurfacePaintPlan {
        self.runtime.paint_plan_with_policy(self.appearance_policy)
    }

    pub(super) fn paint_plan_into(
        &mut self,
        plan: &mut crate::runtime::SurfacePaintPlan,
    ) -> PaintPlanCacheDecision {
        let environment = self.runtime.context().resolved_environment();
        let appearance = self.appearance_policy.resolve(environment);
        let context = self.runtime.base_paint_plan_context();
        if self.runtime.base_paint_plan_reuse_eligible()
            && self
                .base_paint_plan_context
                .is_some_and(|(cached, cached_appearance)| {
                    cached == context && cached_appearance == appearance
                })
        {
            self.resolved_appearance = appearance;
            let observation = self.paint_segment_observer.observe(
                plan,
                &self.runtime.view_delta_diagnostics(),
                true,
            );
            self.runtime.record_paint_segment_observation(observation);
            return PaintPlanCacheDecision::Reused;
        }
        // The caller owns the mutable frame preparation boundary; cache the
        // pass snapshot so runtime overlays cannot drift from the base scene.
        self.resolved_appearance = appearance;
        let theme = appearance.tokens();
        self.runtime
            .base_paint_plan_with_appearance_into(&theme, appearance, environment, plan);
        self.base_paint_plan_context = Some((context, appearance));
        self.runtime.record_base_paint_plan_rebuild();
        let observation = self.paint_segment_observer.observe(
            plan,
            &self.runtime.view_delta_diagnostics(),
            false,
        );
        self.runtime.record_paint_segment_observation(observation);
        PaintPlanCacheDecision::Rebuilt
    }

    pub(super) fn base_paint_plan_context(&self) -> crate::runtime::BasePaintPlanContext {
        self.runtime.base_paint_plan_context()
    }

    pub(super) fn resolved_appearance(&self) -> ResolvedAppearance {
        self.resolved_appearance
    }

    /// Return the latest computed backend-neutral segment observation without
    /// draining the frame diagnostics transport.
    pub(super) fn paint_segment_observation(&self) -> crate::runtime::PaintSegmentObservation {
        self.runtime.latest_paint_segment_observation()
    }

    pub(super) fn paint_transient_overlay(
        &mut self,
        plan: &crate::runtime::SurfacePaintPlan,
        primitives: &mut Vec<crate::runtime::PaintPrimitive>,
        animation_time: std::time::Duration,
    ) {
        let viewport = self.runtime.viewport();
        self.runtime.host_paint_transient_overlay(
            crate::runtime::TransientOverlayContext::new(plan, viewport, animation_time),
            primitives,
        );
    }

    pub(super) fn has_transient_overlay_painter(&self) -> bool {
        self.runtime.has_transient_overlay_host()
    }

    pub(super) fn paint_runtime_overlay(
        &self,
        primitives: &mut Vec<crate::runtime::PaintPrimitive>,
    ) {
        let appearance = self.resolved_appearance;
        let theme = appearance.tokens();
        let environment = self.runtime.context().resolved_environment();
        self.runtime.runtime_overlay_paint_with_appearance_into(
            &theme,
            appearance,
            environment,
            primitives,
        );
    }

    pub(super) fn has_runtime_overlay_paint(&self) -> bool {
        self.runtime.has_runtime_overlay_paint()
    }

    pub(super) fn has_frame_diagnostics_observer(&self) -> bool {
        self.runtime.has_frame_diagnostics_host()
    }

    pub(super) fn has_frame_profile_observer(&self) -> bool {
        self.runtime.has_frame_profile_host()
    }

    pub(super) fn refresh_surface(&mut self) {
        self.runtime.refresh();
    }

    pub(super) fn refresh_surface_with_scope(&mut self, scope: crate::runtime::RepaintScope) {
        self.runtime.refresh_with_scope(scope);
    }

    pub(super) fn animation_activity(&mut self) -> RuntimeAnimationActivity {
        self.runtime.host_animation_activity()
    }

    pub(super) fn queue_animation_frame(&mut self) -> bool {
        self.runtime.host_queue_animation_frame()
    }

    pub(super) fn drain_timed_frame(
        &mut self,
        animation_activity: RuntimeAnimationActivity,
        needs_text_caret_animation: bool,
    ) -> GenericRouteOutcome {
        if animation_activity.needs_frame_message() {
            self.queue_animation_frame();
        }
        let mut outcome = self.drain_runtime_messages();
        if !outcome.needs_redraw() && needs_text_caret_animation {
            outcome.request_scene_rebuild(FrameWorkReason::TextCaretAnimation);
        } else if !outcome.needs_redraw() && animation_activity.needs_animation() {
            outcome.request_paint_only(FrameWorkReason::TimedPaintOnlyAnimation);
        }
        outcome
    }

    pub(in crate::gui_runtime::native_vello) fn drain_runtime_messages(
        &mut self,
    ) -> GenericRouteOutcome {
        let outcome = self.runtime.drain_runtime_messages();
        self.route_command_outcome(outcome)
    }

    pub(in crate::gui_runtime::native_vello) fn route_command_outcome(
        &mut self,
        outcome: CommandOutcome,
    ) -> GenericRouteOutcome {
        let _ = self.runtime.take_repaint_requested();
        let mut route_outcome = GenericRouteOutcome {
            routed: outcome.messages_dispatched > 0,
            exit_requested: outcome.exit_requested,
            runtime_work_remaining: outcome.runtime_work_remaining,
            dpi_scale_override: outcome.dpi_scale_override,
            window_logical_size: outcome.window_logical_size,
            ..GenericRouteOutcome::default()
        };
        if outcome.surface_refresh_requested {
            if outcome.surface_refresh_applied {
                route_outcome.request_scene_rebuild(FrameWorkReason::RuntimeSurfaceRepaint);
            } else {
                route_outcome.request_scene_rebuild(FrameWorkReason::RuntimeSurfaceRefresh);
            }
        }
        if outcome.surface_repaint_requested {
            route_outcome.request_scene_rebuild(FrameWorkReason::RuntimeSurfaceRepaint);
        }
        if outcome.paint_only_requested {
            route_outcome.request_paint_only(FrameWorkReason::RuntimePaintOnly);
        }
        if outcome.window_logical_size.is_some() {
            route_outcome.request_resize_and_rebuild(FrameWorkReason::CommandResize);
        }
        if outcome.exit_requested {
            route_outcome.request_exit();
        }
        route_outcome
    }

    pub(in crate::gui_runtime::native_vello) fn focused_text_selection(&self) -> Option<String> {
        self.runtime.focused_text_selection()
    }

    pub(in crate::gui_runtime::native_vello) fn has_focused_text_input(&self) -> bool {
        self.runtime.focused_text_input_id().is_some()
    }

    pub(in crate::gui_runtime::native_vello) fn focused_widget_preempts_host_shortcut_key(
        &self,
        key: WidgetKey,
    ) -> bool {
        self.runtime.focused_widget_preempts_host_shortcut_key(key)
    }
}

//! Revision-backed surface refresh stages and diagnostics.

use super::SurfaceRuntime;
use crate::runtime::{RepaintScope, RuntimeBridge, SurfaceInvalidation};
use std::time::{Duration, Instant};

/// Cumulative counts for independently measurable refresh stages.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SurfaceRefreshCounters {
    /// Host application surface projections pulled by the runtime.
    pub application_projection: u64,
    /// Runtime projection/traversal rebuilds.
    pub runtime_projection: u64,
    /// Widget-state synchronization passes.
    pub widget_state_sync: u64,
    /// Layout passes.
    pub layout: u64,
}

impl SurfaceRefreshCounters {
    pub(in crate::runtime) const fn startup() -> Self {
        Self {
            application_projection: 1,
            runtime_projection: 1,
            widget_state_sync: 0,
            layout: 1,
        }
    }
}

/// Independent CPU timing buckets for one surface refresh.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SurfaceRefreshTimings {
    /// Time spent pulling the host application projection.
    pub application_projection: Duration,
    /// Time spent rebuilding runtime projection and traversal.
    pub runtime_projection: Duration,
    /// Time spent synchronizing widget state.
    pub widget_state_sync: Duration,
    /// Time spent recomputing layout.
    pub layout: Duration,
}

impl SurfaceRefreshTimings {
    /// Return the sum of the independently measured refresh stages.
    pub fn total(self) -> Duration {
        self.application_projection
            .saturating_add(self.runtime_projection)
            .saturating_add(self.widget_state_sync)
            .saturating_add(self.layout)
    }

    fn merge(&mut self, other: Self) {
        self.application_projection = self
            .application_projection
            .saturating_add(other.application_projection);
        self.runtime_projection = self
            .runtime_projection
            .saturating_add(other.runtime_projection);
        self.widget_state_sync = self
            .widget_state_sync
            .saturating_add(other.widget_state_sync);
        self.layout = self.layout.saturating_add(other.layout);
    }
}

/// Diagnostics for the most recent typed surface invalidation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SurfaceRefreshDiagnostics {
    /// Chosen invalidation stage.
    pub invalidation: SurfaceInvalidation,
    /// Independent timing buckets for work performed by that stage.
    pub timings: SurfaceRefreshTimings,
}

impl SurfaceRefreshDiagnostics {
    pub(in crate::runtime) const fn startup() -> Self {
        Self {
            invalidation: SurfaceInvalidation::Surface,
            timings: SurfaceRefreshTimings {
                application_projection: Duration::ZERO,
                runtime_projection: Duration::ZERO,
                widget_state_sync: Duration::ZERO,
                layout: Duration::ZERO,
            },
        }
    }

    fn merge(&mut self, other: Self) {
        self.invalidation = SurfaceInvalidation::from_repaint_scope(
            match (
                self.invalidation.repaint_scope(),
                other.invalidation.repaint_scope(),
            ) {
                (Some(current), Some(next)) => Some(current.merge(next)),
                (Some(scope), None) | (None, Some(scope)) => Some(scope),
                (None, None) => None,
            },
        );
        self.timings.merge(other.timings);
    }
}

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    /// Reproject the latest host state using the correctness-first full refresh path.
    pub fn refresh(&mut self) {
        self.refresh_with_scope(RepaintScope::Surface);
    }

    /// Apply one typed repaint scope to the current projected surface.
    ///
    /// `Projection` may reuse layout only because the caller supplied an explicit
    /// unchanged structural/layout revision. Startup, resize, identity changes,
    /// and unknown custom-host changes should use `Surface`.
    pub fn refresh_with_scope(&mut self, scope: RepaintScope) {
        let refresh_started = Instant::now();
        let invalidation = SurfaceInvalidation::from_repaint_scope(Some(scope));
        if scope.is_paint_only() {
            self.record_refresh_diagnostics(
                SurfaceRefreshDiagnostics {
                    invalidation,
                    timings: SurfaceRefreshTimings::default(),
                },
                Duration::ZERO,
            );
            return;
        }

        let application_projection_started = Instant::now();
        let mut next_surface = self.bridge.pull_surface();
        let application_projection = application_projection_started.elapsed();
        self.refresh_counters.application_projection = self
            .refresh_counters
            .application_projection
            .saturating_add(1);

        std::mem::swap(
            &mut self.traversal.widgets.paths.previous,
            &mut self.traversal.widgets.paths.current,
        );
        let mut traversal = self.take_reusable_traversal_index(true);
        let runtime_projection_started = Instant::now();
        let layout_root = next_surface.runtime_projection_reusing_with_scratch(
            &mut traversal,
            &mut self.scratch.projection_scroll_stack,
            &mut self.scratch.projection_child_path,
        );
        let runtime_projection = runtime_projection_started.elapsed();
        self.refresh_counters.runtime_projection =
            self.refresh_counters.runtime_projection.saturating_add(1);

        let widget_state_sync_started = Instant::now();
        let sync_policy = self.widget_state_sync_policy();
        next_surface.synchronize_widget_state_from_paths(
            &self.surface,
            &traversal.stateful_widget_order,
            &traversal.widget_paths,
            &self.traversal.widgets.paths.previous,
            sync_policy,
        );
        let widget_state_sync = widget_state_sync_started.elapsed();
        self.refresh_counters.widget_state_sync =
            self.refresh_counters.widget_state_sync.saturating_add(1);

        self.surface = next_surface;
        self.layout_root = layout_root;
        self.restore_pointer_capture_state();
        let layout = if scope.refreshes_layout() {
            let layout_started = Instant::now();
            self.relayout_with_traversal(traversal);
            self.refresh_counters.layout = self.refresh_counters.layout.saturating_add(1);
            layout_started.elapsed()
        } else {
            self.install_traversal_index(traversal);
            Duration::ZERO
        };
        self.clear_stale_interaction_state();
        if let Some(widget_id) = self.interaction.focus.focused_widget {
            self.restore_focused_widget_state(widget_id);
        }

        self.record_refresh_diagnostics(
            SurfaceRefreshDiagnostics {
                invalidation,
                timings: SurfaceRefreshTimings {
                    application_projection,
                    runtime_projection,
                    widget_state_sync,
                    layout,
                },
            },
            refresh_started.elapsed(),
        );
    }

    /// Return diagnostics for the most recent typed invalidation stage.
    pub const fn last_refresh_diagnostics(&self) -> SurfaceRefreshDiagnostics {
        self.last_refresh_diagnostics
    }

    fn record_refresh_diagnostics(
        &mut self,
        diagnostics: SurfaceRefreshDiagnostics,
        total: Duration,
    ) {
        self.last_refresh_diagnostics = diagnostics;
        self.pending_frame_refresh_diagnostics.merge(diagnostics);
        self.pending_frame_refresh_total = self.pending_frame_refresh_total.saturating_add(total);
    }

    pub(crate) fn take_frame_refresh_diagnostics(
        &mut self,
    ) -> (SurfaceRefreshDiagnostics, Duration) {
        (
            std::mem::take(&mut self.pending_frame_refresh_diagnostics),
            std::mem::take(&mut self.pending_frame_refresh_total),
        )
    }

    /// Return cumulative refresh-stage counts for this runtime.
    pub const fn refresh_counters(&self) -> SurfaceRefreshCounters {
        self.refresh_counters
    }
}

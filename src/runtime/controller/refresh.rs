//! Revision-backed surface refresh stages and diagnostics.

use super::SurfaceRuntime;
use crate::runtime::{RepaintScope, RuntimeBridge, SurfaceInvalidation};
use crate::widgets::WidgetId;
use std::time::{Duration, Instant};

const MAX_IDENTITY_REPLACEMENTS_PER_REFRESH: usize = 4;
const MAX_IDENTITY_PATH_COMPONENTS: usize = 8;

/// A bounded, resolved widget path retained in an identity diagnostic.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SurfaceIdentityPath {
    /// Path components from the projected surface root.
    pub components: [usize; MAX_IDENTITY_PATH_COMPONENTS],
    /// Number of valid components in [`Self::components`].
    pub len: u8,
    /// Whether the resolved path exceeded the diagnostic bound.
    pub truncated: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        gui::types::Vector2,
        runtime::{RuntimeBridge, SurfaceNode, UiSurface, WidgetMessageMapper},
        widgets::{ButtonWidget, ScrollbarAxis, ScrollbarWidget, WidgetSizing},
    };
    use std::sync::Arc;

    #[derive(Default)]
    struct ReplacementBridge {
        replace: bool,
    }

    impl RuntimeBridge<()> for ReplacementBridge {
        fn project_surface(&mut self) -> Arc<UiSurface<()>> {
            let node = if self.replace {
                SurfaceNode::widget(
                    ScrollbarWidget::new(
                        20,
                        ScrollbarAxis::Vertical,
                        WidgetSizing::fixed(Vector2::new(16.0, 80.0)),
                    ),
                    WidgetMessageMapper::none(),
                )
            } else {
                SurfaceNode::widget(
                    ButtonWidget::new(
                        20,
                        "Previous",
                        WidgetSizing::fixed(Vector2::new(80.0, 28.0)),
                    ),
                    WidgetMessageMapper::none(),
                )
            };
            crate::runtime::test_arc_surface(UiSurface::new(node))
        }
    }

    #[test]
    fn incompatible_replacement_discards_controller_ownership_and_reports_identity() {
        let mut runtime =
            SurfaceRuntime::new(ReplacementBridge::default(), Vector2::new(120.0, 80.0));
        runtime.interaction.focus.focused_widget = Some(20);
        runtime.interaction.pointer.capture = Some(20);
        runtime.interaction.pointer.capture_state = Some((20, Default::default()));
        runtime.interaction.hover.widget = Some(20);
        runtime.bridge_mut().replace = true;

        runtime.refresh();

        assert_eq!(runtime.focused_widget(), None);
        assert_eq!(runtime.pointer_capture(), None);
        assert_eq!(runtime.hovered_widget(), None);
        assert_eq!(runtime.interaction.pointer.capture_state, None);
        let diagnostics = runtime.last_refresh_diagnostics().identity;
        assert_eq!(diagnostics.replacement_count, 1);
        let replacement = diagnostics.replacements[0].expect("replacement diagnostic");
        assert_eq!(replacement.widget_id, 20);
        assert_ne!(replacement.previous_kind, replacement.current_kind);
        assert_eq!(replacement.previous_path.as_slice(), &[] as &[usize]);
        assert_eq!(replacement.current_path.as_slice(), &[] as &[usize]);
        assert_eq!(
            replacement.discarded_ownership,
            SurfaceIdentityOwnership {
                focus: true,
                pointer_capture: true,
                hover: true,
                widget_state: true,
            }
        );
    }
}

impl SurfaceIdentityPath {
    fn from_slice(path: &[usize]) -> Self {
        let len = path.len().min(MAX_IDENTITY_PATH_COMPONENTS);
        let mut components = [0; MAX_IDENTITY_PATH_COMPONENTS];
        components[..len].copy_from_slice(&path[..len]);
        Self {
            components,
            len: len as u8,
            truncated: path.len() > len,
        }
    }

    /// Return the non-padding path components.
    pub fn as_slice(&self) -> &[usize] {
        &self.components[..self.len as usize]
    }
}

/// Controller-owned interaction domains discarded for one incompatible replacement.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SurfaceIdentityOwnership {
    /// Keyboard focus was owned by the replaced widget.
    pub focus: bool,
    /// Pointer capture or retained capture state was owned by the replaced widget.
    pub pointer_capture: bool,
    /// Widget hover ownership was owned by the replaced widget.
    pub hover: bool,
    /// Retained widget-local interaction state was intentionally not synchronized.
    pub widget_state: bool,
}

/// One bounded incompatible retained-widget replacement diagnostic.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SurfaceIdentityReplacement {
    /// Stable identity shared by the old and new widget.
    pub widget_id: WidgetId,
    /// Concrete compatibility label of the previous widget.
    pub previous_kind: &'static str,
    /// Concrete compatibility label of the replacement widget.
    pub current_kind: &'static str,
    /// Resolved path of the previous widget.
    pub previous_path: SurfaceIdentityPath,
    /// Resolved path of the replacement widget.
    pub current_path: SurfaceIdentityPath,
    /// Controller-owned domains discarded during replacement.
    pub discarded_ownership: SurfaceIdentityOwnership,
}

/// Bounded identity diagnostics emitted while reconciling one refresh.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SurfaceIdentityDiagnostics {
    /// First replacements in deterministic paint order, up to the fixed bound.
    pub replacements: [Option<SurfaceIdentityReplacement>; MAX_IDENTITY_REPLACEMENTS_PER_REFRESH],
    /// Number of replacements observed, including entries omitted by the bound.
    pub replacement_count: u32,
}

impl Default for SurfaceIdentityDiagnostics {
    fn default() -> Self {
        Self {
            replacements: [None; MAX_IDENTITY_REPLACEMENTS_PER_REFRESH],
            replacement_count: 0,
        }
    }
}

impl SurfaceIdentityDiagnostics {
    const fn startup() -> Self {
        Self {
            replacements: [None; MAX_IDENTITY_REPLACEMENTS_PER_REFRESH],
            replacement_count: 0,
        }
    }

    fn push(&mut self, replacement: SurfaceIdentityReplacement) {
        let index = self.replacement_count as usize;
        if index < self.replacements.len() {
            self.replacements[index] = Some(replacement);
        }
        self.replacement_count = self.replacement_count.saturating_add(1);
    }

    fn merge(&mut self, other: Self) {
        let base = self.replacement_count as usize;
        for (offset, replacement) in other.replacements.into_iter().enumerate() {
            let Some(replacement) = replacement else {
                continue;
            };
            let index = base.saturating_add(offset);
            if index < self.replacements.len() {
                self.replacements[index] = Some(replacement);
            }
        }
        self.replacement_count = self
            .replacement_count
            .saturating_add(other.replacement_count);
    }
}

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
    /// Bounded incompatible retained-widget replacement diagnostics.
    pub identity: SurfaceIdentityDiagnostics,
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
            identity: SurfaceIdentityDiagnostics::startup(),
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
        self.identity.merge(other.identity);
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
                    identity: SurfaceIdentityDiagnostics::default(),
                },
                Duration::ZERO,
            );
            return;
        }

        let application_projection_started = Instant::now();
        let mut next_surface = self.bridge.pull_surface();
        next_surface.set_window_environment(self.window_environment);
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

        let previous_paths = std::mem::take(&mut self.traversal.widgets.paths.previous);
        let identity = self.discard_incompatible_widget_ownership(
            &next_surface,
            &traversal.widget_paint_order,
            &traversal.widget_paths,
            &previous_paths,
        );
        let widget_state_sync_started = Instant::now();
        let sync_policy = self.widget_state_sync_policy();
        next_surface.synchronize_widget_state_from_paths(
            &self.surface,
            &traversal.stateful_widget_order,
            &traversal.widget_paths,
            &previous_paths,
            sync_policy,
        );
        let widget_state_sync = widget_state_sync_started.elapsed();
        self.refresh_counters.widget_state_sync =
            self.refresh_counters.widget_state_sync.saturating_add(1);
        self.traversal.widgets.paths.previous = previous_paths;

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
                identity,
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

    fn discard_incompatible_widget_ownership(
        &mut self,
        next_surface: &crate::runtime::UiSurface<Message>,
        widget_paint_order: &[WidgetId],
        current_paths: &std::collections::HashMap<WidgetId, crate::runtime::WidgetPath>,
        previous_paths: &std::collections::HashMap<WidgetId, crate::runtime::WidgetPath>,
    ) -> SurfaceIdentityDiagnostics {
        let mut diagnostics = SurfaceIdentityDiagnostics::default();
        for widget_id in widget_paint_order {
            let Some(current_path) = current_paths.get(widget_id) else {
                continue;
            };
            let Some(previous_path) = previous_paths.get(widget_id) else {
                continue;
            };
            let Some(previous_kind) = self
                .surface
                .widget_compatibility_kind_at_path(previous_path.as_slice())
            else {
                continue;
            };
            let Some(current_kind) =
                next_surface.widget_compatibility_kind_at_path(current_path.as_slice())
            else {
                continue;
            };
            if previous_kind == current_kind {
                continue;
            }
            let discarded_ownership = self.discard_widget_ownership(*widget_id);
            diagnostics.push(SurfaceIdentityReplacement {
                widget_id: *widget_id,
                previous_kind,
                current_kind,
                previous_path: SurfaceIdentityPath::from_slice(previous_path.as_slice()),
                current_path: SurfaceIdentityPath::from_slice(current_path.as_slice()),
                discarded_ownership,
            });
        }
        diagnostics
    }

    fn discard_widget_ownership(&mut self, widget_id: WidgetId) -> SurfaceIdentityOwnership {
        let focus = self.interaction.focus.focused_widget == Some(widget_id);
        let pointer_capture = self.interaction.pointer.capture == Some(widget_id)
            || self
                .interaction
                .pointer
                .capture_state
                .is_some_and(|(captured_id, _)| captured_id == widget_id);
        let hover = self.interaction.hover.widget == Some(widget_id);
        if focus {
            self.interaction.focus.focused_widget = None;
        }
        if pointer_capture {
            self.interaction.pointer.capture = None;
            self.interaction.pointer.capture_state = None;
        }
        if hover {
            self.interaction.hover.widget = None;
        }
        SurfaceIdentityOwnership {
            focus,
            pointer_capture,
            hover,
            widget_state: true,
        }
    }
}

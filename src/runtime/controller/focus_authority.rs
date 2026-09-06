//! Backend-neutral explicit focus transfers qualified by runtime and projection.
use super::{SurfaceRuntime, focus::FocusTransition};
use crate::{gui::automation::AutomationTarget, runtime::RuntimeBridge, widgets::WidgetId};

/// Opaque current materialized focus destination issued by one runtime.
///
/// Querying a target does not move focus, scroll, or materialize virtual content.
/// Targets are conservative projection snapshots: refresh retires old evidence.
#[derive(Clone, Debug)]
pub struct FocusTarget {
    runtime: u64,
    pub(super) target: AutomationTarget,
    widget: WidgetId,
}
impl FocusTarget {
    /// Widget identity for diagnostics; this value alone grants no transfer authority.
    pub fn widget_id(&self) -> WidgetId {
        self.widget
    }
}

/// Exact disposition of an explicit focus transfer. Only `NoDestination` permits fallback.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FocusTransferOutcome {
    /// No eligible destination was found by the requested traversal.
    NoDestination,
    /// The current owner refused focus loss.
    Vetoed,
    /// An admitted attempt was invalidated by synchronous widget/reducer work.
    Invalidated,
    /// The supplied target belongs to another runtime or an obsolete projection.
    Stale,
    /// The runtime is closed or the current target is not eligible.
    Unavailable,
    /// Focus belongs to the requested current widget.
    Admitted(WidgetId),
    /// Focus belongs to a runtime-owned non-widget destination, such as a separator.
    AdmittedRuntimeOwned,
}

/// Direction for geometry-based traversal among current visible keyboard targets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FocusDirection {
    /// Search left of the current target's center.
    Left,
    /// Search right of the current target's center.
    Right,
    /// Search above the current target's center.
    Up,
    /// Search below the current target's center.
    Down,
}

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    /// Observe one current eligible focus destination without invoking input or providers.
    pub fn focus_target(&self, widget: WidgetId) -> Option<FocusTarget> {
        if !self.lifecycle_accepts_work()
            || !self.is_live_focus_target(widget)
            || self
                .traversal
                .widgets
                .duplicate_widget_ids
                .contains(&widget)
        {
            return None;
        }
        let id = widget.to_string();
        let mut matches = self
            .automation_target_snapshot()
            .targets
            .into_iter()
            .filter(|target| target.id.0 == id);
        let target = matches.next()?;
        if matches.next().is_some()
            || !target.enabled
            || !target.focusable
            || !target
                .authority
                .is_some_and(|authority| authority.materialized)
        {
            return None;
        }
        Some(FocusTarget {
            runtime: self.runtime_identity(),
            target,
            widget,
        })
    }

    /// Transfer focus using runtime-issued current evidence and ordinary focus-loss policy.
    ///
    /// Focus loss, composition and capture teardown use the existing runtime path.
    /// Revalidate after synchronous callbacks; veto and invalidation are terminal.
    pub fn transfer_focus(&mut self, target: &FocusTarget) -> FocusTransferOutcome {
        if !self.lifecycle_accepts_work() {
            return FocusTransferOutcome::Unavailable;
        }
        if target.runtime != self.runtime_identity()
            || target.target.authority.is_none_or(|authority| {
                authority.runtime_generation != self.refresh_counters().runtime_projection
            })
        {
            return FocusTransferOutcome::Stale;
        }
        if !self.focus_target_is_current(target) {
            return FocusTransferOutcome::Unavailable;
        }
        match self.request_focus(target.widget) {
            FocusTransition::Vetoed => FocusTransferOutcome::Vetoed,
            FocusTransition::InvalidTarget => FocusTransferOutcome::Invalidated,
            FocusTransition::Changed | FocusTransition::Unchanged => {
                if self.is_authoritative_focus_target(target.widget)
                    && self.focus_target_is_current(target)
                {
                    FocusTransferOutcome::Admitted(target.widget)
                } else {
                    FocusTransferOutcome::Invalidated
                }
            }
        }
    }

    fn focus_target_is_current(&self, target: &FocusTarget) -> bool {
        self.focus_target(target.widget).is_some_and(|current| {
            current.runtime == target.runtime
                && current.target.authority == target.target.authority
                && current.target.path == target.target.path
                && current.target.role == target.target.role
        })
    }
}

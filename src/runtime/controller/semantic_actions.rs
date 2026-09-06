//! One current-authority semantic execution boundary for native and headless consumers.
use super::{
    FocusTransferOutcome, SurfaceRuntime, interaction_state::RuntimeManagedCompositionState,
};
use crate::{
    gui::automation::{AutomationNodeId, AutomationTarget},
    runtime::{
        NumericAccessibilityDispatchResult, NumericAccessibilityRequest, RuntimeBridge,
        WidgetDispatchResult,
    },
    widgets::{SemanticAction, SemanticActionSource, WidgetId},
};

/// Opaque current materialized widget target issued by one runtime.
#[derive(Clone, Debug)]
pub struct SemanticActionTarget {
    runtime: u64,
    widget: WidgetId,
    target: AutomationTarget,
}
impl SemanticActionTarget {
    /// Observational identity; the ID alone is not execution authority.
    pub fn id(&self) -> &AutomationNodeId {
        &self.target.id
    }
}

/// Exact outcome of an explicit semantic request. Failures never trigger fallback.
#[derive(Clone, Debug, PartialEq)]
pub enum SemanticActionOutcome {
    /// The widget handler ran, with zero or one initial mapped message.
    Accepted,
    /// The target belongs to another runtime or obsolete projection/semantic identity.
    Stale,
    /// The runtime or materialized target is unavailable.
    Unavailable,
    /// The current descriptor, handler or action advertisement does not support the action.
    Unsupported,
    /// Text payload exceeds the 64 KiB bound.
    InvalidRequest,
    /// The target is disabled.
    Disabled,
    /// The target is read-only.
    ReadOnly,
    /// An incumbent composition, capture or numeric edit owns interaction.
    Blocked,
    /// Focus admission outcome, including terminal veto and invalidation.
    Focus(FocusTransferOutcome),
    /// The shared command router rejected the semantic activation before mapping.
    CommandRejected(crate::application::CommandDispatchStatus),
    /// The existing typed numeric lifecycle outcome, including local policy results.
    Numeric(NumericAccessibilityDispatchResult),
}

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    /// Observe a current materialized widget target without action, focus or provider calls.
    /// Logical/unmaterialized virtual metadata and runtime-owned non-widget nodes are not targets.
    pub fn semantic_action_target(&self, id: &AutomationNodeId) -> Option<SemanticActionTarget> {
        if !self.lifecycle_accepts_work() || id.0.len() > 256 {
            return None;
        }
        let widget: WidgetId = id.0.parse().ok()?;
        if !self.traversal.widgets.paths.current.contains_key(&widget)
            || self
                .traversal
                .widgets
                .duplicate_widget_ids
                .contains(&widget)
            || !self.layout.rects.contains_key(&widget)
        {
            return None;
        }
        let mut matches = self
            .automation_target_snapshot()
            .targets
            .into_iter()
            .filter(|target| target.id == *id);
        let target = matches.next()?;
        if matches.next().is_some()
            || !target
                .authority
                .is_some_and(|authority| authority.materialized)
        {
            return None;
        }
        Some(SemanticActionTarget {
            runtime: self.runtime_identity(),
            widget,
            target,
        })
    }

    /// Execute one explicit semantic request through current authority and the ordinary reducer.
    /// Required text composition and incumbent captures precede non-focus actions.
    /// Numeric actions require Accessibility provenance and use their existing typed lifecycle.
    pub fn dispatch_semantic_action(
        &mut self,
        target: &SemanticActionTarget,
        action: SemanticAction,
        source: SemanticActionSource,
    ) -> SemanticActionOutcome {
        if !self.lifecycle_accepts_work() {
            return SemanticActionOutcome::Unavailable;
        }
        let payload = match &action {
            SemanticAction::SetText(value)
            | SemanticAction::Numeric(crate::widgets::NumericAccessibilityAction::SetValueText(
                value,
            )) => Some(value),
            _ => None,
        };
        if payload.is_some_and(|value| value.len() > 65_536) {
            return SemanticActionOutcome::InvalidRequest;
        }
        if !self.semantic_action_target_is_current(target) {
            return SemanticActionOutcome::Stale;
        }
        let Some(widget) = self.surface_widget(target.widget) else {
            return SemanticActionOutcome::Unavailable;
        };
        let common = widget.widget_object().common();
        if common.state.disabled {
            return SemanticActionOutcome::Disabled;
        }
        if action != SemanticAction::Focus && common.state.read_only {
            return SemanticActionOutcome::ReadOnly;
        }
        if !target
            .target
            .available_actions
            .iter()
            .any(|name| name == action.identifier())
        {
            return SemanticActionOutcome::Unsupported;
        }
        if action == SemanticAction::Focus {
            return SemanticActionOutcome::Focus(
                self.focus_target(target.widget)
                    .map_or(FocusTransferOutcome::Unavailable, |focus| {
                        self.transfer_focus(&focus)
                    }),
            );
        }
        if self.semantic_action_is_blocked(target.widget) {
            return SemanticActionOutcome::Blocked;
        }
        if let SemanticAction::Numeric(action) = action {
            if source != SemanticActionSource::Accessibility {
                return SemanticActionOutcome::Unsupported;
            }
            return SemanticActionOutcome::Numeric(self.dispatch_numeric_accessibility_action(
                NumericAccessibilityRequest::new(target.target.clone(), action),
            ));
        }
        if !widget.supports_semantic_action(&action)
            || !self
                .surface_widget_mut(target.widget)
                .is_some_and(|widget| widget.has_semantic_action_handler(&action))
        {
            return SemanticActionOutcome::Unsupported;
        }
        if target.target.focusable {
            let Some(focus) = self.focus_target(target.widget) else {
                return SemanticActionOutcome::Unavailable;
            };
            match self.transfer_focus(&focus) {
                FocusTransferOutcome::Admitted(_) => {}
                outcome => return SemanticActionOutcome::Focus(outcome),
            }
        }
        if !self.semantic_action_target_is_current(target) {
            return SemanticActionOutcome::Stale;
        }
        let Some(widget) = self.surface_widget(target.widget) else {
            return SemanticActionOutcome::Unavailable;
        };
        let common = widget.widget_object().common();
        if common.state.disabled {
            return SemanticActionOutcome::Disabled;
        }
        if common.state.read_only {
            return SemanticActionOutcome::ReadOnly;
        }
        if self.semantic_action_is_blocked(target.widget) {
            return SemanticActionOutcome::Blocked;
        }
        if !widget.supports_semantic_action(&action) {
            return SemanticActionOutcome::Unsupported;
        }
        let Some(dispatch) = self
            .surface_widget_mut(target.widget)
            .and_then(|widget| widget.dispatch_semantic_action(action, source).ok())
        else {
            return SemanticActionOutcome::Unsupported;
        };
        match dispatch {
            WidgetDispatchResult::Message(message) => {
                let outcome = self.dispatch_message(message);
                self.pending_input_command_outcome.merge(outcome);
            }
            WidgetDispatchResult::Command(mut activation) => {
                activation.source = match source {
                    SemanticActionSource::Accessibility => {
                        crate::application::CommandSource::Accessibility
                    }
                    SemanticActionSource::Programmatic => {
                        crate::application::CommandSource::Application
                    }
                };
                let dispatch = self.resolve_command_request(
                    activation.request(),
                    crate::gui::focus::FocusSurface::None,
                );
                if let Some(message) = dispatch.message {
                    let outcome = self.dispatch_message(message);
                    self.pending_input_command_outcome.merge(outcome);
                } else {
                    return SemanticActionOutcome::CommandRejected(dispatch.status);
                }
            }
            WidgetDispatchResult::NoOutput | WidgetDispatchResult::UnmappedOutput => {
                self.relayout()
            }
        }
        SemanticActionOutcome::Accepted
    }

    fn semantic_action_is_blocked(&self, widget: WidgetId) -> bool {
        self.interaction.composition.managed_composition != RuntimeManagedCompositionState::Idle
            || (self.gesture_owns_pointer_capture() || self.interaction.pointer.capture.is_some())
            || self.interaction.pointer.managed_capture.is_some()
            || self.interaction.layout_capture.is_some()
            || self
                .surface_widget(widget)
                .is_some_and(|widget| widget.widget_object().common().state.pressed)
            || self.accessibility_incumbent_owner(widget).is_some()
    }

    fn semantic_action_target_is_current(&self, target: &SemanticActionTarget) -> bool {
        target.runtime == self.runtime_identity()
            && self
                .semantic_action_target(&target.target.id)
                .is_some_and(|current| {
                    current.widget == target.widget
                        && current.target.authority == target.target.authority
                        && current.target.path == target.target.path
                        && current.target.role == target.target.role
                        && current.target.available_actions == target.target.available_actions
                })
    }
}

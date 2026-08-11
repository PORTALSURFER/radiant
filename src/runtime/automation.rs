//! Backend-neutral automation snapshot extraction for runtime surfaces.

use super::controller::{
    VirtualLayoutAutomationComposition, VirtualLayoutAutomationCompositionError,
    VirtualLayoutSemanticClassificationBatch,
};
use crate::{
    gui::automation::{
        AutomationTarget, AutomationTargetAuthority, GuiAutomationSnapshot,
        GuiAutomationTargetSnapshot,
    },
    runtime::{RuntimeBridge, SurfaceRuntime},
    widgets::{
        NumericAccessibilityAction, NumericAccessibilityBlockOwner,
        NumericAccessibilityRejectedReason, WidgetId, WidgetOutput,
    },
};

/// Backend-neutral request for one discrete numeric accessibility action.
///
/// The target is a read-only automation snapshot value. Runtime dispatch must
/// resolve it against the current projection and revalidate every authority
/// boundary before invoking a widget policy.
#[derive(Clone, Debug, PartialEq)]
pub struct NumericAccessibilityRequest {
    /// Target evidence captured from [`SurfaceRuntime::automation_target_snapshot`].
    pub target: AutomationTarget,
    /// Neutral numeric action to evaluate.
    pub action: NumericAccessibilityAction,
}

impl NumericAccessibilityRequest {
    /// Build a request from one target and neutral action.
    pub fn new(target: AutomationTarget, action: NumericAccessibilityAction) -> Self {
        Self { target, action }
    }
}

/// Unavailable classification produced before a target can reach widget policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NumericAccessibilityUnavailableReason {
    /// No current target can be identified from the request.
    UnknownTarget,
    /// The captured target authority or identity no longer matches current state.
    StaleTarget,
    /// A previously materialized target is no longer in the current projection.
    RemovedTarget,
    /// The request explicitly identifies a target that is not materialized.
    UnmaterializedTarget,
}

/// Result of the runtime-owned numeric accessibility admission and dispatch.
///
/// `Accepted` means runtime admission succeeded and the widget handler ran; it
/// carries the type-erased [`WidgetOutput`] emitted by the local policy. The
/// local result may still be `NoChange`, a typed rejection, or a typed policy
/// failure. Numeric hosts can downcast it to their typed
/// `NumericAccessibilityOutcome<T, AdjustmentError, FormatError>`; the runtime
/// itself remains generic over those application-owned types. When a widget
/// mapper is configured, the accepted output is also mapped through the normal
/// host-message reduction path.
#[derive(Clone, Debug, PartialEq)]
pub enum NumericAccessibilityDispatchResult {
    /// The request could not reach a current executable target.
    Unavailable {
        /// Deterministic unavailable reason.
        reason: NumericAccessibilityUnavailableReason,
    },
    /// The current target exists but cannot admit this request.
    Rejected {
        /// Deterministic runtime-owned rejection reason.
        reason: NumericAccessibilityRejectedReason,
    },
    /// An incumbent interaction owner prevented admission.
    Blocked {
        /// Owner that remains authoritative.
        owner: NumericAccessibilityBlockOwner,
    },
    /// Runtime admission succeeded and the local widget policy produced output.
    /// This is not a claim that the value changed; inspect the typed local
    /// outcome for `NoChange`, rejection, or policy-failure results.
    Accepted {
        /// Stable widget identity that accepted the request.
        widget_id: WidgetId,
        /// Type-erased typed local policy outcome.
        output: WidgetOutput,
    },
}

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    /// Compose already-classified virtual semantic evidence into a private
    /// automation snapshot without changing the live runtime.
    #[allow(dead_code)]
    pub(crate) fn compose_virtual_layout_automation_snapshot(
        &self,
        ordinary: &GuiAutomationSnapshot,
        batches: &[VirtualLayoutSemanticClassificationBatch],
    ) -> Result<VirtualLayoutAutomationComposition, VirtualLayoutAutomationCompositionError> {
        self.virtual_layout
            .compose_virtual_layout_automation_snapshot(ordinary, batches)
    }

    /// Return a serializable backend-neutral automation snapshot for the current surface.
    pub fn automation_snapshot(&self) -> GuiAutomationSnapshot {
        let viewport = self.context().viewport;
        let root = self
            .surface()
            .root()
            .automation_snapshot_node(self.context().layout);

        GuiAutomationSnapshot {
            schema_version: 2,
            viewport_width: viewport.width().max(0.0).round() as u32,
            viewport_height: viewport.height().max(0.0).round() as u32,
            root,
        }
    }

    /// Return a flattened, coordinate-bearing automation target snapshot for the
    /// current surface.
    pub fn automation_target_snapshot(&self) -> GuiAutomationTargetSnapshot {
        let mut snapshot = self.automation_snapshot().target_snapshot();
        let authority =
            AutomationTargetAuthority::materialized(self.refresh_counters().runtime_projection);
        for target in &mut snapshot.targets {
            target.authority = Some(authority);
        }
        snapshot.schema_version = 2;
        snapshot
    }
}

//! Backend-neutral automation snapshot extraction for runtime surfaces.

use super::controller::{
    VirtualLayoutAutomationComposition, VirtualLayoutAutomationCompositionError,
    VirtualLayoutSemanticClassificationBatch,
};
#[cfg(target_os = "macos")]
use crate::application::virtual_layout::VirtualLayoutSemanticCardinality;
use crate::{
    gui::automation::{
        AutomationTarget, AutomationTargetAuthority, GuiAutomationSnapshot,
        GuiAutomationTargetSnapshot,
    },
    layout::VirtualLayoutItemKey,
    runtime::{RuntimeBridge, SurfaceRuntime},
    widgets::{
        NumericAccessibilityAction, NumericAccessibilityBlockOwner,
        NumericAccessibilityRejectedReason, WidgetId, WidgetOutput,
    },
};

/// Provider-free admission evidence consumed by the private native semantic
/// adapter.  This is deliberately smaller than a public container handle: a
/// native callback may observe cardinality and registration identity without
/// opening a semantic session or creating demand.
#[cfg(target_os = "macos")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct NativeSemanticContainerSnapshot {
    pub(crate) container_id: crate::layout::NodeId,
    pub(crate) mount_generation: u64,
    pub(crate) registration_generation: u64,
    pub(crate) provider_generation: u64,
    pub(crate) cardinality: VirtualLayoutSemanticCardinality,
    pub(crate) has_range_provider: bool,
    pub(crate) max_entries: usize,
}

/// Opaque runtime-issued identity for one semantic automation session.
///
/// A handle is valid only for the [`SurfaceRuntime`] that issued it and only
/// until that session is closed or superseded.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct SemanticAutomationSessionHandle {
    pub(crate) runtime_id: u64,
    pub(crate) generation: u64,
}

impl std::fmt::Debug for SemanticAutomationSessionHandle {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("SemanticAutomationSessionHandle(..)")
    }
}

/// Opaque runtime-issued identity for one currently mounted virtual container
/// in a semantic automation session.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct SemanticAutomationContainerHandle {
    pub(crate) runtime_id: u64,
    pub(crate) session_generation: u64,
    pub(crate) container_id: crate::layout::NodeId,
    pub(crate) mount_generation: u64,
}

impl std::fmt::Debug for SemanticAutomationContainerHandle {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("SemanticAutomationContainerHandle(..)")
    }
}

/// One explicit semantic demand in the complete session demand set.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SemanticAutomationDemand {
    /// Request one contiguous logical range from a mounted container.
    Range {
        /// Runtime-issued mounted-container identity.
        container: SemanticAutomationContainerHandle,
        /// First logical item index.
        start_index: usize,
        /// Number of logical entries to request.
        length: usize,
    },
    /// Request one independent required-item semantic pin.
    RequiredItem {
        /// Runtime-issued mounted-container identity.
        container: SemanticAutomationContainerHandle,
        /// Opaque host-owned item identity.
        key: VirtualLayoutItemKey,
    },
}

impl SemanticAutomationDemand {
    /// Build one bounded contiguous logical range demand.
    #[must_use]
    pub const fn range(
        container: SemanticAutomationContainerHandle,
        start_index: usize,
        length: usize,
    ) -> Self {
        Self::Range {
            container,
            start_index,
            length,
        }
    }

    /// Build one independent required-item pin demand.
    #[must_use]
    pub fn required_item(
        container: SemanticAutomationContainerHandle,
        key: VirtualLayoutItemKey,
    ) -> Self {
        Self::RequiredItem { container, key }
    }
}

/// Typed validation failures for an atomic demand-set update.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SemanticAutomationDemandError {
    /// The same source was listed twice for one mounted container.
    DuplicateSource,
    /// The opaque item key did not have stable reflexive equality.
    InvalidKey,
    /// A range must contain at least one entry.
    RangeLengthZero,
    /// The exclusive range end overflowed `usize`.
    RangeIndexOverflow,
    /// The range exceeded its mounted registration budget or the hard cap.
    RangeOverBudget,
    /// The complete session range set exceeded the aggregate cap.
    AggregateRangeBudgetExceeded,
    /// The mounted registration declares an unsupported custom coordinate
    /// space. No provider is invoked for this demand.
    CustomCoordinateSpace,
    /// An internal bounded generation could not advance.
    CounterOverflow,
}

/// Explicit session lifecycle and demand admission failures.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SemanticAutomationSessionError {
    /// A runtime already has one active semantic automation session.
    SessionAlreadyActive,
    /// The supplied session handle is not active for this runtime.
    UnknownSession,
    /// The supplied mounted-container handle is stale, retired, or belongs to
    /// another session/runtime.
    StaleContainerHandle,
    /// The complete demand set failed validation and was not applied.
    InvalidDemand(SemanticAutomationDemandError),
    /// Retrying requires at least one active demand member.
    NoActiveDemand,
    /// A provider callback attempted forbidden synchronous runtime reentry.
    Reentrant,
}

/// Conservative reason attached to a non-successful selected snapshot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SemanticAutomationFallbackReason {
    /// The complete session demand set is empty.
    NoDemand,
    /// At least one active demand member has not resolved.
    IncompleteDemandSet,
    /// No provider was registered for the requested source.
    NoProvider,
    /// The provider does not support the requested source.
    Unsupported,
    /// The provider could not answer from its current data.
    DataUnavailable,
    /// The provider deferred the request.
    Deferred,
    /// The provider rejected the request or returned a rejected result.
    Rejected,
    /// Provider output or composition evidence was malformed.
    Malformed,
    /// The attempt or authority was stale and was not accepted.
    Stale,
    /// The session publication was cleared by lifecycle or authority change.
    Invalidated,
    /// A bounded generation could not advance.
    CounterOverflow,
}

/// Typed status for an explicit semantic refresh and for a selected read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SemanticAutomationRefreshStatus {
    /// Every active member resolved and the complete logical publication was
    /// accepted. This includes authoritative `NotFound` members.
    Published,
    /// A complete prior selection was retained under an eligible exact fence.
    Retained {
        /// Why the current attempt could not replace the retained selection.
        reason: SemanticAutomationFallbackReason,
    },
    /// The conservative ordinary baseline is exposed because no exact-fence
    /// selected publication was eligible.
    Baseline {
        /// Why semantic content was withheld.
        reason: SemanticAutomationFallbackReason,
    },
}

/// Public selected semantic automation data. The target projection carries
/// coordinate-bearing targets and preserves `materialized = false` authority
/// for unmaterialized provider entries.
#[derive(Clone, Debug, PartialEq)]
pub struct SemanticAutomationSelectedSnapshot {
    /// Complete public automation tree selected for this session.
    pub snapshot: GuiAutomationSnapshot,
    /// Flattened coordinate-bearing target projection for the same tree.
    pub targets: GuiAutomationTargetSnapshot,
    /// Typed publication/ fallback status.
    pub status: SemanticAutomationRefreshStatus,
}

/// Result of one explicit update or retry turn.
#[derive(Clone, Debug, PartialEq)]
pub struct SemanticAutomationRefresh {
    /// The complete selected snapshot after this turn.
    pub selected: SemanticAutomationSelectedSnapshot,
    /// Status for the publication attempted by this turn.
    pub status: SemanticAutomationRefreshStatus,
}

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
    #[cfg(target_os = "macos")]
    /// Return provider-free, current logical virtual-container admission
    /// evidence for the private native primary-window consumer.  The view is
    /// intentionally unavailable through the public automation API.
    pub(crate) fn native_semantic_containers(&self) -> Vec<NativeSemanticContainerSnapshot> {
        let ordinary = self.automation_snapshot();
        self.virtual_layout.native_semantic_containers(&ordinary)
    }

    #[cfg(target_os = "macos")]
    /// Return the exact selected composition retained by the runtime owner.
    /// The native adapter consumes this complete composition and its sidecar;
    /// it never reconstructs provider members from public snapshots.
    pub(crate) fn native_semantic_automation_composition(
        &self,
        session: SemanticAutomationSessionHandle,
    ) -> Result<
        Option<(
            VirtualLayoutAutomationComposition,
            SemanticAutomationRefreshStatus,
        )>,
        SemanticAutomationSessionError,
    > {
        let counters = self.refresh_counters();
        let ordinary = self.automation_snapshot();
        Ok(self
            .virtual_layout
            .selected_semantic_automation(
                self.runtime_identity(),
                session,
                &ordinary,
                counters.runtime_projection,
            )?
            .map(|publication| (publication.composition, publication.status)))
    }

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

    /// Open the one explicit, backend-neutral semantic automation session
    /// owned by this runtime. Opening a session performs no provider call.
    pub fn open_semantic_automation_session(
        &mut self,
    ) -> Result<SemanticAutomationSessionHandle, SemanticAutomationSessionError> {
        self.virtual_layout
            .open_semantic_automation_session(self.runtime_identity())
    }

    /// Enumerate the currently mounted virtual-layout containers for a live
    /// session. Enumeration is pure and does not create demand or call a
    /// provider.
    pub fn semantic_automation_containers(
        &self,
        session: SemanticAutomationSessionHandle,
    ) -> Result<Vec<SemanticAutomationContainerHandle>, SemanticAutomationSessionError> {
        self.virtual_layout
            .semantic_automation_containers(self.runtime_identity(), session)
    }

    /// Replace the complete session demand set and explicitly execute the
    /// resulting bounded provider attempts. Publication is atomic across the
    /// whole set; ordinary snapshots remain untouched.
    pub fn refresh_semantic_automation_session(
        &mut self,
        session: SemanticAutomationSessionHandle,
        demands: &[SemanticAutomationDemand],
    ) -> Result<SemanticAutomationRefresh, SemanticAutomationSessionError> {
        let counters = self.refresh_counters();
        let ordinary = self.automation_snapshot();
        let publication = self.virtual_layout.refresh_semantic_automation(
            self.runtime_identity(),
            session,
            demands,
            &ordinary,
            (
                counters.layout,
                counters.runtime_projection,
                counters.runtime_projection,
            ),
        )?;
        let selected = selected_snapshot_from_publication(
            publication.composition,
            publication.status,
            counters.runtime_projection,
        );
        Ok(SemanticAutomationRefresh {
            status: selected.status,
            selected,
        })
    }

    /// Retry every active session demand exactly once per mounted source.
    pub fn retry_semantic_automation_session(
        &mut self,
        session: SemanticAutomationSessionHandle,
    ) -> Result<SemanticAutomationRefresh, SemanticAutomationSessionError> {
        let counters = self.refresh_counters();
        let ordinary = self.automation_snapshot();
        let publication = self.virtual_layout.retry_semantic_automation(
            self.runtime_identity(),
            session,
            &ordinary,
            (
                counters.layout,
                counters.runtime_projection,
                counters.runtime_projection,
            ),
        )?;
        let selected = selected_snapshot_from_publication(
            publication.composition,
            publication.status,
            counters.runtime_projection,
        );
        Ok(SemanticAutomationRefresh {
            status: selected.status,
            selected,
        })
    }

    /// Read the currently selected session publication without calling a
    /// provider or changing runtime state.
    pub fn selected_semantic_automation_snapshot(
        &self,
        session: SemanticAutomationSessionHandle,
    ) -> Result<SemanticAutomationSelectedSnapshot, SemanticAutomationSessionError> {
        let counters = self.refresh_counters();
        let ordinary = self.automation_snapshot();
        let Some(publication) = self.virtual_layout.selected_semantic_automation(
            self.runtime_identity(),
            session,
            &ordinary,
            counters.runtime_projection,
        )?
        else {
            return Ok(selected_snapshot_from_ordinary(
                ordinary,
                SemanticAutomationRefreshStatus::Baseline {
                    reason: SemanticAutomationFallbackReason::Invalidated,
                },
                counters.runtime_projection,
            ));
        };
        Ok(selected_snapshot_from_publication(
            publication.composition,
            publication.status,
            counters.runtime_projection,
        ))
    }

    /// Close the session and cancel its active demand membership while
    /// preserving the runtime-owned virtual-layout registrations.
    pub fn close_semantic_automation_session(
        &mut self,
        session: SemanticAutomationSessionHandle,
    ) -> Result<(), SemanticAutomationSessionError> {
        self.virtual_layout
            .close_semantic_automation_session(self.runtime_identity(), session)
    }
}

fn selected_snapshot_from_publication(
    composition: VirtualLayoutAutomationComposition,
    status: SemanticAutomationRefreshStatus,
    runtime_projection_generation: u64,
) -> SemanticAutomationSelectedSnapshot {
    SemanticAutomationSelectedSnapshot {
        snapshot: composition.snapshot().clone(),
        targets: composition.target_snapshot(runtime_projection_generation),
        status,
    }
}

fn selected_snapshot_from_ordinary(
    snapshot: GuiAutomationSnapshot,
    status: SemanticAutomationRefreshStatus,
    runtime_projection_generation: u64,
) -> SemanticAutomationSelectedSnapshot {
    let mut targets = snapshot.target_snapshot();
    let authority = AutomationTargetAuthority::materialized(runtime_projection_generation);
    for target in &mut targets.targets {
        target.authority = Some(authority);
    }
    targets.schema_version = 2;
    SemanticAutomationSelectedSnapshot {
        snapshot,
        targets,
        status,
    }
}

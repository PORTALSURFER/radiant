//! Generic declarative view-tree types for message-driven Radiant hosts.

use crate::UiAffinity;
use crate::application::ApplicationEnvironment;
use std::sync::Arc;

mod builders;
mod dispatch;
mod focus;
mod frame;
mod input;
mod interaction_patch;
mod layout;
mod lookup;
mod node;
mod paint;
mod path;
mod projection;
mod revision;
mod source;
mod state_sync;
mod traversal;
mod view;
mod virtual_layout;
mod widget;

pub use frame::SurfaceFrame;
pub(in crate::runtime) use input::WidgetDispatchResult;
pub(in crate::runtime) use layout::SurfaceRuntimeProjection;
pub use node::{
    LayerKind, SurfaceChild, SurfaceContainer, SurfaceFloatingLayer, SurfaceLayer, SurfaceNode,
    SurfaceOverlay, SurfaceScene,
};
pub(in crate::runtime) use paint::{clear_paint_plan_for_layout, empty_paint_plan_for_layout};
pub(in crate::runtime) use path::{ClipAncestors, WidgetPath};
pub(crate) use source::{
    KeyedNodeEvidence, SourceCompatibility, SourceIdentity, SourceMetadata, SourceTopology,
    SourceTraversalIndex, source_metadata_matches,
};
#[cfg(test)]
pub(crate) use source::{OverlayEvidence, OverlayIdentity, SurfaceSourceKind};
pub(in crate::runtime) use state_sync::{
    PreparedWidgetStateSyncEvidence, PreparedWidgetStateSyncVeto, PreparedWidgetStateSyncWitness,
    ValidatedWidgetReplacementPlan, WidgetReplacementCommitResult, WidgetReplacementPlan,
    WidgetReplacementPlanVeto, WidgetStateSyncPolicy,
};
pub(in crate::runtime) use traversal::{
    SurfaceContainerTraversalRecord, SurfaceLayoutInteractionRecord,
    SurfaceSplitPaneFocusOrderCandidate, SurfaceSplitPaneRatioActionCandidate,
    SurfaceTraversalIndex, SurfaceTraversalStats, SurfaceWidgetTraversalRecord, WheelHitTarget,
};
#[cfg(test)]
pub(in crate::runtime) use virtual_layout::VirtualLayoutRegistrationRevisions;
pub(crate) use virtual_layout::lower_public_virtual_layout;
pub(in crate::runtime) use virtual_layout::{
    MAX_VIRTUAL_LAYOUT_REGISTRATIONS, VirtualLayoutRegistration,
};
pub use widget::{
    EventMapper, MessageMapper, NativeFileDropMessageMapper, ScrollMessageMapper, SurfaceWidget,
    WidgetMessageMapper,
};

pub(in crate::runtime) use crate::widgets::WidgetId;
pub(crate) use interaction_patch::inspect_interaction_path;
pub(crate) use revision::InteractionLeafRevision;
#[cfg(test)]
pub(in crate::runtime) use revision::SurfaceDamageCandidate;
#[cfg(test)]
pub(in crate::runtime) use revision::ViewDeltaCause;
pub(in crate::runtime) use revision::{
    DEFAULT_VIEW_DELTA_SCRATCH_CAPACITY, ReconciliationAttemptOutcome, RefreshExecutionDecision,
    SurfaceDamage, ViewDelta, ViewDeltaDiagnostics, ViewDeltaEffect, ViewDeltaScratch,
    classify_view_delta,
};

/// Top-level immutable UI surface projected by a generic Radiant host.
///
/// A surface is owned by its UI runtime and cannot be moved into a worker
/// thread. Its marker is zero-sized and does not affect surface storage.
///
/// ```compile_fail
/// use radiant::{
///     layout::ContainerPolicy,
///     runtime::{SurfaceNode, UiSurface},
/// };
///
/// let surface = UiSurface::new(SurfaceNode::<()>::container(
///     1,
///     ContainerPolicy::default(),
///     Vec::new(),
/// ));
/// std::thread::spawn(move || drop(surface));
/// ```
pub struct UiSurface<Message> {
    _ui_affinity: UiAffinity,
    root: SurfaceNode<Message>,
    window_environment: crate::runtime::WindowEnvironment,
    application_environment: Arc<ApplicationEnvironment>,
}

impl<Message> UiSurface<Message> {
    pub(in crate::runtime) fn timed_repaint_deadline(&self) -> Option<std::time::Instant> {
        self.root.timed_repaint_deadline()
    }

    pub(in crate::runtime) fn advance_timed_repaints(&mut self, now: std::time::Instant) -> bool {
        self.root.advance_timed_repaints(now)
    }
}

/// Public declarative view snapshot alias for host applications.
///
/// `View<Message>` is the framework vocabulary for the top-level immutable UI
/// projection. It is an alias for [`UiSurface`] so existing code keeps the same
/// storage, cloning, layout, input, and paint behavior.
pub type View<Message> = UiSurface<Message>;

/// Public declarative element tree alias for host applications.
///
/// `Element<Message>` is the framework vocabulary for one node in a projected
/// view tree. It is an alias for [`SurfaceNode`] to keep identity and layout
/// behavior exactly shared with the existing runtime surface.
pub type Element<Message> = SurfaceNode<Message>;

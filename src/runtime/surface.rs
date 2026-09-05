//! Generic declarative view-tree types for message-driven Radiant hosts.

use crate::UiAffinity;
use crate::application::ApplicationEnvironment;
use std::sync::Arc;

mod builders;
mod command_scopes;
pub(in crate::runtime) use command_scopes::SurfaceCommandScopes;
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
mod text_scaled_size;
mod traversal;
pub(crate) use text_scaled_size::{TextScaledExtent, TextScaledSize};
mod view;
mod virtual_layout;
mod widget;

pub use frame::SurfaceFrame;
pub(in crate::runtime) use input::{ResolvedWidgetDispatchResult, WidgetDispatchResult};
pub(in crate::runtime) use layout::SurfaceRuntimeProjection;
pub use node::{
    LayerKind, SurfaceChild, SurfaceContainer, SurfaceFloatingLayer, SurfaceLayer, SurfaceNode,
    SurfaceOverlay, SurfaceScene,
};
pub(in crate::runtime) use paint::{clear_paint_plan_for_layout, empty_paint_plan_for_layout};
pub(in crate::runtime) use path::{ClipAncestors, WidgetPath};
pub(crate) use source::{
    FrozenSourceMetadata, KeyedNodeEvidence, SourceCompatibility, SourceIdentity, SourceMetadata,
    SourceTopology, SourceTraversalIndex, source_metadata_matches,
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
#[cfg(test)]
pub(in crate::runtime) use revision::SurfaceDamageCandidate;
#[cfg(test)]
pub(in crate::runtime) use revision::ViewDeltaCause;
pub(in crate::runtime) use revision::{
    DEFAULT_VIEW_DELTA_SCRATCH_CAPACITY, ReconciliationAttemptOutcome, RefreshExecutionDecision,
    SurfaceDamage, ViewDelta, ViewDeltaDiagnostics, ViewDeltaEffect, ViewDeltaScratch,
    classify_view_delta,
};
pub(crate) use revision::{
    InteractionLeafEvidence, InteractionLeafRevision, capture_interaction_leaf_evidence,
    classify_interaction_leaf_evidence,
};

#[allow(dead_code)]
#[derive(Clone)]
pub(crate) enum ApplicationNodeKind {
    Container {
        policy: crate::layout::ContainerPolicy,
        style: Option<crate::widgets::WidgetStyle>,
        hoverable: bool,
        split_pane_runtime: Option<crate::gui::layout_core::SplitPaneRuntimeMode>,
        child_count: usize,
        unsupported_fence: bool,
        scroll_mapper: crate::runtime::surface::widget::MapperDescriptor,
    },
    Widget {
        evidence: InteractionLeafEvidence,
    },
    Unsupported,
}

#[allow(dead_code)]
#[derive(Clone)]
pub(crate) struct ApplicationNodeReceipt {
    pub(crate) path: Box<[usize]>,
    pub(crate) incoming_slot: Option<crate::layout::SlotParams>,
    pub(crate) id: crate::layout::NodeId,
    pub(crate) source: FrozenSourceMetadata,
    pub(crate) kind: ApplicationNodeKind,
}

pub(crate) fn application_node_kind<Message>(node: &SurfaceNode<Message>) -> ApplicationNodeKind {
    match node {
        SurfaceNode::Container(container) => ApplicationNodeKind::Container {
            policy: container.policy.clone(),
            style: container.style,
            hoverable: container.hoverable,
            split_pane_runtime: container.split_pane_runtime,
            child_count: container.children.len(),
            unsupported_fence: container.layout_policy.is_some()
                || container.layout_capabilities.is_some()
                || container.split_pane_ratio_settled.is_some()
                || container.offset_settled.is_some()
                || container.virtual_layout.is_some(),
            scroll_mapper: container.scroll_mapper_descriptor(),
        },
        SurfaceNode::Widget(widget) => ApplicationNodeKind::Widget {
            evidence: capture_interaction_leaf_evidence(widget),
        },
        _ => ApplicationNodeKind::Unsupported,
    }
}

#[allow(dead_code)]
pub(crate) fn application_container_kind_matches(
    previous: &ApplicationNodeKind,
    current: &ApplicationNodeKind,
) -> bool {
    let (
        ApplicationNodeKind::Container {
            policy: previous_policy,
            style: previous_style,
            hoverable: previous_hoverable,
            split_pane_runtime: previous_split,
            child_count: previous_count,
            unsupported_fence: previous_fence,
            scroll_mapper: previous_mapper,
        },
        ApplicationNodeKind::Container {
            policy: current_policy,
            style: current_style,
            hoverable: current_hoverable,
            split_pane_runtime: current_split,
            child_count: current_count,
            unsupported_fence: current_fence,
            scroll_mapper: current_mapper,
        },
    ) = (previous, current)
    else {
        return false;
    };
    previous_policy == current_policy
        && previous_style == current_style
        && previous_hoverable == current_hoverable
        && previous_split == current_split
        && previous_count == current_count
        && !previous_fence
        && !current_fence
        && previous_mapper.relation(current_mapper)
            == crate::runtime::surface::widget::MapperRelation::Unchanged
}

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

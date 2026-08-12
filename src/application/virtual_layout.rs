//! Declarative logical virtual-layout provider attachment.

use super::{View, ViewNode, ViewNodeKind};
use crate::{
    layout::{
        VirtualLayoutBudget, VirtualLayoutItem, VirtualLayoutOverscan, VirtualLayoutPolicy,
        VirtualLayoutPolicyIdentity,
    },
    runtime::{
        VirtualLayoutRevisions, VirtualLayoutSemanticProvider, VirtualLayoutSemanticRangeProvider,
    },
};
use std::rc::Rc;

/// Exact provider-free logical cardinality evidence for one virtual layout.
///
/// The value is declaration evidence, not a provider capability or demand. An
/// exact zero is valid, and the count is intentionally not bounded by the
/// per-query virtual-layout budget.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct VirtualLayoutSemanticCardinality {
    /// Exact number of logical items declared by the host.
    pub logical_item_count: usize,
    /// Host-owned revision for the declared cardinality.
    pub cardinality_revision: u64,
}

impl VirtualLayoutSemanticCardinality {
    /// Build exact logical cardinality evidence.
    #[must_use]
    pub const fn new(logical_item_count: usize, cardinality_revision: u64) -> Self {
        Self {
            logical_item_count,
            cardinality_revision,
        }
    }
}

/// Pure shell projection factory for a logical virtual-layout declaration.
pub type VirtualLayoutShellFactory<Message> = Rc<dyn Fn() -> View<Message>>;
/// Pure item projection factory for a logical virtual-layout declaration.
pub type VirtualLayoutItemFactory<Message> = Rc<dyn Fn(&VirtualLayoutItem) -> View<Message>>;
/// Stable item-kind projection for a logical virtual-layout declaration.
pub type VirtualLayoutKindFactory = Rc<dyn Fn(&VirtualLayoutItem) -> VirtualLayoutPolicyIdentity>;

/// Named construction inputs for one logical virtual-layout declaration.
///
/// The declaration carries only immutable policy, projection, revision, and
/// optional provider capabilities.  Runtime identity, registration lifetime,
/// demand, cancellation, and publication remain private runtime concerns.
pub struct VirtualLayoutParts<Message> {
    /// Object-safe bounded policy used for ordinary virtual-layout queries.
    pub policy: Rc<dyn VirtualLayoutPolicy>,
    /// Stable application-owned identity for the logical policy scope.
    pub policy_identity: VirtualLayoutPolicyIdentity,
    /// Leading and trailing ordinary query overscan.
    pub overscan: VirtualLayoutOverscan,
    /// Maximum entries admitted for one provider query.
    pub budget: VirtualLayoutBudget,
    /// Host-owned revision evidence used by exact runtime fences.
    pub revisions: VirtualLayoutRevisions,
    /// Pure shell projection factory for the mounted container.
    pub shell: VirtualLayoutShellFactory<Message>,
    /// Pure projection factory for an accepted logical item.
    pub item: VirtualLayoutItemFactory<Message>,
    /// Stable item-kind projection used by the private materialization bridge.
    pub kind: VirtualLayoutKindFactory,
    /// Optional required-item semantic provider.
    pub semantic_provider: Option<Rc<dyn VirtualLayoutSemanticProvider>>,
    /// Optional contiguous-range semantic provider.
    pub semantic_range_provider: Option<Rc<dyn VirtualLayoutSemanticRangeProvider>>,
    /// Optional exact provider-free logical cardinality declaration.
    pub semantic_cardinality: Option<VirtualLayoutSemanticCardinality>,
}

impl<Message> VirtualLayoutParts<Message> {
    /// Build a logical virtual-layout declaration without semantic providers.
    #[must_use]
    // Keep the established flat named-parts constructor shape; grouping these
    // declaration fields would change the shipped public contract.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        policy: Rc<dyn VirtualLayoutPolicy>,
        policy_identity: VirtualLayoutPolicyIdentity,
        overscan: VirtualLayoutOverscan,
        budget: VirtualLayoutBudget,
        revisions: VirtualLayoutRevisions,
        shell: VirtualLayoutShellFactory<Message>,
        item: VirtualLayoutItemFactory<Message>,
        kind: VirtualLayoutKindFactory,
    ) -> Self {
        Self {
            policy,
            policy_identity,
            overscan,
            budget,
            revisions,
            shell,
            item,
            kind,
            semantic_provider: None,
            semantic_range_provider: None,
            semantic_cardinality: None,
        }
    }

    /// Attach an optional required-item semantic provider.
    #[must_use]
    pub fn with_semantic_provider(
        mut self,
        provider: Rc<dyn VirtualLayoutSemanticProvider>,
    ) -> Self {
        self.semantic_provider = Some(provider);
        self
    }

    /// Attach an optional contiguous-range semantic provider.
    #[must_use]
    pub fn with_semantic_range_provider(
        mut self,
        provider: Rc<dyn VirtualLayoutSemanticRangeProvider>,
    ) -> Self {
        self.semantic_range_provider = Some(provider);
        self
    }

    /// Attach optional exact provider-free logical cardinality evidence.
    #[must_use]
    pub fn with_semantic_cardinality(
        mut self,
        cardinality: VirtualLayoutSemanticCardinality,
    ) -> Self {
        self.semantic_cardinality = Some(cardinality);
        self
    }
}

/// Declare one logical virtual layout and optional semantic providers.
///
/// This function only creates an immutable declarative view.  Registration is
/// derived later by the mounted [`SurfaceRuntime`](crate::runtime::SurfaceRuntime)
/// from the accepted view node; it never calls a provider.
pub fn virtual_layout_from_parts<Message: 'static>(
    parts: VirtualLayoutParts<Message>,
) -> View<Message> {
    ViewNode::new(ViewNodeKind::VirtualLayout(parts))
}

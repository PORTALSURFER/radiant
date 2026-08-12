//! Qualified public logical virtual-layout provider capabilities.
//!
//! The public types in this module are declaration and callback vocabulary
//! only.  Mounted identity, lifetime, demand, attempt, cancellation, and
//! publication remain owned by [`SurfaceRuntime`](crate::runtime::SurfaceRuntime).

use crate::{
    gui::{
        automation::{AutomationNodeId, AutomationNodeSemantics},
        types::Rect,
    },
    layout::{VirtualLayoutBudget, VirtualLayoutItemKey},
};
use std::{cell::Cell, panic::AssertUnwindSafe, rc::Rc};

use crate::gui::layout_core::{
    VirtualLayoutSemanticQueryOutcome as PrivateItemOutcome,
    VirtualLayoutSemanticRangeProviderOutcome as PrivateRangeOutcome,
    VirtualLayoutSemanticRangeRequest as PrivateRangeRequest, VirtualLayoutSemanticRejectedReason,
    VirtualLayoutSemanticRequest as PrivateItemRequest,
};

/// Host-owned revision evidence for one logical virtual-layout declaration.
///
/// The runtime supplies the mounted container, registration, provider, and
/// mount generations.  Callers cannot provide or advance those identities.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct VirtualLayoutRevisions {
    /// Revision for application data used by semantic providers.
    pub data: u64,
    /// Revision for the declared virtual-layout policy.
    pub policy: u64,
    /// Revision for measured geometry used by semantic providers.
    pub measurement: u64,
    /// Revision for semantic content and ordering.
    pub semantic: u64,
}

impl VirtualLayoutRevisions {
    /// Build revision evidence from the host-owned content revisions.
    #[must_use]
    pub const fn new(data: u64, policy: u64, measurement: u64, semantic: u64) -> Self {
        Self {
            data,
            policy,
            measurement,
            semantic,
        }
    }
}

/// Read-only request for one required-item semantic lookup.
///
/// Runtime-owned container and mount identities are intentionally not exposed
/// to provider code.  The request contains only the opaque item key and the
/// host revision evidence relevant to the lookup.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VirtualLayoutSemanticRequest {
    key: VirtualLayoutItemKey,
    revisions: VirtualLayoutRevisions,
}

impl VirtualLayoutSemanticRequest {
    pub(crate) fn from_private(request: &PrivateItemRequest) -> Self {
        Self {
            key: request.key().clone(),
            revisions: VirtualLayoutRevisions::new(
                request.data_revision(),
                request.policy_revision(),
                request.measurement_revision(),
                request.semantic_revision(),
            ),
        }
    }

    /// Return the opaque stable key requested by the runtime.
    #[must_use]
    pub fn key(&self) -> &VirtualLayoutItemKey {
        &self.key
    }

    /// Return host-owned revision evidence for this request.
    #[must_use]
    pub const fn revisions(&self) -> VirtualLayoutRevisions {
        self.revisions
    }

    /// Return the host data revision captured by this request.
    #[must_use]
    pub const fn data_revision(&self) -> u64 {
        self.revisions.data
    }

    /// Return the host policy revision captured by this request.
    #[must_use]
    pub const fn policy_revision(&self) -> u64 {
        self.revisions.policy
    }

    /// Return the host measurement revision captured by this request.
    #[must_use]
    pub const fn measurement_revision(&self) -> u64 {
        self.revisions.measurement
    }

    /// Return the host semantic revision captured by this request.
    #[must_use]
    pub const fn semantic_revision(&self) -> u64 {
        self.revisions.semantic
    }
}

/// Read-only request for one bounded contiguous logical semantic range.
///
/// The runtime validates the range and budget before invoking a provider.  No
/// container or mount identity is exposed to the callback.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VirtualLayoutSemanticRangeRequest {
    start_index: usize,
    length: usize,
    budget: VirtualLayoutBudget,
    revisions: VirtualLayoutRevisions,
}

impl VirtualLayoutSemanticRangeRequest {
    pub(crate) fn from_private(request: &PrivateRangeRequest) -> Self {
        let range = request.range();
        Self {
            start_index: range.start_index(),
            length: range.length(),
            budget: request.budget(),
            revisions: VirtualLayoutRevisions::new(
                request.data_revision(),
                request.policy_revision(),
                request.measurement_revision(),
                request.semantic_revision(),
            ),
        }
    }

    /// Return the first logical index in the requested range.
    #[must_use]
    pub const fn start_index(self) -> usize {
        self.start_index
    }

    /// Return the number of requested logical entries.
    #[must_use]
    pub const fn length(self) -> usize {
        self.length
    }

    /// Return the exclusive logical end of the requested range.
    #[must_use]
    pub const fn end_index(self) -> usize {
        self.start_index.saturating_add(self.length)
    }

    /// Return the runtime-admitted per-query budget.
    #[must_use]
    pub const fn budget(self) -> VirtualLayoutBudget {
        self.budget
    }

    /// Return host-owned revision evidence for this request.
    #[must_use]
    pub const fn revisions(self) -> VirtualLayoutRevisions {
        self.revisions
    }

    /// Return the host data revision captured by this request.
    #[must_use]
    pub const fn data_revision(self) -> u64 {
        self.revisions.data
    }

    /// Return the host policy revision captured by this request.
    #[must_use]
    pub const fn policy_revision(self) -> u64 {
        self.revisions.policy
    }

    /// Return the host measurement revision captured by this request.
    #[must_use]
    pub const fn measurement_revision(self) -> u64 {
        self.revisions.measurement
    }

    /// Return the host semantic revision captured by this request.
    #[must_use]
    pub const fn semantic_revision(self) -> u64 {
        self.revisions.semantic
    }
}

/// Provider-owned result for one bounded item or range lookup.
///
/// `NoProvider` is deliberately absent.  The runtime synthesizes that
/// terminal outcome when an optional provider slot is not attached.
#[derive(Clone, Debug, PartialEq)]
pub enum VirtualLayoutSemanticProviderOutcome<T> {
    /// The provider supplied a complete result candidate.
    Found(T),
    /// The requested key or range is authoritative empty.
    NotFound,
    /// The provider cannot currently supply data or does not support it.
    Unavailable(VirtualLayoutSemanticUnavailableReason),
    /// The provider needs a later explicit retry.
    Deferred(VirtualLayoutSemanticDeferredReason),
    /// The provider rejected the request without supplying semantic evidence.
    Rejected,
}

/// Provider-owned unavailable reasons.  Missing providers are not forgeable
/// through this enum and are synthesized by the runtime.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum VirtualLayoutSemanticUnavailableReason {
    /// Required application data is not currently available.
    DataUnavailable,
    /// This provider does not support the requested semantic source.
    Unsupported,
}

/// Bounded provider-owned deferred reasons.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum VirtualLayoutSemanticDeferredReason {
    /// Application data is expected later.
    DataPending,
    /// Semantic data is expected later.
    SemanticPending,
    /// The provider requests an explicit retry.
    Retry,
}

/// One provider-supplied logical semantic entry.
#[derive(Clone, Debug, PartialEq)]
pub struct VirtualLayoutSemanticEntry {
    requested_key: VirtualLayoutItemKey,
    logical_index: usize,
    bounds: Rect,
    semantics: AutomationNodeSemantics,
    automation_node_id: AutomationNodeId,
}

impl VirtualLayoutSemanticEntry {
    /// Construct a logical semantic entry.
    #[must_use]
    pub fn new(
        requested_key: VirtualLayoutItemKey,
        logical_index: usize,
        bounds: Rect,
        semantics: AutomationNodeSemantics,
        automation_node_id: AutomationNodeId,
    ) -> Self {
        Self {
            requested_key,
            logical_index,
            bounds,
            semantics,
            automation_node_id,
        }
    }

    /// Return the stable key supplied for this entry.
    #[must_use]
    pub fn requested_key(&self) -> &VirtualLayoutItemKey {
        &self.requested_key
    }

    /// Return the logical index supplied for this entry.
    #[must_use]
    pub const fn logical_index(&self) -> usize {
        self.logical_index
    }

    /// Return the provider-supplied logical bounds.
    #[must_use]
    pub const fn bounds(&self) -> Rect {
        self.bounds
    }

    /// Return the provider-supplied semantic fields.
    #[must_use]
    pub fn semantics(&self) -> &AutomationNodeSemantics {
        &self.semantics
    }

    /// Return the provider-supplied stable automation node identity.
    #[must_use]
    pub fn automation_node_id(&self) -> &AutomationNodeId {
        &self.automation_node_id
    }

    fn into_private(self) -> crate::gui::layout_core::VirtualLayoutSemanticEntry {
        crate::gui::layout_core::VirtualLayoutSemanticEntry::new(
            self.requested_key,
            self.logical_index,
            self.bounds,
            self.semantics,
            self.automation_node_id,
        )
    }
}

/// Read-only synchronous provider for one required semantic item.
pub trait VirtualLayoutSemanticProvider {
    /// Resolve one exact required-item request.
    fn lookup(
        &self,
        request: &VirtualLayoutSemanticRequest,
    ) -> VirtualLayoutSemanticProviderOutcome<VirtualLayoutSemanticEntry>;
}

impl<F> VirtualLayoutSemanticProvider for F
where
    F: Fn(
            &VirtualLayoutSemanticRequest,
        ) -> VirtualLayoutSemanticProviderOutcome<VirtualLayoutSemanticEntry>
        + 'static,
{
    fn lookup(
        &self,
        request: &VirtualLayoutSemanticRequest,
    ) -> VirtualLayoutSemanticProviderOutcome<VirtualLayoutSemanticEntry> {
        self(request)
    }
}

/// Read-only synchronous provider for one contiguous semantic range.
pub trait VirtualLayoutSemanticRangeProvider {
    /// Resolve one exact bounded logical range request.
    fn lookup_range(
        &self,
        request: &VirtualLayoutSemanticRangeRequest,
    ) -> VirtualLayoutSemanticProviderOutcome<Vec<VirtualLayoutSemanticEntry>>;
}

impl<F> VirtualLayoutSemanticRangeProvider for F
where
    F: Fn(
            &VirtualLayoutSemanticRangeRequest,
        ) -> VirtualLayoutSemanticProviderOutcome<Vec<VirtualLayoutSemanticEntry>>
        + 'static,
{
    fn lookup_range(
        &self,
        request: &VirtualLayoutSemanticRangeRequest,
    ) -> VirtualLayoutSemanticProviderOutcome<Vec<VirtualLayoutSemanticEntry>> {
        self(request)
    }
}

struct PublicItemProviderAdapter {
    provider: Rc<dyn VirtualLayoutSemanticProvider>,
    in_call: Cell<bool>,
}

impl crate::gui::layout_core::VirtualLayoutSemanticProvider for PublicItemProviderAdapter {
    fn lookup(&self, request: &PrivateItemRequest) -> PrivateItemOutcome {
        if self.in_call.replace(true) {
            return PrivateItemOutcome::Rejected(
                VirtualLayoutSemanticRejectedReason::ProviderRejected,
            );
        }
        let request = VirtualLayoutSemanticRequest::from_private(request);
        let outcome = std::panic::catch_unwind(AssertUnwindSafe(|| self.provider.lookup(&request)));
        self.in_call.set(false);
        match outcome {
            Ok(VirtualLayoutSemanticProviderOutcome::Found(entry)) => {
                PrivateItemOutcome::Found(Box::new(entry.into_private()))
            }
            Ok(VirtualLayoutSemanticProviderOutcome::NotFound) => PrivateItemOutcome::NotFound,
            Ok(VirtualLayoutSemanticProviderOutcome::Unavailable(reason)) => {
                PrivateItemOutcome::Unavailable(match reason {
                    VirtualLayoutSemanticUnavailableReason::DataUnavailable => {
                        crate::gui::layout_core::VirtualLayoutSemanticUnavailableReason::DataUnavailable
                    }
                    VirtualLayoutSemanticUnavailableReason::Unsupported => {
                        crate::gui::layout_core::VirtualLayoutSemanticUnavailableReason::Unsupported
                    }
                })
            }
            Ok(VirtualLayoutSemanticProviderOutcome::Deferred(reason)) => {
                PrivateItemOutcome::Deferred(match reason {
                    VirtualLayoutSemanticDeferredReason::DataPending => {
                        crate::gui::layout_core::VirtualLayoutSemanticDeferredReason::DataPending
                    }
                    VirtualLayoutSemanticDeferredReason::SemanticPending => {
                        crate::gui::layout_core::VirtualLayoutSemanticDeferredReason::SemanticPending
                    }
                    VirtualLayoutSemanticDeferredReason::Retry => {
                        crate::gui::layout_core::VirtualLayoutSemanticDeferredReason::Retry
                    }
                })
            }
            Ok(VirtualLayoutSemanticProviderOutcome::Rejected) | Err(_) => {
                PrivateItemOutcome::Rejected(VirtualLayoutSemanticRejectedReason::ProviderRejected)
            }
        }
    }
}

struct PublicRangeProviderAdapter {
    provider: Rc<dyn VirtualLayoutSemanticRangeProvider>,
    in_call: Cell<bool>,
}

impl crate::gui::layout_core::VirtualLayoutSemanticRangeProvider for PublicRangeProviderAdapter {
    fn lookup_range(&self, request: &PrivateRangeRequest) -> PrivateRangeOutcome {
        if self.in_call.replace(true) {
            return PrivateRangeOutcome::Rejected(
                VirtualLayoutSemanticRejectedReason::ProviderRejected,
            );
        }
        let request = VirtualLayoutSemanticRangeRequest::from_private(request);
        let outcome =
            std::panic::catch_unwind(AssertUnwindSafe(|| self.provider.lookup_range(&request)));
        self.in_call.set(false);
        match outcome {
            Ok(VirtualLayoutSemanticProviderOutcome::Found(entries)) => {
                PrivateRangeOutcome::Found(
                    entries
                        .into_iter()
                        .map(VirtualLayoutSemanticEntry::into_private)
                        .collect(),
                )
            }
            Ok(VirtualLayoutSemanticProviderOutcome::NotFound) => PrivateRangeOutcome::NotFound,
            Ok(VirtualLayoutSemanticProviderOutcome::Unavailable(reason)) => {
                PrivateRangeOutcome::Unavailable(match reason {
                    VirtualLayoutSemanticUnavailableReason::DataUnavailable => {
                        crate::gui::layout_core::VirtualLayoutSemanticUnavailableReason::DataUnavailable
                    }
                    VirtualLayoutSemanticUnavailableReason::Unsupported => {
                        crate::gui::layout_core::VirtualLayoutSemanticUnavailableReason::Unsupported
                    }
                })
            }
            Ok(VirtualLayoutSemanticProviderOutcome::Deferred(reason)) => {
                PrivateRangeOutcome::Deferred(match reason {
                    VirtualLayoutSemanticDeferredReason::DataPending => {
                        crate::gui::layout_core::VirtualLayoutSemanticDeferredReason::DataPending
                    }
                    VirtualLayoutSemanticDeferredReason::SemanticPending => {
                        crate::gui::layout_core::VirtualLayoutSemanticDeferredReason::SemanticPending
                    }
                    VirtualLayoutSemanticDeferredReason::Retry => {
                        crate::gui::layout_core::VirtualLayoutSemanticDeferredReason::Retry
                    }
                })
            }
            Ok(VirtualLayoutSemanticProviderOutcome::Rejected) | Err(_) => {
                PrivateRangeOutcome::Rejected(VirtualLayoutSemanticRejectedReason::ProviderRejected)
            }
        }
    }
}

pub(crate) fn adapt_item_provider(
    provider: Rc<dyn VirtualLayoutSemanticProvider>,
) -> Rc<dyn crate::gui::layout_core::VirtualLayoutSemanticProvider> {
    Rc::new(PublicItemProviderAdapter {
        provider,
        in_call: Cell::new(false),
    })
}

pub(crate) fn adapt_range_provider(
    provider: Rc<dyn VirtualLayoutSemanticRangeProvider>,
) -> Rc<dyn crate::gui::layout_core::VirtualLayoutSemanticRangeProvider> {
    Rc::new(PublicRangeProviderAdapter {
        provider,
        in_call: Cell::new(false),
    })
}

pub(crate) fn provider_identity<T: ?Sized>(provider: &Rc<T>) -> usize {
    Rc::as_ptr(provider) as *const () as usize
}

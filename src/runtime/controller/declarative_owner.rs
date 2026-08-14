//! Private declarative owner projection and live-generation reconciliation.
//!
//! This module owns the controller-private live-generation boundary, the
//! conversion from an accepted explicit request into a private effect origin,
//! and the bounded retirement handoff to the existing effect registries.

#![allow(dead_code)]

use super::{SurfaceRuntime, owner::EffectOrigin};
use crate::{
    application::DeclarativeEffectOwner,
    layout::NodeId,
    runtime::{
        LayerKind, RuntimeBridge,
        surface::{SourceCompatibility, SourceIdentity, SourceTraversalIndex},
    },
};
use std::{
    cmp::Ordering as CmpOrdering,
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    marker::PhantomData,
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

type LocalOnly = PhantomData<Rc<()>>;

fn local_only() -> LocalOnly {
    PhantomData
}

/// Durable identity for one declarative owner kind and structural scope.
///
/// Owner kinds intentionally remain independent: an overlay and a keyed node
/// with the same structural scope never share a live generation.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum DeclarativeOwnerIdentity {
    Overlay { structural_scope: NodeId },
    KeyedNode { structural_scope: NodeId },
}

/// Compatibility evidence for one declarative owner identity.
///
/// This deliberately contains only the existing source compatibility facts.
/// It excludes resolved ids, traversal positions, callbacks, Rc addresses,
/// and widget paths.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DeclarativeOwnerCompatibility {
    Overlay {
        layer_kind: LayerKind,
        effect_owner: Option<DeclarativeEffectOwner>,
    },
    KeyedNode {
        origin: crate::application::DeclarativeIdentityOrigin,
        compatibility: SourceCompatibility,
        effect_owner: Option<DeclarativeEffectOwner>,
    },
}

#[derive(Debug)]
struct DeclarativeOwnerWitness {
    live: AtomicBool,
}

/// One exact controller-private live owner generation.
///
/// Clones share the witness, so retirement is visible to every captured clone.
/// The witness address is part of token equality in addition to the exact
/// identity and checked generation, preventing equal-looking tokens from two
/// runtime instances from binding to one another.
#[derive(Clone, Debug)]
pub(crate) struct DeclarativeOwnerToken {
    identity: DeclarativeOwnerIdentity,
    generation: u64,
    witness: Arc<DeclarativeOwnerWitness>,
}

impl DeclarativeOwnerToken {
    fn new(identity: DeclarativeOwnerIdentity, generation: u64) -> Self {
        Self {
            identity,
            generation,
            witness: Arc::new(DeclarativeOwnerWitness {
                live: AtomicBool::new(true),
            }),
        }
    }

    pub(crate) fn identity(&self) -> DeclarativeOwnerIdentity {
        self.identity
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }

    pub(crate) fn is_live(&self) -> bool {
        self.witness.live.load(Ordering::Acquire)
    }

    fn retire(&self) {
        self.witness.live.store(false, Ordering::Release);
    }
}

impl PartialEq for DeclarativeOwnerToken {
    fn eq(&self, other: &Self) -> bool {
        self.identity == other.identity
            && self.generation == other.generation
            && Arc::ptr_eq(&self.witness, &other.witness)
    }
}

impl Eq for DeclarativeOwnerToken {}

/// One current live record in the controller-owned declarative ledger.
#[derive(Clone, Debug)]
pub(crate) struct DeclarativeOwnerRecord {
    pub(crate) token: DeclarativeOwnerToken,
    pub(crate) compatibility: DeclarativeOwnerCompatibility,
}

impl DeclarativeOwnerRecord {
    fn new(
        identity: DeclarativeOwnerIdentity,
        generation: u64,
        compatibility: DeclarativeOwnerCompatibility,
    ) -> Self {
        Self {
            token: DeclarativeOwnerToken::new(identity, generation),
            compatibility,
        }
    }
}

/// One exact overlay candidate captured from declarative source metadata.
///
/// The structural scope is the durable identity.  The layer kind remains
/// separate compatibility evidence so a same-scope incompatible replacement
/// cannot silently reuse the old candidate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct DeclarativeOverlayCandidate {
    pub(crate) structural_scope: NodeId,
    pub(crate) layer_kind: LayerKind,
    pub(crate) effect_owner: Option<DeclarativeEffectOwner>,
    local_only: LocalOnly,
}

impl DeclarativeOverlayCandidate {
    fn new(structural_scope: NodeId, layer_kind: LayerKind) -> Self {
        Self {
            structural_scope,
            layer_kind,
            effect_owner: None,
            local_only: local_only(),
        }
    }

    fn with_effect_owner(mut self, effect_owner: Option<DeclarativeEffectOwner>) -> Self {
        self.effect_owner = effect_owner;
        self
    }

    fn owner_identity(self) -> DeclarativeOwnerIdentity {
        DeclarativeOwnerIdentity::Overlay {
            structural_scope: self.structural_scope,
        }
    }

    fn owner_compatibility(self) -> DeclarativeOwnerCompatibility {
        DeclarativeOwnerCompatibility::Overlay {
            layer_kind: self.layer_kind,
            effect_owner: self.effect_owner,
        }
    }
}

/// One exact keyed-node candidate captured from declarative source metadata.
///
/// `SourceIdentity::structural_scope` is the durable owner identity.  The
/// resolved id and identity origin remain compatibility evidence from the
/// final lowered surface; neither a traversal position nor a widget path is
/// used as identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct DeclarativeKeyedNodeCandidate {
    pub(crate) identity: SourceIdentity,
    pub(crate) compatibility: SourceCompatibility,
    pub(crate) effect_owner: Option<DeclarativeEffectOwner>,
    local_only: LocalOnly,
}

impl DeclarativeKeyedNodeCandidate {
    fn new(identity: SourceIdentity, compatibility: SourceCompatibility) -> Self {
        Self {
            identity,
            compatibility,
            effect_owner: None,
            local_only: local_only(),
        }
    }

    fn with_effect_owner(mut self, effect_owner: Option<DeclarativeEffectOwner>) -> Self {
        self.effect_owner = effect_owner;
        self
    }

    fn structural_scope(self) -> NodeId {
        self.identity.structural_scope
    }

    fn owner_identity(self) -> DeclarativeOwnerIdentity {
        DeclarativeOwnerIdentity::KeyedNode {
            structural_scope: self.identity.structural_scope,
        }
    }

    fn owner_compatibility(self) -> DeclarativeOwnerCompatibility {
        DeclarativeOwnerCompatibility::KeyedNode {
            origin: self.identity.origin,
            compatibility: self.compatibility,
            effect_owner: self.effect_owner,
        }
    }
}

impl DeclarativeOverlayCandidate {
    fn structural_scope(self) -> NodeId {
        self.structural_scope
    }
}

/// Source-local candidate context captured for one concrete source record.
///
/// This is intentionally separate from the accepted projection.  Callers may
/// retain this context across a refresh and resolve it against the current
/// accepted projection; removal then produces typed rejection rather than an
/// application or ancestor fallback.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct DeclarativeOwnerSourceContext {
    pub(crate) source_node: NodeId,
    metadata_present: bool,
    pub(crate) keyed_nodes: Vec<DeclarativeKeyedNodeCandidate>,
    pub(crate) overlays: Vec<DeclarativeOverlayCandidate>,
    local_only: LocalOnly,
}

impl DeclarativeOwnerSourceContext {
    fn new(
        source_node: NodeId,
        metadata_present: bool,
        keyed_nodes: Vec<DeclarativeKeyedNodeCandidate>,
        overlays: Vec<DeclarativeOverlayCandidate>,
    ) -> Self {
        Self {
            source_node,
            metadata_present,
            keyed_nodes,
            overlays,
            local_only: local_only(),
        }
    }

    pub(crate) fn metadata_present(&self) -> bool {
        self.metadata_present
    }
}

/// Private request vocabulary for one explicit declarative owner choice.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DeclarativeOwnerRequest {
    /// Keep ordinary work application-owned.
    Default,
    /// Explicitly detach from a declarative source while remaining
    /// application-owned.
    ApplicationOutlive,
    /// Select exactly this overlay candidate.
    Overlay(DeclarativeOverlayCandidate),
    /// Select exactly this keyed-node candidate.
    KeyedNode(DeclarativeKeyedNodeCandidate),
    /// Select the one current accepted candidate carrying this public handle.
    Handle(DeclarativeEffectOwner),
}

/// Typed rejection for a scoped owner request.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DeclarativeOwnerRejection {
    MissingSourceContext,
    IneligibleCandidate,
    AbsentCandidate,
    IncompatibleCandidate,
    AmbiguousCandidate,
    GenerationUnavailable,
}

/// Source-only resolution evidence before a live token is bound.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DeclarativeOwnerCandidateOutcome {
    Application,
    ApplicationOutlive,
    Overlay(DeclarativeOverlayCandidate),
    KeyedNode(DeclarativeKeyedNodeCandidate),
    Rejected(DeclarativeOwnerRejection),
}

/// Private resolution bound to one current live declarative token.
///
/// The controller consumes accepted resolutions through [`Self::effect_origin`]
/// while rejected evidence remains non-admitting.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum DeclarativeOwnerResolution {
    Application,
    ApplicationOutlive,
    Overlay(DeclarativeOwnerToken),
    KeyedNode(DeclarativeOwnerToken),
    Rejected(DeclarativeOwnerRejection),
}

impl DeclarativeOwnerResolution {
    /// Convert one resolved request into the private origin consumed by the
    /// existing controller dispatch and registry paths.  Rejected evidence
    /// cannot acquire an origin and therefore cannot admit work.
    pub(super) fn effect_origin(self) -> Option<EffectOrigin> {
        match self {
            Self::Application | Self::ApplicationOutlive => Some(EffectOrigin::Application),
            Self::Overlay(token) | Self::KeyedNode(token) => Some(EffectOrigin::Declarative(token)),
            Self::Rejected(_) => None,
        }
    }
}

/// Compatibility alias for the source-only outcome used by the preceding
/// projection stage.
pub(crate) type DeclarativeOwnerOutcome = DeclarativeOwnerCandidateOutcome;

#[derive(Clone, Copy, Debug)]
struct CapturedSourceRecord {
    source_node: NodeId,
    metadata_present: bool,
    keyed_start: usize,
    keyed_end: usize,
    overlay_start: usize,
    overlay_end: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct DeclarativeOwnerDescriptor {
    identity: DeclarativeOwnerIdentity,
    compatibility: DeclarativeOwnerCompatibility,
}

/// Controller-owned accepted declarative source projection.
///
/// The flat buffers are deliberately keyed by candidate identity during
/// normalization and lookup.  Their positions are storage details only, not
/// durable identity.  Clearing them retains useful allocation capacity while
/// removing every stale source and candidate entry.
#[derive(Debug, Default)]
pub(crate) struct DeclarativeOwnerProjection {
    captured_sources: Vec<CapturedSourceRecord>,
    captured_keyed_nodes: Vec<DeclarativeKeyedNodeCandidate>,
    captured_overlays: Vec<DeclarativeOverlayCandidate>,
    accepted_keyed_nodes: Vec<DeclarativeKeyedNodeCandidate>,
    accepted_overlays: Vec<DeclarativeOverlayCandidate>,
    installation_count: u64,
    local_only: LocalOnly,
}

impl DeclarativeOwnerProjection {
    /// Replace the accepted projection from one final authoritative source
    /// traversal.  The source-local capture is rebuilt alongside it but stays
    /// in separate storage from accepted candidates.
    pub(super) fn install_from_source(&mut self, source: &SourceTraversalIndex) {
        self.clear();

        for record in &source.records {
            let keyed_start = self.captured_keyed_nodes.len();
            let overlay_start = self.captured_overlays.len();
            if let Some(metadata) = &record.metadata {
                for keyed in &metadata.topology.keyed_nodes {
                    let candidate =
                        DeclarativeKeyedNodeCandidate::new(keyed.identity(), keyed.compatibility())
                            .with_effect_owner(keyed.effect_owner());
                    push_unique(
                        &mut self.captured_keyed_nodes,
                        keyed_start,
                        candidate,
                        keyed_candidates_compatible,
                    );
                    push_unique(
                        &mut self.accepted_keyed_nodes,
                        0,
                        candidate,
                        keyed_candidates_compatible,
                    );
                }
                for overlay in &metadata.topology.overlays {
                    let candidate = DeclarativeOverlayCandidate::new(
                        overlay.identity.structural_scope,
                        overlay.layer_kind,
                    )
                    .with_effect_owner(overlay.effect_owner);
                    push_unique(
                        &mut self.captured_overlays,
                        overlay_start,
                        candidate,
                        overlay_candidates_compatible,
                    );
                    push_unique(
                        &mut self.accepted_overlays,
                        0,
                        candidate,
                        overlay_candidates_compatible,
                    );
                }
            }
            self.captured_sources.push(CapturedSourceRecord {
                source_node: record.node_id,
                metadata_present: record.metadata.is_some(),
                keyed_start,
                keyed_end: self.captured_keyed_nodes.len(),
                overlay_start,
                overlay_end: self.captured_overlays.len(),
            });
        }

        self.installation_count = self.installation_count.saturating_add(1);
    }

    fn clear(&mut self) {
        self.captured_sources.clear();
        self.captured_keyed_nodes.clear();
        self.captured_overlays.clear();
        self.accepted_keyed_nodes.clear();
        self.accepted_overlays.clear();
    }

    /// Capture the source-local context for one final source record.
    pub(crate) fn captured_context(
        &self,
        source_node: NodeId,
    ) -> Option<DeclarativeOwnerSourceContext> {
        let mut matches = self
            .captured_sources
            .iter()
            .filter(|record| record.source_node == source_node);
        let record = matches.next()?;
        if matches.next().is_some() {
            return None;
        }
        Some(DeclarativeOwnerSourceContext::new(
            record.source_node,
            record.metadata_present,
            self.captured_keyed_nodes[record.keyed_start..record.keyed_end].to_vec(),
            self.captured_overlays[record.overlay_start..record.overlay_end].to_vec(),
        ))
    }

    pub(crate) fn accepted_keyed_nodes(&self) -> &[DeclarativeKeyedNodeCandidate] {
        &self.accepted_keyed_nodes
    }

    pub(crate) fn accepted_overlays(&self) -> &[DeclarativeOverlayCandidate] {
        &self.accepted_overlays
    }

    pub(crate) fn installation_count(&self) -> u64 {
        self.installation_count
    }

    pub(crate) fn storage_capacities(&self) -> (usize, usize, usize, usize, usize) {
        (
            self.captured_sources.capacity(),
            self.captured_keyed_nodes.capacity(),
            self.captured_overlays.capacity(),
            self.accepted_keyed_nodes.capacity(),
            self.accepted_overlays.capacity(),
        )
    }

    /// Resolve one public handle against exactly one current accepted
    /// candidate.  The accepted projection is authoritative: captured source
    /// context, ancestry, traversal order, and source precedence are not
    /// consulted here.
    pub(crate) fn resolve_handle_candidates(
        &self,
        handle: DeclarativeEffectOwner,
    ) -> DeclarativeOwnerCandidateOutcome {
        let mut matches = Vec::new();
        for candidate in self
            .accepted_keyed_nodes
            .iter()
            .copied()
            .filter(|candidate| candidate.effect_owner == Some(handle))
        {
            matches.push(DeclarativeOwnerCandidateOutcome::KeyedNode(candidate));
        }
        for candidate in self
            .accepted_overlays
            .iter()
            .copied()
            .filter(|candidate| candidate.effect_owner == Some(handle))
        {
            matches.push(DeclarativeOwnerCandidateOutcome::Overlay(candidate));
        }

        match matches.as_slice() {
            [candidate] => *candidate,
            [] => DeclarativeOwnerCandidateOutcome::Rejected(
                DeclarativeOwnerRejection::AbsentCandidate,
            ),
            _ => DeclarativeOwnerCandidateOutcome::Rejected(
                DeclarativeOwnerRejection::AmbiguousCandidate,
            ),
        }
    }

    /// Bind one public handle to the current live owner generation.
    pub(crate) fn resolve_handle(
        &self,
        handle: DeclarativeEffectOwner,
        ledger: &DeclarativeOwnerLedger,
    ) -> DeclarativeOwnerResolution {
        match self.resolve_handle_candidates(handle) {
            DeclarativeOwnerCandidateOutcome::Overlay(candidate) => ledger
                .resolve_live(candidate.owner_identity(), candidate.owner_compatibility())
                .map(DeclarativeOwnerResolution::Overlay)
                .unwrap_or_else(DeclarativeOwnerResolution::Rejected),
            DeclarativeOwnerCandidateOutcome::KeyedNode(candidate) => ledger
                .resolve_live(candidate.owner_identity(), candidate.owner_compatibility())
                .map(DeclarativeOwnerResolution::KeyedNode)
                .unwrap_or_else(DeclarativeOwnerResolution::Rejected),
            DeclarativeOwnerCandidateOutcome::Rejected(rejection) => {
                DeclarativeOwnerResolution::Rejected(rejection)
            }
            DeclarativeOwnerCandidateOutcome::Application
            | DeclarativeOwnerCandidateOutcome::ApplicationOutlive => {
                DeclarativeOwnerResolution::Rejected(DeclarativeOwnerRejection::IneligibleCandidate)
            }
        }
    }

    /// Resolve one request against captured source context and the current
    /// accepted projection.  Every scoped request is exact and typed: a
    /// failure never becomes application-owned and never searches an ancestor.
    pub(crate) fn resolve_candidates(
        &self,
        request: DeclarativeOwnerRequest,
        context: Option<&DeclarativeOwnerSourceContext>,
    ) -> DeclarativeOwnerCandidateOutcome {
        match request {
            DeclarativeOwnerRequest::Default => DeclarativeOwnerCandidateOutcome::Application,
            DeclarativeOwnerRequest::ApplicationOutlive => {
                DeclarativeOwnerCandidateOutcome::ApplicationOutlive
            }
            DeclarativeOwnerRequest::Overlay(candidate) => match resolve_candidate(
                context,
                candidate,
                context.map(|context| context.overlays.as_slice()),
                &self.accepted_overlays,
                DeclarativeOverlayCandidate::structural_scope,
                overlay_candidates_compatible,
            ) {
                Ok(candidate) => DeclarativeOwnerCandidateOutcome::Overlay(candidate),
                Err(rejection) => DeclarativeOwnerCandidateOutcome::Rejected(rejection),
            },
            DeclarativeOwnerRequest::KeyedNode(candidate) => {
                if !candidate.identity.origin.is_keyed() {
                    return DeclarativeOwnerCandidateOutcome::Rejected(
                        DeclarativeOwnerRejection::IneligibleCandidate,
                    );
                }
                match resolve_candidate(
                    context,
                    candidate,
                    context.map(|context| context.keyed_nodes.as_slice()),
                    &self.accepted_keyed_nodes,
                    DeclarativeKeyedNodeCandidate::structural_scope,
                    keyed_candidates_compatible,
                ) {
                    Ok(candidate) => DeclarativeOwnerCandidateOutcome::KeyedNode(candidate),
                    Err(rejection) => DeclarativeOwnerCandidateOutcome::Rejected(rejection),
                }
            }
            DeclarativeOwnerRequest::Handle(handle) => self.resolve_handle_candidates(handle),
        }
    }

    /// Resolve one request and bind its accepted candidate to the current
    /// live generation.  Application-owned outcomes do not consult the
    /// declarative ledger.
    pub(crate) fn resolve(
        &self,
        request: DeclarativeOwnerRequest,
        context: Option<&DeclarativeOwnerSourceContext>,
        ledger: &DeclarativeOwnerLedger,
    ) -> DeclarativeOwnerResolution {
        match self.resolve_candidates(request, context) {
            DeclarativeOwnerCandidateOutcome::Application => {
                DeclarativeOwnerResolution::Application
            }
            DeclarativeOwnerCandidateOutcome::ApplicationOutlive => {
                DeclarativeOwnerResolution::ApplicationOutlive
            }
            DeclarativeOwnerCandidateOutcome::Overlay(candidate) => ledger
                .resolve_live(candidate.owner_identity(), candidate.owner_compatibility())
                .map(DeclarativeOwnerResolution::Overlay)
                .unwrap_or_else(DeclarativeOwnerResolution::Rejected),
            DeclarativeOwnerCandidateOutcome::KeyedNode(candidate) => ledger
                .resolve_live(candidate.owner_identity(), candidate.owner_compatibility())
                .map(DeclarativeOwnerResolution::KeyedNode)
                .unwrap_or_else(DeclarativeOwnerResolution::Rejected),
            DeclarativeOwnerCandidateOutcome::Rejected(rejection) => {
                DeclarativeOwnerResolution::Rejected(rejection)
            }
        }
    }

    /// Resolve against the captured context for one source record without
    /// allowing a missing or duplicate source id to select another context.
    pub(crate) fn resolve_candidates_for_source(
        &self,
        request: DeclarativeOwnerRequest,
        source_node: NodeId,
    ) -> DeclarativeOwnerCandidateOutcome {
        if matches!(
            request,
            DeclarativeOwnerRequest::Default
                | DeclarativeOwnerRequest::ApplicationOutlive
                | DeclarativeOwnerRequest::Handle(_)
        ) {
            return self.resolve_candidates(request, None);
        }
        let mut matches = self
            .captured_sources
            .iter()
            .filter(|record| record.source_node == source_node);
        let Some(record) = matches.next() else {
            return DeclarativeOwnerCandidateOutcome::Rejected(
                DeclarativeOwnerRejection::MissingSourceContext,
            );
        };
        if matches.next().is_some() {
            return DeclarativeOwnerCandidateOutcome::Rejected(
                DeclarativeOwnerRejection::AmbiguousCandidate,
            );
        }
        let context = DeclarativeOwnerSourceContext::new(
            record.source_node,
            record.metadata_present,
            self.captured_keyed_nodes[record.keyed_start..record.keyed_end].to_vec(),
            self.captured_overlays[record.overlay_start..record.overlay_end].to_vec(),
        );
        self.resolve_candidates(request, Some(&context))
    }

    pub(crate) fn resolve_for_source(
        &self,
        request: DeclarativeOwnerRequest,
        source_node: NodeId,
        ledger: &DeclarativeOwnerLedger,
    ) -> DeclarativeOwnerResolution {
        match request {
            DeclarativeOwnerRequest::Default
            | DeclarativeOwnerRequest::ApplicationOutlive
            | DeclarativeOwnerRequest::Handle(_) => self.resolve(request, None, ledger),
            _ => {
                let mut matches = self
                    .captured_sources
                    .iter()
                    .filter(|record| record.source_node == source_node);
                let Some(record) = matches.next() else {
                    return DeclarativeOwnerResolution::Rejected(
                        DeclarativeOwnerRejection::MissingSourceContext,
                    );
                };
                if matches.next().is_some() {
                    return DeclarativeOwnerResolution::Rejected(
                        DeclarativeOwnerRejection::AmbiguousCandidate,
                    );
                }
                let context = DeclarativeOwnerSourceContext::new(
                    record.source_node,
                    record.metadata_present,
                    self.captured_keyed_nodes[record.keyed_start..record.keyed_end].to_vec(),
                    self.captured_overlays[record.overlay_start..record.overlay_end].to_vec(),
                );
                self.resolve(request, Some(&context), ledger)
            }
        }
    }
}

/// Controller-owned live-generation ledger for the accepted declarative
/// projection.
///
/// `live` contains at most one record for each canonical identity.  The
/// canonical descriptor buffer is sorted before reconciliation so allocation
/// order is independent of source traversal and keyed-child order.
#[derive(Debug, Default)]
pub(crate) struct DeclarativeOwnerLedger {
    pub(crate) live: Vec<DeclarativeOwnerRecord>,
    next_generation: u64,
    next_live: Vec<DeclarativeOwnerRecord>,
    canonical: Vec<DeclarativeOwnerDescriptor>,
    generation_unavailable: Vec<DeclarativeOwnerIdentity>,
    retired_tokens: Vec<DeclarativeOwnerToken>,
    reconciliation_count: u64,
}

impl DeclarativeOwnerLedger {
    /// Reconcile only the accepted projection.  Provisional source probes are
    /// never passed here by the controller installation boundary.
    pub(super) fn reconcile(&mut self, projection: &DeclarativeOwnerProjection) {
        self.reconciliation_count = self.reconciliation_count.saturating_add(1);
        canonical_descriptors(projection, &mut self.canonical);
        self.generation_unavailable.clear();

        let allocations_needed = self
            .canonical
            .iter()
            .filter(|descriptor| !self.has_live_compatible(**descriptor))
            .count();
        if !self.can_allocate(allocations_needed) {
            for descriptor in &self.canonical {
                if !self.has_live_compatible(*descriptor)
                    && !self.generation_unavailable.contains(&descriptor.identity)
                {
                    self.generation_unavailable.push(descriptor.identity);
                }
            }
            self.retain_current_identities_after_exhaustion();
            return;
        }

        // Retirement happens before any replacement record is published.
        let retired = self
            .live
            .iter()
            .filter(|record| {
                !self.canonical.contains(&DeclarativeOwnerDescriptor {
                    identity: record.token.identity(),
                    compatibility: record.compatibility,
                })
            })
            .map(|record| record.token.clone())
            .collect::<Vec<_>>();
        for token in retired {
            self.retire_token(&token);
        }

        self.next_live.clear();
        let mut next_generation = self.next_generation;
        for descriptor in &self.canonical {
            if let Some(record) = self.live.iter().find(|record| {
                record.token.is_live()
                    && record.token.identity() == descriptor.identity
                    && record.compatibility == descriptor.compatibility
            }) {
                self.next_live.push(record.clone());
                continue;
            }

            let Some(generation) = next_generation.checked_add(1) else {
                // The preflight above makes this unreachable without a
                // concurrent mutation, but keep the boundary fail-closed.
                self.generation_unavailable.push(descriptor.identity);
                self.next_live.clear();
                self.retain_current_identities_after_exhaustion();
                return;
            };
            next_generation = generation;
            self.next_live.push(DeclarativeOwnerRecord::new(
                descriptor.identity,
                generation,
                descriptor.compatibility,
            ));
        }
        self.next_generation = next_generation;

        std::mem::swap(&mut self.live, &mut self.next_live);
        self.next_live.clear();
    }

    pub(crate) fn resolve_live(
        &self,
        identity: DeclarativeOwnerIdentity,
        compatibility: DeclarativeOwnerCompatibility,
    ) -> Result<DeclarativeOwnerToken, DeclarativeOwnerRejection> {
        if self.generation_unavailable.contains(&identity) {
            return Err(DeclarativeOwnerRejection::GenerationUnavailable);
        }

        let mut matching = None;
        let mut matching_count = 0;
        for record in &self.live {
            if record.token.is_live()
                && record.token.identity() == identity
                && record.compatibility == compatibility
            {
                matching_count += 1;
                matching = Some(record.token.clone());
            }
        }
        match (matching_count, matching) {
            (1, Some(token)) => Ok(token),
            (count, _) if count > 1 => Err(DeclarativeOwnerRejection::AmbiguousCandidate),
            _ if self
                .live
                .iter()
                .any(|record| record.token.is_live() && record.token.identity() == identity) =>
            {
                Err(DeclarativeOwnerRejection::IncompatibleCandidate)
            }
            _ => Err(DeclarativeOwnerRejection::AbsentCandidate),
        }
    }

    pub(crate) fn is_live(&self, token: &DeclarativeOwnerToken) -> bool {
        token.is_live() && self.live.iter().any(|record| record.token == *token)
    }

    pub(crate) fn retire_all(&mut self) {
        for record in &self.live {
            record.token.retire();
        }
        self.live.clear();
        self.next_live.clear();
        self.canonical.clear();
        self.generation_unavailable.clear();
    }

    pub(crate) fn live_records(&self) -> &[DeclarativeOwnerRecord] {
        &self.live
    }

    pub(crate) fn next_generation(&self) -> u64 {
        self.next_generation
    }

    pub(crate) fn reconciliation_count(&self) -> u64 {
        self.reconciliation_count
    }

    /// Drain the exact live-generation tokens retired by reconciliation.
    ///
    /// The controller consumes this handoff immediately after installing an
    /// accepted projection, before any later registry mapping or reduction.
    /// The backing allocation remains reusable across refreshes.
    pub(super) fn drain_retired_tokens(&mut self) -> Vec<DeclarativeOwnerToken> {
        self.retired_tokens.drain(..).collect()
    }

    fn has_live_compatible(&self, descriptor: DeclarativeOwnerDescriptor) -> bool {
        self.live.iter().any(|record| {
            record.token.is_live()
                && record.token.identity() == descriptor.identity
                && record.compatibility == descriptor.compatibility
        })
    }

    fn can_allocate(&self, count: usize) -> bool {
        u64::try_from(count)
            .ok()
            .and_then(|count| self.next_generation.checked_add(count))
            .is_some()
    }

    fn retain_current_identities_after_exhaustion(&mut self) {
        self.next_live.clear();
        let current = self.live.clone();
        for record in current {
            let current_descriptor = self.canonical.iter().find(|descriptor| {
                descriptor.identity == record.token.identity()
                    && descriptor.compatibility == record.compatibility
            });
            if current_descriptor.is_some() && record.token.is_live() {
                self.next_live.push(record);
            } else {
                self.retire_token(&record.token);
            }
        }
        std::mem::swap(&mut self.live, &mut self.next_live);
        self.next_live.clear();
    }

    fn retire_token(&mut self, token: &DeclarativeOwnerToken) {
        if !token.is_live() {
            return;
        }
        token.retire();
        self.retired_tokens.push(token.clone());
    }
}

fn canonical_descriptors(
    projection: &DeclarativeOwnerProjection,
    descriptors: &mut Vec<DeclarativeOwnerDescriptor>,
) {
    descriptors.clear();
    descriptors.extend(
        projection
            .accepted_keyed_nodes
            .iter()
            .copied()
            .filter(|candidate| candidate.identity.origin.is_keyed())
            .map(|candidate| DeclarativeOwnerDescriptor {
                identity: candidate.owner_identity(),
                compatibility: candidate.owner_compatibility(),
            }),
    );
    descriptors.extend(
        projection
            .accepted_overlays
            .iter()
            .copied()
            .map(|candidate| DeclarativeOwnerDescriptor {
                identity: candidate.owner_identity(),
                compatibility: candidate.owner_compatibility(),
            }),
    );
    descriptors.sort_unstable_by(descriptor_order);

    // Compact each identity group in place.  A group with one compatibility
    // is canonical; a group with more than one is ambiguous and is excluded.
    let mut read = 0;
    let mut write = 0;
    while read < descriptors.len() {
        let first = descriptors[read];
        let mut compatible = true;
        read += 1;
        while read < descriptors.len() && descriptors[read].identity == first.identity {
            compatible &= descriptors[read].compatibility == first.compatibility;
            read += 1;
        }
        if compatible {
            descriptors[write] = first;
            write += 1;
        }
    }
    descriptors.truncate(write);
}

fn descriptor_order(
    first: &DeclarativeOwnerDescriptor,
    second: &DeclarativeOwnerDescriptor,
) -> CmpOrdering {
    identity_order(&first.identity, &second.identity)
        .then_with(|| compatibility_order(&first.compatibility, &second.compatibility))
}

fn identity_order(
    first: &DeclarativeOwnerIdentity,
    second: &DeclarativeOwnerIdentity,
) -> CmpOrdering {
    let first_kind = match first {
        DeclarativeOwnerIdentity::Overlay { .. } => 0,
        DeclarativeOwnerIdentity::KeyedNode { .. } => 1,
    };
    let second_kind = match second {
        DeclarativeOwnerIdentity::Overlay { .. } => 0,
        DeclarativeOwnerIdentity::KeyedNode { .. } => 1,
    };
    first_kind
        .cmp(&second_kind)
        .then_with(|| owner_scope(*first).cmp(&owner_scope(*second)))
}

fn owner_scope(identity: DeclarativeOwnerIdentity) -> NodeId {
    match identity {
        DeclarativeOwnerIdentity::Overlay { structural_scope }
        | DeclarativeOwnerIdentity::KeyedNode { structural_scope } => structural_scope,
    }
}

fn compatibility_order(
    first: &DeclarativeOwnerCompatibility,
    second: &DeclarativeOwnerCompatibility,
) -> CmpOrdering {
    match (first, second) {
        (
            DeclarativeOwnerCompatibility::Overlay {
                layer_kind: first_layer,
                effect_owner: first_owner,
            },
            DeclarativeOwnerCompatibility::Overlay {
                layer_kind: second_layer,
                effect_owner: second_owner,
            },
        ) => first_layer
            .z_order()
            .cmp(&second_layer.z_order())
            .then_with(|| first_owner.cmp(second_owner)),
        (
            DeclarativeOwnerCompatibility::KeyedNode {
                origin: first_origin,
                compatibility: first_compatibility,
                effect_owner: first_owner,
            },
            DeclarativeOwnerCompatibility::KeyedNode {
                origin: second_origin,
                compatibility: second_compatibility,
                effect_owner: second_owner,
            },
        ) => origin_order(*first_origin)
            .cmp(&origin_order(*second_origin))
            .then_with(|| source_compatibility_order(first_compatibility, second_compatibility))
            .then_with(|| first_owner.cmp(second_owner)),
        _ => CmpOrdering::Equal,
    }
}

fn origin_order(origin: crate::application::DeclarativeIdentityOrigin) -> u8 {
    match origin {
        crate::application::DeclarativeIdentityOrigin::GeneratedStructural => 0,
        crate::application::DeclarativeIdentityOrigin::ExplicitNumericId => 1,
        crate::application::DeclarativeIdentityOrigin::ExplicitContinuityKey => 2,
        crate::application::DeclarativeIdentityOrigin::InferredKeyedIdentity => 3,
        crate::application::DeclarativeIdentityOrigin::UnreidentifiedDirectRuntimeRoot => 4,
    }
}

fn source_compatibility_order(
    first: &SourceCompatibility,
    second: &SourceCompatibility,
) -> CmpOrdering {
    surface_kind_order(&first.surface_kind)
        .cmp(&surface_kind_order(&second.surface_kind))
        .then_with(|| {
            first
                .widget_compatibility_kind
                .cmp(&second.widget_compatibility_kind)
        })
}

fn surface_kind_order<T: Hash>(kind: &T) -> u64 {
    let mut hasher = DefaultHasher::new();
    kind.hash(&mut hasher);
    hasher.finish()
}

fn push_unique<T: Copy>(
    entries: &mut Vec<T>,
    start: usize,
    candidate: T,
    compatible: impl Fn(T, T) -> bool,
) {
    if entries[start..]
        .iter()
        .copied()
        .any(|existing| compatible(existing, candidate))
    {
        return;
    }
    // Keep distinct same-scope evidence so resolution can report ambiguity;
    // only repeated identical ancestry records are collapsed.
    entries.push(candidate);
}

fn resolve_candidate<T: Copy>(
    context: Option<&DeclarativeOwnerSourceContext>,
    requested: T,
    captured: Option<&[T]>,
    accepted: &[T],
    structural_scope: impl Fn(T) -> NodeId,
    compatible: impl Fn(T, T) -> bool,
) -> Result<T, DeclarativeOwnerRejection> {
    let Some(context) = context else {
        return Err(DeclarativeOwnerRejection::MissingSourceContext);
    };
    if !context.metadata_present {
        return Err(DeclarativeOwnerRejection::MissingSourceContext);
    }
    let Some(captured) = captured else {
        return Err(DeclarativeOwnerRejection::MissingSourceContext);
    };
    let requested_scope = structural_scope(requested);
    let captured_matches =
        distinct_scope_matches(captured, requested_scope, &structural_scope, &compatible);
    match captured_matches.as_slice() {
        [] => return Err(DeclarativeOwnerRejection::IneligibleCandidate),
        [candidate] if compatible(*candidate, requested) => {}
        [_] => return Err(DeclarativeOwnerRejection::IncompatibleCandidate),
        _ => return Err(DeclarativeOwnerRejection::AmbiguousCandidate),
    }

    let accepted_matches =
        distinct_scope_matches(accepted, requested_scope, &structural_scope, &compatible);
    match accepted_matches.as_slice() {
        [] => Err(DeclarativeOwnerRejection::AbsentCandidate),
        [candidate] if compatible(*candidate, requested) => Ok(*candidate),
        [_] => Err(DeclarativeOwnerRejection::IncompatibleCandidate),
        _ => Err(DeclarativeOwnerRejection::AmbiguousCandidate),
    }
}

fn distinct_scope_matches<T: Copy>(
    entries: &[T],
    scope: NodeId,
    structural_scope: &impl Fn(T) -> NodeId,
    compatible: &impl Fn(T, T) -> bool,
) -> Vec<T> {
    let mut matches = Vec::new();
    for entry in entries
        .iter()
        .copied()
        .filter(|entry| structural_scope(*entry) == scope)
    {
        if !matches
            .iter()
            .copied()
            .any(|existing| compatible(existing, entry))
        {
            matches.push(entry);
        }
    }
    matches
}

fn overlay_candidates_compatible(
    first: DeclarativeOverlayCandidate,
    second: DeclarativeOverlayCandidate,
) -> bool {
    first.structural_scope == second.structural_scope
        && first.layer_kind == second.layer_kind
        && first.effect_owner == second.effect_owner
}

fn keyed_candidates_compatible(
    first: DeclarativeKeyedNodeCandidate,
    second: DeclarativeKeyedNodeCandidate,
) -> bool {
    first.identity.structural_scope == second.identity.structural_scope
        && first.identity.origin == second.identity.origin
        && first.compatibility == second.compatibility
        && first.effect_owner == second.effect_owner
}

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(super) fn install_declarative_owner_projection(&mut self) {
        self.declarative_owner
            .install_from_source(&self.scratch.projection_source);
        self.declarative_owner_ledger
            .reconcile(&self.declarative_owner);

        // Reconciliation is the accepted-projection retirement boundary.  A
        // retired token is removed from every existing registry before any
        // later completion, wake, or result can reach mapping or reduction.
        for token in self.declarative_owner_ledger.drain_retired_tokens() {
            let origin = EffectOrigin::Declarative(token);
            self.worker_effects.retire_origin(&origin);
            self.timer_effects.retire_origin(&origin);
            self.platform_registry.retire_origin(&origin);
        }
    }

    pub(crate) fn declarative_owner_projection(&self) -> &DeclarativeOwnerProjection {
        &self.declarative_owner
    }

    pub(crate) fn declarative_owner_ledger(&self) -> &DeclarativeOwnerLedger {
        &self.declarative_owner_ledger
    }

    pub(super) fn declarative_owner_origin(
        &self,
        request: DeclarativeOwnerRequest,
        source_node: NodeId,
    ) -> Option<EffectOrigin> {
        self.declarative_owner
            .resolve_for_source(request, source_node, &self.declarative_owner_ledger)
            .effect_origin()
    }

    pub(super) fn declarative_owner_origin_for_handle(
        &self,
        handle: DeclarativeEffectOwner,
    ) -> Option<EffectOrigin> {
        if !self.lifecycle_accepts_work() {
            return None;
        }
        self.declarative_owner
            .resolve_handle(handle, &self.declarative_owner_ledger)
            .effect_origin()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        application::{IntoView, Layer, column, for_each_by, overlays, scene, text},
        gui::types::Vector2,
        layout::ContainerPolicy,
        runtime::{
            Command, CommandOutcome, DeclarativeOwnedCommandRuntimeBridge, SurfaceNode, UiSurface,
        },
    };
    use std::{
        cell::{Cell, RefCell},
        rc::Rc,
    };

    fn source_from_surface(surface: UiSurface<()>) -> SourceTraversalIndex {
        surface.runtime_source_traversal_index()
    }

    fn raw_source() -> SourceTraversalIndex {
        source_from_surface(UiSurface::new(SurfaceNode::container(
            1,
            ContainerPolicy::default(),
            Vec::new(),
        )))
    }

    fn keyed_surface() -> UiSurface<()> {
        column([text::<()>("keyed").key("keyed")]).into_surface()
    }

    fn keyed_surface_for<Message: 'static>() -> UiSurface<Message> {
        column([text::<Message>("keyed").key("keyed")]).into_surface()
    }

    fn overlay_surface() -> UiSurface<()> {
        scene(text::<()>("base"))
            .layer(Layer::modal(text("overlay")))
            .into_view()
            .into_surface()
    }

    fn overlay_surface_for<Message: 'static>() -> UiSurface<Message> {
        scene(text::<Message>("base"))
            .layer(Layer::modal(text("overlay")))
            .into_view()
            .into_surface()
    }

    fn context_and_projection(
        surface: UiSurface<()>,
    ) -> (DeclarativeOwnerProjection, DeclarativeOwnerSourceContext) {
        let source = source_from_surface(surface);
        let mut projection = DeclarativeOwnerProjection::default();
        projection.install_from_source(&source);
        let keyed_scope = projection
            .accepted_keyed_nodes
            .first()
            .map(|candidate| candidate.structural_scope());
        let overlay_scope = projection
            .accepted_overlays
            .first()
            .map(|candidate| candidate.structural_scope());
        let source_node = source
            .records
            .iter()
            .find_map(|record| {
                let metadata = record.metadata.as_ref()?;
                let has_keyed = keyed_scope.is_some_and(|scope| {
                    metadata
                        .topology
                        .keyed_nodes
                        .iter()
                        .any(|candidate| candidate.identity().structural_scope == scope)
                });
                let has_overlay = overlay_scope.is_some_and(|scope| {
                    metadata
                        .topology
                        .overlays
                        .iter()
                        .any(|candidate| candidate.identity.structural_scope == scope)
                });
                (has_keyed || has_overlay).then_some(record.node_id)
            })
            .expect("metadata source record");
        let context = projection
            .captured_context(source_node)
            .expect("unique source context");
        (projection, context)
    }

    fn keyed_candidate(projection: &DeclarativeOwnerProjection) -> DeclarativeKeyedNodeCandidate {
        projection
            .accepted_keyed_nodes()
            .first()
            .copied()
            .expect("keyed candidate")
    }

    fn overlay_candidate(projection: &DeclarativeOwnerProjection) -> DeclarativeOverlayCandidate {
        projection
            .accepted_overlays()
            .first()
            .copied()
            .expect("overlay candidate")
    }

    fn first_context_with_keyed_node(
        projection: &DeclarativeOwnerProjection,
    ) -> DeclarativeOwnerSourceContext {
        projection
            .captured_sources
            .iter()
            .filter_map(|record| projection.captured_context(record.source_node))
            .find(|context| !context.keyed_nodes.is_empty())
            .expect("keyed source context")
    }

    fn first_context_with_overlay(
        projection: &DeclarativeOwnerProjection,
    ) -> DeclarativeOwnerSourceContext {
        projection
            .captured_sources
            .iter()
            .filter_map(|record| projection.captured_context(record.source_node))
            .find(|context| !context.overlays.is_empty())
            .expect("overlay source context")
    }

    fn live_token(
        ledger: &DeclarativeOwnerLedger,
        identity: DeclarativeOwnerIdentity,
    ) -> DeclarativeOwnerToken {
        ledger
            .live_records()
            .iter()
            .find(|record| record.token.identity() == identity)
            .map(|record| record.token.clone())
            .expect("live owner token")
    }

    fn synthetic_keyed(
        structural_scope: NodeId,
        resolved_id: NodeId,
        compatibility: SourceCompatibility,
    ) -> DeclarativeKeyedNodeCandidate {
        DeclarativeKeyedNodeCandidate::new(
            SourceIdentity {
                resolved_id,
                structural_scope,
                origin: crate::application::DeclarativeIdentityOrigin::ExplicitContinuityKey,
            },
            compatibility,
        )
    }

    #[test]
    fn handle_resolution_rejects_absent_ambiguous_and_unkeyed_candidates() {
        let handle = DeclarativeEffectOwner::new();
        let other_handle = DeclarativeEffectOwner::new();
        let source = source_from_surface(
            column([
                text::<()>("first").key("first").effect_owner(handle),
                text::<()>("second")
                    .key("second")
                    .effect_owner(other_handle),
            ])
            .into_surface(),
        );
        let mut projection = DeclarativeOwnerProjection::default();
        projection.install_from_source(&source);
        let mut ledger = DeclarativeOwnerLedger::default();
        ledger.reconcile(&projection);

        assert!(matches!(
            projection.resolve_handle(handle, &ledger),
            DeclarativeOwnerResolution::KeyedNode(_)
        ));
        assert_eq!(
            projection.resolve_handle(DeclarativeEffectOwner::new(), &ledger),
            DeclarativeOwnerResolution::Rejected(DeclarativeOwnerRejection::AbsentCandidate)
        );

        let shared_handle = DeclarativeEffectOwner::new();
        let source = source_from_surface(
            column([
                text::<()>("first").key("first").effect_owner(shared_handle),
                text::<()>("second")
                    .key("second")
                    .effect_owner(shared_handle),
            ])
            .into_surface(),
        );
        let mut projection = DeclarativeOwnerProjection::default();
        projection.install_from_source(&source);
        let mut ledger = DeclarativeOwnerLedger::default();
        ledger.reconcile(&projection);

        assert_eq!(
            projection.resolve_handle(shared_handle, &ledger),
            DeclarativeOwnerResolution::Rejected(DeclarativeOwnerRejection::AmbiguousCandidate)
        );

        let unkeyed_handle = DeclarativeEffectOwner::new();
        let source = source_from_surface(
            column([text::<()>("unkeyed").effect_owner(unkeyed_handle)]).into_surface(),
        );
        let mut projection = DeclarativeOwnerProjection::default();
        projection.install_from_source(&source);
        let mut ledger = DeclarativeOwnerLedger::default();
        ledger.reconcile(&projection);

        assert_eq!(
            projection.resolve_handle(unkeyed_handle, &ledger),
            DeclarativeOwnerResolution::Rejected(DeclarativeOwnerRejection::AbsentCandidate)
        );
    }

    #[test]
    fn ledger_allocates_once_and_preserves_equivalent_reprojection_and_reorder() {
        let before = column([text::<()>("a").key("a"), text::<()>("b").key("b")]).into_surface();
        let after = column([text::<()>("b").key("b"), text::<()>("a").key("a")]).into_surface();
        let mut projection = DeclarativeOwnerProjection::default();
        let mut ledger = DeclarativeOwnerLedger::default();

        let before_source = source_from_surface(before);
        projection.install_from_source(&before_source);
        ledger.reconcile(&projection);
        assert!(ledger.drain_retired_tokens().is_empty());
        let identities: Vec<_> = projection
            .accepted_keyed_nodes()
            .iter()
            .map(|candidate| candidate.owner_identity())
            .collect();
        let initial: Vec<_> = identities
            .iter()
            .map(|identity| {
                let token = live_token(&ledger, *identity);
                (*identity, token.clone(), token.generation())
            })
            .collect();
        let allocated = ledger.next_generation();
        assert_eq!(ledger.live_records().len(), 2);

        projection.install_from_source(&before_source);
        ledger.reconcile(&projection);
        assert!(ledger.drain_retired_tokens().is_empty());
        assert_eq!(ledger.next_generation(), allocated);
        for (identity, token, generation) in &initial {
            let current = live_token(&ledger, *identity);
            assert_eq!(&current, token);
            assert_eq!(current.generation(), *generation);
        }

        let after_source = source_from_surface(after);
        projection.install_from_source(&after_source);
        ledger.reconcile(&projection);
        assert_eq!(ledger.next_generation(), allocated);
        for (identity, token, generation) in initial {
            let current = live_token(&ledger, identity);
            assert_eq!(current, token);
            assert_eq!(current.generation(), generation);
        }
    }

    #[test]
    fn resolver_binds_one_current_token_and_keeps_application_outcomes_distinct() {
        let (projection, context) = context_and_projection(keyed_surface());
        let mut ledger = DeclarativeOwnerLedger::default();
        ledger.reconcile(&projection);
        let candidate = keyed_candidate(&projection);

        let resolution = projection.resolve(
            DeclarativeOwnerRequest::KeyedNode(candidate),
            Some(&context),
            &ledger,
        );
        let token = match resolution {
            DeclarativeOwnerResolution::KeyedNode(token) => token,
            other => panic!("unexpected keyed resolution: {other:?}"),
        };
        assert_eq!(token.identity(), candidate.owner_identity());
        assert!(ledger.is_live(&token));
        assert_eq!(
            projection.resolve(DeclarativeOwnerRequest::Default, None, &ledger),
            DeclarativeOwnerResolution::Application
        );
        assert_eq!(
            projection.resolve(DeclarativeOwnerRequest::ApplicationOutlive, None, &ledger,),
            DeclarativeOwnerResolution::ApplicationOutlive
        );
        assert!(matches!(
            DeclarativeOwnerResolution::Application.effect_origin(),
            Some(EffectOrigin::Application)
        ));
        assert!(matches!(
            DeclarativeOwnerResolution::ApplicationOutlive.effect_origin(),
            Some(EffectOrigin::Application)
        ));
        assert!(matches!(
            DeclarativeOwnerResolution::KeyedNode(token.clone()).effect_origin(),
            Some(EffectOrigin::Declarative(actual)) if actual == token
        ));
        assert_eq!(
            projection.resolve(DeclarativeOwnerRequest::KeyedNode(candidate), None, &ledger,),
            DeclarativeOwnerResolution::Rejected(DeclarativeOwnerRejection::MissingSourceContext)
        );
        assert!(
            DeclarativeOwnerResolution::Rejected(DeclarativeOwnerRejection::AbsentCandidate)
                .effect_origin()
                .is_none()
        );

        let (overlay_projection, overlay_context) = context_and_projection(overlay_surface());
        let mut overlay_ledger = DeclarativeOwnerLedger::default();
        overlay_ledger.reconcile(&overlay_projection);
        let overlay = overlay_candidate(&overlay_projection);
        let overlay_token = match overlay_projection.resolve(
            DeclarativeOwnerRequest::Overlay(overlay),
            Some(&overlay_context),
            &overlay_ledger,
        ) {
            DeclarativeOwnerResolution::Overlay(token) => token,
            other => panic!("unexpected overlay resolution: {other:?}"),
        };
        assert!(overlay_ledger.is_live(&overlay_token));
    }

    #[test]
    fn explicit_owner_dispatch_allows_scoped_and_application_fallbacks() {
        let updates = Rc::new(Cell::new(0));
        let updates_for_handler = Rc::clone(&updates);
        let mut runtime = SurfaceRuntime::new(
            DeclarativeOwnedCommandRuntimeBridge::new(
                (),
                |_| keyed_surface_for::<u8>(),
                move |_, message| {
                    updates_for_handler.set(updates_for_handler.get() + usize::from(message));
                    Command::none()
                },
            ),
            Vector2::new(320.0, 240.0),
        );
        let candidate = keyed_candidate(runtime.declarative_owner_projection());
        let context = first_context_with_keyed_node(runtime.declarative_owner_projection());

        let scoped = runtime.dispatch_message_from_declarative_owner(
            1,
            DeclarativeOwnerRequest::KeyedNode(candidate),
            context.source_node,
        );
        assert_eq!(scoped.messages_dispatched, 1);
        assert_eq!(updates.get(), 1);

        let default = runtime.dispatch_message_from_declarative_owner(
            2,
            DeclarativeOwnerRequest::Default,
            context.source_node,
        );
        assert_eq!(default.messages_dispatched, 1);
        assert_eq!(updates.get(), 3);

        let outlive = runtime.dispatch_message_from_declarative_owner(
            4,
            DeclarativeOwnerRequest::ApplicationOutlive,
            context.source_node,
        );
        assert_eq!(outlive.messages_dispatched, 1);
        assert_eq!(updates.get(), 7);
    }

    #[test]
    fn overlay_owner_dispatch_is_explicit_and_does_not_use_keyed_precedence() {
        let updates = Rc::new(Cell::new(0));
        let updates_for_handler = Rc::clone(&updates);
        let mut runtime = SurfaceRuntime::new(
            DeclarativeOwnedCommandRuntimeBridge::new(
                (),
                |_| overlay_surface_for::<u8>(),
                move |_, _| {
                    updates_for_handler.set(updates_for_handler.get() + 1);
                    Command::none()
                },
            ),
            Vector2::new(320.0, 240.0),
        );
        let candidate = overlay_candidate(runtime.declarative_owner_projection());
        let context = first_context_with_overlay(runtime.declarative_owner_projection());

        let outcome = runtime.dispatch_message_from_declarative_owner(
            1,
            DeclarativeOwnerRequest::Overlay(candidate),
            context.source_node,
        );

        assert_eq!(outcome.messages_dispatched, 1);
        assert_eq!(updates.get(), 1);
    }

    #[test]
    fn rejected_owner_dispatches_run_no_handler_or_follow_up_work() {
        let updates = Rc::new(Cell::new(0));
        let updates_for_handler = Rc::clone(&updates);
        let mut runtime = SurfaceRuntime::new(
            DeclarativeOwnedCommandRuntimeBridge::new(
                (),
                |_| keyed_surface_for::<u8>(),
                move |_, _| {
                    updates_for_handler.set(updates_for_handler.get() + 1);
                    Command::message(99)
                },
            ),
            Vector2::new(320.0, 240.0),
        );
        let candidate = keyed_candidate(runtime.declarative_owner_projection());
        let context = first_context_with_keyed_node(runtime.declarative_owner_projection());
        let source_node = context.source_node;
        let incompatible = DeclarativeKeyedNodeCandidate::new(
            candidate.identity,
            SourceCompatibility::from_surface_node(&SurfaceNode::<u8>::container(
                99,
                ContainerPolicy::default(),
                Vec::new(),
            )),
        );
        let ineligible = DeclarativeKeyedNodeCandidate::new(
            SourceIdentity {
                structural_scope: candidate.identity.structural_scope + 1,
                ..candidate.identity
            },
            candidate.compatibility,
        );

        let missing = runtime.dispatch_message_from_declarative_owner(
            1,
            DeclarativeOwnerRequest::KeyedNode(candidate),
            u64::MAX,
        );
        assert_eq!(missing, CommandOutcome::default());

        let ineligible_outcome = runtime.dispatch_message_from_declarative_owner(
            1,
            DeclarativeOwnerRequest::KeyedNode(ineligible),
            source_node,
        );
        assert_eq!(ineligible_outcome, CommandOutcome::default());

        let incompatible_outcome = runtime.dispatch_message_from_declarative_owner(
            1,
            DeclarativeOwnerRequest::KeyedNode(incompatible),
            source_node,
        );
        assert_eq!(incompatible_outcome, CommandOutcome::default());

        runtime.declarative_owner.accepted_keyed_nodes.clear();
        let absent_outcome = runtime.dispatch_message_from_declarative_owner(
            1,
            DeclarativeOwnerRequest::KeyedNode(candidate),
            source_node,
        );
        assert_eq!(absent_outcome, CommandOutcome::default());

        runtime
            .declarative_owner
            .accepted_keyed_nodes
            .push(candidate);
        let source_record_index = runtime
            .declarative_owner
            .captured_sources
            .iter()
            .position(|record| record.source_node == source_node)
            .expect("keyed source record");
        let insert_at = runtime.declarative_owner.captured_sources[source_record_index].keyed_end;
        runtime
            .declarative_owner
            .captured_keyed_nodes
            .insert(insert_at, incompatible);
        runtime.declarative_owner.captured_sources[source_record_index].keyed_end += 1;
        let ambiguous_outcome = runtime.dispatch_message_from_declarative_owner(
            1,
            DeclarativeOwnerRequest::KeyedNode(candidate),
            source_node,
        );
        assert_eq!(ambiguous_outcome, CommandOutcome::default());
        runtime
            .declarative_owner
            .captured_keyed_nodes
            .remove(insert_at);
        runtime.declarative_owner.captured_sources[source_record_index].keyed_end -= 1;

        runtime
            .declarative_owner_ledger
            .generation_unavailable
            .push(candidate.owner_identity());
        let unavailable_outcome = runtime.dispatch_message_from_declarative_owner(
            1,
            DeclarativeOwnerRequest::KeyedNode(candidate),
            source_node,
        );
        assert_eq!(unavailable_outcome, CommandOutcome::default());

        assert_eq!(updates.get(), 0);
        assert!(runtime.timer_effects.is_empty());
        assert_eq!(runtime.drain_runtime_messages(), CommandOutcome::default());
    }

    #[test]
    fn owner_removal_during_update_vetoes_layout_dependent_follow_up() {
        let removed = Rc::new(Cell::new(false));
        let project_removed = Rc::clone(&removed);
        let update_removed = Rc::clone(&removed);
        let mut runtime = SurfaceRuntime::new(
            DeclarativeOwnedCommandRuntimeBridge::new(
                (),
                move |_| {
                    if project_removed.get() {
                        UiSurface::new(SurfaceNode::container(
                            41,
                            ContainerPolicy::default(),
                            Vec::new(),
                        ))
                    } else {
                        keyed_surface_for::<u8>()
                    }
                },
                move |_, _| {
                    update_removed.set(true);
                    Command::focus(42)
                },
            ),
            Vector2::new(320.0, 240.0),
        );
        let candidate = keyed_candidate(runtime.declarative_owner_projection());
        let context = first_context_with_keyed_node(runtime.declarative_owner_projection());
        let token = live_token(
            runtime.declarative_owner_ledger(),
            candidate.owner_identity(),
        );

        let outcome = runtime.dispatch_message_from_declarative_owner(
            1,
            DeclarativeOwnerRequest::KeyedNode(candidate),
            context.source_node,
        );

        assert_eq!(outcome.messages_dispatched, 1);
        assert!(removed.get());
        assert!(!token.is_live());
        assert!(!runtime.declarative_owner_ledger().is_live(&token));
        assert_eq!(runtime.focused_widget(), None);
    }

    #[test]
    fn removal_retires_before_reinsertion_and_replacement_gets_fresh_generation() {
        let (mut projection, context) = context_and_projection(keyed_surface());
        let candidate = keyed_candidate(&projection);
        let mut ledger = DeclarativeOwnerLedger::default();
        ledger.reconcile(&projection);
        let old = live_token(&ledger, candidate.owner_identity());
        let old_generation = old.generation();

        projection.install_from_source(&raw_source());
        ledger.reconcile(&projection);
        assert!(!old.is_live());
        assert!(!ledger.is_live(&old));
        assert!(ledger.live_records().is_empty());
        assert_eq!(ledger.drain_retired_tokens(), vec![old.clone()]);
        assert!(ledger.drain_retired_tokens().is_empty());

        projection.install_from_source(&source_from_surface(keyed_surface()));
        ledger.reconcile(&projection);
        let reinserted = live_token(&ledger, candidate.owner_identity());
        assert!(reinserted.generation() > old_generation);
        assert_ne!(reinserted, old);

        let replacement = DeclarativeKeyedNodeCandidate::new(
            candidate.identity,
            SourceCompatibility::from_surface_node(&SurfaceNode::<()>::container(
                99,
                ContainerPolicy::default(),
                Vec::new(),
            )),
        );
        assert_ne!(replacement.compatibility, candidate.compatibility);
        projection.accepted_keyed_nodes.clear();
        projection.accepted_keyed_nodes.push(replacement);
        ledger.reconcile(&projection);
        let fresh = live_token(&ledger, candidate.owner_identity());
        assert!(fresh.generation() > reinserted.generation());
        assert!(!reinserted.is_live());

        let mut replacement_context = context;
        replacement_context.keyed_nodes = vec![replacement];
        assert_eq!(
            projection.resolve(
                DeclarativeOwnerRequest::KeyedNode(candidate),
                Some(&replacement_context),
                &ledger,
            ),
            DeclarativeOwnerResolution::Rejected(DeclarativeOwnerRejection::IncompatibleCandidate)
        );
        assert_eq!(
            projection.resolve(
                DeclarativeOwnerRequest::KeyedNode(replacement),
                Some(&replacement_context),
                &ledger,
            ),
            DeclarativeOwnerResolution::KeyedNode(fresh)
        );
    }

    #[test]
    fn canonical_ledger_isolates_kinds_and_siblings_and_rejects_incompatible_duplicates() {
        let keyed_compatibility =
            keyed_candidate(&context_and_projection(keyed_surface()).0).compatibility;
        let other_compatibility = SourceCompatibility::from_surface_node(
            &SurfaceNode::<()>::container(77, ContainerPolicy::default(), Vec::new()),
        );
        let keyed = synthetic_keyed(10, 100, keyed_compatibility);
        let keyed_same_identity = synthetic_keyed(10, 999, keyed_compatibility);
        let sibling = synthetic_keyed(11, 101, keyed_compatibility);
        let incompatible = synthetic_keyed(10, 102, other_compatibility);
        let overlay = DeclarativeOverlayCandidate::new(10, LayerKind::Modal);

        let mut projection = DeclarativeOwnerProjection::default();
        projection
            .accepted_keyed_nodes
            .extend([keyed, keyed_same_identity, sibling]);
        projection.accepted_overlays.push(overlay);
        let mut ledger = DeclarativeOwnerLedger::default();
        ledger.reconcile(&projection);
        assert_eq!(ledger.live_records().len(), 3);
        assert!(
            ledger
                .live_records()
                .iter()
                .any(|record| record.token.identity()
                    == DeclarativeOwnerIdentity::KeyedNode {
                        structural_scope: 10
                    })
        );
        assert!(
            ledger
                .live_records()
                .iter()
                .any(|record| record.token.identity()
                    == DeclarativeOwnerIdentity::KeyedNode {
                        structural_scope: 11
                    })
        );
        assert!(
            ledger
                .live_records()
                .iter()
                .any(|record| record.token.identity()
                    == DeclarativeOwnerIdentity::Overlay {
                        structural_scope: 10
                    })
        );

        projection.accepted_keyed_nodes.push(incompatible);
        ledger.reconcile(&projection);
        assert_eq!(ledger.live_records().len(), 2);
        assert!(
            ledger
                .live_records()
                .iter()
                .all(|record| record.token.identity()
                    != DeclarativeOwnerIdentity::KeyedNode {
                        structural_scope: 10
                    })
        );
        assert!(
            ledger
                .live_records()
                .iter()
                .any(|record| record.token.identity()
                    == DeclarativeOwnerIdentity::KeyedNode {
                        structural_scope: 11
                    })
        );

        let mut ambiguous = DeclarativeOwnerProjection::default();
        ambiguous.accepted_keyed_nodes.extend([keyed, incompatible]);
        let mut ambiguous_ledger = DeclarativeOwnerLedger::default();
        ambiguous_ledger.reconcile(&ambiguous);
        assert!(ambiguous_ledger.live_records().is_empty());
        let context =
            DeclarativeOwnerSourceContext::new(1, true, vec![keyed, incompatible], Vec::new());
        assert_eq!(
            ambiguous.resolve(
                DeclarativeOwnerRequest::KeyedNode(keyed),
                Some(&context),
                &ambiguous_ledger,
            ),
            DeclarativeOwnerResolution::Rejected(DeclarativeOwnerRejection::AmbiguousCandidate)
        );
    }

    #[test]
    fn canonical_allocation_order_ignores_projection_order_and_resolved_id() {
        let compatibility =
            keyed_candidate(&context_and_projection(keyed_surface()).0).compatibility;
        let first = synthetic_keyed(20, 200, compatibility);
        let second = synthetic_keyed(21, 201, compatibility);
        let first_reidentified = synthetic_keyed(20, 9_200, compatibility);
        let second_reidentified = synthetic_keyed(21, 9_201, compatibility);

        let mut first_projection = DeclarativeOwnerProjection::default();
        first_projection
            .accepted_keyed_nodes
            .extend([first, second]);
        let mut first_ledger = DeclarativeOwnerLedger::default();
        first_ledger.reconcile(&first_projection);

        let mut second_projection = DeclarativeOwnerProjection::default();
        second_projection
            .accepted_keyed_nodes
            .extend([second_reidentified, first_reidentified]);
        let mut second_ledger = DeclarativeOwnerLedger::default();
        second_ledger.reconcile(&second_projection);

        for identity in [first.owner_identity(), second.owner_identity()] {
            assert_eq!(
                live_token(&first_ledger, identity).generation(),
                live_token(&second_ledger, identity).generation()
            );
        }
    }

    #[test]
    fn exhaustion_is_checked_fail_closed_and_reports_generation_unavailable() {
        let (projection, context) = context_and_projection(keyed_surface());
        let candidate = keyed_candidate(&projection);
        let mut ledger = DeclarativeOwnerLedger {
            next_generation: u64::MAX,
            ..DeclarativeOwnerLedger::default()
        };
        ledger.reconcile(&projection);
        assert!(ledger.live_records().is_empty());
        assert_eq!(ledger.next_generation(), u64::MAX);
        assert_eq!(
            projection.resolve(
                DeclarativeOwnerRequest::KeyedNode(candidate),
                Some(&context),
                &ledger,
            ),
            DeclarativeOwnerResolution::Rejected(DeclarativeOwnerRejection::GenerationUnavailable)
        );

        let compatible = candidate.compatibility;
        let first = synthetic_keyed(30, 300, compatible);
        let sibling = synthetic_keyed(31, 301, compatible);
        let replacement = synthetic_keyed(
            30,
            30_000,
            SourceCompatibility::from_surface_node(&SurfaceNode::<()>::container(
                78,
                ContainerPolicy::default(),
                Vec::new(),
            )),
        );
        let mut current = DeclarativeOwnerProjection::default();
        current.accepted_keyed_nodes.extend([first, sibling]);
        let mut current_ledger = DeclarativeOwnerLedger::default();
        current_ledger.reconcile(&current);
        let sibling_token = live_token(&current_ledger, sibling.owner_identity());
        let old_first = live_token(&current_ledger, first.owner_identity());
        current_ledger.next_generation = u64::MAX;
        current.accepted_keyed_nodes.clear();
        current.accepted_keyed_nodes.extend([replacement, sibling]);
        current_ledger.reconcile(&current);
        assert_eq!(current_ledger.next_generation(), u64::MAX);
        assert!(current_ledger.is_live(&sibling_token));
        assert!(!current_ledger.is_live(&old_first));
        assert!(!old_first.is_live());
        assert_eq!(current_ledger.live_records().len(), 1);
        assert_eq!(
            current_ledger.drain_retired_tokens(),
            vec![old_first.clone()]
        );
        assert!(current_ledger.drain_retired_tokens().is_empty());
        assert_eq!(
            current_ledger.resolve_live(
                replacement.owner_identity(),
                replacement.owner_compatibility(),
            ),
            Err(DeclarativeOwnerRejection::GenerationUnavailable)
        );
    }

    #[test]
    fn defaults_cover_keyed_overlay_mixed_unkeyed_and_raw_sources() {
        let (keyed, keyed_context) = context_and_projection(keyed_surface());
        assert_eq!(
            keyed.resolve_candidates(DeclarativeOwnerRequest::Default, None),
            DeclarativeOwnerOutcome::Application
        );
        assert_eq!(
            keyed.resolve_candidates(DeclarativeOwnerRequest::ApplicationOutlive, None),
            DeclarativeOwnerOutcome::ApplicationOutlive
        );
        assert!(matches!(
            keyed.resolve_candidates(
                DeclarativeOwnerRequest::KeyedNode(keyed_candidate(&keyed)),
                Some(&keyed_context)
            ),
            DeclarativeOwnerOutcome::KeyedNode(_)
        ));

        let (overlay, overlay_context) = context_and_projection(overlay_surface());
        assert_eq!(overlay.accepted_keyed_nodes().len(), 0);
        assert!(matches!(
            overlay.resolve_candidates(
                DeclarativeOwnerRequest::Overlay(overlay_candidate(&overlay)),
                Some(&overlay_context)
            ),
            DeclarativeOwnerOutcome::Overlay(_)
        ));

        let raw = DeclarativeOwnerProjection::default();
        assert_eq!(raw.accepted_keyed_nodes().len(), 0);
        assert_eq!(raw.accepted_overlays().len(), 0);
        assert_eq!(
            raw.resolve_candidates(DeclarativeOwnerRequest::Default, None),
            DeclarativeOwnerOutcome::Application
        );
        assert_eq!(
            raw.resolve_candidates(
                DeclarativeOwnerRequest::Overlay(DeclarativeOverlayCandidate::new(
                    9,
                    LayerKind::Modal,
                )),
                None,
            ),
            DeclarativeOwnerOutcome::Rejected(DeclarativeOwnerRejection::MissingSourceContext)
        );
        let mut raw = raw;
        raw.install_from_source(&raw_source());
        let raw_context = raw.captured_context(1).expect("raw source context");
        assert!(!raw_context.metadata_present());
        assert_eq!(
            raw.resolve_candidates(
                DeclarativeOwnerRequest::Overlay(DeclarativeOverlayCandidate::new(
                    9,
                    LayerKind::Modal,
                )),
                Some(&raw_context),
            ),
            DeclarativeOwnerOutcome::Rejected(DeclarativeOwnerRejection::MissingSourceContext)
        );
    }

    #[test]
    fn non_keyed_source_origins_never_allocate_declarative_generations() {
        let surfaces = [
            text::<()>("generated").into_surface(),
            text::<()>("numeric").id(42).into_surface(),
            column([text::<()>("dynamic-a"), text::<()>("dynamic-b")]).into_surface(),
            UiSurface::new(SurfaceNode::container(
                88,
                ContainerPolicy::default(),
                Vec::new(),
            )),
        ];
        for surface in surfaces {
            let source = source_from_surface(surface);
            let mut projection = DeclarativeOwnerProjection::default();
            let mut ledger = DeclarativeOwnerLedger::default();
            projection.install_from_source(&source);
            ledger.reconcile(&projection);
            assert!(projection.accepted_keyed_nodes().is_empty());
            assert!(ledger.live_records().is_empty());
            assert_eq!(ledger.next_generation(), 0);
        }
    }

    #[test]
    fn keyed_and_overlay_requests_are_independent_without_precedence() {
        let surface = scene(column(for_each_by(
            [1_u32],
            |item| *item,
            |_| {
                column([text::<()>("owner")
                    .key("owner")
                    .overlays(overlays().modal(text("overlay").key("overlay")))])
            },
        )))
        .into_view()
        .into_surface();
        let source = source_from_surface(surface);
        let mut projection = DeclarativeOwnerProjection::default();
        projection.install_from_source(&source);
        let keyed = keyed_candidate(&projection);
        let overlay = overlay_candidate(&projection);
        let context = source
            .records
            .iter()
            .filter_map(|record| {
                let context = projection.captured_context(record.node_id)?;
                (context
                    .keyed_nodes
                    .iter()
                    .any(|candidate| candidate == &keyed)
                    && context
                        .overlays
                        .iter()
                        .any(|candidate| candidate == &overlay))
                .then_some(context)
            })
            .next()
            .expect("one source context should retain both candidates");
        assert!(matches!(
            projection.resolve_candidates(
                DeclarativeOwnerRequest::KeyedNode(keyed),
                Some(&context)
            ),
            DeclarativeOwnerOutcome::KeyedNode(candidate) if candidate == keyed
        ));
        assert!(matches!(
            projection.resolve_candidates(
                DeclarativeOwnerRequest::Overlay(overlay),
                Some(&context)
            ),
            DeclarativeOwnerOutcome::Overlay(candidate) if candidate == overlay
        ));
    }

    #[test]
    fn scoped_failures_are_typed_and_never_fallback() {
        let (projection, context) = context_and_projection(keyed_surface());
        let candidate = keyed_candidate(&projection);
        assert_eq!(
            projection.resolve_candidates(DeclarativeOwnerRequest::KeyedNode(candidate), None),
            DeclarativeOwnerOutcome::Rejected(DeclarativeOwnerRejection::MissingSourceContext)
        );
        assert_eq!(
            projection.resolve_candidates_for_source(
                DeclarativeOwnerRequest::KeyedNode(candidate),
                9_999_999,
            ),
            DeclarativeOwnerOutcome::Rejected(DeclarativeOwnerRejection::MissingSourceContext)
        );
        let other = DeclarativeKeyedNodeCandidate::new(
            SourceIdentity {
                resolved_id: candidate.identity.resolved_id,
                structural_scope: candidate.identity.structural_scope + 1,
                origin: candidate.identity.origin,
            },
            candidate.compatibility,
        );
        assert_eq!(
            projection
                .resolve_candidates(DeclarativeOwnerRequest::KeyedNode(other), Some(&context),),
            DeclarativeOwnerOutcome::Rejected(DeclarativeOwnerRejection::IneligibleCandidate)
        );
        let mut removed = DeclarativeOwnerProjection::default();
        removed.install_from_source(&raw_source());
        assert_eq!(
            removed.resolve_candidates(
                DeclarativeOwnerRequest::KeyedNode(candidate),
                Some(&context),
            ),
            DeclarativeOwnerOutcome::Rejected(DeclarativeOwnerRejection::AbsentCandidate)
        );
    }

    #[test]
    fn repeated_ancestry_normalizes_and_conflicts_are_ambiguous() {
        let surface = scene(column(for_each_by(
            [1_u32],
            |item| *item,
            |_| text::<()>("row").overlays(overlays().modal(text("overlay"))),
        )))
        .into_view()
        .into_surface();
        let (projection, _) = context_and_projection(surface);
        assert_eq!(projection.accepted_keyed_nodes().len(), 1);
        assert_eq!(projection.accepted_overlays().len(), 1);

        let candidate = keyed_candidate(&projection);
        let conflict = DeclarativeKeyedNodeCandidate::new(
            SourceIdentity {
                resolved_id: candidate.identity.resolved_id + 1,
                origin: if candidate.identity.origin
                    == crate::application::DeclarativeIdentityOrigin::ExplicitContinuityKey
                {
                    crate::application::DeclarativeIdentityOrigin::InferredKeyedIdentity
                } else {
                    crate::application::DeclarativeIdentityOrigin::ExplicitContinuityKey
                },
                ..candidate.identity
            },
            candidate.compatibility,
        );
        let mut conflicting = projection;
        conflicting.accepted_keyed_nodes.push(conflict);
        let context = conflicting
            .captured_context(
                conflicting
                    .captured_sources
                    .first()
                    .expect("source")
                    .source_node,
            )
            .expect("context");
        let mut context = context;
        context.keyed_nodes.push(candidate);
        context.keyed_nodes.push(conflict);
        assert_eq!(
            conflicting.resolve_candidates(
                DeclarativeOwnerRequest::KeyedNode(candidate),
                Some(&context),
            ),
            DeclarativeOwnerOutcome::Rejected(DeclarativeOwnerRejection::AmbiguousCandidate)
        );
    }

    #[test]
    fn source_context_survives_capture_but_removed_candidate_is_absent() {
        let (mut projection, context) = context_and_projection(keyed_surface());
        let candidate = keyed_candidate(&projection);
        projection.install_from_source(&raw_source());
        assert_eq!(
            projection.resolve_candidates(
                DeclarativeOwnerRequest::KeyedNode(candidate),
                Some(&context),
            ),
            DeclarativeOwnerOutcome::Rejected(DeclarativeOwnerRejection::AbsentCandidate)
        );
    }

    #[test]
    fn incompatible_replacement_and_exact_compatibility_are_distinct() {
        let (projection, context) = context_and_projection(keyed_surface());
        let candidate = keyed_candidate(&projection);
        let replacement = DeclarativeKeyedNodeCandidate::new(
            candidate.identity,
            SourceCompatibility::from_surface_node(&SurfaceNode::<()>::container(
                99,
                ContainerPolicy::default(),
                Vec::new(),
            )),
        );
        if replacement.compatibility != candidate.compatibility {
            let mut replaced = projection;
            replaced.accepted_keyed_nodes.clear();
            replaced.accepted_keyed_nodes.push(replacement);
            assert_eq!(
                replaced.resolve_candidates(
                    DeclarativeOwnerRequest::KeyedNode(candidate),
                    Some(&context),
                ),
                DeclarativeOwnerOutcome::Rejected(DeclarativeOwnerRejection::IncompatibleCandidate)
            );
        }

        let (projection, context) = context_and_projection(overlay_surface());
        let candidate = overlay_candidate(&projection);
        let replacement = DeclarativeOverlayCandidate::new(
            candidate.structural_scope,
            if candidate.layer_kind == LayerKind::Modal {
                LayerKind::Tooltip
            } else {
                LayerKind::Modal
            },
        );
        let mut replaced = projection;
        replaced.accepted_overlays.clear();
        replaced.accepted_overlays.push(replacement);
        assert_eq!(
            replaced
                .resolve_candidates(DeclarativeOwnerRequest::Overlay(candidate), Some(&context),),
            DeclarativeOwnerOutcome::Rejected(DeclarativeOwnerRejection::IncompatibleCandidate)
        );
    }

    #[test]
    fn reorder_keeps_structural_scope_and_sibling_context_isolation() {
        let before = column([text::<()>("a").key("a"), text::<()>("b").key("b")]).into_surface();
        let after = column([text::<()>("b").key("b"), text::<()>("a").key("a")]).into_surface();
        let (before_projection, before_context) = context_and_projection(before);
        let after_source = source_from_surface(after);
        let mut after_projection = DeclarativeOwnerProjection::default();
        after_projection.install_from_source(&after_source);
        let before_scopes = before_projection
            .accepted_keyed_nodes()
            .iter()
            .map(|candidate| candidate.identity.structural_scope)
            .collect::<std::collections::HashSet<_>>();
        let after_scopes = after_projection
            .accepted_keyed_nodes()
            .iter()
            .map(|candidate| candidate.identity.structural_scope)
            .collect::<std::collections::HashSet<_>>();
        assert_eq!(before_scopes, after_scopes);
        let candidate = before_projection
            .accepted_keyed_nodes()
            .iter()
            .copied()
            .find(|candidate| {
                candidate.identity.structural_scope
                    == before_context.keyed_nodes[0].identity.structural_scope
            })
            .expect("first keyed candidate");
        let after_context = after_source
            .records
            .iter()
            .filter_map(|record| after_projection.captured_context(record.node_id))
            .find(|context| {
                context.keyed_nodes.iter().any(|entry| {
                    entry.identity.structural_scope == candidate.identity.structural_scope
                })
            })
            .expect("reordered source context");
        assert!(matches!(
            before_projection.resolve_candidates(
                DeclarativeOwnerRequest::KeyedNode(candidate),
                Some(&before_context),
            ),
            DeclarativeOwnerOutcome::KeyedNode(_)
        ));
        let reordered = after_projection
            .accepted_keyed_nodes()
            .iter()
            .copied()
            .find(|candidate| {
                candidate.identity.structural_scope
                    == before_context.keyed_nodes[0].identity.structural_scope
            })
            .expect("reordered keyed candidate");
        assert_eq!(
            after_projection.resolve_candidates(
                DeclarativeOwnerRequest::KeyedNode(candidate),
                Some(&after_context),
            ),
            DeclarativeOwnerOutcome::KeyedNode(reordered)
        );
        assert!(
            before_context
                .keyed_nodes
                .iter()
                .all(|candidate| candidate.identity.structural_scope != 9_999)
        );

        let siblings = column([
            text::<()>("first").key("first"),
            text::<()>("second").key("second"),
        ])
        .into_surface();
        let sibling_source = source_from_surface(siblings);
        let mut sibling_projection = DeclarativeOwnerProjection::default();
        sibling_projection.install_from_source(&sibling_source);
        let first_context = sibling_source
            .records
            .iter()
            .filter_map(|record| sibling_projection.captured_context(record.node_id))
            .find(|context| context.keyed_nodes.len() == 1)
            .expect("sibling source context");
        let first = first_context.keyed_nodes[0];
        let second = sibling_projection
            .accepted_keyed_nodes()
            .iter()
            .copied()
            .find(|candidate| {
                candidate.identity.structural_scope != first.identity.structural_scope
            })
            .expect("second sibling candidate");
        assert_eq!(
            sibling_projection.resolve_candidates(
                DeclarativeOwnerRequest::KeyedNode(second),
                Some(&first_context),
            ),
            DeclarativeOwnerOutcome::Rejected(DeclarativeOwnerRejection::IneligibleCandidate)
        );
    }

    #[test]
    fn storage_clears_stale_entries_and_retains_capacity() {
        let (mut projection, _) = context_and_projection(keyed_surface());
        let capacities = projection.storage_capacities();
        projection.install_from_source(&raw_source());
        assert!(projection.accepted_keyed_nodes().is_empty());
        assert!(projection.accepted_overlays().is_empty());
        let next = projection.storage_capacities();
        assert!(next.0 >= capacities.0);
        assert!(next.1 >= capacities.1);
        assert!(next.2 >= capacities.2);
        assert!(next.3 >= capacities.3);
        assert!(next.4 >= capacities.4);
    }

    #[test]
    fn startup_refresh_and_ordinary_relayout_install_only_accepted_source() {
        let changed = Rc::new(Cell::new(false));
        let project_changed = Rc::clone(&changed);
        let mut runtime = SurfaceRuntime::new_declarative_owned(
            (),
            Vector2::new(320.0, 240.0),
            move |_| {
                if project_changed.get() {
                    UiSurface::new(SurfaceNode::container(
                        41,
                        ContainerPolicy::default(),
                        Vec::new(),
                    ))
                } else {
                    keyed_surface()
                }
            },
            |_, ()| {},
        );
        let startup_count = runtime.declarative_owner_projection().installation_count();
        assert_eq!(startup_count, 1);
        assert_eq!(
            runtime.declarative_owner_ledger().reconciliation_count(),
            startup_count
        );
        assert!(
            !runtime
                .declarative_owner_projection()
                .accepted_keyed_nodes()
                .is_empty()
        );
        changed.set(true);
        runtime.refresh();
        assert_eq!(
            runtime.declarative_owner_projection().installation_count(),
            startup_count + 1
        );
        assert_eq!(
            runtime.declarative_owner_ledger().reconciliation_count(),
            startup_count + 1
        );
        assert!(
            runtime
                .declarative_owner_projection()
                .accepted_keyed_nodes()
                .is_empty()
        );
        runtime.relayout();
        assert_eq!(
            runtime.declarative_owner_projection().installation_count(),
            startup_count + 2
        );
        assert_eq!(
            runtime.declarative_owner_ledger().reconciliation_count(),
            startup_count + 2
        );
    }

    #[test]
    fn recovery_and_relayout_preserve_generation_and_refresh_retires_before_chained_reduction() {
        let removed = Rc::new(Cell::new(false));
        let project_removed = Rc::clone(&removed);
        let reduce_removed = Rc::clone(&removed);
        let token_for_chain: Rc<RefCell<Option<DeclarativeOwnerToken>>> =
            Rc::new(RefCell::new(None));
        let token_for_update = Rc::clone(&token_for_chain);
        let chained_reduction_saw_retired = Rc::new(Cell::new(false));
        let observed_by_update = Rc::clone(&chained_reduction_saw_retired);
        let mut runtime = SurfaceRuntime::new(
            DeclarativeOwnedCommandRuntimeBridge::new(
                (),
                move |_| {
                    if project_removed.get() {
                        UiSurface::new(SurfaceNode::container(
                            51,
                            ContainerPolicy::default(),
                            Vec::new(),
                        ))
                    } else {
                        keyed_surface_for::<u8>()
                    }
                },
                move |_, message: u8| match message {
                    0 => {
                        reduce_removed.set(true);
                        Command::message(1)
                    }
                    1 => {
                        let token = token_for_update.borrow();
                        assert!(
                            token.as_ref().is_some_and(|token| !token.is_live()),
                            "chained reduction must observe retirement after accepted refresh"
                        );
                        observed_by_update.set(true);
                        Command::none()
                    }
                    _ => Command::none(),
                },
            ),
            Vector2::new(320.0, 240.0),
        );
        let candidate = keyed_candidate(runtime.declarative_owner_projection());
        let context = first_context_with_keyed_node(runtime.declarative_owner_projection());
        let old = match runtime.declarative_owner_projection().resolve(
            DeclarativeOwnerRequest::KeyedNode(candidate),
            Some(&context),
            runtime.declarative_owner_ledger(),
        ) {
            DeclarativeOwnerResolution::KeyedNode(token) => token,
            other => panic!("unexpected startup resolution: {other:?}"),
        };
        *token_for_chain.borrow_mut() = Some(old.clone());
        let generation = old.generation();

        runtime.relayout();
        assert!(runtime.declarative_owner_ledger().is_live(&old));
        assert_eq!(
            live_token(
                runtime.declarative_owner_ledger(),
                candidate.owner_identity()
            )
            .generation(),
            generation
        );
        assert!(runtime.begin_native_recovery());
        assert!(runtime.finish_native_recovery());
        assert!(runtime.declarative_owner_ledger().is_live(&old));

        let outcome = runtime.dispatch_message(0);
        assert_eq!(outcome.messages_dispatched, 2);
        assert!(chained_reduction_saw_retired.get());
        assert!(!old.is_live());

        assert!(runtime.begin_closing());
        assert!(!runtime.declarative_owner_ledger().is_live(&old));
        assert!(runtime.declarative_owner_ledger().live_records().is_empty());
    }

    #[test]
    fn mixed_surface_has_both_candidates_without_precedence() {
        let (projection, _) = context_and_projection(
            scene(
                text::<()>("keyed")
                    .key("keyed")
                    .overlays(overlays().modal(text("overlay"))),
            )
            .into_view()
            .into_surface(),
        );
        assert!(!projection.accepted_keyed_nodes().is_empty());
        assert!(!projection.accepted_overlays().is_empty());
    }
}

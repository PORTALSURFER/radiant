//! Private declarative owner-selection evidence.
//!
//! This module deliberately stops at accepted source evidence.  It does not
//! create an effect origin, admit work, allocate a generation, or retire any
//! existing owner.  Those are later controller contracts.

#![allow(dead_code)]

use super::SurfaceRuntime;
use crate::{
    layout::NodeId,
    runtime::{
        LayerKind, RuntimeBridge,
        surface::{SourceCompatibility, SourceIdentity, SourceTraversalIndex},
    },
};
use std::{marker::PhantomData, rc::Rc};

type LocalOnly = PhantomData<Rc<()>>;

fn local_only() -> LocalOnly {
    PhantomData
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
    local_only: LocalOnly,
}

impl DeclarativeOverlayCandidate {
    fn new(structural_scope: NodeId, layer_kind: LayerKind) -> Self {
        Self {
            structural_scope,
            layer_kind,
            local_only: local_only(),
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
    local_only: LocalOnly,
}

impl DeclarativeKeyedNodeCandidate {
    fn new(identity: SourceIdentity, compatibility: SourceCompatibility) -> Self {
        Self {
            identity,
            compatibility,
            local_only: local_only(),
        }
    }

    fn structural_scope(self) -> NodeId {
        self.identity.structural_scope
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
}

/// Typed rejection for a scoped owner request.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DeclarativeOwnerRejection {
    MissingSourceContext,
    IneligibleCandidate,
    AbsentCandidate,
    IncompatibleCandidate,
    AmbiguousCandidate,
}

/// Private resolution evidence.  This remains evidence only; it is not an
/// [`super::super::owner::EffectOrigin`] and cannot change effect admission.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DeclarativeOwnerOutcome {
    Application,
    ApplicationOutlive,
    Overlay(DeclarativeOverlayCandidate),
    KeyedNode(DeclarativeKeyedNodeCandidate),
    Rejected(DeclarativeOwnerRejection),
}

#[derive(Clone, Copy, Debug)]
struct CapturedSourceRecord {
    source_node: NodeId,
    metadata_present: bool,
    keyed_start: usize,
    keyed_end: usize,
    overlay_start: usize,
    overlay_end: usize,
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
                        DeclarativeKeyedNodeCandidate::new(keyed.identity(), keyed.compatibility());
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
                    );
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

    /// Resolve one request against captured source context and the current
    /// accepted projection.  Every scoped request is exact and typed: a
    /// failure never becomes application-owned and never searches an ancestor.
    pub(crate) fn resolve(
        &self,
        request: DeclarativeOwnerRequest,
        context: Option<&DeclarativeOwnerSourceContext>,
    ) -> DeclarativeOwnerOutcome {
        match request {
            DeclarativeOwnerRequest::Default => DeclarativeOwnerOutcome::Application,
            DeclarativeOwnerRequest::ApplicationOutlive => {
                DeclarativeOwnerOutcome::ApplicationOutlive
            }
            DeclarativeOwnerRequest::Overlay(candidate) => match resolve_candidate(
                context,
                candidate,
                context.map(|context| context.overlays.as_slice()),
                &self.accepted_overlays,
                DeclarativeOverlayCandidate::structural_scope,
                overlay_candidates_compatible,
            ) {
                Ok(candidate) => DeclarativeOwnerOutcome::Overlay(candidate),
                Err(rejection) => DeclarativeOwnerOutcome::Rejected(rejection),
            },
            DeclarativeOwnerRequest::KeyedNode(candidate) => {
                if !candidate.identity.origin.is_keyed() {
                    return DeclarativeOwnerOutcome::Rejected(
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
                    Ok(candidate) => DeclarativeOwnerOutcome::KeyedNode(candidate),
                    Err(rejection) => DeclarativeOwnerOutcome::Rejected(rejection),
                }
            }
        }
    }

    /// Resolve against the captured context for one source record without
    /// allowing a missing or duplicate source id to select another context.
    pub(crate) fn resolve_for_source(
        &self,
        request: DeclarativeOwnerRequest,
        source_node: NodeId,
    ) -> DeclarativeOwnerOutcome {
        if matches!(
            request,
            DeclarativeOwnerRequest::Default | DeclarativeOwnerRequest::ApplicationOutlive
        ) {
            return self.resolve(request, None);
        }
        let mut matches = self
            .captured_sources
            .iter()
            .filter(|record| record.source_node == source_node);
        let Some(record) = matches.next() else {
            return DeclarativeOwnerOutcome::Rejected(
                DeclarativeOwnerRejection::MissingSourceContext,
            );
        };
        if matches.next().is_some() {
            return DeclarativeOwnerOutcome::Rejected(
                DeclarativeOwnerRejection::AmbiguousCandidate,
            );
        }
        let context = DeclarativeOwnerSourceContext::new(
            record.source_node,
            record.metadata_present,
            self.captured_keyed_nodes[record.keyed_start..record.keyed_end].to_vec(),
            self.captured_overlays[record.overlay_start..record.overlay_end].to_vec(),
        );
        self.resolve(request, Some(&context))
    }
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
    first.structural_scope == second.structural_scope && first.layer_kind == second.layer_kind
}

fn keyed_candidates_compatible(
    first: DeclarativeKeyedNodeCandidate,
    second: DeclarativeKeyedNodeCandidate,
) -> bool {
    first.identity.structural_scope == second.identity.structural_scope
        && first.identity.origin == second.identity.origin
        && first.compatibility == second.compatibility
}

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(super) fn install_declarative_owner_projection(&mut self) {
        self.declarative_owner
            .install_from_source(&self.scratch.projection_source);
    }

    pub(crate) fn declarative_owner_projection(&self) -> &DeclarativeOwnerProjection {
        &self.declarative_owner
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        application::{IntoView, Layer, column, for_each_by, overlays, scene, text},
        gui::types::Vector2,
        layout::ContainerPolicy,
        runtime::{SurfaceNode, UiSurface},
    };
    use std::{cell::Cell, rc::Rc};

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

    fn overlay_surface() -> UiSurface<()> {
        scene(text::<()>("base"))
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

    #[test]
    fn defaults_cover_keyed_overlay_mixed_unkeyed_and_raw_sources() {
        let (keyed, keyed_context) = context_and_projection(keyed_surface());
        assert_eq!(
            keyed.resolve(DeclarativeOwnerRequest::Default, None),
            DeclarativeOwnerOutcome::Application
        );
        assert_eq!(
            keyed.resolve(DeclarativeOwnerRequest::ApplicationOutlive, None),
            DeclarativeOwnerOutcome::ApplicationOutlive
        );
        assert!(matches!(
            keyed.resolve(
                DeclarativeOwnerRequest::KeyedNode(keyed_candidate(&keyed)),
                Some(&keyed_context)
            ),
            DeclarativeOwnerOutcome::KeyedNode(_)
        ));

        let (overlay, overlay_context) = context_and_projection(overlay_surface());
        assert_eq!(overlay.accepted_keyed_nodes().len(), 0);
        assert!(matches!(
            overlay.resolve(
                DeclarativeOwnerRequest::Overlay(overlay_candidate(&overlay)),
                Some(&overlay_context)
            ),
            DeclarativeOwnerOutcome::Overlay(_)
        ));

        let raw = DeclarativeOwnerProjection::default();
        assert_eq!(raw.accepted_keyed_nodes().len(), 0);
        assert_eq!(raw.accepted_overlays().len(), 0);
        assert_eq!(
            raw.resolve(DeclarativeOwnerRequest::Default, None),
            DeclarativeOwnerOutcome::Application
        );
        assert_eq!(
            raw.resolve(
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
            raw.resolve(
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
            projection.resolve(
                DeclarativeOwnerRequest::KeyedNode(keyed),
                Some(&context)
            ),
            DeclarativeOwnerOutcome::KeyedNode(candidate) if candidate == keyed
        ));
        assert!(matches!(
            projection.resolve(
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
            projection.resolve(DeclarativeOwnerRequest::KeyedNode(candidate), None),
            DeclarativeOwnerOutcome::Rejected(DeclarativeOwnerRejection::MissingSourceContext)
        );
        assert_eq!(
            projection
                .resolve_for_source(DeclarativeOwnerRequest::KeyedNode(candidate), 9_999_999,),
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
            projection.resolve(DeclarativeOwnerRequest::KeyedNode(other), Some(&context),),
            DeclarativeOwnerOutcome::Rejected(DeclarativeOwnerRejection::IneligibleCandidate)
        );
        let mut removed = DeclarativeOwnerProjection::default();
        removed.install_from_source(&raw_source());
        assert_eq!(
            removed.resolve(
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
            conflicting.resolve(
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
            projection.resolve(
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
                replaced.resolve(
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
            replaced.resolve(DeclarativeOwnerRequest::Overlay(candidate), Some(&context),),
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
            before_projection.resolve(
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
            after_projection.resolve(
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
            sibling_projection.resolve(
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

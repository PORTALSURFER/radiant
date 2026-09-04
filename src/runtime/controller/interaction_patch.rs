//! The phase-one interaction-only refresh transaction.

use super::SurfaceRuntime;
use super::fresh_surface_preparation::FreshSurfaceRefreshRequest;
use crate::runtime::bridge::{ExactChangedRoots, SurfaceUpdateProviderAuthority};
use crate::runtime::surface::{InteractionLeafRevision, inspect_interaction_path};
use crate::runtime::{RuntimeBridge, RuntimeLifecyclePhase, WidgetPath};
use crate::theme::ResolvedAppearance;
use crate::widgets::WidgetId;
use std::collections::HashSet;
use std::time::Duration;

/// Candidate owned by the native refresh gates for a bounded interaction
/// update. It remains inert until the native publication boundary consumes it.
pub(crate) struct InteractionPatchCandidate<Message> {
    pub(in crate::runtime::controller) candidate: ExactChangedRoots<Message>,
    pub(in crate::runtime::controller) request: FreshSurfaceRefreshRequest,
    pub(in crate::runtime::controller) expected_provider: SurfaceUpdateProviderAuthority,
    pub(in crate::runtime::controller) appearance: ResolvedAppearance,
    pub(in crate::runtime::controller) application_projection: Duration,
}

pub(crate) struct InteractionPatchCommit<Message> {
    pub(crate) changed_count: u32,
    pub(crate) retired_candidate: ExactChangedRoots<Message>,
}

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    /// Apply one exact interaction-only candidate. Returning the candidate
    /// leaves the caller able to use the ordinary full-refresh fallback.
    #[allow(clippy::result_large_err)]
    pub(in crate::runtime::controller) fn try_apply_interaction_update(
        &mut self,
        request: FreshSurfaceRefreshRequest,
        expected_provider: Option<SurfaceUpdateProviderAuthority>,
        candidate: ExactChangedRoots<Message>,
    ) -> Result<InteractionPatchCommit<Message>, ExactChangedRoots<Message>> {
        if !self.interaction_update_is_admissible(request, expected_provider, &candidate) {
            return Err(candidate);
        }
        let Some(paths) = self.interaction_update_paths(&candidate) else {
            return Err(candidate);
        };

        if !self.consume_fresh_surface_refresh_authority(request) {
            return Err(candidate);
        }
        Ok(self.commit_interaction_update(candidate, paths))
    }

    #[allow(clippy::result_large_err)]
    pub(in crate::runtime::controller) fn prepare_interaction_update(
        &self,
        request: FreshSurfaceRefreshRequest,
        expected_provider: Option<SurfaceUpdateProviderAuthority>,
        candidate: ExactChangedRoots<Message>,
        appearance: ResolvedAppearance,
        application_projection: Duration,
    ) -> Result<InteractionPatchCandidate<Message>, ExactChangedRoots<Message>> {
        let Some(expected_provider) = expected_provider else {
            return Err(candidate);
        };
        if !self.interaction_update_is_admissible(request, Some(expected_provider), &candidate) {
            return Err(candidate);
        }
        Ok(InteractionPatchCandidate {
            candidate,
            request,
            expected_provider,
            appearance,
            application_projection,
        })
    }

    pub(in crate::runtime::controller) fn publish_interaction_update(
        &mut self,
        prepared: InteractionPatchCandidate<Message>,
    ) -> Option<InteractionPatchCommit<Message>> {
        if !self.interaction_update_is_admissible(
            prepared.request,
            Some(prepared.expected_provider),
            &prepared.candidate,
        ) {
            return None;
        }
        let paths = self.interaction_update_paths(&prepared.candidate)?;
        if !self.consume_fresh_surface_refresh_authority(prepared.request) {
            return None;
        }
        Some(self.commit_interaction_update(prepared.candidate, paths))
    }

    pub(in crate::runtime::controller) fn interaction_patch_candidate_is_current(
        &self,
        prepared: &InteractionPatchCandidate<Message>,
    ) -> bool {
        self.interaction_update_is_admissible(
            prepared.request,
            Some(prepared.expected_provider),
            &prepared.candidate,
        )
    }

    fn interaction_update_is_admissible(
        &self,
        request: FreshSurfaceRefreshRequest,
        expected_provider: Option<SurfaceUpdateProviderAuthority>,
        candidate: &ExactChangedRoots<Message>,
    ) -> bool {
        if !self.interaction_patch_request_is_current(request)
            || request.lifecycle_phase != RuntimeLifecyclePhase::Running
            || !matches!(
                request.scope,
                crate::runtime::RepaintScope::Projection | crate::runtime::RepaintScope::Surface
            )
            || expected_provider.is_none()
            || self.bridge.surface_update_provider_authority() != expected_provider
            || candidate.provider_authority != expected_provider
            || candidate.runtime_identity != request.runtime_identity
            || candidate.request_revision != request.request_revision
            || candidate.active_surface_generation != request.active_surface_generation
            || !same_rect_bits(candidate.viewport, request.viewport)
            || candidate.window_environment != request.window_environment
            || candidate.surface.window_environment() != self.window_environment
            || candidate.surface.application_environment() != self.surface.application_environment()
            || candidate.changed_roots.is_empty()
            || candidate.changed_roots.len() > crate::runtime::MAX_EXACT_CHANGED_ROOTS
            || candidate
                .changed_roots
                .iter()
                .map(|root| root.child_path.len())
                .sum::<usize>()
                > crate::runtime::MAX_EXACT_CHANGED_ROOT_PATH_COMPONENTS
            || !self.interaction_layout_context_is_current()
            || !self.virtual_layout.is_empty()
            || !self.traversal.widgets.duplicate_widget_ids.is_empty()
        {
            return false;
        }

        let changed_ids: HashSet<WidgetId> = candidate
            .changed_roots
            .iter()
            .map(|changed| changed.node_id)
            .collect();
        if changed_ids.len() != candidate.changed_roots.len()
            || !self.interaction_patch_state_is_safe_for(&changed_ids)
        {
            return false;
        }

        let mut seen_ids = HashSet::with_capacity(candidate.changed_roots.len());
        let mut seen_paths: Vec<&[usize]> = Vec::with_capacity(candidate.changed_roots.len());
        let mut has_interaction = false;
        for changed in &candidate.changed_roots {
            if !seen_ids.insert(changed.node_id)
                || seen_paths.iter().any(|path| {
                    *path == changed.child_path.as_slice()
                        || path.starts_with(changed.child_path.as_slice())
                        || changed.child_path.starts_with(path)
                })
            {
                return false;
            }
            let Some(installed_path) = self.traversal.widgets.paths.current.get(&changed.node_id)
            else {
                return false;
            };
            if installed_path.as_slice() != changed.child_path.as_slice() {
                return false;
            }
            let Some(evidence) = inspect_interaction_path(
                self.surface.root(),
                candidate.surface.root(),
                &changed.child_path,
            ) else {
                return false;
            };
            if matches!(evidence.relation, InteractionLeafRevision::Reject)
                || evidence.previous_membership[5]
                || evidence.current_membership[5]
                || evidence.previous_membership != evidence.current_membership
                || !membership_matches_installed(
                    self,
                    changed.node_id,
                    evidence.previous_membership,
                )
            {
                return false;
            }
            has_interaction |= matches!(evidence.relation, InteractionLeafRevision::Interaction);
            seen_paths.push(changed.child_path.as_slice());
        }
        if !has_interaction {
            return false;
        }
        true
    }

    fn commit_interaction_update(
        &mut self,
        mut candidate: ExactChangedRoots<Message>,
        paths: Vec<WidgetPath>,
    ) -> InteractionPatchCommit<Message> {
        // Every check has completed. Leaf swaps are infallible, with no user
        // callback or fallible allocation in flight.
        for (changed, path) in candidate.changed_roots.iter().zip(paths.iter()) {
            if !self
                .surface
                .swap_widget_at_path(&mut candidate.surface, changed.node_id, path)
            {
                unreachable!("validated interaction patch path disappeared before commit");
            }
        }
        InteractionPatchCommit {
            changed_count: candidate.changed_roots.len() as u32,
            retired_candidate: candidate,
        }
    }

    fn interaction_update_paths(
        &self,
        candidate: &ExactChangedRoots<Message>,
    ) -> Option<Vec<WidgetPath>> {
        candidate
            .changed_roots
            .iter()
            .map(|changed| {
                self.traversal
                    .widgets
                    .paths
                    .current
                    .get(&changed.node_id)
                    .cloned()
            })
            .collect()
    }

    fn interaction_patch_request_is_current(&self, request: FreshSurfaceRefreshRequest) -> bool {
        self.fresh_surface_request
            .is_some_and(|stored| stored.exactly_matches(request))
            && self.fresh_surface_request_revision == request.request_revision
            && self.fresh_surface_active_generation == request.active_surface_generation
            && self.runtime_identity() == request.runtime_identity
            && self.lifecycle_phase() == request.lifecycle_phase
            && self.lifecycle_transition_sequence() == request.lifecycle_transition_sequence
            && same_rect_bits(self.viewport, request.viewport)
            && self.window_environment == request.window_environment
    }

    fn interaction_layout_context_is_current(&self) -> bool {
        self.completed_layout.is_some_and(|completed| {
            completed.viewport == effective_layout_viewport(self.viewport)
                && completed.window_environment == self.window_environment
                && completed.layout_state_generation == self.layout_state_generation
                && completed.layout_debug_options == self.layout_debug_options
        }) && !self.external_layout_dirty
            && !self.pending_current_surface_relayout
            && !self.layout_engine.has_explicit_dirty()
    }

    fn interaction_patch_state_is_safe_for(&self, changed_ids: &HashSet<WidgetId>) -> bool {
        let owned = |id: Option<WidgetId>| id.is_some_and(|id| changed_ids.contains(&id));
        !self
            .interaction
            .focus
            .focused_widget()
            .is_some_and(|id| changed_ids.contains(&id))
            && self.interaction.focus.pending_key_chord.is_none()
            && !self
                .interaction
                .focus
                .focused_key_capture
                .is_some_and(|capture| changed_ids.contains(&capture.widget_id))
            && !owned(self.interaction.hover.widget)
            && !owned(self.interaction.tooltip.target)
            && !owned(self.interaction.pointer.capture)
            && !self
                .interaction
                .pointer
                .capture_state
                .is_some_and(|(id, _)| changed_ids.contains(&id))
            && !self
                .interaction
                .pointer
                .managed_capture
                .is_some_and(|capture| changed_ids.contains(&capture.widget_id))
            && self.interaction.pointer.scroll_drag_capture.is_none()
            && !self.interaction.pointer.has_any_release_tombstone()
            && !matches!(
                self.interaction.wheel.managed_sequence,
                super::interaction_state::RuntimeManagedWheelSequenceState::Active { widget_id }
                    if changed_ids.contains(&widget_id)
            )
            && !matches!(
                self.interaction.composition.managed_composition,
                super::interaction_state::RuntimeManagedCompositionState::Active { widget_id }
                    if changed_ids.contains(&widget_id)
            )
            && self.interaction.layout_capture.is_none()
            && self.interaction.drag.session.is_none()
            && self.interaction.drag.external_session.is_none()
            && self.interaction.drag.external_completion.is_none()
            && self.interaction.drag.pending_external_completion.is_none()
    }
}

fn effective_layout_viewport(viewport: crate::gui::types::Rect) -> crate::gui::types::Rect {
    crate::gui::types::Rect::from_min_size(
        crate::gui::types::Point::new(viewport.min.x.floor(), viewport.min.y.floor()),
        crate::layout::Vector2::new(
            viewport.width().round().max(0.0),
            viewport.height().round().max(0.0),
        ),
    )
}

fn membership_matches_installed<Bridge, Message>(
    runtime: &SurfaceRuntime<Bridge, Message>,
    id: WidgetId,
    membership: [bool; 7],
) -> bool
where
    Bridge: RuntimeBridge<Message>,
{
    runtime
        .traversal
        .widgets
        .membership
        .get(&id)
        .copied()
        .is_some_and(|installed| installed == membership)
}

fn same_rect_bits(left: crate::gui::types::Rect, right: crate::gui::types::Rect) -> bool {
    left.min.x.to_bits() == right.min.x.to_bits()
        && left.min.y.to_bits() == right.min.y.to_bits()
        && left.max.x.to_bits() == right.max.x.to_bits()
        && left.max.y.to_bits() == right.max.y.to_bits()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        application::{DeclarativeEffectOwner, DeclarativeIdentityOrigin, SourceIdentitySeed},
        layout::{ContainerPolicy, LayoutCapabilities, SlotParams, Vector2},
        runtime::{
            ExactChangedRoot, SurfaceNode, SurfaceUpdate,
            surface::{
                KeyedNodeEvidence, OverlayEvidence, OverlayIdentity, SourceCompatibility,
                SourceIdentity, SourceMetadata, SourceTopology, SurfaceSourceKind,
            },
        },
        theme::{ResolvedAppearance, ThemeTokens},
        widgets::{
            Widget, WidgetCommon, WidgetInput, WidgetOutput, WidgetRevision, WidgetStyle,
            WidgetTone,
        },
    };
    use std::{rc::Rc, sync::Arc};

    #[derive(Clone)]
    struct InteractionWidget {
        common: WidgetCommon,
        revision: bool,
        geometry_changed: bool,
        paint_changed: bool,
        stateful: bool,
    }

    impl InteractionWidget {
        fn new(
            id: u64,
            revision: bool,
            geometry_changed: bool,
            paint_changed: bool,
            stateful: bool,
        ) -> Self {
            Self {
                common: WidgetCommon::fixed(id, 20.0, 20.0).with_keyboard_focus(),
                revision,
                geometry_changed,
                paint_changed,
                stateful,
            }
        }
    }

    impl Widget for InteractionWidget {
        fn revision(&self) -> WidgetRevision {
            WidgetRevision::exact((), self.geometry_changed, self.paint_changed, self.revision)
        }

        fn common(&self) -> &WidgetCommon {
            &self.common
        }

        fn common_mut(&mut self) -> &mut WidgetCommon {
            &mut self.common
        }

        fn needs_state_synchronization(&self) -> bool {
            self.stateful
        }

        fn handle_input(
            &mut self,
            _: crate::gui::types::Rect,
            _: WidgetInput,
        ) -> Option<WidgetOutput> {
            None
        }

        fn append_paint(
            &self,
            _: &mut Vec<crate::runtime::PaintPrimitive>,
            _: crate::gui::types::Rect,
            _: &crate::layout::LayoutOutput,
            _: &crate::theme::ThemeTokens,
        ) {
        }
    }

    #[derive(Default)]
    struct Bridge {
        revision: bool,
        path: Vec<usize>,
        authority_revision: u64,
        duplicate: bool,
        exact: bool,
        geometry_changed: bool,
        paint_changed: bool,
        stateful: bool,
        sibling_stateful: bool,
        overlap: bool,
        mapper_opaque: bool,
    }

    impl Bridge {
        fn surface(&self) -> crate::runtime::UiSurface<()> {
            crate::runtime::UiSurface::new(SurfaceNode::container(
                1,
                ContainerPolicy::default(),
                vec![
                    crate::runtime::SurfaceChild::new(
                        SlotParams::fill(),
                        SurfaceNode::widget(
                            InteractionWidget::new(
                                10,
                                self.revision,
                                self.geometry_changed && self.revision,
                                self.paint_changed && self.revision,
                                self.stateful,
                            ),
                            if self.mapper_opaque {
                                crate::runtime::WidgetMessageMapper::dynamic(|_| None)
                            } else {
                                crate::runtime::WidgetMessageMapper::none()
                            },
                        ),
                    ),
                    crate::runtime::SurfaceChild::new(
                        SlotParams::fill(),
                        SurfaceNode::widget(
                            InteractionWidget::new(
                                if self.duplicate { 10 } else { 20 },
                                false,
                                false,
                                false,
                                self.sibling_stateful,
                            ),
                            crate::runtime::WidgetMessageMapper::none(),
                        ),
                    ),
                ],
            ))
        }
    }

    impl RuntimeBridge<()> for Bridge {
        fn project_surface(&mut self) -> Arc<crate::runtime::UiSurface<()>> {
            crate::runtime::test_arc_surface(self.surface())
        }

        fn pull_surface(&mut self) -> crate::runtime::UiSurface<()> {
            self.surface()
        }

        fn surface_update_provider_authority(&self) -> Option<SurfaceUpdateProviderAuthority> {
            Some(SurfaceUpdateProviderAuthority {
                owner: 77,
                checked_revision: self.authority_revision,
            })
        }

        fn pull_surface_update(
            &mut self,
            request: crate::runtime::SurfaceRefreshRequest,
        ) -> SurfaceUpdate<()> {
            if !self.exact {
                return SurfaceUpdate::Full(self.surface());
            }
            SurfaceUpdate::ExactChangedRoots(crate::runtime::ExactChangedRoots {
                surface: self.surface(),
                runtime_identity: request.runtime_identity,
                request_revision: request.request_revision,
                active_surface_generation: request.active_surface_generation,
                viewport: request.viewport,
                window_environment: request.window_environment,
                provider_authority: request.expected_provider_authority,
                changed_roots: {
                    let mut roots = vec![ExactChangedRoot {
                        node_id: 10,
                        child_path: self.path.clone(),
                    }];
                    if self.overlap {
                        roots.push(ExactChangedRoot {
                            node_id: 10,
                            child_path: self.path.clone(),
                        });
                    }
                    roots
                },
            })
        }
    }

    #[derive(Clone, Copy)]
    enum FloatingMutation {
        Offset,
        Size,
        Style,
        Capabilities,
    }

    struct FloatingBridge {
        revision: bool,
        exact: bool,
        mutation: Option<FloatingMutation>,
    }

    impl FloatingBridge {
        fn surface(&self) -> crate::runtime::UiSurface<()> {
            let mutation = self.mutation;
            let mut child = SurfaceNode::container(
                30,
                ContainerPolicy::default(),
                vec![crate::runtime::SurfaceChild::fill(SurfaceNode::widget(
                    InteractionWidget::new(10, self.revision, false, false, false),
                    crate::runtime::WidgetMessageMapper::none(),
                ))],
            );
            if matches!(mutation, Some(FloatingMutation::Style)) {
                child = child.with_container_style(WidgetStyle::strong(WidgetTone::Accent));
            }
            if matches!(mutation, Some(FloatingMutation::Capabilities)) {
                child = child.with_layout_capabilities(LayoutCapabilities::new());
            }
            let (offset, size) = match mutation {
                Some(FloatingMutation::Offset) => (
                    crate::gui::types::Point::new(12.0, 5.0),
                    Vector2::new(40.0, 20.0),
                ),
                Some(FloatingMutation::Size) => (
                    crate::gui::types::Point::new(4.0, 5.0),
                    Vector2::new(64.0, 20.0),
                ),
                Some(FloatingMutation::Style | FloatingMutation::Capabilities) | None => (
                    crate::gui::types::Point::new(4.0, 5.0),
                    Vector2::new(40.0, 20.0),
                ),
            };
            crate::runtime::UiSurface::new(SurfaceNode::floating_layer(
                1, offset, size, child, true,
            ))
        }
    }

    impl RuntimeBridge<()> for FloatingBridge {
        fn project_surface(&mut self) -> Arc<crate::runtime::UiSurface<()>> {
            crate::runtime::test_arc_surface(self.surface())
        }

        fn pull_surface(&mut self) -> crate::runtime::UiSurface<()> {
            self.surface()
        }

        fn surface_update_provider_authority(&self) -> Option<SurfaceUpdateProviderAuthority> {
            Some(SurfaceUpdateProviderAuthority {
                owner: 88,
                checked_revision: 1,
            })
        }

        fn pull_surface_update(
            &mut self,
            request: crate::runtime::SurfaceRefreshRequest,
        ) -> SurfaceUpdate<()> {
            let surface = self.surface();
            if !self.exact {
                return SurfaceUpdate::Full(surface);
            }
            SurfaceUpdate::ExactChangedRoots(crate::runtime::ExactChangedRoots {
                surface,
                runtime_identity: request.runtime_identity,
                request_revision: request.request_revision,
                active_surface_generation: request.active_surface_generation,
                viewport: request.viewport,
                window_environment: request.window_environment,
                provider_authority: request.expected_provider_authority,
                changed_roots: vec![ExactChangedRoot {
                    node_id: 10,
                    child_path: vec![0, 0],
                }],
            })
        }
    }

    #[test]
    fn floating_layer_contract_mutations_match_forced_full_geometry_paint_and_hit_targets() {
        let cases = [
            ("offset", FloatingMutation::Offset),
            ("size", FloatingMutation::Size),
            ("style", FloatingMutation::Style),
            ("capabilities", FloatingMutation::Capabilities),
        ];
        for (name, mutation) in cases {
            let make = |exact| FloatingBridge {
                revision: false,
                exact,
                mutation: None,
            };
            let mut exact = SurfaceRuntime::new(make(true), Vector2::new(100.0, 60.0));
            let mut full = SurfaceRuntime::new(make(false), Vector2::new(100.0, 60.0));
            exact.bridge_mut().mutation = Some(mutation);
            exact.bridge_mut().revision = true;
            full.bridge_mut().mutation = Some(mutation);
            full.bridge_mut().revision = true;
            let before = exact.refresh_counters();
            exact.refresh_with_scope(crate::runtime::RepaintScope::Projection);
            full.refresh_with_scope(crate::runtime::RepaintScope::Projection);
            assert_eq!(
                exact.refresh_counters().runtime_projection,
                before.runtime_projection + 1,
                "{name} must use the full fallback"
            );
            assert_eq!(exact.layout(), full.layout(), "{name} geometry");
            assert_eq!(
                exact.paint_plan(&ThemeTokens::default()),
                full.paint_plan(&ThemeTokens::default()),
                "{name} paint"
            );
            assert_eq!(
                exact.traversal.widgets.pointer.order(),
                full.traversal.widgets.pointer.order(),
                "{name} pointer hit targets"
            );
            assert_eq!(
                exact.automation_snapshot(),
                full.automation_snapshot(),
                "{name} semantics"
            );
        }
    }

    #[derive(Clone, Copy)]
    enum OwnerMutation {
        KeyedOwnerReplacement,
        OverlayOwnerReplacement,
        OverlayAdded,
        OverlayRemoved,
    }

    struct OwnerBridge {
        exact: bool,
        revision: bool,
        mutation: Option<OwnerMutation>,
        keyed_owner: DeclarativeEffectOwner,
        overlay_owner: DeclarativeEffectOwner,
        replacement_owner: DeclarativeEffectOwner,
        added_owner: DeclarativeEffectOwner,
    }

    impl OwnerBridge {
        fn source_metadata(&self) -> SourceMetadata {
            let keyed_owner = match self.mutation {
                Some(OwnerMutation::KeyedOwnerReplacement) => self.replacement_owner,
                _ => self.keyed_owner,
            };
            let overlay_owner = match self.mutation {
                Some(OwnerMutation::OverlayOwnerReplacement) => self.replacement_owner,
                _ => self.overlay_owner,
            };
            let keyed_seed = SourceIdentitySeed {
                resolved_id: 10,
                structural_scope: 11,
                origin: DeclarativeIdentityOrigin::ExplicitContinuityKey,
                effect_owner: Some(keyed_owner),
            };
            let keyed = Rc::new(KeyedNodeEvidence::new(keyed_seed));
            keyed.set_compatibility(SourceCompatibility {
                surface_kind: SurfaceSourceKind::Widget,
                widget_compatibility_kind: Some("interaction"),
            });
            let overlays = match self.mutation {
                Some(OwnerMutation::OverlayAdded) => vec![
                    OverlayEvidence {
                        identity: OverlayIdentity {
                            structural_scope: 12,
                        },
                        layer_kind: crate::runtime::LayerKind::Modal,
                        effect_owner: Some(overlay_owner),
                    },
                    OverlayEvidence {
                        identity: OverlayIdentity {
                            structural_scope: 13,
                        },
                        layer_kind: crate::runtime::LayerKind::Tooltip,
                        effect_owner: Some(self.added_owner),
                    },
                ],
                Some(OwnerMutation::OverlayRemoved) => Vec::new(),
                _ => vec![OverlayEvidence {
                    identity: OverlayIdentity {
                        structural_scope: 12,
                    },
                    layer_kind: crate::runtime::LayerKind::Modal,
                    effect_owner: Some(overlay_owner),
                }],
            };
            SourceMetadata::new(
                SourceIdentity {
                    resolved_id: 10,
                    structural_scope: 11,
                    origin: DeclarativeIdentityOrigin::ExplicitContinuityKey,
                },
                SourceCompatibility {
                    surface_kind: SurfaceSourceKind::Widget,
                    widget_compatibility_kind: Some("interaction"),
                },
                SourceTopology {
                    keyed_nodes: vec![keyed],
                    overlays,
                },
            )
        }

        fn surface(&self) -> crate::runtime::UiSurface<()> {
            crate::runtime::UiSurface::new(SurfaceNode::container(
                1,
                ContainerPolicy::default(),
                vec![crate::runtime::SurfaceChild::fill(
                    SurfaceNode::widget(
                        InteractionWidget::new(10, self.revision, false, false, false),
                        crate::runtime::WidgetMessageMapper::none(),
                    )
                    .with_source_metadata(self.source_metadata()),
                )],
            ))
        }
    }

    impl RuntimeBridge<()> for OwnerBridge {
        fn project_surface(&mut self) -> Arc<crate::runtime::UiSurface<()>> {
            crate::runtime::test_arc_surface(self.surface())
        }

        fn pull_surface(&mut self) -> crate::runtime::UiSurface<()> {
            self.surface()
        }

        fn surface_update_provider_authority(&self) -> Option<SurfaceUpdateProviderAuthority> {
            Some(SurfaceUpdateProviderAuthority {
                owner: 99,
                checked_revision: 1,
            })
        }

        fn pull_surface_update(
            &mut self,
            request: crate::runtime::SurfaceRefreshRequest,
        ) -> SurfaceUpdate<()> {
            let surface = self.surface();
            if !self.exact {
                return SurfaceUpdate::Full(surface);
            }
            SurfaceUpdate::ExactChangedRoots(crate::runtime::ExactChangedRoots {
                surface,
                runtime_identity: request.runtime_identity,
                request_revision: request.request_revision,
                active_surface_generation: request.active_surface_generation,
                viewport: request.viewport,
                window_environment: request.window_environment,
                provider_authority: request.expected_provider_authority,
                changed_roots: vec![ExactChangedRoot {
                    node_id: 10,
                    child_path: vec![0],
                }],
            })
        }
    }

    #[test]
    fn source_owner_and_overlay_topology_mutations_match_forced_full_projection() {
        let cases = [
            (
                "keyed owner replacement",
                OwnerMutation::KeyedOwnerReplacement,
            ),
            (
                "overlay owner replacement",
                OwnerMutation::OverlayOwnerReplacement,
            ),
            ("overlay added", OwnerMutation::OverlayAdded),
            ("overlay removed", OwnerMutation::OverlayRemoved),
        ];
        for (name, mutation) in cases {
            let keyed_owner = DeclarativeEffectOwner::new();
            let overlay_owner = DeclarativeEffectOwner::new();
            let replacement_owner = DeclarativeEffectOwner::new();
            let added_owner = DeclarativeEffectOwner::new();
            let make = |exact| OwnerBridge {
                exact,
                revision: false,
                mutation: None,
                keyed_owner,
                overlay_owner,
                replacement_owner,
                added_owner,
            };
            let mut exact = SurfaceRuntime::new(make(true), Vector2::new(80.0, 40.0));
            let mut full = SurfaceRuntime::new(make(false), Vector2::new(80.0, 40.0));
            exact.bridge_mut().mutation = Some(mutation);
            exact.bridge_mut().revision = true;
            full.bridge_mut().mutation = Some(mutation);
            full.bridge_mut().revision = true;
            let before = exact.refresh_counters();
            exact.refresh_with_scope(crate::runtime::RepaintScope::Projection);
            full.refresh_with_scope(crate::runtime::RepaintScope::Projection);
            assert_eq!(
                exact.refresh_counters().runtime_projection,
                before.runtime_projection + 1,
                "{name} must use the full fallback"
            );
            assert_eq!(exact.layout(), full.layout(), "{name} geometry");
            assert_eq!(
                exact.paint_plan(&ThemeTokens::default()),
                full.paint_plan(&ThemeTokens::default()),
                "{name} paint"
            );
            assert_eq!(
                exact.traversal.widgets.pointer.order(),
                full.traversal.widgets.pointer.order(),
                "{name} pointer hit targets"
            );
            assert_eq!(
                exact.declarative_owner_projection().accepted_keyed_nodes(),
                full.declarative_owner_projection().accepted_keyed_nodes(),
                "{name} keyed owner projection"
            );
            assert_eq!(
                exact.declarative_owner_projection().accepted_overlays(),
                full.declarative_owner_projection().accepted_overlays(),
                "{name} overlay owner projection"
            );
            assert_eq!(
                exact
                    .declarative_owner_origin_for_handle(keyed_owner)
                    .is_some(),
                full.declarative_owner_origin_for_handle(keyed_owner)
                    .is_some(),
                "{name} keyed owner resolution"
            );
            assert_eq!(
                exact
                    .declarative_owner_origin_for_handle(overlay_owner)
                    .is_some(),
                full.declarative_owner_origin_for_handle(overlay_owner)
                    .is_some(),
                "{name} overlay owner resolution"
            );
            assert_eq!(
                exact
                    .declarative_owner_origin_for_handle(replacement_owner)
                    .is_some(),
                full.declarative_owner_origin_for_handle(replacement_owner)
                    .is_some(),
                "{name} replacement owner resolution"
            );
            assert_eq!(
                exact
                    .declarative_owner_origin_for_handle(added_owner)
                    .is_some(),
                full.declarative_owner_origin_for_handle(added_owner)
                    .is_some(),
                "{name} added owner resolution"
            );
        }
    }

    #[test]
    fn exact_interaction_leaf_swaps_without_projection_or_layout() {
        let mut runtime = SurfaceRuntime::new(
            Bridge {
                revision: false,
                path: vec![0],
                authority_revision: 1,
                duplicate: false,
                exact: true,
                ..Default::default()
            },
            Vector2::new(80.0, 40.0),
        );
        let before = runtime.refresh_counters();
        runtime.bridge_mut().revision = true;
        let evidence = inspect_interaction_path(
            runtime.surface.root(),
            runtime.bridge().surface().root(),
            &[0],
        )
        .expect("path evidence");
        assert_eq!(evidence.relation, InteractionLeafRevision::Interaction);
        assert!(membership_matches_installed(
            &runtime,
            10,
            evidence.previous_membership
        ));
        assert!(runtime.interaction_patch_state_is_safe_for(&HashSet::from([10])));
        runtime.refresh_with_scope(crate::runtime::RepaintScope::Projection);
        let after = runtime.refresh_counters();

        assert_eq!(after.runtime_projection, before.runtime_projection);
        assert_eq!(after.layout, before.layout);
        assert_eq!(after.widget_state_sync, before.widget_state_sync);
        assert_eq!(
            after.reconciliation_applied,
            before.reconciliation_applied + 1
        );
        assert_eq!(
            runtime.surface.find_widget(10).unwrap().revision(),
            WidgetRevision::exact((), false, false, true)
        );
        assert_eq!(
            runtime.surface.find_widget(20).unwrap().revision(),
            WidgetRevision::exact((), false, false, false)
        );
    }

    #[test]
    fn invalid_leaf_path_uses_the_same_candidate_for_full_fallback() {
        let mut runtime = SurfaceRuntime::new(
            Bridge {
                revision: false,
                path: vec![1],
                authority_revision: 1,
                duplicate: false,
                exact: true,
                ..Default::default()
            },
            Vector2::new(80.0, 40.0),
        );
        let before = runtime.refresh_counters();
        runtime.bridge_mut().revision = true;
        runtime.refresh_with_scope(crate::runtime::RepaintScope::Projection);
        let after = runtime.refresh_counters();

        assert_eq!(after.runtime_projection, before.runtime_projection + 1);
        assert_eq!(after.layout, before.layout);
        assert_eq!(
            runtime.surface.find_widget(10).unwrap().revision(),
            WidgetRevision::exact((), false, false, true)
        );
    }

    #[test]
    fn native_preparation_retains_the_interaction_variant_before_layout() {
        let mut runtime = SurfaceRuntime::new(
            Bridge {
                revision: false,
                path: vec![0],
                authority_revision: 1,
                duplicate: false,
                exact: true,
                ..Default::default()
            },
            Vector2::new(80.0, 40.0),
        );
        runtime.bridge_mut().revision = true;
        let prepared = runtime
            .prepare_fresh_surface_refresh(
                crate::runtime::RepaintScope::Projection,
                ResolvedAppearance::fixed(ThemeTokens::dark()),
            )
            .expect("exact interaction candidate should be prepared");
        assert!(matches!(
            prepared,
            super::super::fresh_surface_preparation::PreparedSurfaceRefresh::Interaction { .. }
        ));
    }

    #[test]
    fn provider_authority_drift_vetoes_native_interaction_publication() {
        let mut runtime = SurfaceRuntime::new(
            Bridge {
                revision: false,
                path: vec![0],
                authority_revision: 1,
                duplicate: false,
                exact: true,
                ..Default::default()
            },
            Vector2::new(80.0, 40.0),
        );
        runtime.bridge_mut().revision = true;
        let prepared = runtime
            .prepare_fresh_surface_refresh(
                crate::runtime::RepaintScope::Projection,
                ResolvedAppearance::fixed(ThemeTokens::dark()),
            )
            .expect("exact interaction candidate should be prepared");
        runtime.bridge_mut().authority_revision = 2;
        assert!(runtime.publish_prepared_surface_refresh(prepared).is_none());
        assert!(runtime.fresh_surface_request.is_some());
    }

    #[test]
    fn duplicate_ids_and_dirty_layout_use_full_fallback() {
        let mut duplicate = SurfaceRuntime::new(
            Bridge {
                revision: false,
                path: vec![0],
                authority_revision: 1,
                duplicate: true,
                exact: true,
                ..Default::default()
            },
            Vector2::new(80.0, 40.0),
        );
        let before = duplicate.refresh_counters();
        duplicate.bridge_mut().revision = true;
        duplicate.refresh_with_scope(crate::runtime::RepaintScope::Projection);
        assert_eq!(
            duplicate.refresh_counters().runtime_projection,
            before.runtime_projection + 1
        );

        let mut dirty = SurfaceRuntime::new(
            Bridge {
                revision: false,
                path: vec![0],
                authority_revision: 1,
                duplicate: false,
                exact: true,
                ..Default::default()
            },
            Vector2::new(80.0, 40.0),
        );
        let before = dirty.refresh_counters();
        dirty.bridge_mut().revision = true;
        dirty.external_layout_dirty = true;
        dirty.refresh_with_scope(crate::runtime::RepaintScope::Projection);
        assert_eq!(
            dirty.refresh_counters().runtime_projection,
            before.runtime_projection + 1
        );
    }

    #[test]
    fn bounded_admission_falls_back_for_overlapping_geometry_paint_state_and_mapper_changes() {
        let cases = [
            (
                "overlap",
                Bridge {
                    overlap: true,
                    ..base_bridge()
                },
            ),
            (
                "geometry",
                Bridge {
                    geometry_changed: true,
                    ..base_bridge()
                },
            ),
            (
                "paint",
                Bridge {
                    paint_changed: true,
                    ..base_bridge()
                },
            ),
            (
                "stateful",
                Bridge {
                    stateful: true,
                    ..base_bridge()
                },
            ),
            (
                "opaque mapper",
                Bridge {
                    mapper_opaque: true,
                    ..base_bridge()
                },
            ),
        ];

        for (name, bridge) in cases {
            let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(80.0, 40.0));
            let before = runtime.refresh_counters();
            runtime.bridge_mut().revision = true;
            runtime.refresh_with_scope(crate::runtime::RepaintScope::Projection);
            assert_eq!(
                runtime.refresh_counters().runtime_projection,
                before.runtime_projection + 1,
                "{name} must use the full fallback"
            );
        }
    }

    #[test]
    fn unchanged_stateful_sibling_keeps_identity_and_focus_during_exact_refresh() {
        let mut runtime = SurfaceRuntime::new(
            Bridge {
                sibling_stateful: true,
                ..base_bridge()
            },
            Vector2::new(80.0, 40.0),
        );
        assert!(runtime.focus_widget(20));
        let sibling_before = runtime
            .surface()
            .find_widget(20)
            .expect("stateful sibling")
            .widget() as *const dyn Widget as *const ();
        runtime.bridge_mut().revision = true;

        runtime.refresh_with_scope(crate::runtime::RepaintScope::Projection);

        let sibling_after = runtime
            .surface()
            .find_widget(20)
            .expect("stateful sibling")
            .widget() as *const dyn Widget as *const ();
        assert_eq!(sibling_after, sibling_before);
        assert_eq!(runtime.focused_widget(), Some(20));
        assert_eq!(
            runtime.surface().find_widget(10).unwrap().revision(),
            WidgetRevision::exact((), false, false, true)
        );
    }

    #[test]
    fn exact_and_forced_full_refreshes_have_equivalent_surface_and_layout() {
        let bridge = |exact| Bridge {
            revision: false,
            path: vec![0],
            authority_revision: 1,
            duplicate: false,
            exact,
            ..Default::default()
        };
        let mut exact = SurfaceRuntime::new(bridge(true), Vector2::new(80.0, 40.0));
        let mut full = SurfaceRuntime::new(bridge(false), Vector2::new(80.0, 40.0));
        exact.bridge_mut().revision = true;
        full.bridge_mut().revision = true;
        exact.refresh_with_scope(crate::runtime::RepaintScope::Projection);
        full.refresh_with_scope(crate::runtime::RepaintScope::Projection);
        assert_eq!(exact.layout(), full.layout());
        assert_eq!(exact.automation_snapshot(), full.automation_snapshot());
        assert_eq!(
            exact.surface().find_widget(10).unwrap().revision(),
            full.surface().find_widget(10).unwrap().revision()
        );
    }

    #[derive(Clone, Default, Debug, PartialEq, Eq)]
    struct HookCounts {
        clones: u32,
        revisions: u32,
        state_sync: u32,
        paint: u32,
        semantics: u32,
    }

    struct SentinelWidget {
        common: WidgetCommon,
        revision: bool,
        counts: Arc<std::sync::Mutex<HookCounts>>,
    }

    impl Clone for SentinelWidget {
        fn clone(&self) -> Self {
            if let Ok(mut counts) = self.counts.lock() {
                counts.clones = counts.clones.saturating_add(1);
            }
            Self {
                common: self.common.clone(),
                revision: self.revision,
                counts: Arc::clone(&self.counts),
            }
        }
    }

    impl Widget for SentinelWidget {
        fn revision(&self) -> WidgetRevision {
            if let Ok(mut counts) = self.counts.lock() {
                counts.revisions = counts.revisions.saturating_add(1);
            }
            WidgetRevision::exact((), (), (), self.revision)
        }

        fn needs_state_synchronization(&self) -> bool {
            if let Ok(mut counts) = self.counts.lock() {
                counts.state_sync = counts.state_sync.saturating_add(1);
            }
            false
        }

        fn automation_semantics(&self) -> crate::gui::automation::AutomationNodeSemantics {
            if let Ok(mut counts) = self.counts.lock() {
                counts.semantics = counts.semantics.saturating_add(1);
            }
            crate::gui::automation::AutomationNodeSemantics::new(
                crate::gui::automation::AutomationRole::Custom,
            )
        }

        fn common(&self) -> &WidgetCommon {
            &self.common
        }

        fn common_mut(&mut self) -> &mut WidgetCommon {
            &mut self.common
        }

        fn handle_input(
            &mut self,
            _: crate::gui::types::Rect,
            _: WidgetInput,
        ) -> Option<WidgetOutput> {
            None
        }

        fn append_paint(
            &self,
            _: &mut Vec<crate::runtime::PaintPrimitive>,
            _: crate::gui::types::Rect,
            _: &crate::layout::LayoutOutput,
            _: &crate::theme::ThemeTokens,
        ) {
            if let Ok(mut counts) = self.counts.lock() {
                counts.paint = counts.paint.saturating_add(1);
            }
        }
    }

    struct SentinelBridge {
        revision: bool,
        width: usize,
        depth: usize,
        old_sentinel: Arc<std::sync::Mutex<HookCounts>>,
        candidate_sentinel: Arc<std::sync::Mutex<HookCounts>>,
        old_changed: Arc<std::sync::Mutex<HookCounts>>,
        candidate_changed: Arc<std::sync::Mutex<HookCounts>>,
    }

    impl SentinelBridge {
        fn surface(&self, candidate: bool) -> crate::runtime::UiSurface<()> {
            let mut node = SurfaceNode::container(
                1_000,
                ContainerPolicy::default(),
                (0..self.width)
                    .map(|index| {
                        crate::runtime::SurfaceChild::new(
                            SlotParams::fill(),
                            SurfaceNode::widget(
                                SentinelWidget {
                                    common: WidgetCommon::fixed(
                                        if index == 0 {
                                            10
                                        } else {
                                            10_000 + index as u64
                                        },
                                        20.0,
                                        20.0,
                                    ),
                                    revision: self.revision && index == 0,
                                    counts: if index == 0 {
                                        if candidate {
                                            Arc::clone(&self.candidate_changed)
                                        } else {
                                            Arc::clone(&self.old_changed)
                                        }
                                    } else if candidate {
                                        Arc::clone(&self.candidate_sentinel)
                                    } else {
                                        Arc::clone(&self.old_sentinel)
                                    },
                                },
                                crate::runtime::WidgetMessageMapper::none(),
                            ),
                        )
                    })
                    .collect(),
            );
            for level in 1..self.depth {
                node = SurfaceNode::container(
                    1_000 + level as u64,
                    ContainerPolicy::default(),
                    vec![crate::runtime::SurfaceChild::new(SlotParams::fill(), node)],
                );
            }
            crate::runtime::UiSurface::new(node)
        }
    }

    impl RuntimeBridge<()> for SentinelBridge {
        fn project_surface(&mut self) -> Arc<crate::runtime::UiSurface<()>> {
            crate::runtime::test_arc_surface(self.surface(false))
        }

        fn pull_surface_update(
            &mut self,
            request: crate::runtime::SurfaceRefreshRequest,
        ) -> SurfaceUpdate<()> {
            SurfaceUpdate::ExactChangedRoots(crate::runtime::ExactChangedRoots {
                surface: self.surface(true),
                runtime_identity: request.runtime_identity,
                request_revision: request.request_revision,
                active_surface_generation: request.active_surface_generation,
                viewport: request.viewport,
                window_environment: request.window_environment,
                provider_authority: request.expected_provider_authority,
                changed_roots: vec![ExactChangedRoot {
                    node_id: 10,
                    child_path: vec![0; self.depth],
                }],
            })
        }

        fn surface_update_provider_authority(&self) -> Option<SurfaceUpdateProviderAuthority> {
            Some(SurfaceUpdateProviderAuthority {
                owner: 91,
                checked_revision: 1,
            })
        }
    }

    #[test]
    fn wide_and_deep_exact_refreshes_do_not_visit_unchanged_widget_hooks() {
        for (width, depth) in [(64, 1), (2, 12)] {
            let old_sentinel = Arc::new(std::sync::Mutex::new(HookCounts::default()));
            let candidate_sentinel = Arc::new(std::sync::Mutex::new(HookCounts::default()));
            let old_changed = Arc::new(std::sync::Mutex::new(HookCounts::default()));
            let candidate_changed = Arc::new(std::sync::Mutex::new(HookCounts::default()));
            let mut runtime = SurfaceRuntime::new(
                SentinelBridge {
                    revision: false,
                    width,
                    depth,
                    old_sentinel: Arc::clone(&old_sentinel),
                    candidate_sentinel,
                    old_changed,
                    candidate_changed,
                },
                Vector2::new(160.0, 80.0),
            );
            let before_hooks = old_sentinel.lock().unwrap().clone();
            let before = runtime.refresh_counters();
            runtime.bridge_mut().revision = true;

            runtime.refresh_with_scope(crate::runtime::RepaintScope::Projection);

            assert_eq!(
                *old_sentinel.lock().unwrap(),
                before_hooks,
                "unchanged {width}-wide/{depth}-deep sentinel was visited"
            );
            assert_eq!(
                runtime.refresh_counters().runtime_projection,
                before.runtime_projection
            );
            assert_eq!(runtime.refresh_counters().layout, before.layout);
            assert_eq!(
                runtime.refresh_counters().widget_state_sync,
                before.widget_state_sync
            );
            assert_eq!(
                runtime.refresh_counters().base_paint_plan_rebuilds,
                before.base_paint_plan_rebuilds
            );
            let before_ax = old_sentinel.lock().unwrap().clone();
            let _ = runtime.automation_snapshot();
            let after_ax = old_sentinel.lock().unwrap().clone();
            assert!(
                after_ax.semantics > before_ax.semantics,
                "explicit AX snapshot should account for its own semantic traversal"
            );
        }
    }

    fn base_bridge() -> Bridge {
        Bridge {
            revision: false,
            path: vec![0],
            authority_revision: 1,
            duplicate: false,
            exact: true,
            ..Default::default()
        }
    }
}

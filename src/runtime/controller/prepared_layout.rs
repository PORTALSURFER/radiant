use super::{SurfaceRuntime, SurfaceTraversalIndex};
use crate::gui::layout_core::{
    LayoutAuthorityEvidence, LayoutInputEvidence, LayoutOutput, PreparedLayoutPass,
    RootLayoutAuthorityOwner,
};
use crate::layout::LayoutNode;
use crate::runtime::WindowEnvironment;
use crate::runtime::{RuntimeBridge, RuntimeLifecyclePhase, SurfaceRuntimeProjection};

use super::layout_state::RuntimeLayoutContainerStateCandidate;

/// One private runtime-layout result that has not been published to active
/// runtime state.
///
/// The prepared pass owns the candidate output, scratch, and cache delta. The
/// mounted-state candidate owns only staged values; retained accepted values
/// remain in the runtime store and are read through its immutable source.
/// Authority is copied as evidence, never inferred from visible values.
#[allow(dead_code)]
pub(in crate::runtime::controller) struct RuntimeLayoutCandidate {
    prepared_layout: PreparedLayoutPass,
    mounted_state: RuntimeLayoutContainerStateCandidate,
    authority: RuntimeLayoutAuthoritySnapshot,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RuntimeLayoutAuthoritySnapshot {
    input: LayoutInputEvidence,
    layout_state_generation: u64,
    accepted_mounted_source_present: bool,
    candidate_mounted_source_present: bool,
}

#[allow(dead_code)]
impl RuntimeLayoutCandidate {
    /// Borrow the candidate-owned output without exposing mutable or active
    /// runtime layout state.
    pub(in crate::runtime::controller) fn layout_output(&self) -> Option<&LayoutOutput> {
        self.prepared_layout.output()
    }

    /// Return whether all runtime and engine evidence still matches this
    /// candidate. A false result is a discard signal; it never authorizes a
    /// partial publication.
    pub(in crate::runtime::controller) fn is_current<Bridge, Message>(
        &self,
        runtime: &SurfaceRuntime<Bridge, Message>,
    ) -> bool
    where
        Bridge: RuntimeBridge<Message>,
    {
        if runtime.layout_authority_exhausted
            || runtime.mounted_layout_source_present
                != self.authority.accepted_mounted_source_present
            || runtime.layout_state_generation != self.authority.layout_state_generation
        {
            return false;
        }
        let Some(current_input) =
            runtime.runtime_layout_input_evidence(self.authority.candidate_mounted_source_present)
        else {
            return false;
        };
        current_input == self.authority.input
            && self
                .prepared_layout
                .validate_for_engine(&runtime.layout_engine, current_input)
                .is_ok()
    }

    /// Explicitly abandon the candidate. Dropping the wrapper returns the
    /// prepared workspace and drops staged state exactly once.
    pub(in crate::runtime::controller) fn discard(self) {}
}

/// One private, single-consumption publication transaction for an ordinary
/// non-virtual runtime surface.
///
/// The traversal is produced by the accepted surface projection immediately
/// before the layout candidate is prepared. It is never supplied by a caller,
/// and remains owned here until the transaction either publishes it or is
/// discarded.
#[allow(dead_code)]
pub(in crate::runtime::controller) struct PreparedRuntimeLayoutPublication<Message> {
    runtime_layout: RuntimeLayoutCandidate,
    traversal: SurfaceTraversalIndex<Message>,
    authority: RuntimeLayoutPublicationAuthority,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RuntimeLayoutPublicationAuthority {
    runtime_identity: u64,
    lifecycle_phase: RuntimeLifecyclePhase,
    lifecycle_transition_sequence: u64,
    window_environment: WindowEnvironment,
    root_id: u64,
    root_state_version: u64,
    input: LayoutInputEvidence,
    layout_state_generation: u64,
    accepted_mounted_source_present: bool,
    candidate_mounted_source_present: bool,
}

#[allow(dead_code)]
impl<Message> PreparedRuntimeLayoutPublication<Message> {
    /// Return whether every captured runtime and engine authority still
    /// describes the active ordinary surface. This method is observational:
    /// it does not install traversal, mutate mounted state, or touch output.
    pub(in crate::runtime::controller) fn is_current<Bridge>(
        &self,
        runtime: &SurfaceRuntime<Bridge, Message>,
    ) -> bool
    where
        Bridge: RuntimeBridge<Message>,
    {
        let authority = self.authority;
        authority.runtime_identity == runtime.runtime_identity()
            && authority.lifecycle_phase == RuntimeLifecyclePhase::Running
            && authority.lifecycle_phase == runtime.lifecycle_phase()
            && authority.lifecycle_transition_sequence == runtime.lifecycle_transition_sequence()
            && authority.window_environment == runtime.window_environment
            && authority.root_id == runtime.layout_root.id()
            && authority.root_state_version == runtime.layout_root.state_version()
            && authority.input == self.runtime_layout.authority.input
            && authority.layout_state_generation
                == self.runtime_layout.authority.layout_state_generation
            && authority.accepted_mounted_source_present
                == self
                    .runtime_layout
                    .authority
                    .accepted_mounted_source_present
            && authority.candidate_mounted_source_present
                == self
                    .runtime_layout
                    .authority
                    .candidate_mounted_source_present
            && self.traversal.virtual_layout_registrations.is_empty()
            && runtime.virtual_layout.is_empty()
            && self.runtime_layout.mounted_state.source_present()
                == authority.candidate_mounted_source_present
            && runtime.runtime_layout_candidate_is_current(&self.runtime_layout)
    }

    /// Explicitly discard the transaction. Dropping it returns the engine
    /// workspace and drops staged mounted values exactly once.
    pub(in crate::runtime::controller) fn discard(self) {}
}

#[allow(dead_code)]
impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    /// Prepare one private publication transaction from the accepted current
    /// ordinary surface. Projection and mounted declarations share this exact
    /// owned traversal; callers cannot provide unrelated traversal evidence.
    pub(in crate::runtime::controller) fn prepare_runtime_layout_publication(
        &mut self,
    ) -> Option<PreparedRuntimeLayoutPublication<Message>> {
        if self.layout_authority_exhausted
            || self.lifecycle_phase() != RuntimeLifecyclePhase::Running
            || !self.virtual_layout.is_empty()
        {
            return None;
        }

        let SurfaceRuntimeProjection {
            layout_root,
            traversal,
            source: _,
        } = self.surface.runtime_projection();
        if layout_root != self.layout_root || !traversal.virtual_layout_registrations.is_empty() {
            return None;
        }

        let runtime_layout =
            self.prepare_runtime_layout_candidate_from_projection(&layout_root, &traversal)?;
        let authority = RuntimeLayoutPublicationAuthority {
            runtime_identity: self.runtime_identity(),
            lifecycle_phase: self.lifecycle_phase(),
            lifecycle_transition_sequence: self.lifecycle_transition_sequence(),
            window_environment: self.window_environment,
            root_id: layout_root.id(),
            root_state_version: layout_root.state_version(),
            candidate_mounted_source_present: runtime_layout
                .authority
                .candidate_mounted_source_present,
            input: runtime_layout.authority.input,
            layout_state_generation: runtime_layout.authority.layout_state_generation,
            accepted_mounted_source_present: runtime_layout
                .authority
                .accepted_mounted_source_present,
        };

        Some(PreparedRuntimeLayoutPublication {
            runtime_layout,
            traversal,
            authority,
        })
    }

    /// Return whether a prepared publication can still be consumed without
    /// changing active runtime state.
    pub(in crate::runtime::controller) fn prepared_runtime_layout_publication_is_current(
        &self,
        publication: &PreparedRuntimeLayoutPublication<Message>,
    ) -> bool {
        publication.is_current(self)
    }

    /// Consume one checked publication transaction. All fallible validation is
    /// complete before the engine commit; the remaining operations are the
    /// established direct publication sequence and contain no recoverable
    /// veto point.
    pub(in crate::runtime::controller) fn publish_prepared_runtime_layout(
        &mut self,
        publication: PreparedRuntimeLayoutPublication<Message>,
    ) -> bool {
        if !publication.is_current(self) {
            publication.discard();
            return false;
        }

        let PreparedRuntimeLayoutPublication {
            runtime_layout,
            traversal,
            authority: _,
        } = publication;
        let RuntimeLayoutCandidate {
            prepared_layout,
            mounted_state,
            authority,
        } = runtime_layout;
        if prepared_layout
            .commit(&mut self.layout_engine, &mut self.layout, authority.input)
            .is_err()
        {
            std::process::abort();
        }

        self.install_traversal_with_candidate(traversal, mounted_state);
        self.sync_scroll_offsets();
        self.record_completed_layout();
        true
    }

    fn prepare_runtime_layout_candidate_from_projection(
        &mut self,
        layout_root: &LayoutNode,
        traversal: &SurfaceTraversalIndex<Message>,
    ) -> Option<RuntimeLayoutCandidate> {
        let mounted_state = self.prepare_layout_container_state_candidate(traversal);
        if !mounted_state.is_admissible() {
            return None;
        }
        let candidate_mounted_source_present = mounted_state.source_present();
        let authority = RuntimeLayoutAuthoritySnapshot {
            input: self.runtime_layout_input_evidence(candidate_mounted_source_present)?,
            layout_state_generation: self.layout_state_generation,
            accepted_mounted_source_present: self.mounted_layout_source_present,
            candidate_mounted_source_present,
        };

        let prepared_layout = if candidate_mounted_source_present {
            let container_state_source = self.interaction.layout_state.read_source(&mounted_state);
            self.layout_engine.prepare_layout_with_direction_and_source(
                layout_root,
                self.viewport,
                &self.layout_state,
                self.layout_debug_options,
                self.surface.resolved_environment().writing_direction(),
                Some(&container_state_source),
                authority.input,
            )
        } else {
            self.layout_engine.prepare_layout_with_direction_and_source(
                layout_root,
                self.viewport,
                &self.layout_state,
                self.layout_debug_options,
                self.surface.resolved_environment().writing_direction(),
                None,
                authority.input,
            )
        };
        if !prepared_layout.is_usable() {
            return None;
        }

        Some(RuntimeLayoutCandidate {
            prepared_layout,
            mounted_state,
            authority,
        })
    }

    pub(in crate::runtime::controller) fn runtime_layout_candidate_is_current(
        &self,
        candidate: &RuntimeLayoutCandidate,
    ) -> bool {
        candidate.is_current(self)
    }

    fn runtime_layout_input_evidence(
        &self,
        mounted_source_present: bool,
    ) -> Option<LayoutInputEvidence> {
        if self.layout_authority_exhausted {
            return None;
        }
        let input = LayoutInputEvidence::new_with_direction(
            Some(self.layout_root_authority),
            Some(self.layout_state_authority),
            mounted_source_present.then_some(self.mounted_layout_source_authority),
            self.viewport,
            self.layout_debug_options,
            self.surface.resolved_environment().writing_direction(),
        );
        input
            .is_valid_for_prepare_with_direction(
                self.viewport,
                self.layout_debug_options,
                mounted_source_present,
                self.surface.resolved_environment().writing_direction(),
            )
            .then_some(input)
    }

    pub(in crate::runtime::controller) fn runtime_layout_input_evidence_for_root(
        &self,
        root_authority: LayoutAuthorityEvidence<RootLayoutAuthorityOwner>,
        mounted_source_present: bool,
    ) -> Option<LayoutInputEvidence> {
        self.runtime_layout_input_evidence_for_root_with_direction(
            root_authority,
            mounted_source_present,
            self.surface.resolved_environment().writing_direction(),
        )
    }

    pub(in crate::runtime::controller) fn runtime_layout_input_evidence_for_root_with_direction(
        &self,
        root_authority: LayoutAuthorityEvidence<RootLayoutAuthorityOwner>,
        mounted_source_present: bool,
        direction: crate::gui::layout_core::WritingDirection,
    ) -> Option<LayoutInputEvidence> {
        if self.layout_authority_exhausted {
            return None;
        }
        let input = LayoutInputEvidence::new_with_direction(
            Some(root_authority),
            Some(self.layout_state_authority),
            mounted_source_present.then_some(self.mounted_layout_source_authority),
            self.viewport,
            self.layout_debug_options,
            direction,
        );
        input
            .is_valid_for_prepare_with_direction(
                self.viewport,
                self.layout_debug_options,
                mounted_source_present,
                direction,
            )
            .then_some(input)
    }

    pub(super) fn replace_layout_root(&mut self, layout_root: LayoutNode) {
        self.layout_root = layout_root;
        if !advance_layout_authority(&mut self.layout_root_authority, true) {
            self.layout_authority_exhausted = true;
        }
    }

    pub(super) fn note_layout_state_mutation(&mut self) {
        self.layout_state_generation = self.layout_state_generation.saturating_add(1);
        if !advance_layout_authority(&mut self.layout_state_authority, false) {
            self.layout_authority_exhausted = true;
        }
    }

    pub(super) fn note_mounted_layout_source_mutation(&mut self, replaces_owner: bool) {
        if !advance_layout_authority(&mut self.mounted_layout_source_authority, replaces_owner) {
            self.layout_authority_exhausted = true;
        }
    }
}

fn advance_layout_authority<Owner>(
    evidence: &mut LayoutAuthorityEvidence<Owner>,
    replaces_owner: bool,
) -> bool {
    (!replaces_owner || evidence.advance_authority_generation()) && evidence.advance_revision()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        gui::types::Vector2,
        layout::{
            Constraints, ContainerKind, ContainerPolicy, Controlled, LayoutDebugOptions,
            OverflowPolicy, SizeModeCross, SizeModeMain, SlotParams,
        },
        runtime::{
            RuntimeBridge, SurfaceChild, SurfaceNode, UiSurface, WidgetMessageMapper,
            test_arc_surface,
        },
        widgets::{ButtonWidget, TextWidget, WidgetSizing},
    };
    use std::sync::Arc;

    struct CandidateBridge;

    impl RuntimeBridge<()> for CandidateBridge {
        fn project_surface(&mut self) -> Arc<UiSurface<()>> {
            test_arc_surface(UiSurface::new(SurfaceNode::container(
                1,
                ContainerPolicy::default(),
                Vec::new(),
            )))
        }
    }

    fn runtime() -> SurfaceRuntime<CandidateBridge, ()> {
        SurfaceRuntime::new(CandidateBridge, Vector2::new(100.0, 40.0))
    }

    struct SplitCandidateBridge {
        ratio: f32,
        generation: u64,
        runtime_owned: bool,
    }

    impl SplitCandidateBridge {
        fn surface(&self) -> UiSurface<()> {
            let split_policy = crate::layout::SplitPanePolicy::default();
            let split = SurfaceNode::container(
                10,
                ContainerPolicy {
                    kind: ContainerKind::SplitPane,
                    split_pane: split_policy,
                    ..ContainerPolicy::default()
                },
                vec![
                    SurfaceChild::fill(SurfaceNode::widget(
                        TextWidget::new(11, "first", WidgetSizing::fixed(Vector2::new(20.0, 20.0))),
                        WidgetMessageMapper::none(),
                    )),
                    SurfaceChild::fill(SurfaceNode::widget(
                        TextWidget::new(
                            12,
                            "second",
                            WidgetSizing::fixed(Vector2::new(20.0, 20.0)),
                        ),
                        WidgetMessageMapper::none(),
                    )),
                ],
            );
            let split = split.with_split_pane_runtime_mode(Some(if self.runtime_owned {
                crate::gui::layout_core::SplitPaneRuntimeMode::RuntimeOwned {
                    collapse_policy: None,
                }
            } else {
                crate::gui::layout_core::SplitPaneRuntimeMode::Controlled(Controlled::new(
                    self.ratio,
                    self.generation,
                ))
            }));
            let split = if self.runtime_owned {
                split.with_layout_capabilities(
                    crate::gui::layout_core::runtime_owned_split_pane_capabilities(
                        split_policy,
                        None,
                    ),
                )
            } else {
                split
            };
            UiSurface::new(split)
        }
    }

    impl RuntimeBridge<()> for SplitCandidateBridge {
        fn project_surface(&mut self) -> Arc<UiSurface<()>> {
            test_arc_surface(self.surface())
        }
    }

    struct ScrollBridge;

    impl RuntimeBridge<()> for ScrollBridge {
        fn project_surface(&mut self) -> Arc<UiSurface<()>> {
            test_arc_surface(UiSurface::new(SurfaceNode::container(
                1,
                ContainerPolicy {
                    kind: ContainerKind::ScrollView,
                    overflow: OverflowPolicy::Scroll,
                    ..ContainerPolicy::default()
                },
                vec![SurfaceChild::new(
                    SlotParams {
                        size_main: SizeModeMain::Intrinsic,
                        size_cross: SizeModeCross::Fill,
                        constraints: Constraints::unconstrained(),
                        margin: Default::default(),
                        align_cross_override: None,
                        allow_fixed_compress: false,
                    },
                    SurfaceNode::widget(
                        TextWidget::new(
                            2,
                            "scroll content",
                            WidgetSizing::fixed(Vector2::new(80.0, 400.0)),
                        ),
                        WidgetMessageMapper::none(),
                    ),
                )],
            )))
        }
    }

    struct TraversalBridge;

    impl RuntimeBridge<()> for TraversalBridge {
        fn project_surface(&mut self) -> Arc<UiSurface<()>> {
            let button = |id, label| {
                SurfaceNode::widget(
                    ButtonWidget::new(id, label, WidgetSizing::fixed(Vector2::new(40.0, 20.0))),
                    WidgetMessageMapper::none(),
                )
            };
            let scroll = SurfaceNode::container(
                3,
                ContainerPolicy {
                    kind: ContainerKind::ScrollView,
                    overflow: OverflowPolicy::Scroll,
                    ..ContainerPolicy::default()
                },
                vec![SurfaceChild::fill(button(4, "scroll"))],
            );
            test_arc_surface(UiSurface::new(SurfaceNode::container(
                1,
                ContainerPolicy::default(),
                vec![
                    SurfaceChild::fill(button(2, "first")),
                    SurfaceChild::fill(scroll),
                    SurfaceChild::fill(button(5, "last")),
                ],
            )))
        }
    }

    #[test]
    fn candidate_matches_direct_output_and_direct_layout_starts_without_workspace() {
        let mut runtime = runtime();
        runtime.relayout_current_surface();
        let direct = runtime.layout().clone();
        let publication = runtime
            .prepare_runtime_layout_publication()
            .expect("ordinary runtime layout should admit");

        assert_eq!(publication.runtime_layout.layout_output(), Some(&direct));
        assert!(runtime.prepared_runtime_layout_publication_is_current(&publication));
        publication.discard();
        assert_eq!(runtime.layout(), &direct);
    }

    #[test]
    fn candidate_workspace_contention_falls_back_without_partial_authority() {
        let mut runtime = runtime();
        let first = runtime
            .prepare_runtime_layout_publication()
            .expect("first candidate should own the workspace");
        assert!(runtime.prepare_runtime_layout_publication().is_none());
        drop(first);
        assert!(runtime.prepare_runtime_layout_publication().is_some());
    }

    #[test]
    fn candidate_layout_reads_staged_mounted_state_without_active_commit() {
        let mut runtime = SurfaceRuntime::new(
            SplitCandidateBridge {
                ratio: 0.2,
                generation: 1,
                runtime_owned: false,
            },
            Vector2::new(100.0, 40.0),
        );
        runtime.relayout_current_surface();
        let active_before = runtime.layout().clone();
        let publication = runtime
            .prepare_runtime_layout_publication()
            .expect("current split surface should be admitted");

        assert_eq!(
            publication.runtime_layout.layout_output(),
            Some(&active_before)
        );
        assert_eq!(runtime.layout(), &active_before);
        assert!(runtime.prepared_runtime_layout_publication_is_current(&publication));
        publication.discard();
    }

    #[test]
    fn publication_commits_split_layout_traversal_separators_and_completed_context_once() {
        let mut prepared = SurfaceRuntime::new(
            SplitCandidateBridge {
                ratio: 0.35,
                generation: 1,
                runtime_owned: true,
            },
            Vector2::new(100.0, 40.0),
        );
        let mut direct = SurfaceRuntime::new(
            SplitCandidateBridge {
                ratio: 0.35,
                generation: 1,
                runtime_owned: true,
            },
            Vector2::new(100.0, 40.0),
        );
        direct.relayout_current_surface();
        let publication = prepared
            .prepare_runtime_layout_publication()
            .expect("runtime-owned split should admit prepared publication");
        assert!(prepared.prepared_runtime_layout_publication_is_current(&publication));
        assert_eq!(
            publication.runtime_layout.layout_output(),
            Some(direct.layout())
        );

        assert!(prepared.publish_prepared_runtime_layout(publication));
        assert_eq!(prepared.layout(), direct.layout());
        assert_eq!(
            prepared.split_pane_separator_projections(),
            direct.split_pane_separator_projections()
        );
        assert_eq!(prepared.completed_layout, direct.completed_layout);
        assert_eq!(
            prepared.traversal.widgets.focusable.order(),
            direct.traversal.widgets.focusable.order()
        );
        assert_eq!(
            prepared.traversal.widgets.pointer.order(),
            direct.traversal.widgets.pointer.order()
        );
        assert_eq!(
            prepared.traversal.widgets.keyboard_focus.order(),
            direct.traversal.widgets.keyboard_focus.order()
        );
        assert_eq!(
            prepared.traversal.widgets.wheel.order(),
            direct.traversal.widgets.wheel.order()
        );
    }

    #[test]
    fn publication_matches_direct_dirty_debug_layout() {
        let mut prepared = runtime();
        let mut direct = runtime();
        prepared.layout_engine.mark_measure_dirty(1);
        direct.layout_engine.mark_measure_dirty(1);
        prepared.layout_debug_options = LayoutDebugOptions::bounds_only();
        direct.layout_debug_options = LayoutDebugOptions::bounds_only();

        let publication = prepared
            .prepare_runtime_layout_publication()
            .expect("dirty debug layout should admit prepared publication");
        direct.relayout_current_surface();
        assert!(prepared.publish_prepared_runtime_layout(publication));
        assert_eq!(prepared.layout(), direct.layout());
        assert_eq!(prepared.completed_layout, direct.completed_layout);
        assert!(!prepared.layout_engine.has_explicit_dirty());
    }

    #[test]
    fn publication_matches_direct_scroll_clamp_and_completed_state() {
        let mut prepared = SurfaceRuntime::new(ScrollBridge, Vector2::new(100.0, 80.0));
        let mut direct = SurfaceRuntime::new(ScrollBridge, Vector2::new(100.0, 80.0));
        let offset = Vector2::new(10_000.0, 10_000.0);
        prepared.layout_state.scroll_offsets.insert(1, offset);
        direct.layout_state.scroll_offsets.insert(1, offset);
        prepared.note_layout_state_mutation();
        direct.note_layout_state_mutation();

        let publication = prepared
            .prepare_runtime_layout_publication()
            .expect("scroll clamp should admit prepared publication");
        direct.relayout_current_surface();
        assert!(prepared.publish_prepared_runtime_layout(publication));
        assert_eq!(prepared.layout(), direct.layout());
        assert_eq!(
            prepared.layout_state.scroll_offset(1),
            direct.layout_state.scroll_offset(1)
        );
        assert_eq!(prepared.completed_layout, direct.completed_layout);
    }

    #[test]
    fn publication_preserves_all_traversal_orders_after_visibility_refresh() {
        let mut prepared = SurfaceRuntime::new(TraversalBridge, Vector2::new(120.0, 80.0));
        let direct = SurfaceRuntime::new(TraversalBridge, Vector2::new(120.0, 80.0));
        let publication = prepared
            .prepare_runtime_layout_publication()
            .expect("ordinary traversal should admit prepared publication");

        assert!(prepared.publish_prepared_runtime_layout(publication));
        assert_eq!(
            prepared.traversal.widgets.hit_order,
            direct.traversal.widgets.hit_order
        );
        assert_eq!(
            prepared.traversal.widgets.focusable.order(),
            direct.traversal.widgets.focusable.order()
        );
        assert_eq!(
            prepared.traversal.widgets.pointer.order(),
            direct.traversal.widgets.pointer.order()
        );
        assert_eq!(
            prepared.traversal.widgets.keyboard_focus.order(),
            direct.traversal.widgets.keyboard_focus.order()
        );
        assert_eq!(
            prepared.traversal.widgets.wheel.order(),
            direct.traversal.widgets.wheel.order()
        );
        assert_eq!(
            prepared.traversal.widgets.wheel_targets.visible(),
            direct.traversal.widgets.wheel_targets.visible()
        );
    }

    #[test]
    fn lifecycle_veto_does_not_mutate_active_publication_state() {
        let mut runtime = runtime();
        let publication = runtime
            .prepare_runtime_layout_publication()
            .expect("ordinary runtime should admit prepared publication");
        let layout_before = runtime.layout.clone();
        let root_authority_before = runtime.layout_root_authority;
        let state_authority_before = runtime.layout_state_authority;
        let mounted_authority_before = runtime.mounted_layout_source_authority;
        let layout_state_generation_before = runtime.layout_state_generation;
        let mounted_source_present_before = runtime.mounted_layout_source_present;
        let completed_before = runtime.completed_layout;
        let diagnostics_before = runtime.last_layout_state_diagnostics;
        let repaint_before = runtime.repaint_requested;
        assert!(runtime.transition_lifecycle(RuntimeLifecyclePhase::Recovering));

        assert!(!runtime.publish_prepared_runtime_layout(publication));
        assert_eq!(runtime.layout, layout_before);
        assert_eq!(runtime.layout_root_authority, root_authority_before);
        assert_eq!(runtime.layout_state_authority, state_authority_before);
        assert_eq!(
            runtime.mounted_layout_source_authority,
            mounted_authority_before
        );
        assert_eq!(
            runtime.layout_state_generation,
            layout_state_generation_before
        );
        assert_eq!(
            runtime.mounted_layout_source_present,
            mounted_source_present_before
        );
        assert_eq!(runtime.completed_layout, completed_before);
        assert_eq!(runtime.last_layout_state_diagnostics, diagnostics_before);
        assert_eq!(runtime.repaint_requested, repaint_before);
        assert!(!runtime.layout_engine.has_explicit_dirty());
    }

    #[test]
    fn environment_and_engine_dirty_vetoes_preserve_active_publication_state() {
        let mut environment_runtime = runtime();
        let publication = environment_runtime
            .prepare_runtime_layout_publication()
            .expect("ordinary runtime should admit prepared publication");
        let layout_before = environment_runtime.layout.clone();
        let completed_before = environment_runtime.completed_layout;
        let environment = WindowEnvironment::new(
            crate::theme::DpiScale::new(2.0),
            Some(crate::runtime::WindowColorScheme::Dark),
            true,
            false,
        );
        assert!(environment_runtime.set_window_environment(environment));
        let external_dirty_after_environment = environment_runtime.external_layout_dirty;

        assert!(!environment_runtime.publish_prepared_runtime_layout(publication));
        assert_eq!(environment_runtime.layout, layout_before);
        assert_eq!(environment_runtime.completed_layout, completed_before);
        assert_eq!(environment_runtime.window_environment, environment);
        assert_eq!(
            environment_runtime.external_layout_dirty,
            external_dirty_after_environment
        );

        let mut dirty_runtime = runtime();
        let publication = dirty_runtime
            .prepare_runtime_layout_publication()
            .expect("ordinary runtime should admit prepared publication");
        let layout_before = dirty_runtime.layout.clone();
        let completed_before = dirty_runtime.completed_layout;
        dirty_runtime.layout_engine.mark_measure_dirty(1);
        assert!(dirty_runtime.layout_engine.has_explicit_dirty());

        assert!(!dirty_runtime.publish_prepared_runtime_layout(publication));
        assert_eq!(dirty_runtime.layout, layout_before);
        assert_eq!(dirty_runtime.completed_layout, completed_before);
        assert!(dirty_runtime.layout_engine.has_explicit_dirty());
    }

    #[test]
    fn mounted_source_and_intervening_refresh_vetoes_preserve_active_state() {
        let mut mounted_runtime = SurfaceRuntime::new(
            SplitCandidateBridge {
                ratio: 0.4,
                generation: 1,
                runtime_owned: false,
            },
            Vector2::new(100.0, 40.0),
        );
        let publication = mounted_runtime
            .prepare_runtime_layout_publication()
            .expect("mounted split runtime should admit prepared publication");
        let layout_before = mounted_runtime.layout.clone();
        let mounted_source_authority_before = mounted_runtime.mounted_layout_source_authority;
        mounted_runtime.note_mounted_layout_source_mutation(true);
        assert_ne!(
            mounted_runtime.mounted_layout_source_authority,
            mounted_source_authority_before
        );

        assert!(!mounted_runtime.publish_prepared_runtime_layout(publication));
        assert_eq!(mounted_runtime.layout, layout_before);
        assert_ne!(
            mounted_runtime.mounted_layout_source_authority,
            mounted_source_authority_before
        );

        let mut refreshed_runtime = runtime();
        let publication = refreshed_runtime
            .prepare_runtime_layout_publication()
            .expect("ordinary runtime should admit prepared publication");
        refreshed_runtime.refresh();
        let layout_after_refresh = refreshed_runtime.layout.clone();
        let completed_after_refresh = refreshed_runtime.completed_layout;
        assert!(!refreshed_runtime.publish_prepared_runtime_layout(publication));
        assert_eq!(refreshed_runtime.layout, layout_after_refresh);
        assert_eq!(refreshed_runtime.completed_layout, completed_after_refresh);
    }

    #[test]
    fn root_traversal_mismatch_vetoes_before_preparation() {
        let mut runtime = runtime();
        runtime.layout_root = LayoutNode::widget(999, Vector2::new(1.0, 1.0));
        assert!(runtime.prepare_runtime_layout_publication().is_none());
    }

    #[test]
    fn candidate_invalidates_for_viewport_debug_state_and_root_changes() {
        let mut runtime = runtime();
        let publication = runtime
            .prepare_runtime_layout_publication()
            .expect("candidate should be admitted");
        runtime.set_viewport(Vector2::new(100.5, 40.0));
        assert!(!runtime.prepared_runtime_layout_publication_is_current(&publication));
        publication.discard();

        let publication = runtime
            .prepare_runtime_layout_publication()
            .expect("candidate should be admitted after viewport change");
        runtime.set_layout_debug_options(LayoutDebugOptions::bounds_only());
        assert!(!runtime.prepared_runtime_layout_publication_is_current(&publication));
        publication.discard();

        let publication = runtime
            .prepare_runtime_layout_publication()
            .expect("candidate should be admitted after debug change");
        runtime
            .layout_state
            .scroll_offsets
            .insert(1, Vector2::new(2.0, 0.0));
        runtime.note_layout_state_mutation();
        assert!(!runtime.prepared_runtime_layout_publication_is_current(&publication));
        publication.discard();

        let publication = runtime
            .prepare_runtime_layout_publication()
            .expect("candidate should be admitted after state change");
        runtime.refresh();
        assert!(!runtime.prepared_runtime_layout_publication_is_current(&publication));
    }
}

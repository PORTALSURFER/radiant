use super::{SurfaceRuntime, SurfaceTraversalIndex};
use crate::gui::layout_core::{
    LayoutAuthorityEvidence, LayoutInputEvidence, LayoutOutput, PreparedLayoutPass,
};
use crate::layout::LayoutNode;
use crate::runtime::RuntimeBridge;

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
        self.prepared_layout
            .is_current_for_engine(&runtime.layout_engine)
            && current_input == self.authority.input
    }

    /// Explicitly abandon the candidate. Dropping the wrapper returns the
    /// prepared workspace and drops staged state exactly once.
    pub(in crate::runtime::controller) fn discard(self) {}
}

#[allow(dead_code)]
impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    /// Prepare one candidate from the runtime's already-produced layout root
    /// and traversal-derived mounted declarations. The root is borrowed from
    /// the runtime owner, so its authority cannot be confused with a detached
    /// or recreated root supplied by another owner.
    pub(in crate::runtime::controller) fn prepare_runtime_layout_candidate(
        &mut self,
        traversal: &SurfaceTraversalIndex<Message>,
    ) -> Option<RuntimeLayoutCandidate> {
        if self.layout_authority_exhausted {
            return None;
        }

        let layout_root = &self.layout_root;
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
            self.layout_engine.prepare_layout_with_state_and_source(
                layout_root,
                self.viewport,
                &self.layout_state,
                self.layout_debug_options,
                Some(&container_state_source),
                authority.input,
            )
        } else {
            self.layout_engine.prepare_layout_with_state_and_source(
                layout_root,
                self.viewport,
                &self.layout_state,
                self.layout_debug_options,
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
        let input = LayoutInputEvidence::new(
            Some(self.layout_root_authority),
            Some(self.layout_state_authority),
            mounted_source_present.then_some(self.mounted_layout_source_authority),
            self.viewport,
            self.layout_debug_options,
        );
        input
            .is_valid_for_prepare(
                self.viewport,
                self.layout_debug_options,
                mounted_source_present,
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
        layout::{ContainerKind, ContainerPolicy, Controlled, LayoutDebugOptions},
        runtime::{
            RuntimeBridge, SurfaceChild, SurfaceNode, UiSurface, WidgetMessageMapper,
            test_arc_surface,
        },
        widgets::{TextWidget, WidgetSizing},
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
    }

    impl SplitCandidateBridge {
        fn surface(&self) -> UiSurface<()> {
            let split = SurfaceNode::container(
                10,
                ContainerPolicy {
                    kind: ContainerKind::SplitPane,
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
            )
            .with_split_pane_runtime_mode(Some(
                crate::gui::layout_core::SplitPaneRuntimeMode::Controlled(Controlled::new(
                    self.ratio,
                    self.generation,
                )),
            ));
            UiSurface::new(split)
        }
    }

    impl RuntimeBridge<()> for SplitCandidateBridge {
        fn project_surface(&mut self) -> Arc<UiSurface<()>> {
            test_arc_surface(self.surface())
        }
    }

    #[test]
    fn candidate_matches_direct_output_and_direct_layout_starts_without_workspace() {
        let mut runtime = runtime();
        runtime.relayout_current_surface();
        let direct = runtime.layout().clone();
        let traversal = runtime.surface.runtime_projection().traversal;
        let candidate = runtime
            .prepare_runtime_layout_candidate(&traversal)
            .expect("ordinary runtime layout should admit");

        assert_eq!(candidate.layout_output(), Some(&direct));
        assert!(runtime.runtime_layout_candidate_is_current(&candidate));
        candidate.discard();
        assert_eq!(runtime.layout(), &direct);
    }

    #[test]
    fn candidate_workspace_contention_falls_back_without_partial_authority() {
        let mut runtime = runtime();
        let traversal = runtime.surface.runtime_projection().traversal;
        let first = runtime
            .prepare_runtime_layout_candidate(&traversal)
            .expect("first candidate should own the workspace");
        assert!(
            runtime
                .prepare_runtime_layout_candidate(&traversal)
                .is_none()
        );
        drop(first);
        assert!(
            runtime
                .prepare_runtime_layout_candidate(&traversal)
                .is_some()
        );
    }

    #[test]
    fn candidate_layout_reads_staged_mounted_state_without_active_commit() {
        let mut runtime = SurfaceRuntime::new(
            SplitCandidateBridge {
                ratio: 0.2,
                generation: 1,
            },
            Vector2::new(100.0, 40.0),
        );
        runtime.relayout_current_surface();
        let active_before = runtime.layout().clone();

        let staged_surface = SplitCandidateBridge {
            ratio: 0.8,
            generation: 2,
        }
        .surface();
        let staged_traversal = staged_surface.runtime_projection().traversal;
        let candidate = runtime
            .prepare_runtime_layout_candidate(&staged_traversal)
            .expect("staged split state should be admitted");

        let mut direct = SurfaceRuntime::new(
            SplitCandidateBridge {
                ratio: 0.8,
                generation: 2,
            },
            Vector2::new(100.0, 40.0),
        );
        direct.relayout_current_surface();
        assert_eq!(candidate.layout_output(), Some(direct.layout()));
        assert_eq!(runtime.layout(), &active_before);
        assert_ne!(candidate.layout_output(), Some(runtime.layout()));
    }

    #[test]
    fn candidate_invalidates_for_viewport_debug_state_and_root_changes() {
        let mut runtime = runtime();
        let traversal = runtime.surface.runtime_projection().traversal;
        let candidate = runtime
            .prepare_runtime_layout_candidate(&traversal)
            .expect("candidate should be admitted");
        runtime.set_viewport(Vector2::new(100.5, 40.0));
        assert!(!runtime.runtime_layout_candidate_is_current(&candidate));
        drop(candidate);

        let traversal = runtime.surface.runtime_projection().traversal;
        let candidate = runtime
            .prepare_runtime_layout_candidate(&traversal)
            .expect("candidate should be admitted after viewport change");
        runtime.set_layout_debug_options(LayoutDebugOptions::bounds_only());
        assert!(!runtime.runtime_layout_candidate_is_current(&candidate));
        drop(candidate);

        let traversal = runtime.surface.runtime_projection().traversal;
        let candidate = runtime
            .prepare_runtime_layout_candidate(&traversal)
            .expect("candidate should be admitted after debug change");
        runtime
            .layout_state
            .scroll_offsets
            .insert(1, Vector2::new(2.0, 0.0));
        runtime.note_layout_state_mutation();
        assert!(!runtime.runtime_layout_candidate_is_current(&candidate));
        drop(candidate);

        let traversal = runtime.surface.runtime_projection().traversal;
        let candidate = runtime
            .prepare_runtime_layout_candidate(&traversal)
            .expect("candidate should be admitted after state change");
        runtime.refresh();
        assert!(!runtime.runtime_layout_candidate_is_current(&candidate));
    }
}

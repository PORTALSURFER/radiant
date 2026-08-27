use super::super::{SurfaceRuntime, SurfaceTraversalIndex};
use crate::gui::types::{Point, Rect};
use crate::{gui::types::Vector2, layout::LayoutDiagnosticCode, runtime::RuntimeBridge};

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(in crate::runtime::controller) fn relayout(&mut self) {
        let mut traversal = self.take_reusable_traversal_index(true);
        let layout_root = self.surface.runtime_projection_reusing_with_scratch(
            &mut traversal,
            &mut self.scratch.projection_scroll_stack,
            &mut self.scratch.projection_child_path,
            &mut self.scratch.projection_source,
        );
        self.replace_layout_root(layout_root);
        self.relayout_with_traversal(traversal);
        self.install_declarative_owner_projection();
    }

    pub(in crate::runtime::controller) fn relayout_current_surface(&mut self) {
        let traversal = self.take_reusable_traversal_index(true);
        self.relayout_with_traversal(traversal);
    }

    pub(in crate::runtime::controller) fn queue_current_surface_relayout(&mut self) {
        self.pending_current_surface_relayout = true;
        self.repaint_requested = true;
    }

    pub(in crate::runtime::controller) fn service_pending_current_surface_relayout(&mut self) {
        if self.servicing_current_surface_relayout {
            return;
        }
        self.servicing_current_surface_relayout = true;
        for _ in 0..2 {
            if !self.pending_current_surface_relayout {
                break;
            }
            self.pending_current_surface_relayout = false;
            self.relayout_current_surface();
            self.repaint_requested = true;
        }
        self.servicing_current_surface_relayout = false;
    }

    pub(in crate::runtime::controller) fn install_traversal_with_candidate(
        &mut self,
        traversal: SurfaceTraversalIndex<Message>,
        candidate: super::super::layout_state::RuntimeLayoutContainerStateCandidate,
    ) {
        let candidate_source_present = candidate.source_present();
        let candidate_mutates_values_or_identity = candidate.mutates_values_or_identity();
        let accepted_source_present = self.mounted_layout_source_present;
        self.install_traversal_index(traversal);
        self.refresh_visible_traversal_orders();
        self.commit_layout_container_state_candidate(candidate);
        if !candidate_mutates_values_or_identity
            && candidate_source_present != accepted_source_present
        {
            self.note_mounted_layout_source_mutation(true);
        }
        self.mounted_layout_source_present = candidate_source_present;
        self.traversal
            .containers
            .bind_committed_mounted_state_ids(&self.interaction.layout_state);
        self.traversal
            .containers
            .rebuild_split_pane_separator_projections(&self.interaction.layout_state);
        self.traversal
            .rebuild_mixed_focus_order(self.lifecycle_phase(), &self.interaction.layout_state);
        // Separator focus is revalidated only after the committed mounted
        // state and its projections are both installed. Prepared candidates
        // therefore cannot mutate the active owner.
        self.revalidate_focus_owner();
    }

    pub(in crate::runtime::controller) fn relayout_with_traversal(
        &mut self,
        traversal: SurfaceTraversalIndex<Message>,
    ) {
        let candidate = self.prepare_layout_container_state_candidate(&traversal);
        let container_state_source = self.interaction.layout_state.read_source(&candidate);
        self.layout_engine.layout_with_state_and_source_into(
            &self.layout_root,
            self.viewport,
            &self.layout_state,
            self.layout_debug_options,
            Some(&container_state_source),
            &mut self.layout,
        );
        self.install_traversal_with_candidate(traversal, candidate);
        self.sync_scroll_offsets();
        self.record_completed_layout();
    }

    pub(in crate::runtime::controller) fn record_completed_layout(&mut self) {
        self.external_layout_dirty = false;
        self.completed_layout = Some(super::super::CompletedLayoutContext {
            viewport: effective_layout_viewport(self.viewport),
            window_environment: self.window_environment,
            layout_state_generation: self.layout_state_generation,
            layout_debug_options: self.layout_debug_options,
        });
    }

    pub(in crate::runtime::controller) fn sync_scroll_offsets(&mut self) {
        self.scratch.scroll_clamp_updates.clear();
        self.scratch.scroll_clamp_updates.extend(
            self.layout
                .diagnostics
                .iter()
                .filter(|diagnostic| {
                    diagnostic.code == LayoutDiagnosticCode::InvalidScrollOffsetClamped
                })
                .filter_map(|diagnostic| {
                    let child_rect = self.layout.rects.get(
                        self.traversal
                            .containers
                            .scroll_content_by_container
                            .get(&diagnostic.node_id)?,
                    )?;
                    let viewport_rect = self
                        .layout
                        .viewport_bounds
                        .get(&diagnostic.node_id)
                        .or_else(|| self.layout.rects.get(&diagnostic.node_id))?;
                    let current_offset = self.layout_state.scroll_offset(diagnostic.node_id);
                    Some((
                        diagnostic.node_id,
                        clamped_scroll_offset(current_offset, *child_rect, *viewport_rect),
                    ))
                }),
        );
        let scroll_clamp_updates = std::mem::take(&mut self.scratch.scroll_clamp_updates);
        for (node_id, offset) in scroll_clamp_updates {
            if self.layout_state.scroll_offset(node_id) != offset {
                self.layout_state.scroll_offsets.insert(node_id, offset);
                self.note_layout_state_mutation();
            }
        }
    }
}

fn effective_layout_viewport(viewport: Rect) -> Rect {
    Rect::from_min_size(
        Point::new(viewport.min.x.floor(), viewport.min.y.floor()),
        Vector2::new(
            viewport.width().round().max(0.0),
            viewport.height().round().max(0.0),
        ),
    )
}

fn clamped_scroll_offset(current: Vector2, child_rect: Rect, viewport_rect: Rect) -> Vector2 {
    Vector2::new(
        current
            .x
            .min((child_rect.width() - viewport_rect.width()).max(0.0)),
        current
            .y
            .min((child_rect.height() - viewport_rect.height()).max(0.0)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        gui::types::Point,
        layout::{
            Constraints, ContainerKind, ContainerPolicy, OverflowPolicy, SizeModeCross,
            SizeModeMain, SlotParams,
        },
        runtime::{RuntimeBridge, SurfaceChild, SurfaceNode, UiSurface, WidgetMessageMapper},
        widgets::{TextWidget, WidgetSizing},
    };
    use std::sync::Arc;

    #[test]
    fn clamped_scroll_offset_reuses_current_offset_once_for_both_axes() {
        let child = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 260.0));
        let viewport = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(100.0, 200.0));

        assert_eq!(
            clamped_scroll_offset(Vector2::new(80.0, 90.0), child, viewport),
            Vector2::new(20.0, 60.0)
        );
    }

    #[test]
    fn clamped_scroll_offset_keeps_zero_max_when_content_fits_viewport() {
        let child = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(80.0, 160.0));
        let viewport = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(100.0, 200.0));

        assert_eq!(
            clamped_scroll_offset(Vector2::new(8.0, 12.0), child, viewport),
            Vector2::new(0.0, 0.0)
        );
    }

    #[test]
    fn scroll_offset_sync_uses_padded_viewport_bounds() {
        let mut runtime = SurfaceRuntime::new(PaddedScrollBridge, Vector2::new(100.0, 80.0));
        let point = Point::new(8.0, 8.0);

        assert!(runtime.scroll_at(point, Vector2::new(0.0, 10_000.0)));
        let before = runtime
            .layout()
            .rects
            .get(&PaddedScrollBridge::CONTENT_ID)
            .copied()
            .expect("content rect after scroll");

        runtime.refresh();
        let after = runtime
            .layout()
            .rects
            .get(&PaddedScrollBridge::CONTENT_ID)
            .copied()
            .expect("content rect after refresh");

        assert_eq!(
            after, before,
            "refresh should not rewrite a padded scroll viewport offset using the outer container"
        );
    }

    struct PaddedScrollBridge;

    impl PaddedScrollBridge {
        const CONTENT_ID: u64 = 2;
    }

    impl RuntimeBridge<()> for PaddedScrollBridge {
        fn project_surface(&mut self) -> Arc<UiSurface<()>> {
            crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::container(
                1,
                ContainerPolicy {
                    kind: ContainerKind::ScrollView,
                    overflow: OverflowPolicy::Scroll,
                    padding: crate::layout::Insets::all(4.0),
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
                            Self::CONTENT_ID,
                            "Tall",
                            WidgetSizing::fixed(Vector2::new(80.0, 400.0)),
                        ),
                        WidgetMessageMapper::none(),
                    ),
                )],
            )))
        }

        fn reduce_message(&mut self, _message: ()) {}
    }
}

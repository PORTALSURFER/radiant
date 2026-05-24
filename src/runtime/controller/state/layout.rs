use super::super::*;

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(in crate::runtime::controller) fn relayout(&mut self) {
        let mut traversal = self.take_reusable_traversal_index(true);
        self.layout_root = self.surface.runtime_projection_reusing_with_scratch(
            &mut traversal,
            &mut self.scratch.projection_scroll_stack,
            &mut self.scratch.projection_child_path,
        );
        self.relayout_with_traversal(traversal);
    }

    pub(in crate::runtime::controller) fn relayout_current_surface(&mut self) {
        self.layout = self.layout_engine.layout_with_state(
            &self.layout_root,
            self.viewport,
            &self.layout_state,
            self.layout_debug_options,
        );
        self.refresh_visible_traversal_orders();
        self.sync_scroll_offsets();
    }

    pub(in crate::runtime::controller) fn relayout_with_traversal(
        &mut self,
        traversal: SurfaceTraversalIndex,
    ) {
        self.layout = self.layout_engine.layout_with_state(
            &self.layout_root,
            self.viewport,
            &self.layout_state,
            self.layout_debug_options,
        );
        self.install_traversal_index(traversal);
        self.sync_scroll_offsets();
    }

    fn sync_scroll_offsets(&mut self) {
        self.scratch.scroll_clamp_updates.clear();
        self.scratch.scroll_clamp_updates.extend(
            self.layout
                .diagnostics
                .iter()
                .filter(|diagnostic| {
                    diagnostic.code
                        == crate::layout::LayoutDiagnosticCode::InvalidScrollOffsetClamped
                })
                .filter_map(|diagnostic| {
                    let child_rect = self.layout.rects.get(
                        self.traversal
                            .containers
                            .scroll_content_by_container
                            .get(&diagnostic.node_id)?,
                    )?;
                    let viewport_rect = self.layout.rects.get(&diagnostic.node_id)?;
                    Some((
                        diagnostic.node_id,
                        Vector2::new(
                            self.layout_state
                                .scroll_offset(diagnostic.node_id)
                                .x
                                .min((child_rect.width() - viewport_rect.width()).max(0.0)),
                            self.layout_state
                                .scroll_offset(diagnostic.node_id)
                                .y
                                .min((child_rect.height() - viewport_rect.height()).max(0.0)),
                        ),
                    ))
                }),
        );
        for (node_id, offset) in self.scratch.scroll_clamp_updates.drain(..) {
            self.layout_state.scroll_offsets.insert(node_id, offset);
        }
    }
}

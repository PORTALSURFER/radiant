use super::super::{SurfaceRuntime, SurfaceTraversalIndex};
use crate::gui::types::Rect;
use crate::{gui::types::Vector2, layout::LayoutDiagnosticCode, runtime::RuntimeBridge};

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
        self.layout_engine.layout_with_state_into(
            &self.layout_root,
            self.viewport,
            &self.layout_state,
            self.layout_debug_options,
            &mut self.layout,
        );
        self.refresh_visible_traversal_orders();
        self.sync_scroll_offsets();
    }

    pub(in crate::runtime::controller) fn relayout_with_traversal(
        &mut self,
        traversal: SurfaceTraversalIndex,
    ) {
        self.layout_engine.layout_with_state_into(
            &self.layout_root,
            self.viewport,
            &self.layout_state,
            self.layout_debug_options,
            &mut self.layout,
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
                    diagnostic.code == LayoutDiagnosticCode::InvalidScrollOffsetClamped
                })
                .filter_map(|diagnostic| {
                    let child_rect = self.layout.rects.get(
                        self.traversal
                            .containers
                            .scroll_content_by_container
                            .get(&diagnostic.node_id)?,
                    )?;
                    let viewport_rect = self.layout.rects.get(&diagnostic.node_id)?;
                    let current_offset = self.layout_state.scroll_offset(diagnostic.node_id);
                    Some((
                        diagnostic.node_id,
                        clamped_scroll_offset(current_offset, *child_rect, *viewport_rect),
                    ))
                }),
        );
        for (node_id, offset) in self.scratch.scroll_clamp_updates.drain(..) {
            self.layout_state.scroll_offsets.insert(node_id, offset);
        }
    }
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
    use crate::gui::types::Point;

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
}

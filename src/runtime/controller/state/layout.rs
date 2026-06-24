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
            Arc::new(UiSurface::new(SurfaceNode::container(
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

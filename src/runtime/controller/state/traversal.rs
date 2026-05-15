use super::super::*;

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(in crate::runtime::controller) fn install_traversal_index(
        &mut self,
        traversal: SurfaceTraversalIndex,
    ) {
        self.widget_hit_order = traversal.widget_paint_order;
        self.widget_paths = traversal.widget_paths;
        self.focusable_widgets
            .set_order(traversal.focusable_widget_order);
        self.pointer_widgets.set_order(traversal.pointer_hit_order);
        self.container_hover_suppression = traversal.container_hover_suppression;
        self.keyboard_focus_widgets
            .set_order(traversal.keyboard_focus_order);
        self.wheel_widgets.set_order(traversal.wheel_hit_order);
        self.stateful_widget_order = traversal.stateful_widget_order;
        self.styled_containers
            .set_order(traversal.styled_container_order);
        self.scroll_containers
            .set_order(traversal.scroll_container_order);
        self.widget_clip_ancestors = traversal.widget_clip_ancestors;
        self.container_clip_ancestors = traversal.container_clip_ancestors;
        self.scroll_content_by_container = traversal.scroll_content_by_container;
        self.refresh_visible_traversal_orders();
    }

    pub(in crate::runtime::controller) fn refresh_visible_traversal_orders(&mut self) {
        self.pointer_widgets.refresh_visible(&self.layout);
        self.wheel_widgets.refresh_visible(&self.layout);
        self.styled_containers.refresh_visible(&self.layout);
        self.scroll_containers.refresh_visible(&self.layout);
    }

    pub(in crate::runtime::controller) fn take_reusable_traversal_index(
        &mut self,
        reuse_widget_paths: bool,
    ) -> SurfaceTraversalIndex {
        SurfaceTraversalIndex {
            widget_paint_order: std::mem::take(&mut self.widget_hit_order),
            focusable_widget_order: self.focusable_widgets.take_order(),
            keyboard_focus_order: self.keyboard_focus_widgets.take_order(),
            pointer_hit_order: self.pointer_widgets.take_order(),
            wheel_hit_order: self.wheel_widgets.take_order(),
            stateful_widget_order: std::mem::take(&mut self.stateful_widget_order),
            widget_paths: if reuse_widget_paths {
                std::mem::take(&mut self.widget_paths)
            } else {
                Default::default()
            },
            container_hover_suppression: std::mem::take(&mut self.container_hover_suppression),
            styled_container_order: self.styled_containers.take_order(),
            scroll_container_order: self.scroll_containers.take_order(),
            widget_clip_ancestors: std::mem::take(&mut self.widget_clip_ancestors),
            container_clip_ancestors: std::mem::take(&mut self.container_clip_ancestors),
            scroll_content_by_container: std::mem::take(&mut self.scroll_content_by_container),
        }
    }
}

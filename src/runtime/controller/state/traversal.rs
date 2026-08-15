use super::super::SurfaceRuntime;
use crate::runtime::{RuntimeBridge, SurfaceTraversalIndex};

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(in crate::runtime::controller) fn install_traversal_index(
        &mut self,
        traversal: SurfaceTraversalIndex<Message>,
    ) {
        self.traversal.widgets.hit_order = traversal.widget_paint_order;
        self.traversal.widgets.paths.current = traversal.widget_paths;
        self.traversal
            .widgets
            .focusable
            .set_order(traversal.focusable_widget_order);
        self.traversal
            .widgets
            .pointer
            .set_order(traversal.pointer_hit_order);
        self.traversal
            .widgets
            .native_file_drop
            .set_order(traversal.native_file_drop_hit_order);
        self.traversal.widgets.paths.container_hover_suppression =
            traversal.container_hover_suppression;
        self.traversal
            .widgets
            .keyboard_focus
            .set_order(traversal.keyboard_focus_order);
        self.traversal
            .widgets
            .wheel
            .set_order(traversal.wheel_hit_order);
        self.traversal
            .widgets
            .wheel_targets
            .set_order(traversal.wheel_target_order);
        self.traversal.widgets.stateful_order = traversal.stateful_widget_order;
        self.traversal
            .containers
            .styled
            .set_order(traversal.styled_container_order);
        self.traversal
            .containers
            .scroll
            .set_order(traversal.scroll_container_order);
        self.traversal.widgets.paths.clip_ancestors = traversal.widget_clip_ancestors;
        self.traversal.containers.clip_ancestors = traversal.container_clip_ancestors;
        self.traversal.containers.scroll_content_by_container =
            traversal.scroll_content_by_container;
        self.traversal.containers.layout_interactions = traversal.layout_interactions;
        self.traversal.containers.split_pane_runtime = traversal.split_pane_runtime;
        self.traversal.containers.virtual_layout_registrations =
            traversal.virtual_layout_registrations;
    }

    pub(in crate::runtime::controller) fn refresh_visible_traversal_orders(&mut self) {
        self.traversal.widgets.pointer.refresh_visible(&self.layout);
        self.traversal
            .widgets
            .native_file_drop
            .refresh_visible(&self.layout);
        self.traversal.widgets.wheel.refresh_visible(&self.layout);
        self.traversal
            .widgets
            .wheel_targets
            .refresh_visible(&self.layout);
        self.traversal
            .containers
            .styled
            .refresh_visible(&self.layout);
        self.traversal
            .containers
            .scroll
            .refresh_visible(&self.layout);
        self.traversal
            .containers
            .project_layout_targets(&self.layout);
        self.reconcile_layout_pointer_capture();
    }

    pub(in crate::runtime::controller) fn take_reusable_traversal_index(
        &mut self,
        reuse_widget_paths: bool,
    ) -> SurfaceTraversalIndex<Message> {
        SurfaceTraversalIndex {
            widget_paint_order: std::mem::take(&mut self.traversal.widgets.hit_order),
            focusable_widget_order: self.traversal.widgets.focusable.take_order(),
            keyboard_focus_order: self.traversal.widgets.keyboard_focus.take_order(),
            pointer_hit_order: self.traversal.widgets.pointer.take_order(),
            native_file_drop_hit_order: self.traversal.widgets.native_file_drop.take_order(),
            wheel_hit_order: self.traversal.widgets.wheel.take_order(),
            wheel_target_order: self.traversal.widgets.wheel_targets.take_order(),
            stateful_widget_order: std::mem::take(&mut self.traversal.widgets.stateful_order),
            widget_paths: if reuse_widget_paths {
                std::mem::take(&mut self.traversal.widgets.paths.current)
            } else {
                Default::default()
            },
            container_hover_suppression: std::mem::take(
                &mut self.traversal.widgets.paths.container_hover_suppression,
            ),
            styled_container_order: self.traversal.containers.styled.take_order(),
            scroll_container_order: self.traversal.containers.scroll.take_order(),
            widget_clip_ancestors: std::mem::take(&mut self.traversal.widgets.paths.clip_ancestors),
            container_clip_ancestors: std::mem::take(&mut self.traversal.containers.clip_ancestors),
            scroll_content_by_container: std::mem::take(
                &mut self.traversal.containers.scroll_content_by_container,
            ),
            layout_interactions: std::mem::take(&mut self.traversal.containers.layout_interactions),
            split_pane_runtime: std::mem::take(&mut self.traversal.containers.split_pane_runtime),
            virtual_layout_registrations: std::mem::take(
                &mut self.traversal.containers.virtual_layout_registrations,
            ),
        }
    }
}

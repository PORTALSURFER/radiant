//! Generic declarative view-tree types for message-driven Radiant hosts.

mod builders;
mod focus;
mod frame;
mod input;
mod layout;
mod node;
mod paint;
mod traversal;
mod widget;

pub use frame::SurfaceFrame;
pub(in crate::runtime) use input::WidgetDispatchResult;
pub(in crate::runtime) use layout::SurfaceRuntimeProjection;
pub use node::{SurfaceChild, SurfaceContainer, SurfaceNode, SurfaceOverlay};
pub(in crate::runtime) use traversal::{
    ClipAncestors, SurfaceTraversalIndex, SurfaceTraversalStats, WidgetPath,
};
pub use widget::{MessageMapper, SurfaceWidget, WidgetMessageMapper};

use super::paint::SurfacePaintPlan;
use crate::{
    layout::{LayoutNode, LayoutOutput, NodeId},
    theme::ThemeTokens,
    widgets::{WidgetId, WidgetInput, WidgetOutput},
};
use std::collections::HashMap;

/// Top-level immutable UI surface projected by a generic Radiant host.
pub struct UiSurface<Message> {
    root: SurfaceNode<Message>,
}

/// Public declarative view snapshot alias for host applications.
///
/// `View<Message>` is the framework vocabulary for the top-level immutable UI
/// projection. It is an alias for [`UiSurface`] so existing code keeps the same
/// storage, cloning, layout, input, and paint behavior.
pub type View<Message> = UiSurface<Message>;

/// Public declarative element tree alias for host applications.
///
/// `Element<Message>` is the framework vocabulary for one node in a projected
/// view tree. It is an alias for [`SurfaceNode`] to keep identity and layout
/// behavior exactly shared with the existing runtime surface.
pub type Element<Message> = SurfaceNode<Message>;

impl<Message> Clone for UiSurface<Message> {
    fn clone(&self) -> Self {
        Self {
            root: self.root.clone(),
        }
    }
}

impl<Message> UiSurface<Message> {
    /// Build a top-level UI surface from one declarative root node.
    pub fn new(root: SurfaceNode<Message>) -> Self {
        Self { root }
    }

    /// Return the root declarative node.
    pub fn root(&self) -> &SurfaceNode<Message> {
        &self.root
    }

    /// Consume the surface and return its root declarative node.
    pub fn into_root(self) -> SurfaceNode<Message> {
        self.root
    }

    /// Project the surface into the public layout tree consumed by layout engines.
    pub fn layout_node(&self) -> LayoutNode {
        self.root.layout_node()
    }

    /// Project the surface and its layout output into backend-neutral paint data.
    ///
    /// Primitives are emitted in declarative tree order so backends and tests can
    /// compare output deterministically without depending on the native shell.
    pub fn paint_plan(&self, layout: &LayoutOutput, theme: &ThemeTokens) -> SurfacePaintPlan {
        let mut plan = SurfacePaintPlan::empty_with_capacity(
            theme,
            estimated_paint_primitive_capacity(layout),
        );
        self.paint_plan_into(layout, theme, &mut plan);
        plan
    }

    /// Project backend-neutral paint data into an existing plan buffer.
    ///
    /// This is the allocation-lean counterpart to [`Self::paint_plan`] for
    /// hosts and renderers that rebuild paint data every frame.
    pub fn paint_plan_into(
        &self,
        layout: &LayoutOutput,
        theme: &ThemeTokens,
        plan: &mut SurfacePaintPlan,
    ) {
        self.paint_plan_with_hover_into(layout, theme, None, None, plan);
    }

    pub(super) fn paint_plan_with_hover_into(
        &self,
        layout: &LayoutOutput,
        theme: &ThemeTokens,
        hovered_container: Option<NodeId>,
        active_scroll_affordance: Option<NodeId>,
        plan: &mut SurfacePaintPlan,
    ) {
        plan.clear_for_theme_with_capacity(theme, estimated_paint_primitive_capacity(layout));
        self.root.append_paint(
            layout,
            theme,
            plan,
            hovered_container,
            active_scroll_affordance,
        );
    }

    /// Map one widget output back into a host-defined message.
    pub fn dispatch_widget_output(
        &self,
        widget_id: WidgetId,
        output: WidgetOutput,
    ) -> Option<Message> {
        self.root.dispatch_output(widget_id, &output)
    }

    /// Route one backend-neutral interaction into a projected widget.
    pub fn dispatch_widget_input(
        &mut self,
        widget_id: WidgetId,
        bounds: crate::gui::types::Rect,
        input: WidgetInput,
    ) -> Option<WidgetOutput> {
        self.root.handle_input(widget_id, bounds, input)
    }

    pub(in crate::runtime) fn dispatch_widget_input_at_path(
        &mut self,
        widget_id: WidgetId,
        child_path: &WidgetPath,
        bounds: crate::gui::types::Rect,
        input: WidgetInput,
    ) -> Option<WidgetOutput> {
        self.root
            .handle_input_at_path(widget_id, child_path.as_slice(), bounds, input)
    }

    pub(in crate::runtime) fn dispatch_widget_input_message(
        &mut self,
        widget_id: WidgetId,
        bounds: crate::gui::types::Rect,
        input: WidgetInput,
    ) -> Option<WidgetDispatchResult<Message>> {
        self.find_widget_mut(widget_id)
            .map(|widget| widget.dispatch_input(widget_id, bounds, input))
    }

    pub(in crate::runtime) fn dispatch_widget_input_message_at_path(
        &mut self,
        widget_id: WidgetId,
        child_path: &WidgetPath,
        bounds: crate::gui::types::Rect,
        input: WidgetInput,
    ) -> Option<WidgetDispatchResult<Message>> {
        self.root
            .dispatch_input_at_path(widget_id, child_path.as_slice(), bounds, input)
    }

    /// Find one projected widget by stable id.
    pub fn find_widget(&self, widget_id: WidgetId) -> Option<&SurfaceWidget<Message>> {
        self.root.find_widget(widget_id)
    }

    pub(in crate::runtime) fn find_widget_at_path(
        &self,
        widget_id: WidgetId,
        child_path: &WidgetPath,
    ) -> Option<&SurfaceWidget<Message>> {
        self.root
            .find_widget_at_path(child_path.as_slice())
            .filter(|widget| widget.id() == widget_id)
    }

    /// Find one projected widget by stable id for in-place runtime interaction.
    pub fn find_widget_mut(&mut self, widget_id: WidgetId) -> Option<&mut SurfaceWidget<Message>> {
        self.root.find_widget_mut(widget_id)
    }

    pub(in crate::runtime) fn find_widget_mut_at_path(
        &mut self,
        widget_id: WidgetId,
        child_path: &WidgetPath,
    ) -> Option<&mut SurfaceWidget<Message>> {
        self.root
            .find_widget_mut_at_path(child_path.as_slice())
            .filter(|widget| widget.id() == widget_id)
    }

    /// Return whether a projected widget can own runtime focus.
    pub fn is_focusable_widget(&self, widget_id: WidgetId) -> bool {
        self.find_widget(widget_id)
            .is_some_and(SurfaceWidget::is_focusable)
    }

    pub(in crate::runtime) fn synchronize_widget_state_from_paths(
        &mut self,
        previous: &Self,
        current_paths: &HashMap<WidgetId, WidgetPath>,
        previous_paths: &HashMap<WidgetId, WidgetPath>,
    ) {
        self.root.synchronize_widget_state_from_paths(
            current_paths,
            &previous.root,
            previous_paths,
        );
    }
}

pub(in crate::runtime) fn estimated_paint_primitive_capacity(layout: &LayoutOutput) -> usize {
    layout.rects.len().saturating_mul(3)
}

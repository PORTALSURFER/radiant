use super::StatefulAppWithView;
use crate::{application::IntoView, gui::types::Vector2, widgets::RetainedSurfaceDescriptor};

impl<State, Message, Project, View> StatefulAppWithView<State, Message, Project, View>
where
    Project: FnMut(&mut State) -> View + 'static,
    View: IntoView<Message> + 'static,
    Message: Send + 'static,
    State: 'static,
{
    /// Register a retained-surface painter by descriptor key.
    pub fn retained_painter(
        mut self,
        key: u64,
        paint: impl FnMut(
            &mut State,
            RetainedSurfaceDescriptor,
            crate::gui::types::Rect,
            Vector2,
        ) -> Option<crate::gui::paint::PaintFrame>
        + 'static,
    ) -> Self {
        self.lifecycle
            .retained_painters
            .insert(key, Box::new(paint));
        self
    }

    /// Register a lightweight frame-time overlay painter.
    ///
    /// The painter receives the latest cached surface paint plan and appends
    /// transient primitives that native backends can draw over the cached scene
    /// without refreshing layout. Use this for small, continuously animated
    /// overlays such as playheads, drag previews, tooltips, or cursor markers.
    /// Combine it with [`Self::transient_overlay_animation`] to receive
    /// frame-cadenced paint-only redraws without an [`Self::on_frame`] message.
    pub fn transient_overlay(
        mut self,
        paint: impl for<'a> FnMut(
            &mut State,
            crate::runtime::TransientOverlayContext<'a>,
            &mut Vec<crate::runtime::PaintPrimitive>,
        ) + 'static,
    ) -> Self {
        self.lifecycle.transient_overlay = Some(Box::new(paint));
        self
    }

    /// Declare whether the transient overlay currently needs timed frames.
    ///
    /// This is the paint-only animation path for overlays that can derive
    /// motion from [`crate::runtime::TransientOverlayContext::animation_time`].
    /// It keeps the native runtime on the cached-scene path instead of routing
    /// frame messages or refreshing the declarative surface while the app is
    /// otherwise unchanged.
    pub fn transient_overlay_animation(
        mut self,
        activity: impl FnMut(&mut State) -> bool + 'static,
    ) -> Self {
        self.lifecycle.transient_overlay_activity = Some(Box::new(activity));
        self
    }

    /// Register a transient overlay and its paint-only animation activity.
    ///
    /// This is the most direct API for realtime overlays such as playheads,
    /// drag previews, tooltip affordances, cursor markers, and small animated
    /// GPU-surface overlays that should redraw continuously without rebuilding
    /// layout or queuing application frame messages.
    pub fn animated_transient_overlay(
        mut self,
        activity: impl FnMut(&mut State) -> bool + 'static,
        paint: impl for<'a> FnMut(
            &mut State,
            crate::runtime::TransientOverlayContext<'a>,
            &mut Vec<crate::runtime::PaintPrimitive>,
        ) + 'static,
    ) -> Self {
        self.lifecycle.transient_overlay_activity = Some(Box::new(activity));
        self.lifecycle.transient_overlay = Some(Box::new(paint));
        self
    }
}

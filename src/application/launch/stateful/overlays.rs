use super::StatefulAppWithView;
use crate::{
    application::IntoView, gui::types::Vector2, runtime::RuntimeAnimationActivity,
    widgets::RetainedSurfaceDescriptor,
};

impl<State, Message, Project, View> StatefulAppWithView<State, Message, Project, View>
where
    Project: FnMut(&State) -> View + 'static,
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

    /// Advanced lifecycle hook for a lightweight frame-time overlay painter.
    ///
    /// Prefer [`crate::application::Scene::overlay`] or [`Self::presentation`]
    /// with [`crate::application::TransientOverlay`] for normal root-scoped
    /// presentation. Use this lower-level hook when a host needs direct runtime
    /// overlay wiring.
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

    /// Advanced lifecycle hook for transient-overlay timed frames.
    ///
    /// Prefer [`crate::application::Scene::overlay`] or [`Self::presentation`]
    /// with [`crate::application::TransientOverlay`] for ordinary paint-only
    /// presentation. This lower-level hook is the paint-only animation path for
    /// overlays that can derive
    /// motion from [`crate::runtime::TransientOverlayContext::animation_time`].
    /// It keeps the native runtime on the cached-scene path instead of routing
    /// frame messages or refreshing the declarative surface while the app is
    /// otherwise unchanged.
    pub fn transient_overlay_animation(
        mut self,
        activity: impl FnMut(&mut State) -> bool + 'static,
    ) -> Self {
        self.lifecycle.transient_overlay_activity =
            Some(boolean_overlay_activity(Box::new(activity), None));
        self
    }

    /// Advanced lifecycle hook for timed overlay frames capped to a requested frame rate.
    ///
    /// Prefer [`crate::application::Scene::overlay`] or [`Self::presentation`]
    /// with [`crate::application::TransientOverlay::fps`] for ordinary
    /// root-scoped overlays. This lower-level hook lets low-frequency overlays
    /// such as playheads or cursor affordances redraw at their useful cadence
    /// without consuming every native animation frame. The native runtime still
    /// clamps the request to the window-level frame-rate policy.
    pub fn transient_overlay_animation_at(
        mut self,
        target_fps: u32,
        activity: impl FnMut(&mut State) -> bool + 'static,
    ) -> Self {
        self.lifecycle.transient_overlay_activity = Some(boolean_overlay_activity(
            Box::new(activity),
            Some(target_fps),
        ));
        self
    }

    /// Advanced lifecycle hook for a transient overlay and its paint-only activity.
    ///
    /// Prefer [`crate::application::Scene::overlay`] or [`Self::presentation`]
    /// with [`crate::application::TransientOverlay`] for normal root-scoped
    /// overlays. This is the most direct API for realtime overlays such as
    /// playheads, drag previews, tooltip affordances, cursor markers, and small
    /// animated GPU-surface overlays that should redraw continuously without
    /// rebuilding layout or queuing application frame messages.
    pub fn animated_transient_overlay(
        mut self,
        activity: impl FnMut(&mut State) -> bool + 'static,
        paint: impl for<'a> FnMut(
            &mut State,
            crate::runtime::TransientOverlayContext<'a>,
            &mut Vec<crate::runtime::PaintPrimitive>,
        ) + 'static,
    ) -> Self {
        self.lifecycle.transient_overlay_activity =
            Some(boolean_overlay_activity(Box::new(activity), None));
        self.lifecycle.transient_overlay = Some(Box::new(paint));
        self
    }

    /// Advanced lifecycle hook for a transient overlay with capped paint-only cadence.
    ///
    /// Prefer [`crate::application::Scene::overlay`] or [`Self::presentation`]
    /// with [`crate::application::TransientOverlay::fps`] for normal
    /// root-scoped overlays. Use this lower-level hook only when direct
    /// launch-lifecycle wiring is required.
    pub fn animated_transient_overlay_at(
        mut self,
        target_fps: u32,
        activity: impl FnMut(&mut State) -> bool + 'static,
        paint: impl for<'a> FnMut(
            &mut State,
            crate::runtime::TransientOverlayContext<'a>,
            &mut Vec<crate::runtime::PaintPrimitive>,
        ) + 'static,
    ) -> Self {
        self.lifecycle.transient_overlay_activity = Some(boolean_overlay_activity(
            Box::new(activity),
            Some(target_fps),
        ));
        self.lifecycle.transient_overlay = Some(Box::new(paint));
        self
    }
}

fn boolean_overlay_activity<State: 'static>(
    mut activity: Box<dyn FnMut(&mut State) -> bool>,
    target_fps: Option<u32>,
) -> Box<dyn FnMut(&mut State) -> RuntimeAnimationActivity> {
    Box::new(move |state| {
        if !activity(state) {
            return RuntimeAnimationActivity::idle();
        }
        target_fps.map_or_else(
            RuntimeAnimationActivity::paint_only,
            RuntimeAnimationActivity::paint_only_at,
        )
    })
}

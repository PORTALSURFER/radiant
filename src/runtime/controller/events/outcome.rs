use crate::widgets::WidgetId;

/// Routing summary for one pointer-move event.
///
/// Backend adapters that distinguish full scene rebuilds from paint-only
/// overlays can use this instead of [`super::SurfaceRuntime::dispatch_event`] for
/// pointer motion. The outcome drains the runtime repaint/exit flags observed
/// during the route so callers can make one redraw decision without peeking at
/// controller internals.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PointerMoveOutcome {
    /// Widget targeted by the pointer move, when one was hit.
    pub target: Option<WidgetId>,
    /// Whether hover ownership changed during this route.
    pub hover_changed: bool,
    /// Whether a widget currently owns pointer capture.
    pub pointer_captured: bool,
    /// Whether the base surface or Vello scene should be rebuilt.
    pub repaint_requested: bool,
    /// Whether a cached-scene overlay redraw is enough.
    pub paint_only_requested: bool,
    /// Whether routing requested runtime shutdown.
    pub exit_requested: bool,
}

/// Routing summary for a synthetic pointer click dispatched through the normal
/// backend-neutral event path.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PointerClickOutcome {
    /// Widget targeted by the press event, when one was hit.
    pub press_target: Option<WidgetId>,
    /// Widget targeted by the matching release event, when one was hit.
    pub release_target: Option<WidgetId>,
}

impl PointerClickOutcome {
    /// Return whether the press and release both routed to the same widget.
    pub fn completed_on_same_widget(self) -> bool {
        self.press_target.is_some() && self.press_target == self.release_target
    }

    /// Return the widget that received both press and release, when the click
    /// completed on one widget.
    pub fn completed_widget(self) -> Option<WidgetId> {
        self.completed_on_same_widget()
            .then_some(self.press_target)
            .flatten()
    }
}

impl PointerMoveOutcome {
    /// Return whether a projected widget received the pointer move.
    pub fn routed(self) -> bool {
        self.target.is_some()
    }

    /// Return whether a backend should redraw the frame.
    pub fn needs_redraw(self) -> bool {
        self.needs_scene_rebuild() || self.paint_only_requested
    }

    /// Return whether the cached scene is stale.
    pub fn needs_scene_rebuild(self) -> bool {
        self.hover_changed || self.repaint_requested
    }
}

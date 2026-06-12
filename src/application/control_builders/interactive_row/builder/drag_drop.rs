use super::InteractiveRowBuilder;

impl InteractiveRowBuilder {
    /// Emit drag lifecycle messages from this row.
    pub fn draggable(mut self) -> Self {
        self.draggable = true;
        self
    }

    /// Configure this row as a host-tracked drag source.
    ///
    /// This preset keeps pointer-motion routing active while a row interaction
    /// is in progress, marks whether a related row drag is active in the host
    /// container, and records whether this row is the active drag source. Call
    /// [`Self::drag_source_motion`] as needed when the active source should
    /// continue emitting move messages after a projected row refresh.
    pub fn tracked_drag_source(mut self, drag_active: bool, drag_source: bool) -> Self {
        self.draggable = true;
        self.drag_active = drag_active;
        self.drag_source = drag_source;
        self.pointer_motion_during_interaction = true;
        self
    }

    /// Configure this row as a host-tracked drag source that emits retained
    /// source move messages.
    ///
    /// Use this when the active source may be rebuilt from host state during a
    /// drag and should continue reporting pointer movement after projection.
    pub fn tracked_drag_source_with_motion(mut self, drag_active: bool, drag_source: bool) -> Self {
        self = self.tracked_drag_source(drag_active, drag_source);
        self.drag_source_motion = true;
        self
    }

    /// Emit drop and hover-drop-target messages.
    pub fn droppable(mut self, drag_active: bool) -> Self {
        self.droppable = true;
        self.drop_hover = true;
        self.drag_active = drag_active;
        self
    }

    /// Emit drop messages without hover-drop-target messages.
    pub fn drop_only(mut self, drag_active: bool) -> Self {
        self.droppable = true;
        self.drop_hover = false;
        self.drag_active = drag_active;
        self
    }

    /// Configure whether this row is a drop target and whether hover-drop
    /// messages should be emitted.
    pub fn drop_target_mode(mut self, drag_active: bool, hover_messages: bool) -> Self {
        self.droppable = drag_active;
        self.drop_hover = drag_active && hover_messages;
        self.drag_active = drag_active;
        self
    }

    /// Configure a host-tracked drop target.
    ///
    /// While `active_target` is true, the row still accepts the eventual drop
    /// but suppresses duplicate hover-drop messages and keeps pointer-motion
    /// routing active through the host-owned interaction state.
    pub fn tracked_drop_target(mut self, drag_active: bool, active_target: bool) -> Self {
        self.pointer_motion_during_interaction = true;
        self.pointer_motion_active = active_target;
        self.droppable = drag_active;
        self.drop_hover = drag_active && !active_target;
        self.drag_active = drag_active;
        self
    }

    /// Configure a host-tracked conditional drop target.
    ///
    /// This variant is useful when the host owns candidate validation and also
    /// needs pointer motion on non-candidates to clear a previously active drop
    /// target. Current targets still accept the final drop while suppressing
    /// duplicate hover-drop messages.
    pub fn tracked_drop_candidate(
        mut self,
        drag_active: bool,
        current_target: bool,
        candidate: bool,
        active_target: bool,
    ) -> Self {
        self.pointer_motion_during_interaction = true;
        self.pointer_motion_active = active_target;
        self.droppable = drag_active;
        self.drop_hover = drag_active && !current_target && (candidate || active_target);
        self.drag_active = drag_active;
        self
    }

    /// Mark whether a related row drag is active in this row's container.
    pub fn drag_active(mut self, active: bool) -> Self {
        self.drag_active = active;
        self
    }

    /// Mark this row as the source of the current container drag.
    pub fn drag_source(mut self, source: bool) -> Self {
        self.drag_source = source;
        self
    }

    /// Emit drag move messages while this row remains the active drag source.
    pub fn drag_source_motion(mut self, enabled: bool) -> Self {
        self.drag_source_motion = enabled;
        self
    }
}

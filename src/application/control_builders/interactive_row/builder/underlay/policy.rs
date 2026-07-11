use super::super::{
    InteractiveRowBuilder,
    policy::{DenseRowDragPolicy, DenseRowDropPolicy, DenseRowPolicy},
};
use super::InteractiveRowUnderlayBuilder;

impl<Message: 'static> InteractiveRowUnderlayBuilder<Message> {
    /// Apply a reusable dense-row behavior policy.
    ///
    /// Policies bundle common input, visual-state, drag, and drop behavior for
    /// app-owned row content. Low-level row, chrome, identity, and message
    /// methods remain available before or after this call for custom cases.
    pub fn dense_row_policy(mut self, policy: DenseRowPolicy) -> Self {
        if policy.custom_paint_hit_target {
            self.row = self.row.custom_paint_hit_target();
        }
        if policy.activation_modifiers {
            self.row = self.row.activation_modifiers();
        }
        match policy.drag {
            DenseRowDragPolicy::None => {}
            DenseRowDragPolicy::Source {
                drag_active,
                drag_source,
                source_motion,
            } => {
                self.row = if source_motion {
                    self.row
                        .tracked_drag_source_with_motion(drag_active, drag_source)
                } else {
                    self.row.tracked_drag_source(drag_active, drag_source)
                };
            }
        }
        match policy.drop {
            DenseRowDropPolicy::None => {}
            DenseRowDropPolicy::Target {
                drag_active,
                active_target,
            } => {
                self.row = self.row.tracked_drop_target(drag_active, active_target);
            }
            DenseRowDropPolicy::Candidate {
                drag_active,
                current_target,
                candidate,
                active_target,
            } => {
                self.row = self.row.tracked_drop_candidate(
                    drag_active,
                    current_target,
                    candidate,
                    active_target,
                );
            }
        }
        if policy.drag_session_motion {
            self.row = self.row.drag_active(true).pointer_motion_active(true);
        }
        if let Some(style) = policy.style {
            self.style = Some(style);
        }
        if let Some(selected) = policy.visual_state_overrides.selected {
            self.visual_state.selected = selected;
        }
        if let Some(active_target) = policy.visual_state_overrides.active_target {
            self.visual_state.active_target = active_target;
        }
        if let Some(candidate) = policy.visual_state_overrides.candidate {
            self.visual_state.candidate = candidate;
        }
        self.dense_chrome = self.dense_chrome || policy.dense_chrome;
        self
    }

    /// Configure the backing interactive row before binding messages.
    pub fn row(
        mut self,
        configure: impl FnOnce(InteractiveRowBuilder) -> InteractiveRowBuilder,
    ) -> Self {
        self.row = configure(self.row);
        self
    }

    /// Configure the backing row as an input-only layer for app-owned row paint.
    ///
    /// Use this when the visible content or dense underlay chrome owns all row
    /// feedback, while Radiant should still route generic row input behavior.
    pub fn custom_paint_hit_target(mut self) -> Self {
        self.row = self.row.custom_paint_hit_target();
        self
    }

    /// Include primary-release modifier state in row activation messages.
    pub fn activation_modifiers(mut self) -> Self {
        self.row = self.row.activation_modifiers();
        self
    }

    /// Configure the backing row as a host-tracked drag source.
    ///
    /// Use this when arbitrary visible row content should keep its own paint
    /// tree while the underlay owns generic drag lifecycle routing.
    pub fn tracked_drag_source(mut self, drag_active: bool, drag_source: bool) -> Self {
        self.row = self.row.tracked_drag_source(drag_active, drag_source);
        self
    }

    /// Configure the backing row as a host-tracked drag source that keeps
    /// emitting pointer movement after the active source is rebuilt.
    pub fn tracked_drag_source_with_motion(mut self, drag_active: bool, drag_source: bool) -> Self {
        self.row = self
            .row
            .tracked_drag_source_with_motion(drag_active, drag_source);
        self
    }

    /// Configure the backing row as a host-tracked drop target.
    ///
    /// Use this when arbitrary visible row content should keep its own paint
    /// tree while the underlay owns generic drop and hover-drop routing.
    pub fn tracked_drop_target(mut self, drag_active: bool, active_target: bool) -> Self {
        self.row = self.row.tracked_drop_target(drag_active, active_target);
        self.visual_state.active_target = active_target;
        self
    }

    /// Configure the backing row as a host-tracked conditional drop target.
    ///
    /// Use this when arbitrary visible row content should keep its own paint
    /// tree while Radiant owns the generic candidate hover and stale-target
    /// clear lifecycle for host-validated drops.
    pub fn tracked_drop_candidate(
        mut self,
        drag_active: bool,
        current_target: bool,
        candidate: bool,
        active_target: bool,
    ) -> Self {
        self.row =
            self.row
                .tracked_drop_candidate(drag_active, current_target, candidate, active_target);
        self.visual_state.active_target = current_target;
        self.visual_state.candidate = candidate;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::super::interactive_row_underlay;
    use super::*;
    use crate::{application::text, widgets::Widget};

    #[test]
    fn dense_row_policy_preserves_drag_session_motion_after_drop_candidate_policy() {
        let builder = interactive_row_underlay::<()>(text("Sample")).dense_row_policy(
            DenseRowPolicy::new()
                .drag_session_motion(true)
                .tracked_drop_candidate(true, false, false, false),
        );
        let row = builder.row.widget();

        assert!(row.props.drag_active);
        assert!(row.props.droppable);
        assert!(!row.props.drop_hover);
        assert!(!row.props.clear_drop_on_hover);
        assert!(row.props.pointer_motion_active);
        assert!(row.accepts_pointer_move());
    }
}

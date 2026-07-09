use crate::widgets::{InteractiveRowVisualStateParts, WidgetStyle};

/// Reusable behavior policy for dense application rows backed by an interactive row underlay.
///
/// Use this when a list, tree, sidebar, picker, or inspector row wants the
/// standard dense-row interaction bundles without repeating the low-level row
/// toggles at every call site. Host applications still own row identity,
/// messages, domain state, visible content, and optional chrome such as
/// markers, palettes, and outlines.
///
/// # Example
///
/// ```ignore
/// use radiant::prelude as ui;
///
/// let row = ui::interactive_row_underlay(ui::text("Kick"))
///     .dense_row_policy(
///         ui::DenseRowPolicy::selectable(true)
///             .activation_modifiers()
///             .tracked_drag_source(drag_active, drag_source)
///             .style(ui::WidgetStyle::subtle(ui::WidgetTone::Accent)),
///     )
///     .actions(ui::row_actions().primary(|| Message::SelectKick));
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DenseRowPolicy {
    pub(super) style: Option<WidgetStyle>,
    pub(super) visual_state: InteractiveRowVisualStateParts,
    pub(super) visual_state_overrides: DenseRowVisualStateOverrides,
    pub(super) dense_chrome: bool,
    pub(super) custom_paint_hit_target: bool,
    pub(super) activation_modifiers: bool,
    pub(super) drag_session_motion: bool,
    pub(super) drag: DenseRowDragPolicy,
    pub(super) drop: DenseRowDropPolicy,
}

impl DenseRowPolicy {
    /// Build an inert dense-row policy.
    pub const fn new() -> Self {
        Self {
            style: None,
            visual_state: InteractiveRowVisualStateParts {
                selected: false,
                active_target: false,
                candidate: false,
            },
            visual_state_overrides: DenseRowVisualStateOverrides::none(),
            dense_chrome: false,
            custom_paint_hit_target: false,
            activation_modifiers: false,
            drag_session_motion: false,
            drag: DenseRowDragPolicy::None,
            drop: DenseRowDropPolicy::None,
        }
    }

    /// Build the default policy for app-painted action rows.
    ///
    /// The row receives generic input, paints dense-row feedback behind the
    /// app-owned content, and leaves host messages/domain state to
    /// [`crate::application::InteractiveRowActions`].
    pub const fn action_row() -> Self {
        Self::new().custom_paint_hit_target().dense_chrome()
    }

    /// Build an app-painted selectable row policy.
    pub const fn selectable(selected: bool) -> Self {
        Self::action_row().selected(selected)
    }

    /// Build an app-painted dense row with explicit visual-state parts.
    pub const fn with_visual_state(visual_state: InteractiveRowVisualStateParts) -> Self {
        Self::action_row().visual_state(visual_state)
    }

    /// Apply an explicit widget style to the backing interactive row.
    pub const fn style(mut self, style: WidgetStyle) -> Self {
        self.style = Some(style);
        self
    }

    /// Paint Radiant's dense-row chrome behind the app-owned visible content.
    pub const fn dense_chrome(mut self) -> Self {
        self.dense_chrome = true;
        self
    }

    /// Configure the backing input row for app-owned row paint.
    pub const fn custom_paint_hit_target(mut self) -> Self {
        self.custom_paint_hit_target = true;
        self
    }

    /// Include primary-release modifier state in row activation messages.
    pub const fn activation_modifiers(mut self) -> Self {
        self.activation_modifiers = true;
        self
    }

    /// Keep pointer motion routed while a host-owned drag session is active.
    ///
    /// Use this for custom-painted dense rows that are not the active drag
    /// source or drop target but still need a motion sample to clear stale
    /// hover or drag-session chrome.
    pub const fn drag_session_motion(mut self, active: bool) -> Self {
        self.drag_session_motion = active;
        self
    }

    /// Mark the row as selected by host-owned state.
    pub const fn selected(mut self, selected: bool) -> Self {
        self.visual_state.selected = selected;
        self.visual_state_overrides.selected = Some(selected);
        self.dense_chrome = true;
        self
    }

    /// Mark the row as the committed target for a host-owned operation.
    pub const fn active_target(mut self, active_target: bool) -> Self {
        self.visual_state.active_target = active_target;
        self.visual_state_overrides.active_target = Some(active_target);
        self.dense_chrome = true;
        self
    }

    /// Mark the row as a valid candidate for a host-owned operation.
    pub const fn candidate(mut self, candidate: bool) -> Self {
        self.visual_state.candidate = candidate;
        self.visual_state_overrides.candidate = Some(candidate);
        self.dense_chrome = true;
        self
    }

    /// Apply all host-owned visual-state parts.
    pub const fn visual_state(mut self, visual_state: InteractiveRowVisualStateParts) -> Self {
        self.visual_state = visual_state;
        self.visual_state_overrides = DenseRowVisualStateOverrides::from_parts(visual_state);
        self.dense_chrome = true;
        self
    }

    /// Configure the row as a host-tracked drag source.
    pub const fn tracked_drag_source(mut self, drag_active: bool, drag_source: bool) -> Self {
        self.drag = DenseRowDragPolicy::Source {
            drag_active,
            drag_source,
            source_motion: false,
        };
        self
    }

    /// Configure the row as a host-tracked drag source that emits retained source motion.
    pub const fn tracked_drag_source_with_motion(
        mut self,
        drag_active: bool,
        drag_source: bool,
    ) -> Self {
        self.drag = DenseRowDragPolicy::Source {
            drag_active,
            drag_source,
            source_motion: true,
        };
        self
    }

    /// Configure the row as a host-tracked drop target.
    pub const fn tracked_drop_target(mut self, drag_active: bool, active_target: bool) -> Self {
        self.drop = DenseRowDropPolicy::Target {
            drag_active,
            active_target,
        };
        self.visual_state.active_target = active_target;
        self.visual_state_overrides.active_target = Some(active_target);
        self.dense_chrome = true;
        self
    }

    /// Configure the row as a host-tracked conditional drop target.
    pub const fn tracked_drop_candidate(
        mut self,
        drag_active: bool,
        current_target: bool,
        candidate: bool,
        active_target: bool,
    ) -> Self {
        self.drop = DenseRowDropPolicy::Candidate {
            drag_active,
            current_target,
            candidate,
            active_target,
        };
        self.visual_state.active_target = current_target;
        self.visual_state.candidate = candidate;
        self.visual_state_overrides.active_target = Some(current_target);
        self.visual_state_overrides.candidate = Some(candidate);
        self.dense_chrome = true;
        self
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub(super) struct DenseRowVisualStateOverrides {
    pub selected: Option<bool>,
    pub active_target: Option<bool>,
    pub candidate: Option<bool>,
}

impl DenseRowVisualStateOverrides {
    pub const fn none() -> Self {
        Self {
            selected: None,
            active_target: None,
            candidate: None,
        }
    }

    pub const fn from_parts(parts: InteractiveRowVisualStateParts) -> Self {
        Self {
            selected: Some(parts.selected),
            active_target: Some(parts.active_target),
            candidate: Some(parts.candidate),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub(super) enum DenseRowDragPolicy {
    #[default]
    None,
    Source {
        drag_active: bool,
        drag_source: bool,
        source_motion: bool,
    },
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub(super) enum DenseRowDropPolicy {
    #[default]
    None,
    Target {
        drag_active: bool,
        active_target: bool,
    },
    Candidate {
        drag_active: bool,
        current_target: bool,
        candidate: bool,
        active_target: bool,
    },
}

impl Default for DenseRowPolicy {
    fn default() -> Self {
        Self::new()
    }
}

//! Generic badge and pill primitives.

mod inline;

pub use inline::{
    InlineBadgeMetrics, inline_badge_cluster_reserved_width, inline_badge_height,
    inline_badge_labels, inline_badge_labels_owned, inline_badge_labels_owned_into,
    inline_badge_rects, inline_badge_rects_for_labels, inline_badge_rects_for_labels_into,
    inline_badge_rects_into, inline_badge_text_origin, inline_badge_text_width, inline_badge_width,
};

/// Selectable badge/pill model with host-chosen state semantics.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct SelectablePill<State> {
    /// Stable identifier for hit testing and automation.
    pub id: String,
    /// User-facing pill label.
    pub label: String,
    /// Selection value for the current target set.
    pub state: State,
}

/// Generic pill-editor panel with a text input and grouped selectable pills.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct PillEditorPanel<State> {
    /// Whether the panel should render in the current view.
    pub open: bool,
    /// Count of selected rows or items represented by the panel target set.
    pub selected_count: usize,
    /// Header line describing the current selection/focus context.
    pub header_label: String,
    /// Whether the host-defined primary side effect is enabled.
    pub primary_action_enabled: bool,
    /// Current search/create input value.
    pub input_value: String,
    /// Placeholder shown for the input when empty.
    pub input_placeholder: String,
    /// Whether the input currently owns text-editing focus.
    pub input_focused: bool,
    /// Caret position measured in Unicode scalar values from the start.
    pub input_caret: usize,
    /// Selected text range measured in Unicode scalar values, when any.
    pub input_selection: Option<(usize, usize)>,
    /// Exclusive or high-priority pill choices.
    pub exclusive_pills: [SelectablePill<State>; 2],
    /// Accepted pills that are already applied to the represented target set.
    pub accepted_pills: Vec<SelectablePill<State>>,
    /// Normal pill candidates from common usage or search.
    pub option_pills: Vec<SelectablePill<State>>,
    /// Create-new candidate when the input does not exactly match an existing option.
    pub create_pill: Option<SelectablePill<State>>,
}

#[cfg(test)]
mod tests {
    use super::{
        InlineBadgeMetrics, PillEditorPanel, SelectablePill, inline_badge_cluster_reserved_width,
        inline_badge_height, inline_badge_labels_owned, inline_badge_labels_owned_into,
        inline_badge_rects, inline_badge_rects_for_labels, inline_badge_rects_for_labels_into,
        inline_badge_rects_into, inline_badge_text_origin, inline_badge_width,
    };
    use crate::gui::selection::TriState;
    use crate::gui::types::{Point, Rect};

    #[test]
    fn selectable_pill_preserves_identity_label_and_state() {
        let pill = SelectablePill {
            id: String::from("priority"),
            label: String::from("Priority"),
            state: TriState::Mixed,
        };

        assert_eq!(pill.id, "priority");
        assert_eq!(pill.label, "Priority");
        assert_eq!(pill.state, TriState::Mixed);
    }

    #[test]
    fn pill_editor_panel_defaults_to_closed_empty_panel() {
        let panel: PillEditorPanel<TriState> = PillEditorPanel::default();

        assert!(!panel.open);
        assert_eq!(panel.selected_count, 0);
        assert_eq!(panel.header_label, "");
        assert!(!panel.primary_action_enabled);
        assert_eq!(panel.input_value, "");
        assert_eq!(panel.input_placeholder, "");
        assert_eq!(panel.exclusive_pills.len(), 2);
        assert!(panel.option_pills.is_empty());
        assert!(panel.create_pill.is_none());
    }

    #[test]
    fn inline_badge_labels_and_widths_are_stable() {
        let metrics = InlineBadgeMetrics::new(10.0, 3.0, 1.0, 3.0, 4.0, 10.0);
        let labels = inline_badge_labels_owned("  One  · Two ·  · Three ", "·");

        assert_eq!(labels, ["One", "Two", "Three"]);
        assert_eq!(inline_badge_width("One", metrics), 23.0);
        assert_eq!(inline_badge_cluster_reserved_width(&labels, metrics), 90.0);
    }

    #[test]
    fn inline_badge_labels_owned_into_reuses_output_storage() {
        let mut labels = Vec::with_capacity(8);
        labels.push(String::from("stale"));
        let capacity = labels.capacity();

        inline_badge_labels_owned_into("  One  · Two ·  · Three ", "·", &mut labels);

        assert_eq!(labels, ["One", "Two", "Three"]);
        assert_eq!(labels.capacity(), capacity);

        inline_badge_labels_owned_into("", "·", &mut labels);

        assert!(labels.is_empty());
        assert_eq!(labels.capacity(), capacity);
    }

    #[test]
    fn inline_badge_rects_clamp_to_available_item_row() {
        let metrics = InlineBadgeMetrics::new(10.0, 3.0, 1.0, 3.0, 4.0, 10.0);
        let item = Rect::from_min_max(Point::new(0.0, 4.0), Point::new(100.0, 18.0));
        let labels = vec![String::from("One"), String::from("Two")];

        let rects = inline_badge_rects_for_labels(item, &labels, 5.0, metrics);

        assert_eq!(rects.len(), 2);
        assert_eq!(rects[0].min.y, 5.0);
        assert_eq!(rects[0].max.y, 17.0);
        assert_eq!(rects[1].max.x, 95.0);
        assert_eq!(
            inline_badge_text_origin(rects[0], metrics),
            Point::new(49.0, 6.0)
        );
    }

    #[test]
    fn inline_badge_rects_into_reuses_label_and_rect_storage() {
        let metrics = InlineBadgeMetrics::new(10.0, 3.0, 1.0, 3.0, 4.0, 10.0);
        let item = Rect::from_min_max(Point::new(0.0, 4.0), Point::new(100.0, 18.0));
        let labels = vec![String::from("One"), String::from("Two")];
        let mut rects = Vec::with_capacity(8);
        rects.push(Rect::from_min_max(
            Point::new(0.0, 0.0),
            Point::new(1.0, 1.0),
        ));
        let rect_capacity = rects.capacity();

        inline_badge_rects_for_labels_into(item, &labels, 5.0, metrics, &mut rects);

        assert_eq!(rects.len(), 2);
        assert_eq!(rects.capacity(), rect_capacity);
        assert_eq!(rects[1].max.x, 95.0);

        let mut owned_labels = Vec::with_capacity(8);
        owned_labels.push(String::from("stale"));
        let label_capacity = owned_labels.capacity();

        inline_badge_rects_into(
            item,
            "One · Two",
            "·",
            5.0,
            metrics,
            &mut owned_labels,
            &mut rects,
        );

        assert_eq!(owned_labels, ["One", "Two"]);
        assert_eq!(owned_labels.capacity(), label_capacity);
        assert_eq!(rects.len(), 2);
        assert_eq!(rects.capacity(), rect_capacity);
    }

    #[test]
    fn inline_badge_rects_handle_empty_or_cramped_inputs() {
        let metrics = InlineBadgeMetrics::new(10.0, 3.0, 1.0, 3.0, 4.0, 10.0);
        let item = Rect::from_min_max(Point::new(0.0, 0.0), Point::new(20.0, 6.0));

        assert!(inline_badge_rects(item, "", "·", 0.0, metrics).is_empty());
        assert_eq!(inline_badge_height(item, metrics), 6.0);
    }
}

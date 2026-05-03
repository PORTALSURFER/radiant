//! Generic badge and pill primitives.

use crate::gui::types::{Point, Rect};

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
    /// Exclusive or high-priority pill choices.
    pub exclusive_pills: [SelectablePill<State>; 2],
    /// Normal pill candidates from common usage or search.
    pub option_pills: Vec<SelectablePill<State>>,
    /// Create-new candidate when the input does not exactly match an existing option.
    pub create_pill: Option<SelectablePill<State>>,
}

/// Layout metrics for a compact inline badge cluster.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InlineBadgeMetrics {
    /// Font size used for label measurement and vertical text placement.
    pub font_size: f32,
    /// Horizontal inset inside each badge.
    pub padding_x: f32,
    /// Vertical inset inside each badge.
    pub padding_y: f32,
    /// Horizontal gap between adjacent badges.
    pub badge_gap: f32,
    /// Gap between the host item label and the badge cluster.
    pub cluster_gap: f32,
    /// Minimum desired badge height before clamping to the available row height.
    pub min_height: f32,
}

impl InlineBadgeMetrics {
    /// Construct metrics from already-resolved geometry tokens.
    pub fn new(
        font_size: f32,
        padding_x: f32,
        padding_y: f32,
        badge_gap: f32,
        cluster_gap: f32,
        min_height: f32,
    ) -> Self {
        Self {
            font_size,
            padding_x,
            padding_y,
            badge_gap,
            cluster_gap,
            min_height,
        }
    }
}

/// Approximate the rendered width of one inline badge label.
pub fn inline_badge_text_width(text: &str, metrics: InlineBadgeMetrics) -> f32 {
    if text.is_empty() {
        return 0.0;
    }
    ((text.chars().count() as f32) * (metrics.font_size * 0.56).max(1.0)).ceil()
}

/// Split one delimited inline badge payload into stable per-badge labels.
pub fn inline_badge_labels<'a>(
    text: &'a str,
    delimiter: &'a str,
) -> impl Iterator<Item = &'a str> + 'a {
    text.split(delimiter)
        .map(str::trim)
        .filter(|label| !label.is_empty())
}

/// Materialize inline badge labels once when a cache boundary owns them.
pub fn inline_badge_labels_owned(text: &str, delimiter: &str) -> Vec<String> {
    inline_badge_labels(text, delimiter)
        .map(str::to_owned)
        .collect::<Vec<_>>()
}

/// Return the filled badge width needed for one inline badge label.
pub fn inline_badge_width(text: &str, metrics: InlineBadgeMetrics) -> f32 {
    if text.is_empty() {
        return 0.0;
    }
    inline_badge_text_width(text, metrics) + (metrics.padding_x * 2.0)
}

/// Return reserved width for a pre-split inline badge cluster.
pub fn inline_badge_cluster_reserved_width(labels: &[String], metrics: InlineBadgeMetrics) -> f32 {
    if labels.is_empty() {
        return 0.0;
    }
    let badges_width = labels
        .iter()
        .map(|label| inline_badge_width(label, metrics))
        .sum::<f32>();
    let badge_gap_count = labels.len().saturating_sub(1) as f32;
    badges_width + (badge_gap_count * metrics.badge_gap) + metrics.cluster_gap
}

/// Return the desired badge height for one item-label row.
pub fn inline_badge_height(item_label: Rect, metrics: InlineBadgeMetrics) -> f32 {
    let available_height = item_label.height().max(0.0);
    if available_height <= 0.0 {
        return 0.0;
    }
    let desired_height = (metrics.font_size + (metrics.padding_y * 2.0)).round();
    let min_height = metrics.min_height.min(available_height);
    desired_height.clamp(min_height, available_height)
}

/// Compute badge rects for pre-split inline badge labels.
pub fn inline_badge_rects_for_labels(
    item_label: Rect,
    labels: &[String],
    trailing_reserved_width: f32,
    metrics: InlineBadgeMetrics,
) -> Vec<Rect> {
    if labels.is_empty() || item_label.width() <= 0.0 || item_label.height() <= 0.0 {
        return Vec::new();
    }
    let total_width = labels
        .iter()
        .map(|label| inline_badge_width(label, metrics))
        .sum::<f32>()
        + (labels.len().saturating_sub(1) as f32 * metrics.badge_gap);
    let right_edge = (item_label.max.x - trailing_reserved_width).max(item_label.min.x);
    let start_x = (right_edge - total_width).max(item_label.min.x);
    let badge_height = inline_badge_height(item_label, metrics);
    if badge_height <= 0.0 || right_edge <= start_x {
        return Vec::new();
    }
    let min_y = item_label.min.y + ((item_label.height() - badge_height) * 0.5).floor();
    let max_y = (min_y + badge_height).min(item_label.max.y);
    if max_y <= min_y {
        return Vec::new();
    }
    let mut x = start_x;
    labels
        .iter()
        .map(|label| {
            let width = inline_badge_width(label, metrics);
            let rect = Rect::from_min_max(
                Point::new(x, min_y),
                Point::new((x + width).min(right_edge), max_y),
            );
            x = (rect.max.x + metrics.badge_gap).min(right_edge);
            rect
        })
        .collect()
}

/// Compute badge rects for one delimited inline badge cluster.
pub fn inline_badge_rects(
    item_label: Rect,
    text: &str,
    delimiter: &str,
    trailing_reserved_width: f32,
    metrics: InlineBadgeMetrics,
) -> Vec<Rect> {
    if text.is_empty() || item_label.width() <= 0.0 || item_label.height() <= 0.0 {
        return Vec::new();
    }
    let labels = inline_badge_labels_owned(text, delimiter);
    inline_badge_rects_for_labels(item_label, &labels, trailing_reserved_width, metrics)
}

/// Return the inset text origin for one inline badge.
pub fn inline_badge_text_origin(badge_rect: Rect, metrics: InlineBadgeMetrics) -> Point {
    Point::new(
        badge_rect.min.x + metrics.padding_x,
        badge_rect.min.y + ((badge_rect.height() - metrics.font_size) * 0.5).floor(),
    )
}

#[cfg(test)]
mod tests {
    use super::{
        InlineBadgeMetrics, PillEditorPanel, SelectablePill, inline_badge_cluster_reserved_width,
        inline_badge_height, inline_badge_labels_owned, inline_badge_rects,
        inline_badge_rects_for_labels, inline_badge_text_origin, inline_badge_width,
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
    fn inline_badge_rects_handle_empty_or_cramped_inputs() {
        let metrics = InlineBadgeMetrics::new(10.0, 3.0, 1.0, 3.0, 4.0, 10.0);
        let item = Rect::from_min_max(Point::new(0.0, 0.0), Point::new(20.0, 6.0));

        assert!(inline_badge_rects(item, "", "·", 0.0, metrics).is_empty());
        assert_eq!(inline_badge_height(item, metrics), 6.0);
    }
}

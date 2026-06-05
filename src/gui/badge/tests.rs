use super::{
    InlineBadgeMetrics, InlineBadgeMetricsParts, PillEditorPanel, SelectablePill,
    inline_badge_cluster_reserved_width, inline_badge_height, inline_badge_labels_owned,
    inline_badge_labels_owned_into, inline_badge_rects, inline_badge_rects_for_labels,
    inline_badge_rects_for_labels_into, inline_badge_rects_for_labels_with_widths_into,
    inline_badge_rects_into, inline_badge_text_origin, inline_badge_width,
    inline_badge_width_in_range,
};
use crate::gui::selection::TriState;
use crate::gui::types::{Point, Rect};

fn badge_metrics() -> InlineBadgeMetrics {
    InlineBadgeMetrics::from_parts(InlineBadgeMetricsParts {
        font_size: 10.0,
        padding_x: 3.0,
        padding_y: 1.0,
        badge_gap: 3.0,
        cluster_gap: 4.0,
        min_height: 10.0,
    })
}

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

    assert!(!panel.status.open);
    assert_eq!(panel.status.selected_count, 0);
    assert_eq!(panel.status.header_label, "");
    assert!(!panel.status.primary_action_enabled);
    assert_eq!(panel.input.input_value, "");
    assert_eq!(panel.input.input_placeholder, "");
    assert_eq!(panel.choices.exclusive_pills.len(), 2);
    assert!(panel.choices.option_pills.is_empty());
    assert!(panel.choices.create_pill.is_none());
}

#[test]
fn inline_badge_labels_and_widths_are_stable() {
    let metrics = badge_metrics();
    let labels = inline_badge_labels_owned("  One  · Two ·  · Three ", "·");

    assert_eq!(labels, ["One", "Two", "Three"]);
    assert_eq!(inline_badge_width("One", metrics), 23.0);
    assert_eq!(inline_badge_cluster_reserved_width(&labels, metrics), 90.0);
}

#[test]
fn inline_badge_width_in_range_clamps_non_empty_labels() {
    let metrics = badge_metrics();

    assert_eq!(
        inline_badge_width_in_range("One", metrics, 30.0, 80.0),
        30.0
    );
    assert_eq!(
        inline_badge_width_in_range("Long badge label", metrics, 30.0, 80.0),
        80.0
    );
    assert_eq!(inline_badge_width_in_range("", metrics, 30.0, 80.0), 0.0);
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
    let metrics = badge_metrics();
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
    let metrics = badge_metrics();
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
fn inline_badge_rects_with_widths_reuses_width_and_rect_storage() {
    let metrics = badge_metrics();
    let item = Rect::from_min_max(Point::new(0.0, 4.0), Point::new(100.0, 18.0));
    let labels = vec![String::from("One"), String::from("Two")];
    let mut widths = Vec::with_capacity(8);
    widths.push(99.0);
    let width_capacity = widths.capacity();
    let mut rects = Vec::with_capacity(8);
    rects.push(Rect::from_min_max(
        Point::new(0.0, 0.0),
        Point::new(1.0, 1.0),
    ));
    let rect_capacity = rects.capacity();

    inline_badge_rects_for_labels_with_widths_into(
        item,
        &labels,
        5.0,
        metrics,
        &mut widths,
        &mut rects,
    );

    assert_eq!(widths, [23.0, 23.0]);
    assert_eq!(widths.capacity(), width_capacity);
    assert_eq!(rects.len(), 2);
    assert_eq!(rects.capacity(), rect_capacity);
    assert_eq!(rects[1].max.x, 95.0);
}

#[test]
fn inline_badge_rects_handle_empty_or_cramped_inputs() {
    let metrics = badge_metrics();
    let item = Rect::from_min_max(Point::new(0.0, 0.0), Point::new(20.0, 6.0));

    assert!(inline_badge_rects(item, "", "·", 0.0, metrics).is_empty());
    assert_eq!(inline_badge_height(item, metrics), 6.0);
}

use super::{ContentViewChrome, StatusSegments, StatusSegmentsParts};

#[test]
fn status_segments_default_to_empty_text() {
    assert_eq!(StatusSegments::default().left, "");
    assert_eq!(StatusSegments::default().center, "");
    assert_eq!(StatusSegments::default().right, "");
}

#[test]
fn status_segments_build_explicit_and_incremental_labels() {
    let segments = StatusSegments::new("Ready", "Autosave on", "Idle")
        .with_left("Saved")
        .with_center("Autosave off")
        .with_right("Busy");

    assert_eq!(segments.left, "Saved");
    assert_eq!(segments.center, "Autosave off");
    assert_eq!(segments.right, "Busy");
}

#[test]
fn status_segments_support_named_parts_construction() {
    let segments = StatusSegments::from_parts(StatusSegmentsParts {
        left: "Ready".to_owned(),
        center: "Autosave on".to_owned(),
        right: "Idle".to_owned(),
    });

    assert_eq!(segments.left, "Ready");
    assert_eq!(segments.center, "Autosave on");
    assert_eq!(segments.right, "Idle");
}

#[test]
fn status_segments_primary_populates_left_label_only() {
    let segments = StatusSegments::primary("Ready");

    assert_eq!(segments.left, "Ready");
    assert_eq!(segments.center, "");
    assert_eq!(segments.right, "");
}

#[test]
fn content_view_chrome_defaults_to_product_neutral_copy() {
    let chrome = ContentViewChrome::default();

    assert_eq!(chrome.items_tab_label, "Items");
    assert_eq!(chrome.item_column_label, "Item");
    assert_eq!(chrome.map_tab_label, "Map");
    assert_eq!(chrome.pill_editor_label, "Pills");
    assert_eq!(chrome.search_placeholder, "Search items (Ctrl+F)");
    assert_eq!(chrome.activity_busy_label, "Filtering");
    assert_eq!(chrome.item_count_label, "0 items");
}

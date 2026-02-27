//! Tests for slotized control-row and toolbar partition helpers.

use super::{
    compute_browser_toolbar_sections, compute_sidebar_action_button_rects,
    compute_update_action_button_rects,
};
use crate::gui::native_shell::style::StyleTokens;
use crate::gui::types::{Point, Rect};

#[test]
fn update_action_buttons_right_align_and_fit_cluster() {
    let style = StyleTokens::for_viewport_width(1280.0);
    let row = Rect::from_min_max(Point::new(0.0, 0.0), Point::new(1280.0, 24.0));
    let cluster = Rect::from_min_max(Point::new(980.0, 0.0), Point::new(1260.0, 24.0));
    let rects = compute_update_action_button_rects(
        row,
        cluster,
        style.sizing,
        &["Open", "Install", "Dismiss"],
    );
    assert!(!rects.is_empty());
    for rect in &rects {
        assert!(rect.min.x >= cluster.min.x);
        assert!(rect.max.x <= cluster.max.x);
        assert!(rect.min.y >= row.min.y);
        assert!(rect.max.y <= row.max.y);
    }
}

#[test]
fn toolbar_sections_stay_left_of_action_cluster() {
    let style = StyleTokens::for_viewport_width(1280.0);
    let toolbar = Rect::from_min_max(Point::new(300.0, 200.0), Point::new(1180.0, 220.0));
    let sections = compute_browser_toolbar_sections(toolbar, style.sizing, Some(980.0));
    assert!(sections.search_field.min.x >= toolbar.min.x);
    assert!(sections.search_field.max.x <= 980.0);
    if sections.activity_chip.width() > 0.0 {
        assert!(sections.activity_chip.min.x >= sections.search_field.max.x);
        assert!(sections.activity_chip.max.x <= 980.0);
    }
    for chip in sections.triage_chips {
        if chip.width() > 0.0 {
            assert!(chip.min.x >= toolbar.min.x);
            assert!(chip.max.x <= 980.0);
            assert!(chip.min.y >= toolbar.min.y);
            assert!(chip.max.y <= toolbar.max.y);
        }
    }
}

#[test]
fn sidebar_buttons_stay_inside_footer() {
    let style = StyleTokens::for_viewport_width(1280.0);
    let footer = Rect::from_min_max(Point::new(20.0, 640.0), Point::new(280.0, 700.0));
    let rects = compute_sidebar_action_button_rects(footer, style.sizing, 5);
    assert_eq!(rects.len(), 5);
    for rect in &rects {
        assert!(rect.min.x >= footer.min.x);
        assert!(rect.max.x <= footer.max.x);
        assert!(rect.min.y >= footer.min.y);
        assert!(rect.max.y <= footer.max.y);
    }
}

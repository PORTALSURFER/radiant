//! Slotized text-line geometry helpers for control rows and action buttons.

use super::super::style::SizingTokens;
use crate::gui::text_layout::{TextLineInsets, centered_text_line};
use crate::gui::types::Rect;

const ACTION_BUTTON_TEXT_BASE_ID: u64 = 1610;

/// Compute an action-button label line rect with horizontal inset.
pub(crate) fn compute_action_button_text_rect(rect: Rect, sizing: SizingTokens) -> Rect {
    compute_text_line_rect(
        rect,
        sizing,
        sizing.font_meta,
        sizing.text_inset_x.max(0.0),
        ACTION_BUTTON_TEXT_BASE_ID,
    )
}

fn compute_text_line_rect(
    rect: Rect,
    sizing: SizingTokens,
    font_size: f32,
    horizontal_inset: f32,
    node_id: u64,
) -> Rect {
    let empty = empty_rect(rect);
    if rect.width() <= 0.0 || rect.height() <= 0.0 || font_size <= 0.0 {
        return empty;
    }
    let text_bounds = rect.inset_horizontal(horizontal_inset, horizontal_inset);
    if text_bounds.width() <= 0.0 || text_bounds.height() <= 0.0 {
        return empty;
    }
    centered_text_line(
        text_bounds,
        font_size,
        TextLineInsets::horizontal(0.0),
        sizing.text_inset_y.max(0.0),
        node_id,
    )
}

fn empty_rect(bounds: Rect) -> Rect {
    bounds.empty_at_min()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::native_shell::style::StyleTokens;

    fn assert_inside(outer: Rect, inner: Rect) {
        assert!(inner.min.x >= outer.min.x);
        assert!(inner.min.y >= outer.min.y);
        assert!(inner.max.x <= outer.max.x);
        assert!(inner.max.y <= outer.max.y);
    }

    #[test]
    fn action_button_text_rect_respects_horizontal_inset() {
        let style = StyleTokens::for_viewport_width(1280.0);
        let button = Rect::from_min_max(Point::new(920.0, 16.0), Point::new(1020.0, 34.0));
        let text_rect = compute_action_button_text_rect(button, style.sizing);
        assert_inside(button, text_rect);
        assert!(text_rect.min.x >= button.min.x + style.sizing.text_inset_x);
        assert!(text_rect.max.x <= button.max.x - style.sizing.text_inset_x);
    }

    #[test]
    fn action_button_text_rect_collapses_for_empty_button() {
        let style = StyleTokens::for_viewport_width(1280.0);
        let button = Rect::from_min_max(Point::new(920.0, 16.0), Point::new(920.0, 16.0));
        let text_rect = compute_action_button_text_rect(button, style.sizing);
        assert_eq!(text_rect, button);
    }
}

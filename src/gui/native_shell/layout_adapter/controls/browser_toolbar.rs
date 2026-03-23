use super::super::super::style::SizingTokens;
use super::shared::{
    center_square_rect, clamp_rect_to_bounds, empty_rect, layout_left_aligned_fixed_widths,
};
use crate::gui::types::{Point, Rect};

const TOOLBAR_FILTER_ID: u64 = 801;
const TOOLBAR_FILTER_CHIP_BASE_ID: u64 = 820;
const RATING_FILTER_CHIP_COUNT: usize = 8;

/// Slot-resolved browser toolbar sections for search and chip controls.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BrowserToolbarSections {
    pub rating_filter_chips: [Rect; 8],
    pub action_slot: Rect,
    pub search_field: Rect,
    pub activity_chip: Rect,
    pub sort_chip: Rect,
    pub triage_chips: [Rect; 3],
}

/// Compute browser toolbar search/activity/sort partitions from slot rows.
pub(crate) fn compute_browser_toolbar_sections(
    toolbar: Rect,
    sizing: SizingTokens,
) -> BrowserToolbarSections {
    let empty = empty_rect(toolbar);
    let empty_chips = [empty; 3];
    let empty_filter_chips = [empty; RATING_FILTER_CHIP_COUNT];
    if toolbar.width() <= 0.0 || toolbar.height() <= 0.0 {
        return BrowserToolbarSections {
            rating_filter_chips: empty_filter_chips,
            action_slot: empty,
            search_field: empty,
            activity_chip: empty,
            sort_chip: empty,
            triage_chips: empty_chips,
        };
    }
    let gap = sizing.action_button_gap.max(1.0);
    let host = toolbar;
    if host.width() <= 1.0 || host.height() <= 0.0 {
        return BrowserToolbarSections {
            rating_filter_chips: empty_filter_chips,
            action_slot: empty,
            search_field: empty,
            activity_chip: empty,
            sort_chip: empty,
            triage_chips: empty_chips,
        };
    }
    let left_min = host.min.x + sizing.text_inset_x;
    let left_max = (host.max.x - sizing.text_inset_x).max(left_min);
    let available = (left_max - left_min).max(0.0);
    if available <= 1.0 {
        return BrowserToolbarSections {
            rating_filter_chips: empty_filter_chips,
            action_slot: empty,
            search_field: empty,
            activity_chip: empty,
            sort_chip: empty,
            triage_chips: empty_chips,
        };
    }
    let filter_gap = sizing.border_width.max(1.0) + 1.0;
    let max_filter_side = (host.height() - (sizing.text_inset_y * 2.0))
        .floor()
        .clamp(6.0, 14.0);
    let desired_search_width = ((host.width() * sizing.browser_search_field_ratio)
        .max(sizing.browser_search_field_min_width))
    .min(
        (available * sizing.browser_search_field_ratio).max(sizing.browser_search_field_min_width),
    );
    let action_side = (host.height() - (sizing.text_inset_y * 0.4))
        .floor()
        .clamp(14.0, 24.0)
        .min((available - gap).max(0.0));
    let min_search_width = sizing.browser_search_field_min_width.min(available);
    let available_for_filters =
        (available - desired_search_width - action_side - (gap * 2.0)).max(0.0);
    let filter_side = ((available_for_filters
        - (filter_gap * (RATING_FILTER_CHIP_COUNT.saturating_sub(1) as f32)))
        / RATING_FILTER_CHIP_COUNT as f32)
        .floor()
        .clamp(6.0, max_filter_side);
    let filter_total_width = ((filter_side * RATING_FILTER_CHIP_COUNT as f32)
        + (filter_gap * (RATING_FILTER_CHIP_COUNT.saturating_sub(1) as f32)))
        .min(available);
    let remaining_after_filters =
        (available - filter_total_width - action_side - (gap * 2.0)).max(0.0);
    let search_width = desired_search_width
        .min(remaining_after_filters.max(min_search_width))
        .max(0.0);
    let filter_bounds = Rect::from_min_max(
        Point::new(left_min, host.min.y),
        Point::new((left_min + filter_total_width).min(left_max), host.max.y),
    );
    let filter_strip = if filter_total_width > 0.0 {
        clamp_rect_to_bounds(filter_bounds, host)
    } else {
        empty
    };
    let search_field = if search_width > 0.0 {
        Rect::from_min_max(
            Point::new((left_max - search_width).max(left_min), host.min.y),
            Point::new(left_max, host.max.y),
        )
    } else {
        empty
    };
    let action_slot = if action_side > 0.0 && search_field.width() > 1.0 {
        let action_max_x = (search_field.min.x - gap).max(left_min);
        Rect::from_min_max(
            Point::new(
                (action_max_x - action_side).max(filter_strip.max.x + gap),
                host.min.y,
            ),
            Point::new(action_max_x, host.max.y),
        )
    } else {
        empty
    };
    let rating_filter_chips = compute_rating_filter_chip_rects(
        filter_strip,
        filter_side,
        filter_gap,
        TOOLBAR_FILTER_CHIP_BASE_ID,
    );
    BrowserToolbarSections {
        rating_filter_chips,
        action_slot: center_square_rect(clamp_rect_to_bounds(action_slot, host), action_side),
        search_field,
        activity_chip: empty,
        sort_chip: empty,
        triage_chips: empty_chips,
    }
}

fn compute_rating_filter_chip_rects(
    strip: Rect,
    chip_side: f32,
    gap: f32,
    first_chip_id: u64,
) -> [Rect; 8] {
    let empty = empty_rect(strip);
    if strip.width() <= 1.0 || strip.height() <= 0.0 || chip_side <= 0.0 {
        return [empty; RATING_FILTER_CHIP_COUNT];
    }
    let widths = [chip_side; RATING_FILTER_CHIP_COUNT];
    let rects = layout_left_aligned_fixed_widths(
        strip,
        gap,
        &widths,
        TOOLBAR_FILTER_ID + 10,
        first_chip_id,
    );
    let mut chips = [empty; RATING_FILTER_CHIP_COUNT];
    for (index, rect) in rects.into_iter().take(RATING_FILTER_CHIP_COUNT).enumerate() {
        chips[index] = center_square_rect(rect, chip_side);
    }
    chips
}

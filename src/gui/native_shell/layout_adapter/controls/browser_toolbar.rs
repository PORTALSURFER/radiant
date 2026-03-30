use super::super::super::style::SizingTokens;
use super::shared::{
    center_square_rect, clamp_rect_to_bounds, empty_rect, layout_left_aligned_fixed_widths,
};
use crate::gui::types::{Point, Rect};

const TOOLBAR_FILTER_ID: u64 = 801;
const TOOLBAR_FILTER_CHIP_BASE_ID: u64 = 820;
const RATING_FILTER_CHIP_COUNT: usize = 8;
const PLAYBACK_AGE_FILTER_CHIP_COUNT: usize = 3;

/// Slot-resolved browser toolbar sections for search and chip controls.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BrowserToolbarSections {
    pub rating_filter_chips: [Rect; 8],
    pub playback_age_filter_chips: [Rect; 3],
    pub marked_filter_chip: Rect,
    pub action_slots: [Rect; 2],
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
    let empty_playback_age_filter_chips = [empty; PLAYBACK_AGE_FILTER_CHIP_COUNT];
    let empty_action_slots = [empty; 2];
    if toolbar.width() <= 0.0 || toolbar.height() <= 0.0 {
        return BrowserToolbarSections {
            rating_filter_chips: empty_filter_chips,
            playback_age_filter_chips: empty_playback_age_filter_chips,
            marked_filter_chip: empty,
            action_slots: empty_action_slots,
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
            playback_age_filter_chips: empty_playback_age_filter_chips,
            marked_filter_chip: empty,
            action_slots: empty_action_slots,
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
            playback_age_filter_chips: empty_playback_age_filter_chips,
            marked_filter_chip: empty,
            action_slots: empty_action_slots,
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
    let action_button_count = 2usize;
    let action_cluster_gap = gap;
    let action_cluster_width = if action_side > 0.0 {
        (action_side * action_button_count as f32)
            + (action_cluster_gap * action_button_count.saturating_sub(1) as f32)
    } else {
        0.0
    };
    let min_search_width = sizing.browser_search_field_min_width.min(available);
    let filter_chip_count = RATING_FILTER_CHIP_COUNT + PLAYBACK_AGE_FILTER_CHIP_COUNT;
    let available_for_filters =
        (available - desired_search_width - action_cluster_width - (gap * 2.0)).max(0.0);
    let filter_side = ((available_for_filters
        - (filter_gap * (filter_chip_count.saturating_sub(1) as f32)))
        / filter_chip_count as f32)
        .floor()
        .clamp(6.0, max_filter_side);
    let filter_total_width = ((filter_side * filter_chip_count as f32)
        + (filter_gap * (filter_chip_count.saturating_sub(1) as f32)))
        .min(available);
    let marked_chip_side = filter_side.max(0.0);
    let marked_chip_width = if marked_chip_side > 0.0 {
        marked_chip_side + gap
    } else {
        0.0
    };
    let remaining_after_filters =
        (available - filter_total_width - marked_chip_width - action_cluster_width - (gap * 2.0))
            .max(0.0);
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
    let action_cluster = if action_side > 0.0 && search_field.width() > 1.0 {
        let action_max_x = (search_field.min.x - gap).max(left_min);
        Rect::from_min_max(
            Point::new(
                (action_max_x - action_cluster_width).max(filter_strip.max.x + gap),
                host.min.y,
            ),
            Point::new(action_max_x, host.max.y),
        )
    } else {
        empty
    };
    let action_slots = compute_action_slot_rects(
        clamp_rect_to_bounds(action_cluster, host),
        action_side,
        action_cluster_gap,
    );
    let marked_filter_chip = if marked_chip_side > 0.0 {
        let min_x = (filter_strip.max.x + gap).min(left_max);
        clamp_rect_to_bounds(
            Rect::from_min_max(
                Point::new(min_x, host.min.y),
                Point::new((min_x + marked_chip_side).min(left_max), host.max.y),
            ),
            host,
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
    let playback_age_filter_chips = compute_playback_age_filter_chip_rects(
        filter_strip,
        filter_side,
        filter_gap,
    );
    BrowserToolbarSections {
        rating_filter_chips,
        playback_age_filter_chips,
        marked_filter_chip,
        action_slots,
        search_field,
        activity_chip: empty,
        sort_chip: empty,
        triage_chips: empty_chips,
    }
}

fn compute_action_slot_rects(cluster: Rect, action_side: f32, gap: f32) -> [Rect; 2] {
    let empty = empty_rect(cluster);
    if cluster.width() <= 1.0 || cluster.height() <= 0.0 || action_side <= 0.0 {
        return [empty; 2];
    }
    let widths = [action_side; 2];
    let rects = layout_left_aligned_fixed_widths(cluster, gap, &widths, TOOLBAR_FILTER_ID + 30, 0);
    let mut slots = [empty; 2];
    for (index, rect) in rects.into_iter().take(2).enumerate() {
        slots[index] = center_square_rect(rect, action_side);
    }
    slots
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

fn compute_playback_age_filter_chip_rects(
    strip: Rect,
    chip_side: f32,
    gap: f32,
) -> [Rect; 3] {
    let empty = empty_rect(strip);
    if strip.width() <= 1.0 || strip.height() <= 0.0 || chip_side <= 0.0 {
        return [empty; PLAYBACK_AGE_FILTER_CHIP_COUNT];
    }
    let widths = [chip_side; PLAYBACK_AGE_FILTER_CHIP_COUNT];
    let rating_strip_width = (chip_side * RATING_FILTER_CHIP_COUNT as f32)
        + (gap * (RATING_FILTER_CHIP_COUNT.saturating_sub(1) as f32));
    let age_strip = Rect::from_min_max(
        Point::new((strip.min.x + rating_strip_width + gap).min(strip.max.x), strip.min.y),
        strip.max,
    );
    let rects = layout_left_aligned_fixed_widths(
        age_strip,
        gap,
        &widths,
        TOOLBAR_FILTER_ID + 20,
        TOOLBAR_FILTER_CHIP_BASE_ID + RATING_FILTER_CHIP_COUNT as u64,
    );
    let mut chips = [empty; PLAYBACK_AGE_FILTER_CHIP_COUNT];
    for (index, rect) in rects
        .into_iter()
        .take(PLAYBACK_AGE_FILTER_CHIP_COUNT)
        .enumerate()
    {
        chips[index] = center_square_rect(rect, chip_side);
    }
    chips
}

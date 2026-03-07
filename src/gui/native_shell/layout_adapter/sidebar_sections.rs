//! Slotized source/folder section partitioning for the sidebar rows band.

use super::super::style::SizingTokens;
use crate::gui::types::{Point, Rect};

/// Slot-resolved source/folder section rectangles inside the sidebar rows band.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SidebarRowSections {
    pub source_rows: Rect,
    pub folder_header: Rect,
    pub folder_rows: Rect,
}

/// Rendered row-count inputs used for source/folder section partitioning.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SidebarRowCounts {
    pub source_rows: usize,
    pub folder_rows: usize,
}

/// Resolved section heights used to partition `layout.sidebar_rows`.
#[derive(Clone, Copy, Debug, PartialEq)]
struct SidebarSectionHeights {
    source_rows: f32,
    section_gap: f32,
    folder_header: f32,
    folder_rows: f32,
}

/// Compute source/folder sections inside `layout.sidebar_rows`.
pub(crate) fn compute_sidebar_row_sections(
    sidebar_rows: Rect,
    sizing: SizingTokens,
    counts: SidebarRowCounts,
) -> SidebarRowSections {
    let section_bounds = inset_vertical(
        sidebar_rows,
        sizing.panel_section_padding_top,
        sizing.panel_section_padding_bottom,
    );
    let empty = Rect::from_min_max(section_bounds.max, section_bounds.max);
    if counts.folder_rows == 0 {
        return SidebarRowSections {
            source_rows: section_bounds,
            folder_header: empty,
            folder_rows: empty,
        };
    }
    let heights = resolve_section_heights(section_bounds.height(), sizing, counts);
    let source_rows = rect_from_top(section_bounds, section_bounds.min.y, heights.source_rows);
    let folder_header_top = (source_rows.max.y + heights.section_gap).min(section_bounds.max.y);
    let folder_header = rect_from_top(section_bounds, folder_header_top, heights.folder_header);
    let folder_rows = rect_from_top(section_bounds, folder_header.max.y, heights.folder_rows);
    SidebarRowSections {
        source_rows: clamp_rect_to_bounds(source_rows, section_bounds),
        folder_header: clamp_rect_to_bounds(folder_header, section_bounds),
        folder_rows: clamp_rect_to_bounds(folder_rows, section_bounds),
    }
}

fn clamp_rect_to_bounds(rect: Rect, bounds: Rect) -> Rect {
    let min = Point::new(rect.min.x.max(bounds.min.x), rect.min.y.max(bounds.min.y));
    let max = Point::new(rect.max.x.min(bounds.max.x), rect.max.y.min(bounds.max.y));
    if max.x < min.x || max.y < min.y {
        return Rect::from_min_max(bounds.min, bounds.min);
    }
    Rect::from_min_max(min, max)
}

fn inset_vertical(rect: Rect, top: f32, bottom: f32) -> Rect {
    let top = top.max(0.0);
    let bottom = bottom.max(0.0);
    let max_inset = (rect.height() * 0.5).max(0.0);
    let top = top.min(max_inset);
    let bottom = bottom.min(max_inset);
    Rect::from_min_max(
        Point::new(rect.min.x, (rect.min.y + top).min(rect.max.y)),
        Point::new(rect.max.x, (rect.max.y - bottom).max(rect.min.y)),
    )
}

fn resolve_section_heights(
    available_height: f32,
    sizing: SizingTokens,
    counts: SidebarRowCounts,
) -> SidebarSectionHeights {
    let source_demand_height = stack_height(
        counts.source_rows,
        sizing.source_row_height,
        sizing.source_row_gap,
    );
    let folder_demand_height = stack_height(
        counts.folder_rows,
        sizing.folder_row_height,
        sizing.folder_row_gap,
    );
    let source_min_rows = if counts.source_rows == 0 {
        0
    } else {
        counts
            .source_rows
            .min(sizing.source_rows_min_when_split)
            .max(1)
    };
    let folder_min_rows = counts.folder_rows.min(sizing.folder_rows_min).max(1);
    let source_min_height = stack_height(
        source_min_rows,
        sizing.source_row_height,
        sizing.source_row_gap,
    );
    let folder_min_height = stack_height(
        folder_min_rows,
        sizing.folder_row_height,
        sizing.folder_row_gap,
    );
    let header_height = sizing
        .folder_header_block_height
        .min(available_height.max(0.0));
    let mut section_gap = if counts.source_rows > 0 {
        sizing.sidebar_section_gap
    } else {
        0.0
    };
    let minimum_height = source_min_height + section_gap + header_height + folder_min_height;
    if minimum_height <= available_height {
        let remaining = available_height - minimum_height;
        let source_extra_cap = (source_demand_height - source_min_height).max(0.0);
        let folder_extra_cap = (folder_demand_height - folder_min_height).max(0.0);
        let (source_extra, folder_extra) =
            distribute_extra_height(remaining, source_extra_cap, folder_extra_cap);
        return SidebarSectionHeights {
            source_rows: source_min_height + source_extra,
            section_gap,
            folder_header: header_height,
            folder_rows: folder_min_height + folder_extra,
        };
    }

    let compact_source_height = stack_height(
        counts.source_rows.min(1),
        sizing.source_row_height,
        sizing.source_row_gap,
    );
    section_gap = if counts.source_rows > 0 {
        sizing.sidebar_section_gap.min(2.0)
    } else {
        0.0
    };
    let compact_folder_height = stack_height(
        counts.folder_rows.min(1),
        sizing.folder_row_height,
        sizing.folder_row_gap,
    );
    let compact_minimum =
        compact_source_height + section_gap + header_height + compact_folder_height;
    if compact_minimum <= available_height {
        return SidebarSectionHeights {
            source_rows: compact_source_height,
            section_gap,
            folder_header: header_height,
            folder_rows: (available_height - compact_source_height - section_gap - header_height)
                .max(0.0),
        };
    }

    SidebarSectionHeights {
        source_rows: 0.0,
        section_gap: 0.0,
        folder_header: header_height,
        folder_rows: (available_height - header_height).max(0.0),
    }
}

fn rect_from_top(bounds: Rect, top: f32, height: f32) -> Rect {
    let min_y = top.clamp(bounds.min.y, bounds.max.y);
    let max_y = (min_y + height.max(0.0)).min(bounds.max.y);
    Rect::from_min_max(
        Point::new(bounds.min.x, min_y),
        Point::new(bounds.max.x, max_y),
    )
}

fn distribute_extra_height(
    remaining_height: f32,
    source_extra_cap: f32,
    folder_extra_cap: f32,
) -> (f32, f32) {
    let cap_sum = source_extra_cap + folder_extra_cap;
    if cap_sum <= 0.0 || remaining_height <= 0.0 {
        return (0.0, 0.0);
    }
    let source_share = (remaining_height * (source_extra_cap / cap_sum)).min(source_extra_cap);
    let folder_share = (remaining_height - source_share).min(folder_extra_cap);
    let source_extra = source_share + ((remaining_height - source_share - folder_share).max(0.0));
    (source_extra.min(source_extra_cap), folder_share)
}

fn stack_height(rows: usize, row_height: f32, gap: f32) -> f32 {
    if rows == 0 {
        return 0.0;
    }
    (rows as f32 * row_height.max(0.0)) + ((rows.saturating_sub(1)) as f32 * gap.max(0.0))
}

//! Slotized source/folder section partitioning for the sidebar rows band.

use super::super::style::SizingTokens;
use crate::gui::types::{Point, Rect};

/// Slot-resolved rectangles for one fixed folder pane inside the sidebar band.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SidebarFolderPaneSections {
    pub header: Rect,
    pub rows: Rect,
}

/// Slot-resolved source/folder section rectangles inside the sidebar rows band.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SidebarRowSections {
    pub source_rows: Rect,
    pub upper_folder_pane: SidebarFolderPaneSections,
    pub lower_folder_pane: SidebarFolderPaneSections,
}

/// Rendered row-count inputs used for source/folder section partitioning.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SidebarRowCounts {
    pub source_rows: usize,
    pub upper_folder_rows: usize,
    pub lower_folder_rows: usize,
}

/// Resolved section heights used to partition `layout.sidebar_rows`.
#[derive(Clone, Copy, Debug, PartialEq)]
struct SidebarSectionHeights {
    source_rows: f32,
    source_gap: f32,
    upper_header: f32,
    upper_rows: f32,
    pane_gap: f32,
    lower_header: f32,
    lower_rows: f32,
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
    let heights = resolve_section_heights(section_bounds.height(), sizing, counts);
    let source_rows = rect_from_top(section_bounds, section_bounds.min.y, heights.source_rows);
    let upper_header_top = (source_rows.max.y + heights.source_gap).min(section_bounds.max.y);
    let upper_header = rect_from_top(section_bounds, upper_header_top, heights.upper_header);
    let upper_rows = rect_from_top(section_bounds, upper_header.max.y, heights.upper_rows);
    let lower_header_top = (upper_rows.max.y + heights.pane_gap).min(section_bounds.max.y);
    let lower_header = rect_from_top(section_bounds, lower_header_top, heights.lower_header);
    let lower_rows = rect_from_top(section_bounds, lower_header.max.y, heights.lower_rows);
    SidebarRowSections {
        source_rows: clamp_rect_to_bounds(source_rows, section_bounds),
        upper_folder_pane: SidebarFolderPaneSections {
            header: clamp_rect_to_bounds(upper_header, section_bounds),
            rows: clamp_rect_to_bounds(upper_rows, section_bounds),
        },
        lower_folder_pane: SidebarFolderPaneSections {
            header: clamp_rect_to_bounds(lower_header, section_bounds),
            rows: clamp_rect_to_bounds(lower_rows, section_bounds),
        },
    }
    .with_empty_fallback(empty)
}

impl SidebarRowSections {
    fn with_empty_fallback(mut self, empty: Rect) -> Self {
        if self.upper_folder_pane.header.height() <= 0.0 {
            self.upper_folder_pane.header = empty;
            self.upper_folder_pane.rows = empty;
        }
        if self.lower_folder_pane.header.height() <= 0.0 {
            self.lower_folder_pane.header = empty;
            self.lower_folder_pane.rows = empty;
        }
        self
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
    let source_min_rows = if counts.source_rows == 0 {
        0
    } else {
        counts
            .source_rows
            .min(sizing.source_rows_min_when_split)
            .max(1)
    };
    let source_min_height = stack_height(
        source_min_rows,
        sizing.source_row_height,
        sizing.source_row_gap,
    );
    let header_height = sizing
        .folder_header_block_height
        .min(available_height.max(0.0));
    let source_gap = if counts.source_rows > 0 {
        sizing.sidebar_section_gap
    } else {
        0.0
    };
    let pane_gap = sizing.sidebar_section_gap.min(6.0);
    let upper_row_min = compact_folder_rows_height(sizing);
    let lower_row_min = compact_folder_rows_height(sizing);
    let reserved_folder_height =
        header_height + upper_row_min + pane_gap + header_height + lower_row_min;

    let (source_rows, source_gap) = if source_min_height + source_gap + reserved_folder_height
        <= available_height
    {
        let max_source_height = (available_height - source_gap - reserved_folder_height).max(0.0);
        (source_demand_height.min(max_source_height), source_gap)
    } else if reserved_folder_height <= available_height {
        (0.0, 0.0)
    } else {
        (0.0, 0.0)
    };

    let rows_height_total =
        (available_height - source_rows - source_gap - header_height - pane_gap - header_height)
            .max(0.0);
    let (upper_rows, lower_rows) = distribute_folder_rows_height(
        rows_height_total,
        counts.upper_folder_rows,
        counts.lower_folder_rows,
        sizing,
    );

    SidebarSectionHeights {
        source_rows,
        source_gap,
        upper_header: header_height,
        upper_rows,
        pane_gap,
        lower_header: header_height,
        lower_rows,
    }
}

fn compact_folder_rows_height(sizing: SizingTokens) -> f32 {
    stack_height(1, sizing.folder_row_height, sizing.folder_row_gap)
}

fn distribute_folder_rows_height(
    total_height: f32,
    upper_rows: usize,
    lower_rows: usize,
    sizing: SizingTokens,
) -> (f32, f32) {
    if total_height <= 0.0 {
        return (0.0, 0.0);
    }
    let upper_min = compact_folder_rows_height(sizing);
    let lower_min = compact_folder_rows_height(sizing);
    if total_height <= upper_min + lower_min {
        let split = total_height * 0.5;
        return (split, total_height - split);
    }
    let upper_demand = stack_height(
        upper_rows.max(1),
        sizing.folder_row_height,
        sizing.folder_row_gap,
    );
    let lower_demand = stack_height(
        lower_rows.max(1),
        sizing.folder_row_height,
        sizing.folder_row_gap,
    );
    let remaining = total_height - upper_min - lower_min;
    let upper_extra_cap = (upper_demand - upper_min).max(0.0);
    let lower_extra_cap = (lower_demand - lower_min).max(0.0);
    let cap_sum = upper_extra_cap + lower_extra_cap;
    if cap_sum <= 0.0 {
        return (upper_min + (remaining * 0.5), lower_min + (remaining * 0.5));
    }
    let upper_extra = (remaining * (upper_extra_cap / cap_sum)).min(upper_extra_cap);
    let lower_extra = (remaining - upper_extra).min(lower_extra_cap);
    let spill = (remaining - upper_extra - lower_extra).max(0.0);
    (
        upper_min + upper_extra + (spill * 0.5),
        lower_min + lower_extra + (spill * 0.5),
    )
}

fn rect_from_top(bounds: Rect, top: f32, height: f32) -> Rect {
    let min_y = top.clamp(bounds.min.y, bounds.max.y);
    let max_y = (min_y + height.max(0.0)).min(bounds.max.y);
    Rect::from_min_max(
        Point::new(bounds.min.x, min_y),
        Point::new(bounds.max.x, max_y),
    )
}

fn stack_height(rows: usize, row_height: f32, gap: f32) -> f32 {
    if rows == 0 {
        return 0.0;
    }
    (rows as f32 * row_height.max(0.0)) + ((rows.saturating_sub(1)) as f32 * gap.max(0.0))
}

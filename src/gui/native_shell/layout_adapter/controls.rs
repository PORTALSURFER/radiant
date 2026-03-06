//! Slotized helpers for native-shell action-button rows and toolbar partitions.

use super::super::style::SizingTokens;
use crate::gui::layout_core::{
    Constraints, ContainerKind, ContainerPolicy, CrossAlign, Insets, LayoutNode, MainAlign,
    OverflowPolicy, SizeModeCross, SizeModeMain, SlotChild, SlotParams, layout_tree,
};
use crate::gui::types::{Point, Rect, Vector2};

const UPDATE_BUTTON_ROW_ID: u64 = 700;
const UPDATE_BUTTON_SPACER_ID: u64 = 701;
const UPDATE_BUTTON_BASE_ID: u64 = 710;
const SIDEBAR_BUTTON_ROW_ID: u64 = 770;
const SIDEBAR_BUTTON_SPACER_ID: u64 = 771;
const SIDEBAR_BUTTON_BASE_ID: u64 = 780;
const TOOLBAR_SECTION_ROW_ID: u64 = 800;
const TOOLBAR_SEARCH_ID: u64 = 801;

/// Slot-resolved browser toolbar sections for search and chip controls.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BrowserToolbarSections {
    pub search_field: Rect,
    pub activity_chip: Rect,
    pub sort_chip: Rect,
    pub triage_chips: [Rect; 3],
}

/// Compute top-bar update action button rects aligned to the right cluster.
pub(crate) fn compute_update_action_button_rects(
    row: Rect,
    action_cluster: Rect,
    sizing: SizingTokens,
    labels: &[&str],
) -> Vec<Rect> {
    if labels.is_empty() || row.width() <= 0.0 || row.height() <= 0.0 {
        return Vec::new();
    }
    let gap = sizing.action_button_gap.max(1.0);
    let button_height = (row.height() - (sizing.text_inset_y * 0.4))
        .max(12.0)
        .min(row.height());
    let widths: Vec<f32> = labels
        .iter()
        .map(|label| {
            ((*label).chars().count() as f32 * (sizing.font_meta * 0.62)
                + (sizing.text_inset_x * 2.0))
                .clamp(42.0, 84.0)
        })
        .collect();
    let available_width =
        ((action_cluster.max.x - sizing.text_inset_x) - action_cluster.min.x).max(0.0);
    let visible_widths = visible_suffix_widths(&widths, available_width, gap);
    if visible_widths.is_empty() {
        return Vec::new();
    }
    let y = row.min.y + ((row.height() - button_height) * 0.5);
    let bounds = Rect::from_min_max(
        Point::new(action_cluster.min.x, y),
        Point::new(
            (action_cluster.max.x - sizing.text_inset_x).max(action_cluster.min.x),
            (y + button_height).min(row.max.y),
        ),
    );
    layout_right_aligned_fixed_widths(
        bounds,
        gap,
        &visible_widths,
        UPDATE_BUTTON_ROW_ID,
        UPDATE_BUTTON_SPACER_ID,
        UPDATE_BUTTON_BASE_ID,
    )
}

/// Compute sidebar footer action button rects aligned to the right edge.
pub(crate) fn compute_sidebar_action_button_rects(
    footer: Rect,
    sizing: SizingTokens,
    button_count: usize,
) -> Vec<Rect> {
    if button_count == 0 || footer.width() <= 0.0 || footer.height() <= 0.0 {
        return Vec::new();
    }
    let gap = sizing.sidebar_action_button_gap;
    let available_width = (footer.width() - (sizing.text_inset_x * 2.0)).max(0.0);
    let button_count_f32 = button_count as f32;
    let button_width = if button_count_f32 > 0.0 {
        ((available_width - (gap * (button_count_f32 - 1.0)).max(0.0)).max(0.0) / button_count_f32)
            .min(sizing.sidebar_action_button_width)
    } else {
        sizing.sidebar_action_button_width
    };
    let button_height = sizing
        .sidebar_action_button_height
        .min((footer.height() - 1.0).max(1.0));
    let y_min = footer.min.y + 1.0;
    let y_max = (footer.max.y - button_height).max(y_min);
    let y = (footer.max.y - button_height - sizing.text_inset_y)
        .max(y_min)
        .min(y_max);
    let bounds = Rect::from_min_max(
        Point::new(footer.min.x + sizing.text_inset_x, y),
        Point::new(
            (footer.max.x - sizing.text_inset_x).max(footer.min.x),
            (y + button_height).min(footer.max.y),
        ),
    );
    let widths = vec![button_width; button_count];
    layout_right_aligned_fixed_widths(
        bounds,
        gap,
        &widths,
        SIDEBAR_BUTTON_ROW_ID,
        SIDEBAR_BUTTON_SPACER_ID,
        SIDEBAR_BUTTON_BASE_ID,
    )
    .into_iter()
    .map(|rect| clamp_rect_right_edge(rect, footer, footer.max.x - 1.0))
    .collect()
}

/// Compute browser toolbar search/activity/sort partitions from slot rows.
pub(crate) fn compute_browser_toolbar_sections(
    toolbar: Rect,
    sizing: SizingTokens,
    action_cluster_left: Option<f32>,
) -> BrowserToolbarSections {
    let empty = empty_rect(toolbar);
    let empty_chips = [empty; 3];
    if toolbar.width() <= 0.0 || toolbar.height() <= 0.0 {
        return BrowserToolbarSections {
            search_field: empty,
            activity_chip: empty,
            sort_chip: empty,
            triage_chips: empty_chips,
        };
    }
    let gap = sizing.action_button_gap.max(1.0);
    let left_min = toolbar.min.x + sizing.text_inset_x;
    let action_left = action_cluster_left.unwrap_or(toolbar.max.x - sizing.text_inset_x);
    let left_max = (action_left - gap).max(left_min);
    let available = (left_max - left_min).max(0.0);
    if available <= 1.0 {
        return BrowserToolbarSections {
            search_field: empty,
            activity_chip: empty,
            sort_chip: empty,
            triage_chips: empty_chips,
        };
    }
    let bounds = Rect::from_min_max(
        Point::new(left_min, toolbar.min.y),
        Point::new(left_max, toolbar.max.y),
    );
    let rects = layout_left_aligned_fixed_widths(
        bounds,
        gap,
        &[available],
        TOOLBAR_SECTION_ROW_ID,
        TOOLBAR_SEARCH_ID,
    );
    let search_field = rects.first().copied().unwrap_or(empty);
    BrowserToolbarSections {
        search_field,
        activity_chip: empty,
        sort_chip: empty,
        triage_chips: empty_chips,
    }
}

fn visible_suffix_widths(widths: &[f32], available_width: f32, gap: f32) -> Vec<f32> {
    if available_width <= 0.0 || widths.is_empty() {
        return Vec::new();
    }
    let mut used = 0.0;
    let mut reversed = Vec::new();
    for (index, width) in widths.iter().rev().enumerate() {
        let candidate = used + width + if index > 0 { gap } else { 0.0 };
        if candidate >= available_width {
            break;
        }
        reversed.push(*width);
        used = candidate;
    }
    reversed.reverse();
    reversed
}

fn layout_right_aligned_fixed_widths(
    bounds: Rect,
    gap: f32,
    widths: &[f32],
    row_id: u64,
    spacer_id: u64,
    first_button_id: u64,
) -> Vec<Rect> {
    if widths.is_empty() || bounds.width() <= 0.0 || bounds.height() <= 0.0 {
        return Vec::new();
    }
    let mut children = Vec::with_capacity(widths.len() + 1);
    children.push(SlotChild {
        slot: SlotParams::fill(),
        child: LayoutNode::widget(spacer_id, Vector2::new(1.0, 1.0)),
    });
    for (index, width) in widths.iter().enumerate() {
        children.push(fixed_width_child(
            first_button_id + index as u64,
            *width,
            if index == 0 { 0.0 } else { gap },
        ));
    }
    let tree = LayoutNode::container(
        row_id,
        ContainerPolicy {
            kind: ContainerKind::Row,
            spacing: 0.0,
            align_main: MainAlign::Start,
            align_cross: CrossAlign::Stretch,
            overflow: OverflowPolicy::Clip,
            ..ContainerPolicy::default()
        },
        children,
    );
    let output = layout_tree(&tree, bounds);
    widths
        .iter()
        .enumerate()
        .map(|(index, _)| {
            let id = first_button_id + index as u64;
            let rect = rect_for(&output.rects, id, empty_rect(bounds));
            clamp_rect_to_bounds(rect, bounds)
        })
        .collect()
}

fn layout_left_aligned_fixed_widths(
    bounds: Rect,
    gap: f32,
    widths: &[f32],
    row_id: u64,
    first_button_id: u64,
) -> Vec<Rect> {
    if widths.is_empty() || bounds.width() <= 0.0 || bounds.height() <= 0.0 {
        return Vec::new();
    }
    let mut children = Vec::with_capacity(widths.len());
    for (index, width) in widths.iter().enumerate() {
        children.push(fixed_width_child(
            first_button_id + index as u64,
            *width,
            if index == 0 { 0.0 } else { gap },
        ));
    }
    let tree = LayoutNode::container(
        row_id,
        ContainerPolicy {
            kind: ContainerKind::Row,
            spacing: 0.0,
            align_main: MainAlign::Start,
            align_cross: CrossAlign::Stretch,
            overflow: OverflowPolicy::Clip,
            ..ContainerPolicy::default()
        },
        children,
    );
    let output = layout_tree(&tree, bounds);
    widths
        .iter()
        .enumerate()
        .map(|(index, _)| {
            let id = first_button_id + index as u64;
            let rect = rect_for(&output.rects, id, empty_rect(bounds));
            clamp_rect_to_bounds(rect, bounds)
        })
        .collect()
}

fn fixed_width_child(node_id: u64, width: f32, left_margin: f32) -> SlotChild {
    SlotChild {
        slot: SlotParams {
            size_main: SizeModeMain::Fixed(width.max(0.0)),
            size_cross: SizeModeCross::Fill,
            constraints: Constraints::new(width.max(0.0), width.max(0.0), 0.0, f32::INFINITY),
            margin: Insets {
                left: left_margin.max(0.0),
                ..Insets::default()
            },
            align_cross_override: None,
            allow_fixed_compress: false,
        },
        child: LayoutNode::widget(node_id, Vector2::new(width.max(1.0), 1.0)),
    }
}

fn rect_for(rects: &std::collections::BTreeMap<u64, Rect>, id: u64, fallback: Rect) -> Rect {
    rects.get(&id).copied().unwrap_or(fallback)
}

fn clamp_rect_to_bounds(rect: Rect, bounds: Rect) -> Rect {
    let min = Point::new(rect.min.x.max(bounds.min.x), rect.min.y.max(bounds.min.y));
    let max = Point::new(rect.max.x.min(bounds.max.x), rect.max.y.min(bounds.max.y));
    if max.x < min.x || max.y < min.y {
        return Rect::from_min_max(bounds.min, bounds.min);
    }
    Rect::from_min_max(min, max)
}

fn clamp_rect_right_edge(rect: Rect, bounds: Rect, right_edge: f32) -> Rect {
    let clamped = clamp_rect_to_bounds(rect, bounds);
    let max_x = clamped.max.x.min(right_edge.max(bounds.min.x));
    if max_x < clamped.min.x {
        return Rect::from_min_max(bounds.min, bounds.min);
    }
    Rect::from_min_max(clamped.min, Point::new(max_x, clamped.max.y))
}

fn empty_rect(bounds: Rect) -> Rect {
    Rect::from_min_max(bounds.min, bounds.min)
}

#[cfg(test)]
#[path = "controls_tests.rs"]
mod controls_tests;

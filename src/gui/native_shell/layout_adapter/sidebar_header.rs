//! Slotized sidebar folder-header text/badge/divider micro-layout helpers.

use super::super::style::SizingTokens;
use crate::gui::layout_core::{
    Constraints, ContainerKind, ContainerPolicy, CrossAlign, Insets, LayoutNode, MainAlign,
    OverflowPolicy, SizeModeCross, SizeModeMain, SlotChild, SlotParams, layout_tree,
};
use crate::gui::types::{Point, Rect, Vector2};

const FOLDER_HEADER_TEXT_ROOT_ID: u64 = 1000;
const FOLDER_HEADER_TEXT_COLUMN_ID: u64 = 1001;
const FOLDER_HEADER_TITLE_ID: u64 = 1002;
const FOLDER_HEADER_META_ID: u64 = 1003;
const FOLDER_HEADER_TEXT_FILL_ID: u64 = 1004;
const FOLDER_HEADER_BADGE_ALIGN_ID: u64 = 1005;
const FOLDER_HEADER_BADGE_ID: u64 = 1006;
const SOURCE_DIVIDER_ALIGN_ID: u64 = 1010;
const SOURCE_DIVIDER_ID: u64 = 1011;

/// Slot-resolved recovery badge layout inside the folder-header surface.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct RecoveryBadgeLayout {
    pub rect: Rect,
    pub label: String,
    pub active: bool,
}

/// Slot-resolved folder-header text rows and optional recovery badge.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SidebarFolderHeaderLayout {
    pub title_row: Rect,
    pub metadata_row: Option<Rect>,
    pub badge: Option<RecoveryBadgeLayout>,
}

/// Compute folder-header text rows and recovery badge through slotized helpers.
pub(crate) fn compute_sidebar_folder_header_layout(
    header_rect: Rect,
    sizing: SizingTokens,
    recovery_in_progress: bool,
    recovery_entry_count: usize,
) -> SidebarFolderHeaderLayout {
    if header_rect.width() <= 0.0 || header_rect.height() <= 0.0 {
        return SidebarFolderHeaderLayout {
            title_row: empty_rect(header_rect),
            metadata_row: None,
            badge: None,
        };
    }

    let badge = compute_recovery_badge_layout(
        header_rect,
        sizing,
        recovery_in_progress,
        recovery_entry_count,
    );
    let text_start_x = header_rect.min.x + sizing.text_inset_x + sizing.header_label_gutter;
    let text_end_x = badge
        .as_ref()
        .map(|badge| badge.rect.min.x - sizing.text_inset_x)
        .unwrap_or_else(|| header_rect.max.x - sizing.text_inset_x);
    let text_bounds = Rect::from_min_max(
        Point::new(text_start_x, header_rect.min.y),
        Point::new(text_end_x.max(text_start_x), header_rect.max.y),
    );
    if text_bounds.width() <= 0.0 {
        return SidebarFolderHeaderLayout {
            title_row: empty_rect(header_rect),
            metadata_row: None,
            badge,
        };
    }

    let show_metadata = folder_header_has_metadata_row(header_rect, sizing);
    let column_children = build_text_rows(show_metadata, sizing);
    let text_tree = LayoutNode::container(
        FOLDER_HEADER_TEXT_ROOT_ID,
        ContainerPolicy {
            kind: ContainerKind::PaddingBox,
            padding: Insets {
                top: sizing.text_inset_y.max(0.0),
                bottom: sizing.text_inset_y.max(0.0),
                ..Insets::default()
            },
            align_cross: CrossAlign::Stretch,
            ..ContainerPolicy::default()
        },
        vec![SlotChild {
            slot: SlotParams::fill(),
            child: LayoutNode::container(
                FOLDER_HEADER_TEXT_COLUMN_ID,
                ContainerPolicy {
                    kind: ContainerKind::Column,
                    spacing: sizing.text_row_gap.max(0.0),
                    align_main: MainAlign::Start,
                    align_cross: CrossAlign::Stretch,
                    overflow: OverflowPolicy::Clip,
                    ..ContainerPolicy::default()
                },
                column_children,
            ),
        }],
    );
    let output = layout_tree(&text_tree, text_bounds);
    let title_row = clamp_rect_to_bounds(
        rect_for(
            &output.rects,
            FOLDER_HEADER_TITLE_ID,
            empty_rect(text_bounds),
        ),
        text_bounds,
    );
    let metadata_row = if show_metadata {
        let row = clamp_rect_to_bounds(
            rect_for(
                &output.rects,
                FOLDER_HEADER_META_ID,
                empty_rect(text_bounds),
            ),
            text_bounds,
        );
        (row.height() > 0.0).then_some(row)
    } else {
        None
    };
    SidebarFolderHeaderLayout {
        title_row,
        metadata_row,
        badge,
    }
}

/// Compute source/folder section divider geometry through slotized alignment.
pub(crate) fn compute_source_section_divider_rect(
    source_rows: Rect,
    folder_header: Rect,
    sizing: SizingTokens,
) -> Option<Rect> {
    if folder_header.height() <= 0.0 || source_rows.width() <= 0.0 {
        return None;
    }
    let divider_height = sizing.source_section_divider_width.max(0.5);
    let gap_top = source_rows.max.y;
    let gap_bottom = folder_header.min.y.max(gap_top);
    let fallback_bottom = (folder_header.min.y + divider_height).min(folder_header.max.y);
    let align_bounds = if gap_bottom - gap_top >= divider_height {
        Rect::from_min_max(
            Point::new(source_rows.min.x, gap_top),
            Point::new(source_rows.max.x, gap_bottom),
        )
    } else {
        Rect::from_min_max(
            Point::new(source_rows.min.x, folder_header.min.y),
            Point::new(source_rows.max.x, fallback_bottom),
        )
    };
    if align_bounds.height() <= 0.0 {
        return None;
    }
    let divider_tree = LayoutNode::container(
        SOURCE_DIVIDER_ALIGN_ID,
        ContainerPolicy {
            kind: ContainerKind::AlignBox,
            align_main: MainAlign::Center,
            align_cross: CrossAlign::Stretch,
            overflow: OverflowPolicy::Clip,
            ..ContainerPolicy::default()
        },
        vec![SlotChild {
            slot: SlotParams {
                size_main: SizeModeMain::Fixed(divider_height),
                size_cross: SizeModeCross::Fill,
                constraints: Constraints::new(0.0, f32::INFINITY, divider_height, divider_height),
                margin: Insets::default(),
                align_cross_override: Some(CrossAlign::Stretch),
                allow_fixed_compress: false,
            },
            child: LayoutNode::widget(
                SOURCE_DIVIDER_ID,
                Vector2::new(source_rows.width().max(1.0), divider_height.max(1.0)),
            ),
        }],
    );
    let output = layout_tree(&divider_tree, align_bounds);
    let rect = clamp_rect_to_bounds(
        rect_for(&output.rects, SOURCE_DIVIDER_ID, empty_rect(align_bounds)),
        Rect::from_min_max(
            Point::new(source_rows.min.x, source_rows.min.y),
            Point::new(source_rows.max.x, folder_header.max.y),
        ),
    );
    (rect.height() > 0.0).then_some(rect)
}

fn compute_recovery_badge_layout(
    header_rect: Rect,
    sizing: SizingTokens,
    recovery_in_progress: bool,
    recovery_entry_count: usize,
) -> Option<RecoveryBadgeLayout> {
    if !recovery_in_progress && recovery_entry_count == 0 {
        return None;
    }
    let available_width = (header_rect.width() - (sizing.text_inset_x * 2.0)).max(0.0);
    if available_width < 12.0 {
        return None;
    }
    let label = compact_recovery_badge_label(
        recovery_in_progress,
        recovery_entry_count,
        available_width,
        sizing,
    )?;
    let approx_char_width = (sizing.font_meta * 0.56).max(1.0);
    let label_width = label.chars().count() as f32 * approx_char_width;
    let badge_width = (label_width + (sizing.recovery_badge_padding_x * 2.0))
        .max(sizing.recovery_badge_min_width.min(available_width))
        .min(available_width);
    let badge_height = sizing
        .recovery_badge_height
        .min((header_rect.height() - 2.0).max(10.0));
    let badge_bounds = inset_horizontal(header_rect, sizing.text_inset_x.max(0.0));
    let badge_tree = LayoutNode::container(
        FOLDER_HEADER_BADGE_ALIGN_ID,
        ContainerPolicy {
            kind: ContainerKind::AlignBox,
            align_main: MainAlign::Center,
            align_cross: CrossAlign::End,
            overflow: OverflowPolicy::Clip,
            ..ContainerPolicy::default()
        },
        vec![SlotChild {
            slot: SlotParams {
                size_main: SizeModeMain::Fixed(badge_height),
                size_cross: SizeModeCross::Fixed(badge_width),
                constraints: Constraints::new(badge_width, badge_width, badge_height, badge_height),
                margin: Insets::default(),
                align_cross_override: Some(CrossAlign::End),
                allow_fixed_compress: false,
            },
            child: LayoutNode::widget(
                FOLDER_HEADER_BADGE_ID,
                Vector2::new(badge_width.max(1.0), badge_height.max(1.0)),
            ),
        }],
    );
    let output = layout_tree(&badge_tree, badge_bounds);
    let rect = clamp_rect_to_bounds(
        rect_for(
            &output.rects,
            FOLDER_HEADER_BADGE_ID,
            empty_rect(badge_bounds),
        ),
        header_rect,
    );
    (rect.height() > 0.0).then_some(RecoveryBadgeLayout {
        rect,
        label,
        active: recovery_in_progress,
    })
}

fn compact_recovery_badge_label(
    recovery_in_progress: bool,
    recovery_entry_count: usize,
    available_width: f32,
    sizing: SizingTokens,
) -> Option<String> {
    let approx_char_width = (sizing.font_meta * 0.56).max(1.0);
    let wide_label_fits = |label: &str| {
        (label.chars().count() as f32 * approx_char_width) + (sizing.recovery_badge_padding_x * 2.0)
            <= available_width
    };
    if recovery_in_progress {
        return ["Recovery", "Active", "R"]
            .iter()
            .find(|label| wide_label_fits(label))
            .map(|label| (*label).to_string());
    }
    let long_label = format!("{recovery_entry_count} entries");
    if wide_label_fits(&long_label) {
        return Some(long_label);
    }
    let short_label = recovery_entry_count.to_string();
    Some(short_label)
}

fn folder_header_has_metadata_row(header_rect: Rect, sizing: SizingTokens) -> bool {
    let required_height =
        (sizing.text_inset_y * 2.0) + sizing.font_header + sizing.text_row_gap + sizing.font_meta;
    header_rect.height() >= required_height
}

fn build_text_rows(show_metadata: bool, sizing: SizingTokens) -> Vec<SlotChild> {
    let mut rows = Vec::with_capacity(if show_metadata { 3 } else { 2 });
    rows.push(fixed_height_child(
        FOLDER_HEADER_TITLE_ID,
        sizing.font_header.max(1.0),
    ));
    if show_metadata {
        rows.push(fixed_height_child(
            FOLDER_HEADER_META_ID,
            sizing.font_meta.max(1.0),
        ));
    }
    rows.push(SlotChild {
        slot: SlotParams::fill(),
        child: LayoutNode::widget(FOLDER_HEADER_TEXT_FILL_ID, Vector2::new(1.0, 1.0)),
    });
    rows
}

fn fixed_height_child(node_id: u64, height: f32) -> SlotChild {
    SlotChild {
        slot: SlotParams {
            size_main: SizeModeMain::Fixed(height),
            size_cross: SizeModeCross::Fill,
            constraints: Constraints::new(0.0, f32::INFINITY, height, height),
            margin: Insets::default(),
            align_cross_override: Some(CrossAlign::Stretch),
            allow_fixed_compress: false,
        },
        child: LayoutNode::widget(node_id, Vector2::new(1.0, height.max(1.0))),
    }
}

fn inset_horizontal(rect: Rect, inset: f32) -> Rect {
    let inset = inset.max(0.0).min((rect.width() * 0.5).max(0.0));
    Rect::from_min_max(
        Point::new(rect.min.x + inset, rect.min.y),
        Point::new(rect.max.x - inset, rect.max.y),
    )
}

fn clamp_rect_to_bounds(rect: Rect, bounds: Rect) -> Rect {
    let min = Point::new(rect.min.x.max(bounds.min.x), rect.min.y.max(bounds.min.y));
    let max = Point::new(rect.max.x.min(bounds.max.x), rect.max.y.min(bounds.max.y));
    if max.x < min.x || max.y < min.y {
        return Rect::from_min_max(bounds.min, bounds.min);
    }
    Rect::from_min_max(min, max)
}

fn rect_for(rects: &std::collections::BTreeMap<u64, Rect>, id: u64, fallback: Rect) -> Rect {
    rects.get(&id).copied().unwrap_or(fallback)
}

fn empty_rect(bounds: Rect) -> Rect {
    Rect::from_min_max(bounds.min, bounds.min)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::native_shell::style::StyleTokens;

    #[test]
    fn folder_recovery_badge_compacts_label_when_header_is_narrow() {
        let style = StyleTokens::for_viewport_width(820.0);
        let header_rect = Rect::from_min_max(
            Point::new(0.0, 0.0),
            Point::new(58.0, style.sizing.folder_header_block_height),
        );
        let layout = compute_sidebar_folder_header_layout(header_rect, style.sizing, false, 153);
        let badge = layout.badge.expect("badge should still render");
        assert!(badge.label.chars().count() <= 3);
        assert!(badge.rect.min.x >= header_rect.min.x);
        assert!(badge.rect.max.x <= header_rect.max.x);
    }

    #[test]
    fn folder_header_text_rows_do_not_overlap_recovery_badge() {
        let style = StyleTokens::for_viewport_width(820.0);
        let header_rect = Rect::from_min_max(
            Point::new(24.0, 40.0),
            Point::new(120.0, 40.0 + style.sizing.folder_header_block_height),
        );
        let layout = compute_sidebar_folder_header_layout(header_rect, style.sizing, true, 0);
        let badge = layout
            .badge
            .expect("badge should render for active recovery");
        assert!(layout.title_row.max.x <= badge.rect.min.x);
        if let Some(meta) = layout.metadata_row {
            assert!(meta.max.x <= badge.rect.min.x);
        }
    }

    #[test]
    fn source_divider_stays_between_sections_when_space_is_tight() {
        let style = StyleTokens::for_viewport_width(820.0);
        let source_rows = Rect::from_min_max(Point::new(12.0, 80.0), Point::new(220.0, 220.0));
        let folder_header = Rect::from_min_max(Point::new(12.0, 224.0), Point::new(220.0, 252.0));
        let divider = compute_source_section_divider_rect(source_rows, folder_header, style.sizing)
            .expect("divider should exist");
        assert!(divider.min.x >= source_rows.min.x);
        assert!(divider.max.x <= source_rows.max.x);
        assert!(divider.min.y >= source_rows.max.y);
        assert!(divider.max.y <= folder_header.max.y);
    }
}

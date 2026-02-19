//! Slotized browser chrome text-line geometry helpers.

use super::super::style::SizingTokens;
use crate::gui::layout_core::{
    Constraints, ContainerKind, ContainerPolicy, CrossAlign, Insets, LayoutNode, MainAlign,
    OverflowPolicy, SizeModeCross, SizeModeMain, SlotChild, SlotParams, layout_tree,
};
use crate::gui::types::{Point, Rect, Vector2};

const TABS_TEXT_SAMPLE_ID: u64 = 1500;
const TABS_TEXT_MAP_ID: u64 = 1501;
const TOOLBAR_TEXT_SEARCH_ID: u64 = 1510;
const TOOLBAR_TEXT_ACTIVITY_ID: u64 = 1511;
const TOOLBAR_TEXT_SORT_ID: u64 = 1512;
const FOOTER_TEXT_SUMMARY_ID: u64 = 1520;
const TEXT_LINE_ROOT_ID: u64 = 1530;
const TEXT_LINE_ALIGN_ID: u64 = 1531;

/// Slot-resolved browser-tab label bounds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BrowserTabsTextLayout {
    pub samples_label: Rect,
    pub map_label: Rect,
}

/// Slot-resolved browser-toolbar chip and field label bounds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BrowserToolbarTextLayout {
    pub search_label: Rect,
    pub activity_label: Rect,
    pub sort_label: Rect,
}

/// Compute browser tab label bounds through strict slotized text-line layout.
pub(crate) fn compute_browser_tabs_text_layout(
    samples_tab: Rect,
    map_tab: Rect,
    sizing: SizingTokens,
) -> BrowserTabsTextLayout {
    BrowserTabsTextLayout {
        samples_label: compute_text_line_rect(
            samples_tab,
            sizing,
            sizing.font_header,
            TABS_TEXT_SAMPLE_ID,
        ),
        map_label: compute_text_line_rect(map_tab, sizing, sizing.font_header, TABS_TEXT_MAP_ID),
    }
}

/// Compute browser toolbar search/activity/sort label bounds.
pub(crate) fn compute_browser_toolbar_text_layout(
    search_field: Rect,
    activity_chip: Rect,
    sort_chip: Rect,
    sizing: SizingTokens,
) -> BrowserToolbarTextLayout {
    BrowserToolbarTextLayout {
        search_label: compute_text_line_rect(
            search_field,
            sizing,
            sizing.font_meta,
            TOOLBAR_TEXT_SEARCH_ID,
        ),
        activity_label: compute_text_line_rect(
            activity_chip,
            sizing,
            sizing.font_meta,
            TOOLBAR_TEXT_ACTIVITY_ID,
        ),
        sort_label: compute_text_line_rect(
            sort_chip,
            sizing,
            sizing.font_meta,
            TOOLBAR_TEXT_SORT_ID,
        ),
    }
}

/// Compute browser footer summary label bounds.
pub(crate) fn compute_browser_footer_text_rect(footer: Rect, sizing: SizingTokens) -> Rect {
    compute_text_line_rect(footer, sizing, sizing.font_meta, FOOTER_TEXT_SUMMARY_ID)
}

fn compute_text_line_rect(rect: Rect, sizing: SizingTokens, font_size: f32, node_id: u64) -> Rect {
    let empty = empty_rect(rect);
    if rect.width() <= 0.0 || rect.height() <= 0.0 || font_size <= 0.0 {
        return empty;
    }
    let tree = LayoutNode::container(
        TEXT_LINE_ROOT_ID + node_id,
        ContainerPolicy {
            kind: ContainerKind::PaddingBox,
            padding: Insets {
                left: sizing.text_inset_x.max(0.0),
                right: sizing.text_inset_x.max(0.0),
                top: sizing.text_inset_y.max(0.0),
                bottom: sizing.text_inset_y.max(0.0),
            },
            align_cross: CrossAlign::Stretch,
            overflow: OverflowPolicy::Clip,
            ..ContainerPolicy::default()
        },
        vec![SlotChild {
            slot: SlotParams::fill(),
            child: LayoutNode::container(
                TEXT_LINE_ALIGN_ID + node_id,
                ContainerPolicy {
                    kind: ContainerKind::AlignBox,
                    align_main: MainAlign::Center,
                    align_cross: CrossAlign::Stretch,
                    overflow: OverflowPolicy::Clip,
                    ..ContainerPolicy::default()
                },
                vec![SlotChild {
                    slot: SlotParams {
                        size_main: SizeModeMain::Fixed(font_size.max(1.0)),
                        size_cross: SizeModeCross::Fill,
                        constraints: Constraints::new(
                            0.0,
                            f32::INFINITY,
                            font_size.max(1.0),
                            font_size.max(1.0),
                        ),
                        margin: Insets::default(),
                        align_cross_override: Some(CrossAlign::Stretch),
                        allow_fixed_compress: false,
                    },
                    child: LayoutNode::widget(node_id, Vector2::new(1.0, font_size.max(1.0))),
                }],
            ),
        }],
    );
    let output = layout_tree(&tree, rect);
    clamp_rect_to_bounds(rect_for(&output.rects, node_id, empty), rect)
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

    fn assert_inside(outer: Rect, inner: Rect) {
        assert!(inner.min.x >= outer.min.x);
        assert!(inner.min.y >= outer.min.y);
        assert!(inner.max.x <= outer.max.x);
        assert!(inner.max.y <= outer.max.y);
    }

    #[test]
    fn tabs_text_layout_stays_within_each_tab() {
        let style = StyleTokens::for_viewport_width(1280.0);
        let samples = Rect::from_min_max(Point::new(220.0, 292.0), Point::new(720.0, 320.0));
        let map = Rect::from_min_max(Point::new(724.0, 292.0), Point::new(1220.0, 320.0));
        let layout = compute_browser_tabs_text_layout(samples, map, style.sizing);
        assert_inside(samples, layout.samples_label);
        assert_inside(map, layout.map_label);
    }

    #[test]
    fn toolbar_text_layout_stays_within_toolbar_sections() {
        let style = StyleTokens::for_viewport_width(1280.0);
        let search = Rect::from_min_max(Point::new(220.0, 326.0), Point::new(760.0, 350.0));
        let activity = Rect::from_min_max(Point::new(768.0, 326.0), Point::new(920.0, 350.0));
        let sort = Rect::from_min_max(Point::new(928.0, 326.0), Point::new(1080.0, 350.0));
        let layout = compute_browser_toolbar_text_layout(search, activity, sort, style.sizing);
        assert_inside(search, layout.search_label);
        assert_inside(activity, layout.activity_label);
        assert_inside(sort, layout.sort_label);
    }

    #[test]
    fn footer_text_layout_stays_inside_footer_band() {
        let style = StyleTokens::for_viewport_width(1280.0);
        let footer = Rect::from_min_max(Point::new(220.0, 722.0), Point::new(1220.0, 748.0));
        let line = compute_browser_footer_text_rect(footer, style.sizing);
        assert_inside(footer, line);
    }

    #[test]
    fn toolbar_text_layout_collapses_for_empty_chip() {
        let style = StyleTokens::for_viewport_width(1280.0);
        let search = Rect::from_min_max(Point::new(220.0, 326.0), Point::new(760.0, 350.0));
        let empty = Rect::from_min_max(Point::new(768.0, 326.0), Point::new(768.0, 326.0));
        let layout = compute_browser_toolbar_text_layout(search, empty, empty, style.sizing);
        assert_eq!(layout.activity_label, empty);
        assert_eq!(layout.sort_label, empty);
    }
}

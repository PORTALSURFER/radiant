//! Adapter that maps native shell section geometry onto the slot-based layout core.
mod bands;
mod browser_chrome_text;
mod browser_tabs;
mod browser_text;
mod control_text;
mod controls;
mod map_canvas;
mod map_header;
mod overlay_visuals;
mod overlays;
mod row_hit_test;
mod sidebar_bands;
mod sidebar_chrome_text;
mod sidebar_header;
mod sidebar_sections;
mod sidebar_text;
mod status_bar;
mod top_controls;
mod top_title_text;
mod update_text;
mod waveform_annotations;
mod waveform_header;
use super::style::StyleTokens;
use crate::gui::layout_core::{
    Constraints, ContainerKind, ContainerPolicy, CrossAlign, Insets, LayoutDebugOptions,
    LayoutEngine, LayoutNode, LayoutState, MainAlign, OverflowPolicy, SizeModeCross, SizeModeMain,
    SlotChild, SlotParams,
};
use crate::gui::types::{Point, Rect, Vector2};
pub(super) use bands::compute_top_bar_band_sections;
pub(crate) use bands::{
    BROWSER_BANDS_ROOT_ID, BrowserBandSections, build_browser_bands_tree,
    compute_browser_band_sections_with_layout_engine,
};
pub(super) use browser_chrome_text::{
    compute_browser_footer_text_rect, compute_browser_tabs_text_layout,
    compute_browser_toolbar_text_layout,
};
pub(super) use browser_tabs::{BrowserTabsRects, compute_browser_tabs_rects};
pub(super) use browser_text::{
    compute_browser_header_text_layout, compute_browser_row_text_layout,
};
pub(super) use control_text::{
    compute_action_button_text_rect, compute_top_bar_controls_text_layout,
};
pub(super) use controls::{
    compute_browser_action_button_rects, compute_browser_toolbar_sections,
    compute_sidebar_action_button_rects, compute_update_action_button_rects,
};
pub(super) use map_canvas::{compute_browser_map_canvas_rect, compute_browser_map_point_center};
pub(super) use map_header::compute_browser_map_header_text_layout;
pub(super) use overlay_visuals::{
    compute_drag_overlay_visual_layout, compute_progress_overlay_visual_layout,
    compute_prompt_overlay_visual_layout,
};
pub(super) use overlays::{
    compute_drag_overlay_text_layout, compute_progress_overlay_text_layout,
    compute_prompt_overlay_text_layout,
};
pub(super) use row_hit_test::compute_row_index_at_point;
pub(crate) use sidebar_bands::{
    SIDEBAR_BANDS_ROOT_ID, SidebarBandSections, build_sidebar_bands_tree,
    compute_sidebar_band_sections_with_layout_engine,
};
pub(super) use sidebar_chrome_text::{
    compute_sidebar_footer_text_layout, compute_sidebar_header_text_layout,
};
pub(super) use sidebar_header::{
    compute_sidebar_folder_header_layout, compute_source_section_divider_rect,
};
pub(super) use sidebar_sections::{SidebarRowCounts, compute_sidebar_row_sections};
pub(super) use sidebar_text::{
    compute_sidebar_folder_row_text_rect, compute_sidebar_recovery_badge_text_rect,
    compute_sidebar_source_row_text_rect,
};
pub(super) use status_bar::{compute_status_bar_segments, compute_status_text_line_rect};
pub(super) use top_controls::compute_top_bar_controls_sections;
pub(super) use top_title_text::compute_top_bar_title_text_rect;
pub(super) use update_text::compute_top_bar_update_text_layout;
pub(super) use waveform_annotations::compute_waveform_annotation_rects;
pub(super) use waveform_header::compute_waveform_header_text_layout;

pub(crate) const SHELL_ROOT_ID: u64 = 1;
const TOP_BAR_ID: u64 = 2;
const SIDEBAR_ID: u64 = 3;
const CONTENT_ID: u64 = 4;
const WAVEFORM_ID: u64 = 5;
const STATUS_ID: u64 = 6;
const BODY_ID: u64 = 40;
const BROWSER_ID: u64 = 100;

/// Top-level section rectangles used by `ShellLayout`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ShellSectionRects {
    pub root: Rect,
    pub top_bar: Rect,
    pub sidebar: Rect,
    pub content: Rect,
    pub waveform_card: Rect,
    pub browser_panel: Rect,
    pub status_bar: Rect,
}

/// Compute top-level shell sections using the strict slot-based layout engine.
#[allow(dead_code)]
pub(super) fn compute_shell_sections(viewport: Vector2, style: &StyleTokens) -> ShellSectionRects {
    let mut engine = LayoutEngine::default();
    compute_shell_sections_with_layout_engine(viewport, style, &mut engine, &LayoutState::default())
}

/// Compute top-level shell sections with a caller-provided persistent layout engine.
pub(crate) fn compute_shell_sections_with_layout_engine(
    viewport: Vector2,
    style: &StyleTokens,
    engine: &mut LayoutEngine,
    state: &LayoutState,
) -> ShellSectionRects {
    let sizing = style.sizing;
    let viewport_width = viewport.x.max(sizing.min_viewport_width);
    let viewport_height = viewport.y.max(sizing.min_viewport_height);
    let root_rect = Rect::from_min_size(
        Point::new(0.0, 0.0),
        Vector2::new(viewport_width, viewport_height),
    );

    let root = build_shell_sections_tree(style, viewport_width);
    let output = engine.layout_with_state(&root, root_rect, state, LayoutDebugOptions::default());

    ShellSectionRects {
        root: rect_for(&output.rects, SHELL_ROOT_ID, root_rect),
        top_bar: rect_for(&output.rects, TOP_BAR_ID, root_rect),
        sidebar: rect_for(&output.rects, SIDEBAR_ID, root_rect),
        content: rect_for(&output.rects, CONTENT_ID, root_rect),
        waveform_card: rect_for(&output.rects, WAVEFORM_ID, root_rect),
        browser_panel: rect_for(&output.rects, BROWSER_ID, root_rect),
        status_bar: rect_for(&output.rects, STATUS_ID, root_rect),
    }
}

/// Build the shell section tree used by top-level shell layout partitioning.
pub(crate) fn build_shell_sections_tree(style: &StyleTokens, viewport_width: f32) -> LayoutNode {
    let sizing = style.sizing;
    let body = LayoutNode::container(
        BODY_ID,
        ContainerPolicy {
            kind: ContainerKind::Row,
            spacing: sizing.panel_gap,
            padding: Insets::default(),
            align_main: MainAlign::Start,
            align_cross: CrossAlign::Stretch,
            overflow: OverflowPolicy::Clip,
            ..ContainerPolicy::default()
        },
        vec![
            SlotChild {
                slot: SlotParams {
                    size_main: SizeModeMain::Percent(sizing.sidebar_ratio),
                    size_cross: SizeModeCross::Fill,
                    constraints: Constraints::new(
                        sizing.sidebar_min_width,
                        sizing.sidebar_max_width,
                        0.0,
                        f32::INFINITY,
                    ),
                    margin: Insets::default(),
                    align_cross_override: None,
                    allow_fixed_compress: false,
                },
                child: LayoutNode::widget(SIDEBAR_ID, Vector2::new(180.0, 200.0)),
            },
            SlotChild {
                slot: SlotParams {
                    size_main: SizeModeMain::Fill(1.0),
                    size_cross: SizeModeCross::Fill,
                    constraints: Constraints::new(
                        sizing.content_min_width,
                        f32::INFINITY,
                        0.0,
                        f32::INFINITY,
                    ),
                    margin: Insets::default(),
                    align_cross_override: None,
                    allow_fixed_compress: false,
                },
                child: build_content_tree(style),
            },
        ],
    );

    LayoutNode::container(
        SHELL_ROOT_ID,
        ContainerPolicy {
            kind: ContainerKind::Column,
            spacing: sizing.panel_gap,
            padding: Insets::all(sizing.frame_inset),
            align_main: MainAlign::Start,
            align_cross: CrossAlign::Stretch,
            overflow: OverflowPolicy::Clip,
            ..ContainerPolicy::default()
        },
        vec![
            SlotChild {
                slot: SlotParams {
                    size_main: SizeModeMain::Fixed(sizing.top_bar_height),
                    size_cross: SizeModeCross::Fill,
                    constraints: Constraints::new(
                        0.0,
                        f32::INFINITY,
                        sizing.top_bar_height,
                        sizing.top_bar_height,
                    ),
                    margin: Insets::default(),
                    align_cross_override: None,
                    allow_fixed_compress: false,
                },
                child: LayoutNode::widget(
                    TOP_BAR_ID,
                    Vector2::new(viewport_width, sizing.top_bar_height),
                ),
            },
            SlotChild {
                slot: SlotParams {
                    size_main: SizeModeMain::Fill(1.0),
                    size_cross: SizeModeCross::Fill,
                    constraints: Constraints::new(0.0, f32::INFINITY, 0.0, f32::INFINITY),
                    margin: Insets::default(),
                    align_cross_override: None,
                    allow_fixed_compress: false,
                },
                child: body,
            },
            SlotChild {
                slot: SlotParams {
                    size_main: SizeModeMain::Fixed(sizing.status_bar_height),
                    size_cross: SizeModeCross::Fill,
                    constraints: Constraints::new(
                        0.0,
                        f32::INFINITY,
                        sizing.status_bar_height,
                        sizing.status_bar_height,
                    ),
                    margin: Insets::default(),
                    align_cross_override: None,
                    allow_fixed_compress: false,
                },
                child: LayoutNode::widget(
                    STATUS_ID,
                    Vector2::new(viewport_width, sizing.status_bar_height),
                ),
            },
        ],
    )
}

fn build_content_tree(style: &StyleTokens) -> LayoutNode {
    let sizing = style.sizing;
    LayoutNode::container(
        CONTENT_ID,
        ContainerPolicy {
            kind: ContainerKind::Column,
            spacing: sizing.panel_gap,
            padding: Insets::default(),
            align_main: MainAlign::Start,
            align_cross: CrossAlign::Stretch,
            overflow: OverflowPolicy::Clip,
            ..ContainerPolicy::default()
        },
        vec![
            SlotChild {
                slot: SlotParams {
                    size_main: SizeModeMain::Percent(sizing.waveform_ratio),
                    size_cross: SizeModeCross::Fill,
                    constraints: Constraints::new(
                        0.0,
                        f32::INFINITY,
                        sizing.waveform_min_height,
                        sizing.waveform_max_height,
                    ),
                    margin: Insets::default(),
                    align_cross_override: None,
                    allow_fixed_compress: false,
                },
                child: LayoutNode::widget(
                    WAVEFORM_ID,
                    Vector2::new(220.0, sizing.waveform_min_height),
                ),
            },
            SlotChild {
                slot: SlotParams {
                    size_main: SizeModeMain::Fill(1.0),
                    size_cross: SizeModeCross::Fill,
                    constraints: Constraints::new(
                        0.0,
                        f32::INFINITY,
                        sizing.content_browser_min_height,
                        f32::INFINITY,
                    ),
                    margin: Insets::default(),
                    align_cross_override: None,
                    allow_fixed_compress: false,
                },
                child: LayoutNode::widget(
                    BROWSER_ID,
                    Vector2::new(220.0, sizing.content_browser_min_height),
                ),
            },
        ],
    )
}

fn rect_for(rects: &std::collections::BTreeMap<u64, Rect>, id: u64, fallback: Rect) -> Rect {
    rects.get(&id).copied().unwrap_or(fallback)
}

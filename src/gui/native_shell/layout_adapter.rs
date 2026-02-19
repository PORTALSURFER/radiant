//! Adapter that maps native shell section geometry onto the slot-based layout core.
mod bands;
mod sidebar_bands;
mod sidebar_sections;
use super::style::{SizingTokens, StyleTokens};
use crate::gui::layout_core::{
    Constraints, ContainerKind, ContainerPolicy, CrossAlign, Insets, LayoutNode, MainAlign,
    OverflowPolicy, SizeModeCross, SizeModeMain, SlotChild, SlotParams, layout_tree,
};
use crate::gui::types::{Point, Rect, Vector2};
pub(super) use bands::{compute_browser_band_sections, compute_top_bar_band_sections};
pub(super) use sidebar_bands::compute_sidebar_band_sections;
pub(super) use sidebar_sections::{SidebarRowCounts, compute_sidebar_row_sections};

const ROOT_ID: u64 = 1;
const TOP_BAR_ID: u64 = 2;
const SIDEBAR_ID: u64 = 3;
const CONTENT_ID: u64 = 4;
const WAVEFORM_ID: u64 = 5;
const STATUS_ID: u64 = 6;
const BODY_ID: u64 = 40;
const BROWSER_ID: u64 = 100;
const TOP_CONTROLS_ROOT_ID: u64 = 200;
const TOP_CONTROLS_ROW_ID: u64 = 201;
const TOP_CONTROLS_OPTIONS_ID: u64 = 202;
const TOP_CONTROLS_METER_ID: u64 = 203;
const TOP_CONTROLS_VALUE_ID: u64 = 204;
const TOP_CONTROLS_LABEL_ID: u64 = 205;

/// Top-level section rectangles used by `ShellLayout`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct ShellSectionRects {
    pub root: Rect,
    pub top_bar: Rect,
    pub sidebar: Rect,
    pub content: Rect,
    pub waveform_card: Rect,
    pub browser_panel: Rect,
    pub status_bar: Rect,
}

/// Slot-resolved rectangles for top-bar controls.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct TopBarControlsSections {
    pub active: bool,
    pub options_label: Rect,
    pub volume_meter: Rect,
    pub volume_value: Rect,
    pub volume_label: Rect,
}

/// Compute top-level shell sections using the strict slot-based layout engine.
pub(super) fn compute_shell_sections(viewport: Vector2, style: &StyleTokens) -> ShellSectionRects {
    let sizing = style.sizing;
    let viewport_width = viewport.x.max(sizing.min_viewport_width);
    let viewport_height = viewport.y.max(sizing.min_viewport_height);
    let root_rect = Rect::from_min_size(
        Point::new(0.0, 0.0),
        Vector2::new(viewport_width, viewport_height),
    );

    let root = build_root_tree(style, viewport_width);
    let output = layout_tree(&root, root_rect);

    ShellSectionRects {
        root: rect_for(&output.rects, ROOT_ID, root_rect),
        top_bar: rect_for(&output.rects, TOP_BAR_ID, root_rect),
        sidebar: rect_for(&output.rects, SIDEBAR_ID, root_rect),
        content: rect_for(&output.rects, CONTENT_ID, root_rect),
        waveform_card: rect_for(&output.rects, WAVEFORM_ID, root_rect),
        browser_panel: rect_for(&output.rects, BROWSER_ID, root_rect),
        status_bar: rect_for(&output.rects, STATUS_ID, root_rect),
    }
}

/// Compute top-bar control rectangles from a strict slot tree.
pub(super) fn compute_top_bar_controls_sections(
    layout: &super::layout::ShellLayout,
    sizing: SizingTokens,
) -> TopBarControlsSections {
    let row = layout.top_bar_controls_row;
    if row.height() <= 1.0 || row.width() <= 1.0 {
        return inactive_top_controls(row);
    }

    let horizontal_inset = sizing.text_inset_x + sizing.header_label_gutter;
    let options_width = 64.0_f32.min((row.width() * 0.35).max(24.0));
    let meter_width = sizing
        .top_volume_meter_width
        .min((row.width() * 0.45).max(26.0))
        .max(26.0);
    let value_width = 44.0_f32.min((row.width() * 0.2).max(20.0));
    let label_width = 28.0_f32.min((row.width() * 0.12).max(16.0));
    let gap = sizing.action_button_gap.max(2.0);
    let available_width = row.width() - (horizontal_inset * 2.0);
    let total_width = options_width + gap + meter_width + gap + value_width + gap + label_width;
    if available_width <= 12.0 || total_width > available_width {
        return inactive_top_controls(row);
    }
    let meter_height = sizing
        .top_volume_meter_height
        .min(row.height().max(1.0))
        .max(3.0);

    let controls_tree = LayoutNode::container(
        TOP_CONTROLS_ROOT_ID,
        ContainerPolicy {
            kind: ContainerKind::PaddingBox,
            padding: Insets {
                left: horizontal_inset,
                right: horizontal_inset,
                top: 0.0,
                bottom: 0.0,
            },
            align_cross: CrossAlign::Stretch,
            ..ContainerPolicy::default()
        },
        vec![SlotChild {
            slot: SlotParams::fill(),
            child: LayoutNode::container(
                TOP_CONTROLS_ROW_ID,
                ContainerPolicy {
                    kind: ContainerKind::Row,
                    spacing: gap,
                    align_cross: CrossAlign::Center,
                    overflow: OverflowPolicy::Clip,
                    ..ContainerPolicy::default()
                },
                vec![
                    fixed_slot_child(TOP_CONTROLS_OPTIONS_ID, options_width, 0.0, false),
                    fixed_slot_child(TOP_CONTROLS_METER_ID, meter_width, meter_height, false),
                    fixed_slot_child(TOP_CONTROLS_VALUE_ID, value_width, 0.0, false),
                    fixed_slot_child(TOP_CONTROLS_LABEL_ID, label_width, 0.0, false),
                ],
            ),
        }],
    );
    let output = layout_tree(&controls_tree, row);
    let empty = Rect::from_min_max(row.min, row.min);
    let options_label = output
        .rects
        .get(&TOP_CONTROLS_OPTIONS_ID)
        .copied()
        .unwrap_or(empty);
    let volume_meter = output
        .rects
        .get(&TOP_CONTROLS_METER_ID)
        .copied()
        .unwrap_or(empty);
    let volume_value = output
        .rects
        .get(&TOP_CONTROLS_VALUE_ID)
        .copied()
        .unwrap_or(empty);
    let volume_label = output
        .rects
        .get(&TOP_CONTROLS_LABEL_ID)
        .copied()
        .unwrap_or(empty);
    let options_label = clamp_rect_to_bounds(options_label, row);
    let volume_meter = clamp_rect_to_bounds(volume_meter, row);
    let volume_value = clamp_rect_to_bounds(volume_value, row);
    let volume_label = clamp_rect_to_bounds(volume_label, row);
    if options_label.width() <= 0.0
        || volume_meter.width() <= 0.0
        || volume_value.width() <= 0.0
        || volume_label.width() <= 0.0
    {
        return inactive_top_controls(row);
    }

    TopBarControlsSections {
        active: true,
        options_label,
        volume_meter,
        volume_value,
        volume_label,
    }
}

fn build_root_tree(style: &StyleTokens, viewport_width: f32) -> LayoutNode {
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
        ROOT_ID,
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

fn fixed_slot_child(node_id: u64, width: f32, height: f32, allow_compress: bool) -> SlotChild {
    let cross_mode = if height > 0.0 {
        SizeModeCross::Fixed(height)
    } else {
        SizeModeCross::Fill
    };
    let constraints = if height > 0.0 {
        Constraints::new(width, width, height, height)
    } else {
        Constraints::new(width, width, 0.0, f32::INFINITY)
    };
    SlotChild {
        slot: SlotParams {
            size_main: SizeModeMain::Fixed(width),
            size_cross: cross_mode,
            constraints,
            margin: Insets::default(),
            align_cross_override: Some(CrossAlign::Center),
            allow_fixed_compress: allow_compress,
        },
        child: LayoutNode::widget(node_id, Vector2::new(width.max(0.0), height.max(1.0))),
    }
}

fn inactive_top_controls(row: Rect) -> TopBarControlsSections {
    let empty = Rect::from_min_max(row.min, row.min);
    TopBarControlsSections {
        active: false,
        options_label: empty,
        volume_meter: empty,
        volume_value: empty,
        volume_label: empty,
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

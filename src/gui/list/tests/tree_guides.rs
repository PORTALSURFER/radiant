use super::super::{
    StyledTreeGuideStyle, TreeGuideOverlay, TreeGuideRow, TreeGuideSegment, TreeGuideStyle,
    tree_guide_segments,
};
use crate::{
    gui::types::{Rect, Rgba8},
    layout::LayoutOutput,
    theme::ThemeTokens,
    widgets::{Widget, WidgetStyle, WidgetTone},
};

const ROW_HEIGHT: f32 = 23.0;
const INDENT: f32 = 10.0;
const GUIDE_COLOR: Rgba8 = Rgba8 {
    r: 255,
    g: 126,
    b: 64,
    a: 152,
};

fn style() -> TreeGuideStyle {
    TreeGuideStyle::new(INDENT, ROW_HEIGHT, GUIDE_COLOR)
}

#[test]
fn expanded_row_segment_spans_descendant_block_only() {
    let rows = vec![
        row(0, false),
        row(1, true),
        row(2, false),
        row(2, false),
        row(1, false),
    ];

    assert_eq!(
        tree_guide_segments(&rows),
        vec![TreeGuideSegment {
            level: 0,
            start_row: 1,
            end_row_exclusive: 4,
        }]
    );
}

#[test]
fn expanded_row_guides_leave_gaps_between_direct_child_siblings() {
    let rows = vec![
        row(0, false),
        row(1, true),
        row(2, false),
        row(2, true),
        row(3, false),
        row(2, true),
        row(3, false),
        row(1, false),
    ];

    assert_eq!(
        tree_guide_segments(&rows),
        vec![
            TreeGuideSegment {
                level: 0,
                start_row: 1,
                end_row_exclusive: 7,
            },
            TreeGuideSegment {
                level: 1,
                start_row: 3,
                end_row_exclusive: 5,
            },
            TreeGuideSegment {
                level: 1,
                start_row: 5,
                end_row_exclusive: 7,
            },
        ]
    );
}

#[test]
fn expanded_row_without_descendants_does_not_emit_segment() {
    let rows = vec![row(0, false), row(1, true), row(1, false)];

    assert!(tree_guide_segments(&rows).is_empty());
}

#[test]
fn nested_guides_preserve_start_order_in_one_pass() {
    let rows = vec![
        row(0, false),
        row(1, true),
        row(2, true),
        row(3, true),
        row(4, false),
        row(1, false),
    ];

    assert_eq!(
        tree_guide_segments(&rows),
        vec![
            TreeGuideSegment {
                level: 0,
                start_row: 1,
                end_row_exclusive: 5,
            },
            TreeGuideSegment {
                level: 1,
                start_row: 2,
                end_row_exclusive: 5,
            },
            TreeGuideSegment {
                level: 2,
                start_row: 3,
                end_row_exclusive: 5,
            },
        ]
    );
}

#[test]
fn overlay_paints_one_continuous_rect_for_visible_segment() {
    let overlay = TreeGuideOverlay::new(
        2,
        2,
        vec![TreeGuideSegment {
            level: 1,
            start_row: 2,
            end_row_exclusive: 4,
        }],
        style(),
    );
    let plan = overlay.paint_plan_with_defaults(Rect::from_size(80.0, ROW_HEIGHT * 2.0));
    let lines = plan.fill_rects().collect::<Vec<_>>();

    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].color, GUIDE_COLOR);
    assert_eq!(lines[0].rect.min.y, 0.0);
    assert_eq!(lines[0].rect.max.y, ROW_HEIGHT * 2.0 - style().end_gap);
}

#[test]
fn overlay_resolves_styled_guide_color_from_frame_theme() {
    let theme = ThemeTokens {
        accent_mint: Rgba8::new(42, 180, 210, 255),
        ..ThemeTokens::default()
    };
    let overlay = TreeGuideOverlay::new(
        2,
        2,
        vec![TreeGuideSegment {
            level: 1,
            start_row: 2,
            end_row_exclusive: 4,
        }],
        StyledTreeGuideStyle::new(INDENT, ROW_HEIGHT, WidgetStyle::subtle(WidgetTone::Accent)),
    );

    let plan = overlay.paint_plan(
        Rect::from_size(80.0, ROW_HEIGHT * 2.0),
        &LayoutOutput::default(),
        &theme,
    );
    let line = plan.fill_rects().next().expect("expected styled guide");

    assert_eq!(line.color, theme.accent_mint.with_alpha(152));
}

#[test]
fn overlay_clips_segment_to_materialized_window() {
    let overlay = TreeGuideOverlay::new(
        3,
        1,
        vec![TreeGuideSegment {
            level: 1,
            start_row: 2,
            end_row_exclusive: 4,
        }],
        style(),
    );
    let plan = overlay.paint_plan_with_defaults(Rect::from_size(80.0, ROW_HEIGHT));
    let line = plan.fill_rects().next().expect("expected clipped guide");

    assert_eq!(line.rect.min.y, 0.0);
    assert_eq!(line.rect.max.y, ROW_HEIGHT - style().end_gap);
}

#[test]
fn invalid_style_skips_overlay_paint() {
    let overlay = TreeGuideOverlay::new(
        0,
        1,
        vec![TreeGuideSegment {
            level: 0,
            start_row: 0,
            end_row_exclusive: 1,
        }],
        TreeGuideStyle::new(0.0, ROW_HEIGHT, GUIDE_COLOR),
    );

    assert!(
        overlay
            .paint_plan_with_defaults(Rect::from_size(80.0, ROW_HEIGHT))
            .fill_rects()
            .next()
            .is_none()
    );
}

fn row(depth: usize, starts_descendant_group: bool) -> TreeGuideRow {
    TreeGuideRow::new(depth, starts_descendant_group)
}

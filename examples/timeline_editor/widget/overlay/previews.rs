use super::super::super::model::BeatRange;
use super::super::paint::{push_rect, push_resize_handles, push_stroke, push_text};
use super::super::{
    ArrangementTimelineWidget, RESIZE_HANDLE_WIDTH, TimelineDrag, TimelineGeometry,
};
use radiant::layout::{Point, Rect};
use radiant::runtime::{PaintPrimitive, PaintTextAlign};
use radiant::theme::ThemeTokens;

pub(super) fn append_clip_overlays(
    widget: &ArrangementTimelineWidget,
    primitives: &mut Vec<PaintPrimitive>,
    geometry: TimelineGeometry,
    theme: &ThemeTokens,
) {
    if let Some(clip_id) = widget
        .hover_clip_id
        .filter(|clip_id| widget.selected_clip != Some(*clip_id) && widget.drag.is_none())
        && let Some(clip) = widget.clips.iter().find(|clip| clip.id == clip_id)
    {
        let rect = geometry.clip_rect(clip);
        push_stroke(primitives, widget.common.id, rect, theme.text_primary, 2.0);
        push_rect(
            primitives,
            widget.common.id,
            rect.top_edge_strip(4.0),
            theme.highlight_orange_soft,
        );
        if rect.width() >= RESIZE_HANDLE_WIDTH {
            push_resize_handles(primitives, widget.common.id, rect, theme.highlight_orange);
        }
    }
    match widget.drag {
        Some(TimelineDrag::Selecting { lane, .. }) => {
            if let Some(range) = widget.selection.filter(|range| range.duration() > 0) {
                paint_clip_preview(
                    primitives,
                    widget.common.id,
                    geometry.clip_rect_for_range(lane, range),
                    "New clip",
                    preview_fill(theme.accent_mint),
                    theme,
                    false,
                );
            }
        }
        Some(TimelineDrag::MovingClip {
            clip_name,
            source_lane,
            current_lane,
            current_start,
            duration,
            ..
        }) => {
            paint_clip_preview(
                primitives,
                widget.common.id,
                geometry.clip_rect_for_range(
                    current_lane,
                    BeatRange {
                        start: current_start,
                        end: current_start + duration,
                    },
                ),
                clip_name,
                preview_fill(clip_fill_for_lane(current_lane, theme)),
                theme,
                true,
            );
            if source_lane != current_lane {
                paint_lane_transfer_marker(
                    primitives,
                    widget.common.id,
                    geometry.clip_rect_for_range(
                        current_lane,
                        BeatRange {
                            start: current_start,
                            end: current_start + duration,
                        },
                    ),
                    theme,
                );
            }
        }
        Some(TimelineDrag::ResizingClip {
            clip_name,
            source_lane,
            current_range,
            ..
        }) => {
            paint_clip_preview(
                primitives,
                widget.common.id,
                geometry.clip_rect_for_range(source_lane, current_range),
                clip_name,
                preview_fill(clip_fill_for_lane(source_lane, theme)),
                theme,
                true,
            );
        }
        None => {}
    }
}

fn paint_clip_preview(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: u64,
    rect: Rect,
    label: impl Into<String>,
    fill: radiant::gui::types::Rgba8,
    theme: &ThemeTokens,
    handles: bool,
) {
    push_rect(primitives, widget_id, rect, fill);
    push_stroke(primitives, widget_id, rect, theme.text_primary, 2.0);
    push_rect(
        primitives,
        widget_id,
        Rect::from_min_max(rect.min, Point::new(rect.min.x + 5.0, rect.max.y)),
        theme.surface_overlay,
    );
    push_text(
        primitives,
        widget_id,
        label,
        Rect::from_min_max(
            Point::new(rect.min.x + 12.0, rect.min.y + 6.0),
            Point::new(rect.max.x - 8.0, rect.max.y),
        ),
        theme.text_primary,
        PaintTextAlign::Left,
    );
    if handles {
        push_resize_handles(primitives, widget_id, rect, theme.text_primary);
    }
}

fn paint_lane_transfer_marker(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: u64,
    rect: Rect,
    theme: &ThemeTokens,
) {
    push_rect(
        primitives,
        widget_id,
        rect.top_edge_strip(3.0),
        theme.highlight_orange,
    );
}

fn clip_fill_for_lane(lane: usize, theme: &ThemeTokens) -> radiant::gui::types::Rgba8 {
    match lane {
        0 => theme.accent_mint,
        1 => theme.highlight_cyan,
        2 => theme.accent_copper,
        _ => theme.highlight_blue,
    }
}

fn preview_fill(mut color: radiant::gui::types::Rgba8) -> radiant::gui::types::Rgba8 {
    color.a = 210;
    color
}

use super::super::model::BeatRange;
use super::TimelineGeometry;
use radiant::layout::{Point, Rect};
use radiant::runtime::PaintPrimitive;
use radiant::theme::ThemeTokens;

use super::paint::{push_rect, push_resize_handles, push_stroke, push_text};
use super::{ArrangementTimelineWidget, RESIZE_HANDLE_WIDTH, TimelineDrag};
use radiant::runtime::PaintTextAlign;

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct TimelineCursorOverlay {
    hover: Option<TimelineHoverCursor>,
}

#[derive(Clone, Copy, Debug)]
struct TimelineHoverCursor {
    beat: u32,
    x: f32,
}

impl TimelineCursorOverlay {
    pub(super) fn set_hover(&mut self, geometry: TimelineGeometry, position: Point) -> Option<u32> {
        let beat = geometry.beat_at(position)?;
        let x = geometry.cursor_x_at(position)?;
        self.hover = Some(TimelineHoverCursor { beat, x });
        Some(beat)
    }

    pub(super) fn clear_hover(&mut self) {
        self.hover = None;
    }

    pub(super) fn hover_beat(self) -> Option<u32> {
        self.hover.map(|cursor| cursor.beat)
    }

    pub(super) fn active_beat(self, playhead_beat: u32) -> u32 {
        self.hover_beat().unwrap_or(playhead_beat)
    }

    pub(super) fn append_paint(
        self,
        primitives: &mut Vec<PaintPrimitive>,
        widget_id: u64,
        geometry: TimelineGeometry,
        bounds: Rect,
        playhead_beat: u32,
        theme: &ThemeTokens,
    ) {
        let indicator_x = self
            .hover
            .map(|cursor| cursor.x)
            .unwrap_or_else(|| geometry.x_for_beat(playhead_beat));
        let indicator_color = if self.hover.is_some() {
            theme.highlight_orange_soft
        } else {
            theme.highlight_orange
        };
        push_rect(
            primitives,
            widget_id,
            Rect::from_min_max(
                Point::new(indicator_x - 1.5, geometry.ruler.min.y),
                Point::new(indicator_x + 1.5, bounds.max.y),
            ),
            indicator_color,
        );
    }
}

pub(super) fn append_runtime_timeline_overlay(
    widget: &ArrangementTimelineWidget,
    primitives: &mut Vec<PaintPrimitive>,
    bounds: Rect,
    theme: &ThemeTokens,
) {
    let geometry = widget.geometry(bounds);
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
            clip_id,
            current_lane,
            current_start,
            duration,
            ..
        }) => {
            if let Some(clip) = widget.clips.iter().find(|clip| clip.id == clip_id) {
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
                    clip.name,
                    preview_fill(clip_fill_for_lane(current_lane, theme)),
                    theme,
                    true,
                );
            }
        }
        Some(TimelineDrag::ResizingClip {
            clip_id,
            current_range,
            ..
        }) => {
            if let Some(clip) = widget.clips.iter().find(|clip| clip.id == clip_id) {
                paint_clip_preview(
                    primitives,
                    widget.common.id,
                    geometry.clip_rect_for_range(clip.lane, current_range),
                    clip.name,
                    preview_fill(clip_fill_for_lane(clip.lane, theme)),
                    theme,
                    true,
                );
            }
        }
        None => {}
    }
    widget.cursor.append_paint(
        primitives,
        widget.common.id,
        geometry,
        bounds,
        widget.playhead_beat,
        theme,
    );
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

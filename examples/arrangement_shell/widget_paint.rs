use radiant::prelude::*;

use super::{
    TOTAL_BEATS, TRACKS,
    geometry::{track_height, track_label_rect, x_for_beat},
    model::ArrangementClip,
    paint::{push_rect, push_stroke, push_text, rgba, translucent},
    widget::ArrangementOverviewWidget,
};

pub(super) fn append_grid(
    widget: &ArrangementOverviewWidget,
    primitives: &mut Vec<PaintPrimitive>,
    timeline: Rect,
    theme: &ThemeTokens,
) {
    push_rect(primitives, widget.common.id, timeline, rgba(8, 12, 18, 255));
    append_track_rows(widget, primitives, timeline, theme);
    append_beat_lines(widget, primitives, timeline, theme);
}

pub(super) fn append_clip(
    widget: &ArrangementOverviewWidget,
    primitives: &mut Vec<PaintPrimitive>,
    timeline: Rect,
    clip: ArrangementClip,
    theme: &ThemeTokens,
) {
    let rect = widget.clip_rect(timeline, clip);
    let selected = widget.selected_clip == Some(clip.id);
    push_rect(
        primitives,
        widget.common.id,
        rect,
        clip_fill(selected, theme),
    );
    push_stroke(
        primitives,
        widget.common.id,
        rect,
        clip_stroke(selected, theme),
        1.0,
    );
    push_text(
        primitives,
        widget.common.id,
        clip.label,
        rect,
        theme.text_primary,
        PaintTextAlign::Center,
    );
}

pub(super) fn append_hover_guides(
    widget: &ArrangementOverviewWidget,
    primitives: &mut Vec<PaintPrimitive>,
    timeline: Rect,
    theme: &ThemeTokens,
) {
    if let Some(position) = widget.hover_position {
        push_rect(
            primitives,
            widget.common.id,
            Rect::from_min_max(
                Point::new(position.x, timeline.min.y),
                Point::new(position.x + 1.0, timeline.max.y),
            ),
            translucent(theme.text_muted, 80),
        );
    }
    if let Some(id) = widget.hover_clip
        && let Some(clip) = widget.clips.iter().copied().find(|clip| clip.id == id)
    {
        push_stroke(
            primitives,
            widget.common.id,
            widget.clip_rect(timeline, clip),
            translucent(theme.highlight_cyan, 190),
            2.0,
        );
    }
}

fn append_track_rows(
    widget: &ArrangementOverviewWidget,
    primitives: &mut Vec<PaintPrimitive>,
    timeline: Rect,
    theme: &ThemeTokens,
) {
    for (track, label) in TRACKS.iter().enumerate() {
        let row = track_row_rect(timeline, track);
        push_rect(primitives, widget.common.id, row, track_row_fill(track));
        push_text(
            primitives,
            widget.common.id,
            *label,
            track_label_rect(timeline, track),
            theme.text_muted,
            PaintTextAlign::Right,
        );
    }
}

fn append_beat_lines(
    widget: &ArrangementOverviewWidget,
    primitives: &mut Vec<PaintPrimitive>,
    timeline: Rect,
    theme: &ThemeTokens,
) {
    for beat in 0..=TOTAL_BEATS as usize {
        let x = x_for_beat(timeline, beat as f32);
        push_rect(
            primitives,
            widget.common.id,
            Rect::from_min_max(
                Point::new(x, timeline.min.y),
                Point::new(x + 1.0, timeline.max.y),
            ),
            beat_line_color(beat, theme),
        );
    }
}

fn track_row_rect(timeline: Rect, track: usize) -> Rect {
    let y = timeline.min.y + track as f32 * track_height(timeline);
    Rect::from_min_max(
        Point::new(timeline.min.x, y),
        Point::new(timeline.max.x, y + track_height(timeline)),
    )
}

fn track_row_fill(track: usize) -> Rgba8 {
    if track.is_multiple_of(2) {
        rgba(11, 16, 23, 255)
    } else {
        rgba(14, 19, 27, 255)
    }
}

fn beat_line_color(beat: usize, theme: &ThemeTokens) -> Rgba8 {
    if beat.is_multiple_of(4) {
        translucent(theme.grid_strong, 160)
    } else {
        translucent(theme.grid_soft, 90)
    }
}

fn clip_fill(selected: bool, theme: &ThemeTokens) -> Rgba8 {
    if selected {
        theme.highlight_blue
    } else {
        theme.highlight_cyan_soft
    }
}

fn clip_stroke(selected: bool, theme: &ThemeTokens) -> Rgba8 {
    if selected {
        theme.border_emphasis
    } else {
        translucent(theme.border_emphasis, 140)
    }
}

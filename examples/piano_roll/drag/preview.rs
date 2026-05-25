use radiant::prelude::{Point, Rect};

use super::PianoDrag;
use crate::piano_roll::{
    geometry::{beat_for_x_view, pitch_for_y_view, quantize_beat},
    model::{PianoNote, PianoRollViewport},
};

impl PianoDrag {
    pub(in crate::piano_roll) fn preview_note(
        self,
        grid: Rect,
        viewport: PianoRollViewport,
        position: Point,
        source: PianoNote,
        snap_enabled: bool,
    ) -> PianoNote {
        let projection = DragProjection {
            grid,
            viewport,
            position,
            snap_enabled,
        };
        match self {
            Self::Create { pitch, start_beat } => {
                create_preview(projection, source, pitch, start_beat)
            }
            Self::Move {
                beat_offset,
                pitch_offset,
                source_start_beat,
                source_pitch,
                ..
            } => move_preview(
                projection,
                source,
                MovePreviewOffsets {
                    beat_offset,
                    pitch_offset,
                    source_start_beat,
                    source_pitch,
                },
            ),
            Self::ResizeStart { end_beat, .. } => {
                resize_start_preview(projection, source, end_beat)
            }
            Self::ResizeEnd { start_beat, .. } => {
                resize_end_preview(projection, source, start_beat)
            }
            Self::Pan { .. }
            | Self::Marquee { .. }
            | Self::VelocityMarquee { .. }
            | Self::TimeSelection { .. }
            | Self::MoveTimeSelection { .. }
            | Self::Velocity { .. } => source,
        }
    }
}

#[derive(Clone, Copy)]
struct DragProjection {
    grid: Rect,
    viewport: PianoRollViewport,
    position: Point,
    snap_enabled: bool,
}

struct MovePreviewOffsets {
    beat_offset: f32,
    pitch_offset: i32,
    source_start_beat: f32,
    source_pitch: i32,
}

fn create_preview(
    projection: DragProjection,
    source: PianoNote,
    pitch: i32,
    start_beat: f32,
) -> PianoNote {
    let start_beat = projection.resolve_beat(start_beat);
    let end_beat = projection
        .resolve_beat(beat_for_x_view(
            projection.grid,
            projection.viewport,
            projection.position.x,
        ))
        .max(start_beat + 0.25);
    PianoNote {
        pitch,
        start_beat,
        length_beats: (end_beat - start_beat).max(0.25).clamp(0.25, 4.0),
        ..source
    }
}

fn move_preview(
    projection: DragProjection,
    source: PianoNote,
    offsets: MovePreviewOffsets,
) -> PianoNote {
    PianoNote {
        pitch: source.pitch
            + pitch_for_y_view(projection.grid, projection.viewport, projection.position.y)
            - offsets.pitch_offset
            - offsets.source_pitch,
        start_beat: source.start_beat
            + projection.resolve_beat(
                beat_for_x_view(projection.grid, projection.viewport, projection.position.x)
                    - offsets.beat_offset,
            )
            - offsets.source_start_beat,
        ..source
    }
}

fn resize_start_preview(projection: DragProjection, source: PianoNote, end_beat: f32) -> PianoNote {
    let start_beat = projection
        .resolve_beat(beat_for_x_view(
            projection.grid,
            projection.viewport,
            projection.position.x,
        ))
        .min(end_beat - 0.25);
    PianoNote {
        start_beat,
        length_beats: end_beat - start_beat,
        ..source
    }
}

fn resize_end_preview(projection: DragProjection, source: PianoNote, start_beat: f32) -> PianoNote {
    PianoNote {
        start_beat,
        length_beats: projection
            .resolve_beat(
                beat_for_x_view(projection.grid, projection.viewport, projection.position.x)
                    - start_beat,
            )
            .max(0.25),
        ..source
    }
}

impl DragProjection {
    fn resolve_beat(self, beat: f32) -> f32 {
        if self.snap_enabled {
            quantize_beat(beat)
        } else {
            beat
        }
    }
}

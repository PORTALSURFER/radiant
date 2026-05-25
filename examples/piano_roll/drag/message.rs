use radiant::prelude::{Point, Rect};

use super::PianoDrag;
use crate::piano_roll::{
    NoteSelectionMode, PianoRollMessage,
    geometry::{beat_for_x_view, pitch_for_y_view, quantize_beat, row_height_for},
    model::PianoRollViewport,
};

impl PianoDrag {
    pub(in crate::piano_roll) fn message_for(
        self,
        grid: Rect,
        viewport: PianoRollViewport,
        position: Point,
    ) -> PianoRollMessage {
        let projection = DragProjection {
            grid,
            viewport,
            position,
        };
        match self {
            Self::Create { pitch, start_beat } => create_message(projection, pitch, start_beat),
            Self::Move {
                id,
                ids,
                beat_offset,
                pitch_offset,
                source_start_beat,
                source_pitch,
                ..
            } => move_message(
                projection,
                MoveMessageParts {
                    id,
                    ids,
                    beat_offset,
                    pitch_offset,
                    source_start_beat,
                    source_pitch,
                },
            ),
            Self::ResizeStart { id, end_beat } => resize_start_message(projection, id, end_beat),
            Self::ResizeEnd { id, start_beat } => resize_end_message(projection, id, start_beat),
            Self::Pan {
                start,
                viewport: start_viewport,
            } => pan_message(projection, start, start_viewport),
            Self::Marquee { .. } => PianoRollMessage::SelectNotes {
                ids: Vec::new(),
                mode: NoteSelectionMode::Replace,
            },
            Self::Velocity { ids, velocity } => PianoRollMessage::SetVelocity { ids, velocity },
        }
    }
}

#[derive(Clone, Copy)]
struct DragProjection {
    grid: Rect,
    viewport: PianoRollViewport,
    position: Point,
}

struct MoveMessageParts {
    id: u32,
    ids: Vec<u32>,
    beat_offset: f32,
    pitch_offset: i32,
    source_start_beat: f32,
    source_pitch: i32,
}

fn create_message(projection: DragProjection, pitch: i32, start_beat: f32) -> PianoRollMessage {
    let end_beat = quantize_beat(beat_for_x_view(
        projection.grid,
        projection.viewport,
        projection.position.x,
    ))
    .max(start_beat + 0.25);
    PianoRollMessage::CreateNote {
        pitch,
        start_beat,
        length_beats: (end_beat - start_beat).clamp(0.25, 4.0),
    }
}

fn move_message(projection: DragProjection, parts: MoveMessageParts) -> PianoRollMessage {
    let pitch = pitch_for_y_view(projection.grid, projection.viewport, projection.position.y)
        - parts.pitch_offset;
    let start_beat = quantize_beat(
        beat_for_x_view(projection.grid, projection.viewport, projection.position.x)
            - parts.beat_offset,
    );
    if parts.ids.len() > 1 {
        return PianoRollMessage::MoveNotes {
            ids: parts.ids,
            pitch_delta: pitch - parts.source_pitch,
            beat_delta: start_beat - parts.source_start_beat,
        };
    }
    PianoRollMessage::MoveNote {
        id: parts.id,
        pitch,
        start_beat,
    }
}

fn resize_start_message(projection: DragProjection, id: u32, end_beat: f32) -> PianoRollMessage {
    let start_beat = quantize_beat(beat_for_x_view(
        projection.grid,
        projection.viewport,
        projection.position.x,
    ))
    .min(end_beat - 0.25);
    PianoRollMessage::ResizeNote {
        id,
        start_beat,
        length_beats: end_beat - start_beat,
    }
}

fn resize_end_message(projection: DragProjection, id: u32, start_beat: f32) -> PianoRollMessage {
    PianoRollMessage::ResizeNote {
        id,
        start_beat,
        length_beats: quantize_beat(
            beat_for_x_view(projection.grid, projection.viewport, projection.position.x)
                - start_beat,
        )
        .max(0.25),
    }
}

fn pan_message(
    projection: DragProjection,
    start: Point,
    start_viewport: PianoRollViewport,
) -> PianoRollMessage {
    PianoRollMessage::PanViewport {
        beat_delta: -(projection.position.x - start.x) * start_viewport.visible_beats
            / projection.grid.width().max(1.0),
        pitch_delta: ((projection.position.y - start.y)
            / row_height_for(projection.grid, start_viewport).max(1.0))
        .round() as i32,
    }
}

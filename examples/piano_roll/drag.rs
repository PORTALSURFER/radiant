use radiant::prelude::*;

use super::{
    PianoRollMessage,
    geometry::{beat_for_x, pitch_for_y, quantize_beat, row_height, x_for_beat, y_for_pitch},
    model::PianoNote,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum PianoDrag {
    Move {
        id: u32,
        beat_offset: f32,
        pitch_offset: i32,
        length_beats: f32,
    },
    ResizeStart {
        id: u32,
        end_beat: f32,
    },
    ResizeEnd {
        id: u32,
        start_beat: f32,
    },
}

impl PianoDrag {
    pub(super) fn from_note_hit(grid: Rect, note: PianoNote, position: Point) -> Self {
        let rect = Rect::from_min_max(
            Point::new(
                x_for_beat(grid, note.start_beat),
                y_for_pitch(grid, note.pitch),
            ),
            Point::new(
                x_for_beat(grid, note.end_beat()),
                y_for_pitch(grid, note.pitch) + row_height(grid),
            ),
        );
        if position.x <= rect.min.x + 8.0 {
            return Self::ResizeStart {
                id: note.id,
                end_beat: note.end_beat(),
            };
        }
        if position.x >= rect.max.x - 8.0 {
            return Self::ResizeEnd {
                id: note.id,
                start_beat: note.start_beat,
            };
        }
        Self::Move {
            id: note.id,
            beat_offset: beat_for_x(grid, position.x) - note.start_beat,
            pitch_offset: pitch_for_y(grid, position.y) - note.pitch,
            length_beats: note.length_beats,
        }
    }

    pub(super) fn message_for(self, grid: Rect, position: Point) -> PianoRollMessage {
        match self {
            Self::Move {
                id,
                beat_offset,
                pitch_offset,
                ..
            } => PianoRollMessage::MoveNote {
                id,
                pitch: pitch_for_y(grid, position.y) - pitch_offset,
                start_beat: beat_for_x(grid, position.x) - beat_offset,
            },
            Self::ResizeStart { id, end_beat } => {
                let start_beat = quantize_beat(beat_for_x(grid, position.x)).min(end_beat - 0.25);
                PianoRollMessage::ResizeNote {
                    id,
                    start_beat,
                    length_beats: end_beat - start_beat,
                }
            }
            Self::ResizeEnd { id, start_beat } => PianoRollMessage::ResizeNote {
                id,
                start_beat,
                length_beats: quantize_beat(beat_for_x(grid, position.x) - start_beat).max(0.25),
            },
        }
    }
}

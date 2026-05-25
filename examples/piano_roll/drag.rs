use radiant::prelude::*;

use super::{
    geometry::{
        beat_for_x_view, pitch_for_y_view, quantize_beat, row_height_for, x_for_beat_view,
        y_for_pitch_view,
    },
    model::{PianoNote, PianoRollViewport},
};
use radiant::widgets::PointerModifiers;

#[path = "drag/message.rs"]
mod message;
#[path = "drag/preview.rs"]
mod preview;

#[derive(Clone, Debug, PartialEq)]
pub(super) enum PianoDrag {
    Create {
        pitch: i32,
        start_beat: f32,
    },
    Move {
        id: u32,
        ids: Vec<u32>,
        beat_offset: f32,
        pitch_offset: i32,
        source_start_beat: f32,
        source_pitch: i32,
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
    Pan {
        start: Point,
        viewport: PianoRollViewport,
    },
    Marquee {
        start: Point,
        current: Point,
        modifiers: PointerModifiers,
    },
    TimeSelection {
        start: Point,
        current: Point,
    },
    MoveTimeSelection {
        source_start_beat: f32,
        source_end_beat: f32,
        grab_beat: f32,
        current: Point,
    },
    Velocity {
        ids: Vec<u32>,
        velocity: f32,
    },
}

impl PianoDrag {
    pub(super) fn create(pitch: i32, start_beat: f32) -> Self {
        Self::Create {
            pitch,
            start_beat: quantize_beat(start_beat),
        }
    }

    pub(super) fn from_note_hit(
        grid: Rect,
        viewport: PianoRollViewport,
        note: PianoNote,
        ids: Vec<u32>,
        position: Point,
    ) -> Self {
        let rect = Rect::from_min_max(
            Point::new(
                x_for_beat_view(grid, viewport, note.start_beat),
                y_for_pitch_view(grid, viewport, note.pitch),
            ),
            Point::new(
                x_for_beat_view(grid, viewport, note.end_beat()),
                y_for_pitch_view(grid, viewport, note.pitch) + row_height_for(grid, viewport),
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
            ids,
            beat_offset: beat_for_x_view(grid, viewport, position.x) - note.start_beat,
            pitch_offset: pitch_for_y_view(grid, viewport, position.y) - note.pitch,
            source_start_beat: note.start_beat,
            source_pitch: note.pitch,
            length_beats: note.length_beats,
        }
    }
}

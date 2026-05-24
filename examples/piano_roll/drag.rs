use radiant::prelude::*;

use super::{
    NoteSelectionMode, PianoRollMessage,
    geometry::{
        beat_for_x_view, pitch_for_y_view, quantize_beat, row_height_for, x_for_beat_view,
        y_for_pitch_view,
    },
    model::{PianoNote, PianoRollViewport},
};
use radiant::widgets::PointerModifiers;

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

    pub(super) fn message_for(
        self,
        grid: Rect,
        viewport: PianoRollViewport,
        position: Point,
    ) -> PianoRollMessage {
        match self {
            Self::Create { pitch, start_beat } => {
                let end_beat = quantize_beat(beat_for_x_view(grid, viewport, position.x))
                    .max(start_beat + 0.25);
                PianoRollMessage::CreateNote {
                    pitch,
                    start_beat,
                    length_beats: (end_beat - start_beat).clamp(0.25, 4.0),
                }
            }
            Self::Move {
                id,
                ids,
                beat_offset,
                pitch_offset,
                source_start_beat,
                source_pitch,
                ..
            } => {
                let pitch = pitch_for_y_view(grid, viewport, position.y) - pitch_offset;
                let start_beat =
                    quantize_beat(beat_for_x_view(grid, viewport, position.x) - beat_offset);
                if ids.len() > 1 {
                    PianoRollMessage::MoveNotes {
                        ids,
                        pitch_delta: pitch - source_pitch,
                        beat_delta: start_beat - source_start_beat,
                    }
                } else {
                    PianoRollMessage::MoveNote {
                        id,
                        pitch,
                        start_beat,
                    }
                }
            }
            Self::ResizeStart { id, end_beat } => {
                let start_beat =
                    quantize_beat(beat_for_x_view(grid, viewport, position.x)).min(end_beat - 0.25);
                PianoRollMessage::ResizeNote {
                    id,
                    start_beat,
                    length_beats: end_beat - start_beat,
                }
            }
            Self::ResizeEnd { id, start_beat } => PianoRollMessage::ResizeNote {
                id,
                start_beat,
                length_beats: quantize_beat(
                    beat_for_x_view(grid, viewport, position.x) - start_beat,
                )
                .max(0.25),
            },
            Self::Pan {
                start,
                viewport: start_viewport,
            } => PianoRollMessage::PanViewport {
                beat_delta: -(position.x - start.x) * start_viewport.visible_beats
                    / grid.width().max(1.0),
                pitch_delta: ((position.y - start.y)
                    / row_height_for(grid, start_viewport).max(1.0))
                .round() as i32,
            },
            Self::Marquee { .. } => PianoRollMessage::SelectNotes {
                ids: Vec::new(),
                mode: NoteSelectionMode::Replace,
            },
            Self::Velocity { ids, velocity } => PianoRollMessage::SetVelocity { ids, velocity },
        }
    }

    pub(super) fn preview_note(
        self,
        grid: Rect,
        viewport: PianoRollViewport,
        position: Point,
        source: PianoNote,
    ) -> PianoNote {
        match self {
            Self::Create { pitch, start_beat } => {
                let end_beat = quantize_beat(beat_for_x_view(grid, viewport, position.x))
                    .max(start_beat + 0.25);
                PianoNote {
                    pitch,
                    start_beat,
                    length_beats: (end_beat - start_beat).clamp(0.25, 4.0),
                    ..source
                }
            }
            Self::Move {
                beat_offset,
                pitch_offset,
                source_start_beat,
                source_pitch,
                length_beats: _,
                ..
            } => PianoNote {
                pitch: source.pitch + pitch_for_y_view(grid, viewport, position.y)
                    - pitch_offset
                    - source_pitch,
                start_beat: source.start_beat
                    + quantize_beat(beat_for_x_view(grid, viewport, position.x) - beat_offset)
                    - source_start_beat,
                ..source
            },
            Self::ResizeStart { end_beat, .. } => {
                let start_beat =
                    quantize_beat(beat_for_x_view(grid, viewport, position.x)).min(end_beat - 0.25);
                PianoNote {
                    start_beat,
                    length_beats: end_beat - start_beat,
                    ..source
                }
            }
            Self::ResizeEnd { start_beat, .. } => PianoNote {
                start_beat,
                length_beats: quantize_beat(
                    beat_for_x_view(grid, viewport, position.x) - start_beat,
                )
                .max(0.25),
                ..source
            },
            Self::Pan { .. } | Self::Marquee { .. } | Self::Velocity { .. } => source,
        }
    }
}

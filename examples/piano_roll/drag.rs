use radiant::prelude::*;

use super::{
    geometry::{beat_for_x_view, pitch_for_y_view, pitch_item_layout, quantize_beat},
    model::{PianoNote, PianoRollViewport},
    widget::NOTE_RESIZE_EDGE_WIDTH,
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
    VelocityMarquee {
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
        start_pointer_velocity: f32,
        current_pointer_velocity: f32,
        start_velocities: Vec<(u32, f32)>,
    },
    VelocityRelative {
        ids: Vec<u32>,
        start_y: f32,
        current_y: f32,
        start_velocities: Vec<(u32, f32)>,
    },
}

impl PianoDrag {
    pub(super) fn velocity_values(&self) -> Option<Vec<(u32, f32)>> {
        let (delta, start_velocities) = match self {
            Self::Velocity {
                start_pointer_velocity,
                current_pointer_velocity,
                start_velocities,
                ..
            } => (
                current_pointer_velocity - start_pointer_velocity,
                start_velocities,
            ),
            Self::VelocityRelative {
                start_y,
                current_y,
                start_velocities,
                ..
            } => (
                (start_y - current_y) / NOTE_VELOCITY_DRAG_PIXELS_PER_UNIT,
                start_velocities,
            ),
            _ => return None,
        };
        Some(
            start_velocities
                .iter()
                .map(|(id, velocity)| (*id, (velocity + delta).clamp(0.0, 1.0)))
                .collect(),
        )
    }

    pub(super) fn velocity_for_note(&self, id: u32) -> Option<f32> {
        let (delta, start_velocities) = match self {
            Self::Velocity {
                start_pointer_velocity,
                current_pointer_velocity,
                start_velocities,
                ..
            } => (
                current_pointer_velocity - start_pointer_velocity,
                start_velocities,
            ),
            Self::VelocityRelative {
                start_y,
                current_y,
                start_velocities,
                ..
            } => (
                (start_y - current_y) / NOTE_VELOCITY_DRAG_PIXELS_PER_UNIT,
                start_velocities,
            ),
            _ => return None,
        };
        start_velocities
            .binary_search_by_key(&id, |(note_id, _)| *note_id)
            .ok()
            .map(|index| (start_velocities[index].1 + delta).clamp(0.0, 1.0))
    }
}

pub(super) const NOTE_VELOCITY_DRAG_PIXELS_PER_UNIT: f32 = 160.0;

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
        let rect = pitch_item_layout(grid, viewport).item_rect_unclamped(
            note.pitch,
            note.start_beat,
            note.end_beat(),
        );
        if position.x <= rect.min.x + NOTE_RESIZE_EDGE_WIDTH {
            return Self::ResizeStart {
                id: note.id,
                end_beat: note.end_beat(),
            };
        }
        if position.x >= rect.max.x - NOTE_RESIZE_EDGE_WIDTH {
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

use radiant::prelude::{Point, Rect};

use super::PianoDrag;
use crate::piano_roll::{
    NoteSelectionMode, PianoRollMessage, TOTAL_BEATS,
    geometry::{beat_for_x_view, pitch_for_y_view, quantize_beat, row_height_for},
    model::PianoRollViewport,
};

impl PianoDrag {
    pub(in crate::piano_roll) fn message_for(
        self,
        grid: Rect,
        viewport: PianoRollViewport,
        position: Point,
        snap_enabled: bool,
    ) -> PianoRollMessage {
        let projection = DragProjection {
            grid,
            viewport,
            position,
            snap_enabled,
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
            Self::Marquee { .. } | Self::VelocityMarquee { .. } => PianoRollMessage::SelectNotes {
                ids: Vec::new(),
                mode: NoteSelectionMode::Replace,
            },
            Self::TimeSelection { start, current } => {
                time_selection_message(projection, start, current)
            }
            Self::MoveTimeSelection {
                source_start_beat,
                source_end_beat,
                grab_beat,
                current,
            } => move_time_selection_message(
                projection,
                source_start_beat,
                source_end_beat,
                grab_beat,
                current,
            ),
            Self::Velocity {
                start_pointer_velocity,
                current_pointer_velocity,
                start_velocities,
                ..
            } => velocity_message(
                start_pointer_velocity,
                current_pointer_velocity,
                start_velocities,
            ),
            Self::VelocityRelative {
                start_y,
                current_y,
                start_velocities,
                ..
            } => relative_velocity_message(start_y, current_y, start_velocities),
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

struct MoveMessageParts {
    id: u32,
    ids: Vec<u32>,
    beat_offset: f32,
    pitch_offset: i32,
    source_start_beat: f32,
    source_pitch: i32,
}

fn create_message(projection: DragProjection, pitch: i32, start_beat: f32) -> PianoRollMessage {
    let start_beat = projection.resolve_beat(start_beat);
    let end_beat = projection
        .resolve_beat(beat_for_x_view(
            projection.grid,
            projection.viewport,
            projection.position.x,
        ))
        .max(start_beat + 0.25);
    PianoRollMessage::CreateNote {
        pitch,
        start_beat,
        length_beats: (end_beat - start_beat).max(0.25),
    }
}

fn move_message(projection: DragProjection, parts: MoveMessageParts) -> PianoRollMessage {
    let pitch = pitch_for_y_view(projection.grid, projection.viewport, projection.position.y)
        - parts.pitch_offset;
    let start_beat = projection.resolve_beat(
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
    let start_beat = projection
        .resolve_beat(beat_for_x_view(
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
        length_beats: projection
            .resolve_beat(
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

fn time_selection_message(
    projection: DragProjection,
    start: Point,
    current: Point,
) -> PianoRollMessage {
    let start_beat = projection.resolve_beat(beat_for_x_view(
        projection.grid,
        projection.viewport,
        start.x,
    ));
    let current_beat = projection.resolve_beat(beat_for_x_view(
        projection.grid,
        projection.viewport,
        current.x,
    ));
    if (current_beat - start_beat).abs() < f32::EPSILON {
        return PianoRollMessage::SetCursor { beat: start_beat };
    }
    PianoRollMessage::SetTimeSelection {
        start_beat,
        end_beat: current_beat,
    }
}

fn move_time_selection_message(
    projection: DragProjection,
    source_start_beat: f32,
    source_end_beat: f32,
    grab_beat: f32,
    current: Point,
) -> PianoRollMessage {
    let length = (source_end_beat - source_start_beat).abs();
    let current_beat = projection.resolve_beat(beat_for_x_view(
        projection.grid,
        projection.viewport,
        current.x,
    ));
    let target_start =
        (source_start_beat + current_beat - grab_beat).clamp(0.0, TOTAL_BEATS - length);
    PianoRollMessage::SetTimeSelection {
        start_beat: target_start,
        end_beat: target_start + length,
    }
}

fn velocity_message(
    start_pointer_velocity: f32,
    current_pointer_velocity: f32,
    start_velocities: Vec<(u32, f32)>,
) -> PianoRollMessage {
    let delta = current_pointer_velocity - start_pointer_velocity;
    PianoRollMessage::SetVelocities {
        velocities: start_velocities
            .into_iter()
            .map(|(id, velocity)| (id, (velocity + delta).clamp(0.0, 1.0)))
            .collect(),
    }
}

fn relative_velocity_message(
    start_y: f32,
    current_y: f32,
    start_velocities: Vec<(u32, f32)>,
) -> PianoRollMessage {
    let delta = (start_y - current_y) / super::NOTE_VELOCITY_DRAG_PIXELS_PER_UNIT;
    PianoRollMessage::SetVelocities {
        velocities: start_velocities
            .into_iter()
            .map(|(id, velocity)| (id, (velocity + delta).clamp(0.0, 1.0)))
            .collect(),
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

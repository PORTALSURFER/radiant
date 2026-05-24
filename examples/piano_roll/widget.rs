use radiant::prelude::*;
use radiant::widgets::{PaintBounds, PointerModifiers};

use super::{
    NoteSelectionMode, PianoRollMessage, PianoRollTool,
    drag::PianoDrag,
    geometry::{
        beat_for_x_view, pitch_for_y_view, row_height_for, x_for_beat_view, y_for_pitch_view,
    },
    model::{PianoNote, PianoRollViewport},
    paint::{push_rect, push_stroke, translucent},
    widget_paint::{
        append_drag_preview, append_editor_clip_end, append_editor_clip_start, append_grid,
        append_hover_guides, append_keyboard, append_keyboard_interaction, append_note,
        append_selected_pitch_lane, append_velocity_lane,
    },
};

#[derive(Clone, Debug)]
pub(crate) struct PianoRollWidget {
    pub(super) common: WidgetCommon,
    pub(super) notes: Vec<PianoNote>,
    pub(super) selected_note: Option<u32>,
    pub(super) selected_notes: Vec<u32>,
    pub(super) selected_pitch: Option<i32>,
    pub(super) playhead_beat: f32,
    pub(super) viewport: PianoRollViewport,
    pub(super) tool: PianoRollTool,
    pub(crate) hover_note: Option<u32>,
    pub(crate) hover_pitch: Option<i32>,
    pub(crate) active_pitch: Option<i32>,
    pub(super) hover_position: Option<Point>,
    pub(super) drag: Option<PianoDrag>,
}

impl PianoRollWidget {
    pub(crate) fn new(
        notes: Vec<PianoNote>,
        selected_note: Option<u32>,
        selected_notes: Vec<u32>,
        selected_pitch: Option<i32>,
        playhead_beat: f32,
        viewport: PianoRollViewport,
        tool: PianoRollTool,
    ) -> Self {
        let mut common = WidgetCommon::new(
            0,
            WidgetSizing::new(Vector2::new(760.0, 340.0), Vector2::new(1000.0, 390.0)),
        );
        common.focus = FocusBehavior::Pointer;
        common.paint.bounds = PaintBounds::ClipToRect;
        common.paint.paints_focus = false;
        common.paint.paints_state_layers = false;
        Self {
            common,
            notes,
            selected_note,
            selected_notes,
            selected_pitch,
            playhead_beat,
            viewport,
            tool,
            hover_note: None,
            hover_pitch: None,
            active_pitch: None,
            hover_position: None,
            drag: None,
        }
    }

    pub(super) fn keyboard_rect(&self, bounds: Rect) -> Rect {
        let editor = self.editor_rect(bounds);
        Rect::from_min_max(
            Point::new(bounds.min.x + 12.0, editor.min.y),
            Point::new(editor.min.x - 1.0, editor.max.y),
        )
    }

    pub(crate) fn editor_rect(&self, bounds: Rect) -> Rect {
        Rect::from_min_max(
            Point::new(bounds.min.x + 76.0, bounds.min.y + 36.0),
            Point::new(bounds.max.x - 14.0, bounds.max.y - 92.0),
        )
    }

    pub(crate) fn velocity_rect(&self, bounds: Rect) -> Rect {
        let editor = self.editor_rect(bounds);
        Rect::from_min_max(
            Point::new(editor.min.x, editor.max.y + 12.0),
            Point::new(editor.max.x, bounds.max.y - 20.0),
        )
    }

    pub(crate) fn note_rect(&self, grid: Rect, note: PianoNote) -> Rect {
        let x0 = x_for_beat_view(grid, self.viewport, note.start_beat);
        let x1 = x_for_beat_view(grid, self.viewport, note.end_beat());
        let y0 = y_for_pitch_view(grid, self.viewport, note.pitch);
        Rect::from_min_max(
            Point::new(x0, y0 + 2.0),
            Point::new(x1, y0 + row_height_for(grid, self.viewport) - 2.0),
        )
    }

    fn note_at_position(&self, grid: Rect, position: Point) -> Option<u32> {
        self.notes
            .iter()
            .rev()
            .find(|note| self.note_rect(grid, **note).contains(position))
            .map(|note| note.id)
    }

    pub(crate) fn note_by_id(&self, id: u32) -> Option<PianoNote> {
        self.notes.iter().copied().find(|note| note.id == id)
    }

    pub(crate) fn note_is_selected(&self, id: u32) -> bool {
        self.selected_note == Some(id) || self.selected_notes.binary_search(&id).is_ok()
    }

    pub(crate) fn marquee_rect(&self) -> Option<Rect> {
        let PianoDrag::Marquee { start, current, .. } = self.drag.as_ref()? else {
            return None;
        };
        Some(rect_from_points(*start, *current))
    }

    pub(crate) fn keyboard_pitch_rect(&self, keyboard: Rect, pitch: i32) -> Rect {
        let y = y_for_pitch_view(keyboard, self.viewport, pitch);
        Rect::from_min_max(
            Point::new(keyboard.min.x, y),
            Point::new(keyboard.max.x, y + row_height_for(keyboard, self.viewport)),
        )
    }

    pub(crate) fn drag_preview_note(&self, grid: Rect, position: Point) -> Option<PianoNote> {
        let drag = self.drag.as_ref()?;
        let source = match drag {
            PianoDrag::Pan { .. } | PianoDrag::Marquee { .. } => return None,
            PianoDrag::Velocity { .. } => return None,
            PianoDrag::Create { pitch, start_beat } => PianoNote {
                id: u32::MAX,
                pitch: *pitch,
                start_beat: *start_beat,
                length_beats: 0.25,
                velocity: 0.86,
            },
            PianoDrag::Move { id, .. }
            | PianoDrag::ResizeStart { id, .. }
            | PianoDrag::ResizeEnd { id, .. } => self.note_by_id(*id)?,
        };
        Some(
            drag.clone()
                .preview_note(grid, self.viewport, position, source),
        )
    }

    pub(crate) fn drag_preview_notes(&self, grid: Rect, position: Point) -> Vec<PianoNote> {
        let Some(PianoDrag::Move { ids, .. }) = self.drag.as_ref() else {
            return self
                .drag_preview_note(grid, position)
                .into_iter()
                .collect::<Vec<_>>();
        };
        ids.iter()
            .filter_map(|id| self.note_by_id(*id))
            .map(|note| {
                self.drag.clone().expect("active drag exists").preview_note(
                    grid,
                    self.viewport,
                    position,
                    note,
                )
            })
            .collect()
    }
}

impl Widget for PianoRollWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        let grid = self.editor_rect(bounds);
        let velocity = self.velocity_rect(bounds);
        let keyboard = self.keyboard_rect(bounds);
        match input {
            WidgetInput::PointerMove { position } => {
                self.handle_pointer_move(grid, bounds, position)
            }
            WidgetInput::PointerPress {
                position,
                button: PointerButton::Primary,
                ..
            } if keyboard.contains(position) => self.handle_keyboard_press(keyboard, position),
            WidgetInput::PointerPress {
                position,
                button: PointerButton::Primary,
                ..
            } if velocity.contains(position) => self.handle_velocity_press(velocity, position),
            WidgetInput::PointerPress {
                position,
                button: PointerButton::Primary,
                modifiers,
            } if grid.contains(position) => self.handle_primary_press(grid, position, modifiers),
            WidgetInput::PointerPress {
                position,
                button: PointerButton::Auxiliary,
                ..
            } if bounds.contains(position) => {
                self.hover_position = Some(position);
                self.drag = Some(PianoDrag::Pan {
                    start: position,
                    viewport: self.viewport,
                });
                None
            }
            WidgetInput::PointerRelease {
                position,
                button: PointerButton::Primary | PointerButton::Auxiliary,
                ..
            }
            | WidgetInput::PointerDrop {
                position,
                button: PointerButton::Primary | PointerButton::Auxiliary,
                ..
            } => self.finish_drag(grid, bounds, position),
            WidgetInput::Wheel {
                position,
                delta,
                modifiers,
            } if bounds.contains(position) => {
                if delta.y.abs() >= delta.x.abs() && delta.y.abs() > f32::EPSILON {
                    let zooming_in = delta.y < 0.0;
                    let time_factor = if zooming_in { 0.8 } else { 1.25 };
                    return Some(WidgetOutput::custom(PianoRollMessage::ZoomViewport {
                        time_factor: modifiers
                            .alt
                            .then(|| {
                                self.viewport
                                    .can_zoom_time(time_factor)
                                    .then_some(time_factor)
                            })
                            .flatten(),
                        rows_delta: if modifiers.alt {
                            0
                        } else if zooming_in {
                            -2
                        } else {
                            2
                        },
                    }));
                }
                let beat_delta = delta.x * self.viewport.visible_beats / grid.width().max(1.0);
                Some(WidgetOutput::custom(PianoRollMessage::PanViewport {
                    beat_delta,
                    pitch_delta: 0,
                }))
            }
            WidgetInput::KeyPress(WidgetKey::Delete | WidgetKey::Backspace)
                if self.common.state.focused =>
            {
                Some(WidgetOutput::custom(PianoRollMessage::DeleteSelected))
            }
            WidgetInput::FocusChanged(focused) => {
                self.common.state.focused = focused;
                None
            }
            _ => None,
        }
    }

    fn prefers_pointer_move_paint_only(&self) -> bool {
        true
    }

    fn accepts_wheel_input(&self) -> bool {
        true
    }

    fn synchronize_from_previous(&mut self, previous: &dyn Widget) {
        if let Some(previous) = previous.as_any().downcast_ref::<Self>() {
            self.common.state = previous.common.state;
            self.hover_note = previous.hover_note;
            self.hover_pitch = previous.hover_pitch;
            self.active_pitch = previous.active_pitch;
            self.hover_position = previous.hover_position;
            self.drag = previous.drag.clone();
        }
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        theme: &ThemeTokens,
    ) {
        let grid = self.editor_rect(bounds);
        push_rect(primitives, self.common.id, bounds, theme.bg_secondary);
        append_keyboard(self, primitives, bounds, theme);
        append_grid(self, primitives, grid, theme);
        append_selected_pitch_lane(self, primitives, bounds, grid, theme);
        append_editor_clip_start(self, primitives, grid);
        for note in &self.notes {
            append_note(self, primitives, grid, *note, theme);
        }
        append_editor_clip_end(self, primitives);
        push_stroke(primitives, self.common.id, grid, theme.border_emphasis, 1.0);
        append_velocity_lane(self, primitives, grid, self.velocity_rect(bounds), theme);
    }

    fn append_runtime_overlay_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        theme: &ThemeTokens,
    ) {
        let grid = self.editor_rect(bounds);
        let playhead_x = x_for_beat_view(grid, self.viewport, self.playhead_beat);
        push_rect(
            primitives,
            self.common.id,
            Rect::from_min_max(
                Point::new(playhead_x, grid.min.y),
                Point::new(playhead_x + 2.0, grid.max.y),
            ),
            translucent(theme.highlight_orange, 210),
        );
        append_keyboard_interaction(self, primitives, bounds, theme);
        append_editor_clip_start(self, primitives, grid);
        append_hover_guides(self, primitives, grid, theme);
        if let Some(position) = self.hover_position {
            append_drag_preview(
                self,
                primitives,
                grid,
                self.velocity_rect(bounds),
                position,
                theme,
            );
        }
        append_editor_clip_end(self, primitives);
    }
}

impl PianoRollWidget {
    fn handle_pointer_move(
        &mut self,
        grid: Rect,
        bounds: Rect,
        position: Point,
    ) -> Option<WidgetOutput> {
        self.common.state.hovered = bounds.contains(position);
        self.hover_pitch = self.hovered_pitch(grid, bounds, position);
        if let Some(PianoDrag::Pan {
            start,
            viewport: start_viewport,
        }) = self.drag.as_ref()
        {
            self.hover_position = Some(position);
            let beat_delta =
                -(position.x - start.x) * start_viewport.visible_beats / grid.width().max(1.0);
            let pitch_delta = ((position.y - start.y)
                / row_height_for(grid, *start_viewport).max(1.0))
            .round() as i32;
            let target = start_viewport.panned(beat_delta, pitch_delta);
            let (beat_delta, pitch_delta) = self.viewport.pan_delta_to(target);
            if beat_delta.abs() < f32::EPSILON && pitch_delta == 0 {
                return None;
            }
            return Some(WidgetOutput::custom(PianoRollMessage::PanViewport {
                beat_delta,
                pitch_delta,
            }));
        }
        if let Some(PianoDrag::Marquee {
            ref mut current, ..
        }) = self.drag
        {
            *current = position;
            self.hover_position = Some(position);
            self.hover_note = None;
            self.hover_pitch = self.hovered_pitch(grid, bounds, position);
            return None;
        }
        let velocity_lane = self.velocity_rect(bounds);
        if let Some(PianoDrag::Velocity {
            ref mut velocity, ..
        }) = self.drag
        {
            *velocity = velocity_for_y(velocity_lane, position.y);
            self.hover_position = Some(position);
            self.hover_pitch = None;
            return None;
        }
        self.hover_position = grid.contains(position).then_some(position);
        if self.drag.is_some() {
            return None;
        }
        self.hover_note = self.note_at_position(grid, position);
        None
    }

    fn handle_primary_press(
        &mut self,
        grid: Rect,
        position: Point,
        modifiers: PointerModifiers,
    ) -> Option<WidgetOutput> {
        let beat = beat_for_x_view(grid, self.viewport, position.x);
        let pitch = pitch_for_y_view(grid, self.viewport, position.y);
        if let Some(id) = self.note_at_position(grid, position) {
            if modifiers.shift || modifiers.command {
                self.hover_note = Some(id);
                self.hover_position = Some(position);
                return Some(WidgetOutput::custom(PianoRollMessage::SelectNotes {
                    ids: vec![id],
                    mode: selection_mode(modifiers),
                }));
            }
            return self.start_note_drag(grid, id, position);
        }
        self.hover_position = Some(position);
        self.hover_note = None;
        self.hover_pitch = Some(pitch);
        if self.tool == PianoRollTool::Select || modifiers.shift {
            self.drag = Some(PianoDrag::Marquee {
                start: position,
                current: position,
                modifiers,
            });
            return None;
        }
        self.drag = Some(PianoDrag::create(pitch, beat));
        None
    }

    fn handle_keyboard_press(&mut self, keyboard: Rect, position: Point) -> Option<WidgetOutput> {
        let pitch = pitch_for_y_view(keyboard, self.viewport, position.y);
        self.hover_position = Some(position);
        self.hover_note = None;
        self.hover_pitch = Some(pitch);
        self.active_pitch = Some(pitch);
        Some(WidgetOutput::custom(PianoRollMessage::SelectPitch(pitch)))
    }

    fn handle_velocity_press(&mut self, lane: Rect, position: Point) -> Option<WidgetOutput> {
        let note = self.velocity_note_at(lane, position)?;
        let ids = if self.note_is_selected(note.id) && !self.selected_notes.is_empty() {
            self.selected_notes.clone()
        } else {
            self.selected_note = Some(note.id);
            self.selected_notes = vec![note.id];
            vec![note.id]
        };
        self.hover_note = Some(note.id);
        self.hover_position = Some(position);
        self.drag = Some(PianoDrag::Velocity {
            ids: ids.clone(),
            velocity: velocity_for_y(lane, position.y),
        });
        None
    }

    fn start_note_drag(&mut self, grid: Rect, id: u32, position: Point) -> Option<WidgetOutput> {
        let note = self.note_by_id(id)?;
        let ids = if self.note_is_selected(id) && !self.selected_notes.is_empty() {
            self.selected_notes.clone()
        } else {
            self.selected_note = Some(id);
            self.selected_notes = vec![id];
            vec![id]
        };
        self.hover_note = Some(id);
        self.drag = Some(PianoDrag::from_note_hit(
            grid,
            self.viewport,
            note,
            ids.clone(),
            position,
        ));
        if ids.len() == 1 && ids[0] == id {
            Some(WidgetOutput::custom(PianoRollMessage::SelectNote(id)))
        } else {
            None
        }
    }

    fn finish_drag(&mut self, grid: Rect, bounds: Rect, position: Point) -> Option<WidgetOutput> {
        let drag = self.drag.take();
        self.active_pitch = None;
        self.hover_note = self.note_at_position(grid, position);
        self.hover_pitch = self.hovered_pitch(grid, bounds, position);
        drag.and_then(|drag| {
            if matches!(drag, PianoDrag::Pan { .. }) {
                None
            } else if let PianoDrag::Marquee {
                start, modifiers, ..
            } = drag
            {
                let rect = rect_from_points(start, position).clamp_to(grid);
                Some(WidgetOutput::custom(PianoRollMessage::SelectNotes {
                    ids: self.note_ids_intersecting(grid, rect),
                    mode: marquee_selection_mode(modifiers),
                }))
            } else if let PianoDrag::Velocity { ids, velocity } = drag {
                Some(WidgetOutput::custom(PianoRollMessage::SetVelocity {
                    ids,
                    velocity,
                }))
            } else {
                Some(WidgetOutput::custom(drag.message_for(
                    grid,
                    self.viewport,
                    position,
                )))
            }
        })
    }

    fn note_ids_intersecting(&self, grid: Rect, rect: Rect) -> Vec<u32> {
        self.notes
            .iter()
            .filter(|note| rects_overlap(self.note_rect(grid, **note), rect))
            .map(|note| note.id)
            .collect()
    }

    fn hovered_pitch(&self, grid: Rect, bounds: Rect, position: Point) -> Option<i32> {
        let keyboard = self.keyboard_rect(bounds);
        if keyboard.contains(position) {
            return Some(pitch_for_y_view(keyboard, self.viewport, position.y));
        }
        if grid.contains(position) {
            return Some(pitch_for_y_view(grid, self.viewport, position.y));
        }
        None
    }

    fn velocity_note_at(&self, lane: Rect, position: Point) -> Option<PianoNote> {
        self.notes
            .iter()
            .rev()
            .copied()
            .find(|note| {
                self.note_is_selected(note.id)
                    && self.velocity_column_rect(lane, *note).contains(position)
            })
            .or_else(|| {
                self.notes
                    .iter()
                    .rev()
                    .copied()
                    .find(|note| self.velocity_handle_rect(lane, *note).contains(position))
            })
    }

    pub(crate) fn velocity_preview_stem_rect(&self, lane: Rect, note: PianoNote) -> Rect {
        let velocity = match self.drag.as_ref() {
            Some(PianoDrag::Velocity { ids, velocity }) if ids.contains(&note.id) => *velocity,
            _ => note.velocity,
        };
        let x0 = x_for_beat_view(lane, self.viewport, note.start_beat);
        let y = lane.max.y - lane.height() * velocity.clamp(0.0, 1.0);
        Rect::from_min_max(Point::new(x0 - 1.0, y), Point::new(x0 + 1.0, lane.max.y))
    }

    pub(crate) fn velocity_handle_rect(&self, lane: Rect, note: PianoNote) -> Rect {
        let stem = self.velocity_preview_stem_rect(lane, note);
        Rect::from_min_size(
            Point::new(stem.center().x - 4.0, stem.min.y - 4.0),
            Vector2::new(8.0, 8.0),
        )
    }

    fn velocity_column_rect(&self, lane: Rect, note: PianoNote) -> Rect {
        let x0 = x_for_beat_view(lane, self.viewport, note.start_beat);
        let x1 = x_for_beat_view(lane, self.viewport, note.end_beat()).max(x0 + 8.0);
        Rect::from_min_max(Point::new(x0, lane.min.y), Point::new(x1, lane.max.y))
    }
}

fn velocity_for_y(lane: Rect, y: f32) -> f32 {
    ((lane.max.y - y) / lane.height().max(1.0)).clamp(0.0, 1.0)
}

fn selection_mode(modifiers: PointerModifiers) -> NoteSelectionMode {
    if modifiers.command {
        NoteSelectionMode::Toggle
    } else if modifiers.shift {
        NoteSelectionMode::Add
    } else {
        NoteSelectionMode::Replace
    }
}

fn marquee_selection_mode(modifiers: PointerModifiers) -> NoteSelectionMode {
    if modifiers.shift && modifiers.command {
        NoteSelectionMode::Add
    } else {
        NoteSelectionMode::Replace
    }
}

fn rect_from_points(a: Point, b: Point) -> Rect {
    Rect::from_min_max(
        Point::new(a.x.min(b.x), a.y.min(b.y)),
        Point::new(a.x.max(b.x), a.y.max(b.y)),
    )
}

fn rects_overlap(a: Rect, b: Rect) -> bool {
    a.min.x <= b.max.x && a.max.x >= b.min.x && a.min.y <= b.max.y && a.max.y >= b.min.y
}

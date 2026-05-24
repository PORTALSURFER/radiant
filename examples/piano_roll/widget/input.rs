use radiant::prelude::*;

use super::super::{
    PianoRollMessage,
    drag::PianoDrag,
    geometry::{beat_for_x, pitch_for_y, x_for_beat},
    paint::{push_rect, push_stroke, translucent},
    widget::PianoRollWidget,
    widget_paint::{append_grid, append_hover_guides, append_keyboard, append_note},
};

impl Widget for PianoRollWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        let grid = self.editor_rect(bounds);
        match input {
            WidgetInput::PointerMove { position } => {
                self.handle_pointer_move(grid, bounds, position)
            }
            WidgetInput::PointerPress {
                position,
                button: PointerButton::Primary,
                ..
            } if grid.contains(position) => self.handle_primary_press(grid, position),
            WidgetInput::PointerRelease {
                position,
                button: PointerButton::Primary,
                ..
            }
            | WidgetInput::PointerDrop {
                position,
                button: PointerButton::Primary,
                ..
            } => self.finish_drag(grid, position),
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
        self.drag.is_none()
    }

    fn synchronize_from_previous(&mut self, previous: &dyn Widget) {
        if let Some(previous) = previous.as_any().downcast_ref::<Self>() {
            self.common.state = previous.common.state;
            self.hover_note = previous.hover_note;
            self.hover_position = previous.hover_position;
            self.drag = previous.drag;
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
        for note in &self.notes {
            append_note(self, primitives, grid, *note, theme);
        }
        push_stroke(primitives, self.common.id, grid, theme.border_emphasis, 1.0);
    }

    fn append_runtime_overlay_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        theme: &ThemeTokens,
    ) {
        let grid = self.editor_rect(bounds);
        let playhead_x = x_for_beat(grid, self.playhead_beat);
        push_rect(
            primitives,
            self.common.id,
            Rect::from_min_max(
                Point::new(playhead_x, grid.min.y),
                Point::new(playhead_x + 2.0, grid.max.y),
            ),
            translucent(theme.highlight_orange, 210),
        );
        append_hover_guides(self, primitives, grid, theme);
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
        self.hover_position = grid.contains(position).then_some(position);
        if let Some(drag) = self.drag {
            return Some(WidgetOutput::custom(drag.message_for(grid, position)));
        }
        self.hover_note = self.note_at_position(grid, position);
        None
    }

    fn handle_primary_press(&mut self, grid: Rect, position: Point) -> Option<WidgetOutput> {
        let beat = beat_for_x(grid, position.x);
        let pitch = pitch_for_y(grid, position.y);
        if let Some(id) = self.note_at_position(grid, position) {
            return self.start_note_drag(grid, id, position);
        }
        Some(WidgetOutput::custom(PianoRollMessage::CreateNote {
            pitch,
            start_beat: beat,
        }))
    }

    fn start_note_drag(&mut self, grid: Rect, id: u32, position: Point) -> Option<WidgetOutput> {
        let note = self.note_by_id(id)?;
        self.selected_note = Some(id);
        self.hover_note = Some(id);
        self.drag = Some(PianoDrag::from_note_hit(grid, note, position));
        Some(WidgetOutput::custom(PianoRollMessage::SelectNote(id)))
    }

    fn finish_drag(&mut self, grid: Rect, position: Point) -> Option<WidgetOutput> {
        let drag = self.drag.take();
        self.hover_note = self.note_at_position(grid, position);
        drag.map(|drag| WidgetOutput::custom(drag.message_for(grid, position)))
    }
}

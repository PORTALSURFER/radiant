use radiant::prelude::*;

use super::super::{
    PianoRollMessage,
    drag::PianoDrag,
    geometry::x_for_beat_view,
    paint::{push_rect, push_stroke, translucent},
    widget::PianoRollWidget,
    widget_paint::{
        append_drag_preview, append_editor_clip_end, append_editor_clip_start, append_grid,
        append_hover_guides, append_keyboard, append_keyboard_interaction, append_notes,
        append_selected_pitch_lane, append_time_selection, append_velocity_handle_hover,
        append_velocity_lane,
    },
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
        let velocity = self.velocity_rect(bounds);
        let keyboard = self.keyboard_rect(bounds);
        match input {
            WidgetInput::PointerMove { position } => {
                self.handle_pointer_move(grid, bounds, position)
            }
            WidgetInput::PointerModifiersChanged { modifiers } => {
                self.pointer_modifiers = modifiers;
                None
            }
            WidgetInput::PointerPress {
                position,
                button: PointerButton::Primary,
                ..
            } if keyboard.contains(position) => self.handle_keyboard_press(keyboard, position),
            WidgetInput::PointerPress {
                position,
                button: PointerButton::Primary,
                modifiers,
            } if velocity.contains(position) => {
                self.handle_velocity_press(velocity, position, modifiers)
            }
            WidgetInput::PointerPress {
                position,
                button: PointerButton::Primary,
                modifiers,
            } if grid.contains(position) => self.handle_primary_press(grid, position, modifiers),
            WidgetInput::PointerDoubleClick {
                position,
                button: PointerButton::Primary,
                ..
            } if grid.contains(position) => self.handle_primary_double_click(grid, position),
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
                modifiers,
            }
            | WidgetInput::PointerDrop {
                position,
                button: PointerButton::Primary | PointerButton::Auxiliary,
                modifiers,
            } => self.finish_drag(grid, bounds, position, modifiers),
            WidgetInput::Wheel {
                position,
                delta,
                modifiers,
            } if bounds.contains(position) => self.handle_wheel(grid, delta, modifiers),
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
            self.hover_velocity_note = previous.hover_velocity_note;
            self.hover_pitch = previous.hover_pitch;
            self.active_pitch = previous.active_pitch;
            self.hover_position = previous.hover_position;
            self.pointer_modifiers = previous.pointer_modifiers;
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
        append_notes(self, primitives, grid, &self.notes, theme);
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
        append_time_selection(self, primitives, grid, theme);
        append_hover_guides(self, primitives, grid, theme);
        append_velocity_handle_hover(self, primitives, self.velocity_rect(bounds), theme);
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

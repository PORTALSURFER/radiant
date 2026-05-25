use radiant::prelude::*;

use super::ArrangementOverviewWidget;
use crate::arrangement_shell::{
    ShellMessage,
    geometry::{beat_for_x, x_for_beat},
    paint::{push_rect, push_stroke, translucent},
    widget_paint::{append_clip, append_grid, append_hover_guides},
};

impl Widget for ArrangementOverviewWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        let timeline = self.timeline_rect(bounds);
        match input {
            WidgetInput::PointerMove { position } => {
                self.handle_pointer_move(bounds, timeline, position);
                None
            }
            WidgetInput::PointerPress {
                position,
                button: PointerButton::Primary,
                ..
            } if timeline.contains(position) => self.handle_primary_press(timeline, position),
            WidgetInput::PointerDrop { .. } => {
                self.hover_clip = None;
                self.hover_position = None;
                None
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

    fn synchronize_from_previous(&mut self, previous: &dyn Widget) {
        if let Some(previous) = previous.as_any().downcast_ref::<Self>() {
            self.common.state = previous.common.state;
            self.hover_clip = previous.hover_clip;
            self.hover_position = previous.hover_position;
        }
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        theme: &ThemeTokens,
    ) {
        let timeline = self.timeline_rect(bounds);
        push_rect(primitives, self.common.id, bounds, theme.bg_secondary);
        append_grid(self, primitives, timeline, theme);
        for clip in &self.clips {
            append_clip(self, primitives, timeline, *clip, theme);
        }
        push_stroke(
            primitives,
            self.common.id,
            timeline,
            theme.border_emphasis,
            1.0,
        );
    }

    fn append_runtime_overlay_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        theme: &ThemeTokens,
    ) {
        let timeline = self.timeline_rect(bounds);
        let playhead_x = x_for_beat(timeline, self.playhead_beat);
        push_rect(
            primitives,
            self.common.id,
            Rect::from_min_max(
                Point::new(playhead_x, timeline.min.y),
                Point::new(playhead_x + 2.0, timeline.max.y),
            ),
            translucent(theme.highlight_orange, 210),
        );
        append_hover_guides(self, primitives, timeline, theme);
    }
}

impl ArrangementOverviewWidget {
    fn handle_pointer_move(&mut self, bounds: Rect, timeline: Rect, position: Point) {
        self.common.state.hovered = bounds.contains(position);
        self.hover_position = timeline.contains(position).then_some(position);
        self.hover_clip = self.clip_at_position(timeline, position);
    }

    fn handle_primary_press(&mut self, timeline: Rect, position: Point) -> Option<WidgetOutput> {
        if let Some(id) = self.clip_at_position(timeline, position) {
            self.selected_clip = Some(id);
            return Some(WidgetOutput::custom(ShellMessage::SelectClip(id)));
        }
        Some(WidgetOutput::custom(ShellMessage::Seek {
            beat: beat_for_x(timeline, position.x),
        }))
    }
}

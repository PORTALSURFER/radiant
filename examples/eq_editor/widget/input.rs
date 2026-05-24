use super::{EqEditorWidget, geometry, paint, translucent};
use crate::model::EqEditorMessage;
use radiant::prelude::*;

impl Widget for EqEditorWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        let plot = self.plot_rect(bounds);
        match input {
            WidgetInput::PointerMove { position } => {
                self.handle_pointer_move(bounds, plot, position)
            }
            WidgetInput::PointerPress {
                position,
                button: PointerButton::Primary,
                ..
            } if bounds.contains(position) => self.handle_primary_press(plot, position),
            WidgetInput::PointerRelease {
                position,
                button: PointerButton::Primary,
                ..
            }
            | WidgetInput::PointerDrop {
                position,
                button: PointerButton::Primary,
                ..
            } => self.finish_primary_drag(bounds, plot, position),
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
            self.hover_band = previous.hover_band;
            self.drag_band = previous.drag_band;
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
        paint::append_eq_paint(self, primitives, bounds, theme);
    }

    fn append_runtime_overlay_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        theme: &ThemeTokens,
    ) {
        let Some(position) = self.hover_position else {
            return;
        };
        if !bounds.contains(position) {
            return;
        }
        let plot = self.plot_rect(bounds);
        if !plot.contains(position) {
            return;
        }
        append_hover_crosshair(self, primitives, plot, position, theme);
    }
}

impl EqEditorWidget {
    fn handle_pointer_move(
        &mut self,
        bounds: Rect,
        plot: Rect,
        position: Point,
    ) -> Option<WidgetOutput> {
        self.common.state.hovered = bounds.contains(position);
        self.hover_position = bounds.contains(position).then_some(position);
        if let Some(id) = self.drag_band {
            return Some(move_band_output(id, plot, position));
        }
        self.hover_band = self.band_at_position(plot, position);
        None
    }

    fn handle_primary_press(&mut self, plot: Rect, position: Point) -> Option<WidgetOutput> {
        let id = self
            .band_at_position(plot, position)
            .unwrap_or(self.selected_band);
        self.drag_band = Some(id);
        self.selected_band = id;
        self.hover_band = Some(id);
        Some(WidgetOutput::custom(EqEditorMessage::SelectBand(id)))
    }

    fn finish_primary_drag(
        &mut self,
        bounds: Rect,
        plot: Rect,
        position: Point,
    ) -> Option<WidgetOutput> {
        let drag = self.drag_band.take();
        self.hover_band = bounds
            .contains(position)
            .then(|| self.band_at_position(plot, position))
            .flatten();
        drag.map(|id| move_band_output(id, plot, position))
    }
}

fn move_band_output(id: u32, plot: Rect, position: Point) -> WidgetOutput {
    WidgetOutput::custom(EqEditorMessage::MoveBand {
        id,
        freq_hz: geometry::freq_for_x(plot, position.x),
        gain_db: geometry::gain_for_y(plot, position.y),
    })
}

fn append_hover_crosshair(
    widget: &EqEditorWidget,
    primitives: &mut Vec<PaintPrimitive>,
    plot: Rect,
    position: Point,
    theme: &ThemeTokens,
) {
    paint::push_rect(
        primitives,
        widget.common.id,
        Rect::from_min_max(
            Point::new(position.x, plot.min.y),
            Point::new(position.x + 1.0, plot.max.y),
        ),
        translucent(theme.text_muted, 110),
    );
    paint::push_rect(
        primitives,
        widget.common.id,
        Rect::from_min_max(
            Point::new(plot.min.x, position.y),
            Point::new(plot.max.x, position.y + 1.0),
        ),
        translucent(theme.text_muted, 80),
    );
}

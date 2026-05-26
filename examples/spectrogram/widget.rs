use radiant::prelude::*;
use radiant::widgets::PaintBounds;

#[path = "widget/paint.rs"]
mod paint;

use super::model::SpectralColumn;

#[derive(Clone, Debug)]
pub(super) struct SpectrogramWidget {
    common: WidgetCommon,
    columns: Vec<SpectralColumn>,
    frame: u64,
    pub(super) hover_column: Option<usize>,
    hover_position: Option<Point>,
}

impl SpectrogramWidget {
    pub(super) fn new(columns: Vec<SpectralColumn>, frame: u64) -> Self {
        let mut common = WidgetCommon::new(
            0,
            WidgetSizing::new(Vector2::new(560.0, 280.0), Vector2::new(940.0, 320.0)),
        );
        common.focus = FocusBehavior::Pointer;
        common.paint.bounds = PaintBounds::ClipToRect;
        common.paint.paints_focus = false;
        common.paint.paints_state_layers = false;
        Self {
            common,
            columns,
            frame,
            hover_column: None,
            hover_position: None,
        }
    }

    pub(super) fn plot_rect(&self, bounds: Rect) -> Rect {
        bounds.inset(54.0, 18.0, 18.0, 36.0)
    }

    fn column_at_position(&self, plot: Rect, position: Point) -> Option<usize> {
        if !plot.contains(position) || self.columns.is_empty() {
            return None;
        }
        let ratio = plot.ratio_for_x(position.x).min(0.999);
        Some((ratio * self.columns.len() as f32).floor() as usize)
    }
}

impl Widget for SpectrogramWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        match input {
            WidgetInput::PointerMove { position } => {
                self.common.state.hovered = bounds.contains(position);
                let plot = self.plot_rect(bounds);
                self.hover_column = self.column_at_position(plot, position);
                self.hover_position = plot.contains(position).then_some(position);
                None
            }
            WidgetInput::PointerDrop { .. } => {
                self.hover_column = None;
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
            self.hover_column = previous.hover_column;
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
        let plot = self.plot_rect(bounds);
        paint::append_base(
            primitives,
            paint::SpectrogramPaintFrame {
                widget_id: self.common.id,
                bounds,
                plot,
                frame: self.frame,
                columns: &self.columns,
            },
            theme,
        );
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
        let plot = self.plot_rect(bounds);
        if plot.contains(position) {
            paint::append_hover(primitives, self.common.id, plot, position.x, theme);
        }
    }
}

#[cfg(test)]
pub(super) fn visible_bin_count() -> usize {
    paint::visible_bin_count()
}

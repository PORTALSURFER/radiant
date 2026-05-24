use super::model::{EqBand, EqEditorMessage};
#[path = "widget/geometry.rs"]
mod geometry;
#[path = "widget/paint.rs"]
mod paint;
#[path = "widget/response.rs"]
mod response;

use radiant::prelude::*;
use radiant::widgets::PaintBounds;

pub(super) use geometry::{x_for_freq, y_for_gain};
pub(super) use response::response_gain_db;

const HANDLE_SIZE: f32 = 12.0;

#[derive(Clone, Debug)]
pub(super) struct EqEditorWidget {
    common: WidgetCommon,
    bands: Vec<EqBand>,
    selected_band: u32,
    analyzer: bool,
    hover_band: Option<u32>,
    drag_band: Option<u32>,
    hover_position: Option<Point>,
}

impl EqEditorWidget {
    pub(super) fn new(bands: Vec<EqBand>, selected_band: u32, analyzer: bool) -> Self {
        let mut common = WidgetCommon::new(
            0,
            WidgetSizing::new(Vector2::new(520.0, 260.0), Vector2::new(880.0, 300.0)),
        );
        common.focus = FocusBehavior::Pointer;
        common.paint.bounds = PaintBounds::ClipToRect;
        common.paint.paints_focus = false;
        common.paint.paints_state_layers = false;
        Self {
            common,
            bands,
            selected_band,
            analyzer,
            hover_band: None,
            drag_band: None,
            hover_position: None,
        }
    }

    pub(super) fn plot_rect(&self, bounds: Rect) -> Rect {
        Rect::from_min_max(
            Point::new(bounds.min.x + 48.0, bounds.min.y + 22.0),
            Point::new(bounds.max.x - 18.0, bounds.max.y - 34.0),
        )
    }

    pub(super) fn handle_center(&self, plot: Rect, band: EqBand) -> Point {
        Point::new(
            x_for_freq(plot, band.freq_hz),
            y_for_gain(plot, band.gain_db),
        )
    }

    fn band_at_position(&self, plot: Rect, position: Point) -> Option<u32> {
        self.bands
            .iter()
            .filter(|band| band.enabled)
            .map(|band| {
                let center = self.handle_center(plot, *band);
                let dx = center.x - position.x;
                let dy = center.y - position.y;
                (band.id, dx * dx + dy * dy)
            })
            .filter(|(_, distance)| *distance <= 18.0 * 18.0)
            .min_by(|(_, a), (_, b)| a.total_cmp(b))
            .map(|(id, _)| id)
    }
}

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
                self.common.state.hovered = bounds.contains(position);
                self.hover_position = bounds.contains(position).then_some(position);
                if let Some(id) = self.drag_band {
                    return Some(WidgetOutput::custom(EqEditorMessage::MoveBand {
                        id,
                        freq_hz: geometry::freq_for_x(plot, position.x),
                        gain_db: geometry::gain_for_y(plot, position.y),
                    }));
                }
                self.hover_band = self.band_at_position(plot, position);
                None
            }
            WidgetInput::PointerPress {
                position,
                button: PointerButton::Primary,
                ..
            } if bounds.contains(position) => {
                let id = self
                    .band_at_position(plot, position)
                    .unwrap_or(self.selected_band);
                self.drag_band = Some(id);
                self.selected_band = id;
                self.hover_band = Some(id);
                Some(WidgetOutput::custom(EqEditorMessage::SelectBand(id)))
            }
            WidgetInput::PointerRelease {
                position,
                button: PointerButton::Primary,
                ..
            }
            | WidgetInput::PointerDrop {
                position,
                button: PointerButton::Primary,
                ..
            } => {
                let drag = self.drag_band.take();
                self.hover_band = bounds
                    .contains(position)
                    .then(|| self.band_at_position(plot, position))
                    .flatten();
                drag.map(|id| {
                    WidgetOutput::custom(EqEditorMessage::MoveBand {
                        id,
                        freq_hz: geometry::freq_for_x(plot, position.x),
                        gain_db: geometry::gain_for_y(plot, position.y),
                    })
                })
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
        if plot.contains(position) {
            paint::push_rect(
                primitives,
                self.common.id,
                Rect::from_min_max(
                    Point::new(position.x, plot.min.y),
                    Point::new(position.x + 1.0, plot.max.y),
                ),
                translucent(theme.text_muted, 110),
            );
            paint::push_rect(
                primitives,
                self.common.id,
                Rect::from_min_max(
                    Point::new(plot.min.x, position.y),
                    Point::new(plot.max.x, position.y + 1.0),
                ),
                translucent(theme.text_muted, 80),
            );
        }
    }
}

fn translucent(mut color: Rgba8, alpha: u8) -> Rgba8 {
    color.a = alpha;
    color
}

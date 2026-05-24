use super::model::EqBand;
#[path = "widget/geometry.rs"]
mod geometry;
#[path = "widget/input.rs"]
mod input;
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

fn translucent(mut color: Rgba8, alpha: u8) -> Rgba8 {
    color.a = alpha;
    color
}

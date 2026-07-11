//! Efficient gradient fill beneath a sampled curve using one vector path.

use radiant::prelude::*;
use radiant::{
    gui::visualization::{
        SampledCurveAreaBaseline, SampledCurveAreaFillParts, SampledCurveStrokeParts,
        push_sampled_curve_area_fill, push_sampled_curve_stroke,
    },
    runtime::{PaintBrush, PaintLinearGradient, PaintPrimitive, push_fill_rect, push_stroke_rect},
    widgets::PaintBounds,
};

const CURVE_WIDGET_ID: u64 = 10;
const CURVE_STEPS: usize = 192;

#[derive(Clone)]
struct CurveAreaWidget {
    common: WidgetCommon,
}

impl CurveAreaWidget {
    fn new() -> Self {
        let mut common = WidgetCommon::fixed(0, 640.0, 280.0);
        common.paint.bounds = PaintBounds::ClipToRect;
        common.paint.paints_focus = false;
        common.paint.paints_state_layers = false;
        Self { common }
    }
}

impl Widget for CurveAreaWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, _bounds: Rect, _input: WidgetInput) -> Option<WidgetOutput> {
        None
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        theme: &ThemeTokens,
    ) {
        push_fill_rect(primitives, self.common.id, bounds, theme.bg_secondary);
        let plot = bounds.inset(28.0, 20.0, 28.0, 24.0);
        push_fill_rect(primitives, self.common.id, plot, theme.surface_base);

        for step in 1..8 {
            let x = plot.min.x + plot.width() * step as f32 / 8.0;
            push_fill_rect(
                primitives,
                self.common.id,
                Rect::from_xy_size(x, plot.min.y, 1.0, plot.height()),
                theme.grid_soft,
            );
        }
        for step in 1..4 {
            let y = plot.min.y + plot.height() * step as f32 / 4.0;
            push_fill_rect(
                primitives,
                self.common.id,
                Rect::from_xy_size(plot.min.x, y, plot.width(), 1.0),
                theme.grid_soft,
            );
        }

        let gradient = PaintLinearGradient::vertical(
            plot,
            theme.accent_mint.with_alpha(112),
            theme.accent_mint.with_alpha(8),
        );
        push_sampled_curve_area_fill(
            primitives,
            SampledCurveAreaFillParts::new(
                self.common.id,
                plot,
                CURVE_STEPS,
                SampledCurveAreaBaseline::Bottom,
                PaintBrush::linear_gradient(gradient),
            ),
            |t| Some(curve_point(plot, t)),
        );
        push_sampled_curve_stroke(
            primitives,
            SampledCurveStrokeParts::new(self.common.id, plot, CURVE_STEPS, theme.accent_mint, 2.5),
            |t| Some(curve_point(plot, t)),
        );
        push_stroke_rect(primitives, self.common.id, plot, theme.border_emphasis, 1.0);
    }
}

fn curve_point(bounds: Rect, t: f32) -> Point {
    let attack = (-8.0 * t).exp();
    let recovery = t.powf(0.65);
    let ripple = (t * std::f32::consts::TAU * 1.6).sin() * (1.0 - t) * 0.035;
    let value = (0.12 + attack * 0.82 + recovery * 0.72 + ripple).clamp(0.0, 1.0);
    Point::new(
        bounds.min.x + bounds.width() * t,
        bounds.max.y - bounds.height() * value,
    )
}

fn main() -> radiant::Result {
    radiant::window("Radiant Curve Area Fill")
        .size(760, 420)
        .min_size(560, 340)
        .run(
            column([
                text("Sampled curve area fill"),
                text("One path, one gradient brush, constant renderer submission cost.")
                    .muted_text(),
                custom_widget_direct(CurveAreaWidget::new())
                    .id(CURVE_WIDGET_ID)
                    .fill(),
            ])
            .padding(24.0)
            .spacing(12.0),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn example_emits_one_gradient_fill_path_and_one_curve_stroke() {
        let widget = CurveAreaWidget::new();
        let bounds = Rect::from_xy_size(0.0, 0.0, 640.0, 280.0);
        let mut primitives = Vec::new();
        widget.append_paint(
            &mut primitives,
            bounds,
            &LayoutOutput::default(),
            &ThemeTokens::default(),
        );

        assert_eq!(
            primitives
                .iter()
                .filter(|primitive| primitive.fill_path().is_some())
                .count(),
            1
        );
        assert_eq!(
            primitives
                .iter()
                .filter(|primitive| primitive.stroke_polyline().is_some())
                .count(),
            1
        );
    }
}

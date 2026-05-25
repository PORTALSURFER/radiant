//! Slider paint command generation.

use crate::gui::types::{Point, Rect};
use crate::runtime::{PaintFillRect, PaintPrimitive, PaintStrokeRect};
use crate::theme::ThemeTokens;
use crate::widgets::primitives::slider::{SliderWidget, geometry::track_rect};

pub(super) fn push_slider_widget_paint(
    primitives: &mut Vec<PaintPrimitive>,
    slider: &SliderWidget,
    bounds: Rect,
    theme: &ThemeTokens,
) {
    let track = track_rect(bounds);
    let tokens = crate::widgets::resolve_widget_visual_tokens(
        theme,
        slider.common.style,
        slider.common.state,
    );
    primitives.push(PaintPrimitive::FillRect(PaintFillRect {
        widget_id: slider.common.id,
        rect: track,
        color: theme.bg_tertiary,
    }));
    let fill_width = (track.width() * slider.state.value.clamp(0.0, 1.0)).clamp(0.0, track.width());
    primitives.push(PaintPrimitive::FillRect(PaintFillRect {
        widget_id: slider.common.id,
        rect: Rect::from_min_max(track.min, Point::new(track.min.x + fill_width, track.max.y)),
        color: tokens.emphasis,
    }));
    if slider.common.state.focused && slider.common.paint.paints_focus {
        primitives.push(PaintPrimitive::StrokeRect(PaintStrokeRect {
            widget_id: slider.common.id,
            rect: bounds,
            color: tokens.emphasis,
            width: 1.0,
        }));
    }
}

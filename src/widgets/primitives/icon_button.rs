//! Reusable SVG icon button primitive.

use crate::gui::{svg::SvgIcon, types::Rect};
use crate::layout::LayoutOutput;
use crate::runtime::{
    PaintFillPolygon, PaintPrimitive, PaintStrokePolygon, diagonal_cut_rect_points, inset_rect,
};
use crate::theme::ThemeTokens;
use crate::widgets::contract::{FocusBehavior, PaintBounds, Widget, WidgetId, WidgetSizing};
use crate::widgets::interaction::{
    ActivationInputPolicy, ButtonMessage, WidgetInput, WidgetOutput, handle_activation_input,
};
use crate::widgets::primitives::support::{WidgetCommon, push_button_chrome};

mod builders;

/// Public SVG icon button primitive.
#[derive(Clone, Debug)]
pub struct IconButtonWidget {
    /// Shared widget contract.
    pub common: WidgetCommon,
    /// Retained icon painted in the button bounds.
    pub icon: SvgIcon,
}

/// Named construction fields for [`IconButtonWidget`].
#[derive(Clone, Debug)]
pub struct IconButtonWidgetParts {
    /// Stable widget identity used by layout, events, and state synchronization.
    pub id: WidgetId,
    /// Retained icon painted in the button bounds.
    pub icon: SvgIcon,
    /// Intrinsic icon-button sizing contract.
    pub sizing: WidgetSizing,
}

impl IconButtonWidget {
    /// Build an SVG icon button descriptor from named identity, icon, and sizing fields.
    pub fn from_parts(parts: IconButtonWidgetParts) -> Self {
        let mut common = WidgetCommon::new(parts.id, parts.sizing);
        common.focus = FocusBehavior::Keyboard;
        common.paint.bounds = PaintBounds::ClipToRect;
        common.paint.paints_focus = false;
        Self {
            common,
            icon: parts.icon,
        }
    }

    /// Build an SVG icon button descriptor.
    pub fn new(id: WidgetId, icon: SvgIcon, sizing: WidgetSizing) -> Self {
        Self::from_parts(IconButtonWidgetParts { id, icon, sizing })
    }
}

impl Widget for IconButtonWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        handle_activation_input(
            &mut self.common.state,
            bounds,
            &input,
            ActivationInputPolicy::focusable(),
        )
        .activated()
        .then(|| WidgetOutput::typed(ButtonMessage::Activate))
    }

    fn accepts_pointer_move(&self) -> bool {
        true
    }

    fn synchronize_from_previous(&mut self, previous: &dyn Widget) {
        if let Some(previous) = previous.as_any().downcast_ref::<Self>() {
            self.common.state = previous.common.state;
        }
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        theme: &ThemeTokens,
    ) {
        push_button_chrome(primitives, &self.common, bounds, theme);
        if !self.common.state.disabled && (self.common.state.hovered || self.common.state.pressed) {
            let mut fill = theme.accent_danger;
            fill.a = if self.common.state.pressed { 92 } else { 48 };
            let mut border = theme.accent_danger;
            border.a = if self.common.state.pressed { 240 } else { 180 };
            let points = diagonal_cut_rect_points(inset_rect(bounds, 1.0, 1.0));
            primitives.push(PaintPrimitive::FillPolygon(PaintFillPolygon {
                widget_id: self.common.id,
                points: std::sync::Arc::clone(&points),
                color: fill,
            }));
            primitives.push(PaintPrimitive::StrokePolygon(PaintStrokePolygon {
                widget_id: self.common.id,
                points,
                color: border,
                width: 1.0,
            }));
        }
        let side = bounds.width().min(bounds.height()).clamp(8.0, 16.0);
        let rect = inset_rect(
            bounds,
            (bounds.width() - side) * 0.5,
            (bounds.height() - side) * 0.5,
        );
        self.icon.append_paint(primitives, self.common.id, rect);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        gui::{svg::SvgIcon, types::Point},
        runtime::PaintPrimitive,
        widgets::{PointerButton, WidgetInput},
    };

    #[test]
    fn icon_button_paints_clear_hover_and_pressed_feedback() {
        let icon = SvgIcon::from_svg(
            r##"<svg viewBox="0 0 16 16" xmlns="http://www.w3.org/2000/svg">
  <rect fill="#ffffff" x="4" y="4" width="8" height="8"/>
</svg>"##,
        )
        .expect("valid icon");
        let bounds = Rect::from_min_size(
            Point::new(0.0, 0.0),
            crate::layout::Vector2::new(28.0, 24.0),
        );
        let theme = ThemeTokens::default();
        let mut widget = IconButtonWidget::new(
            101,
            icon,
            WidgetSizing::fixed(crate::layout::Vector2::new(28.0, 24.0)),
        );

        widget.handle_input(
            bounds,
            WidgetInput::PointerMove {
                position: Point::new(10.0, 10.0),
            },
        );
        let mut hover = Vec::new();
        widget.append_paint(&mut hover, bounds, &Default::default(), &theme);

        widget.handle_input(
            bounds,
            WidgetInput::PointerPress {
                position: Point::new(10.0, 10.0),
                button: PointerButton::Primary,
                modifiers: Default::default(),
            },
        );
        let mut pressed = Vec::new();
        widget.append_paint(&mut pressed, bounds, &Default::default(), &theme);

        let hover_overlay = accent_overlay_alpha(&hover, theme.accent_danger)
            .expect("hover should paint an accent overlay");
        let pressed_overlay = accent_overlay_alpha(&pressed, theme.accent_danger)
            .expect("pressed should paint an accent overlay");
        assert!(pressed_overlay > hover_overlay);
    }

    fn accent_overlay_alpha(
        primitives: &[PaintPrimitive],
        accent: crate::gui::types::Rgba8,
    ) -> Option<u8> {
        primitives.iter().find_map(|primitive| match primitive {
            PaintPrimitive::FillPolygon(fill)
                if fill.color.r == accent.r
                    && fill.color.g == accent.g
                    && fill.color.b == accent.b =>
            {
                Some(fill.color.a)
            }
            _ => None,
        })
    }
}

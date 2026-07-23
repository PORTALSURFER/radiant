//! Reusable SVG icon button primitive.

use crate::gui::{svg::SvgIcon, types::Rect};
use crate::layout::LayoutOutput;
use crate::runtime::{PaintPrimitive, inset_rect};
use crate::theme::ThemeTokens;
use crate::widgets::contract::{FocusBehavior, PaintBounds, Widget, WidgetId, WidgetSizing};
use crate::widgets::interaction::{
    ActivationInputPolicy, ButtonMessage, PointerButton, WidgetInput, WidgetOutput,
    handle_activation_input,
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
    /// Optional retained icon painted while hovered or pressed.
    pub hover_icon: Option<SvgIcon>,
    /// Whether the button paints standard button chrome behind the icon.
    pub chrome: IconButtonChrome,
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

/// Chrome treatment for an icon button.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IconButtonChrome {
    /// Paint the usual button fill, border, and hover/pressed overlay.
    Standard,
    /// Paint only the icon while preserving normal hit testing and activation.
    Bare,
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
            hover_icon: None,
            chrome: IconButtonChrome::Standard,
        }
    }

    /// Build an SVG icon button descriptor.
    pub fn new(id: WidgetId, icon: SvgIcon, sizing: WidgetSizing) -> Self {
        Self::from_parts(IconButtonWidgetParts { id, icon, sizing })
    }

    /// Return this icon button without the standard button fill or border.
    pub fn bare(mut self) -> Self {
        self.chrome = IconButtonChrome::Bare;
        self
    }

    /// Use a different retained icon while hovered or pressed.
    pub fn with_hover_icon(mut self, icon: SvgIcon) -> Self {
        self.hover_icon = Some(icon);
        self
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
        let result = handle_activation_input(
            &mut self.common.state,
            bounds,
            &input,
            ActivationInputPolicy::focusable(),
        );
        result.activated().then(|| {
            WidgetOutput::typed(match input {
                WidgetInput::PointerRelease {
                    button: PointerButton::Primary,
                    modifiers,
                    ..
                } => ButtonMessage::ActivateWithModifiers { modifiers },
                _ => ButtonMessage::Activate,
            })
        })
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
        if self.chrome == IconButtonChrome::Standard {
            push_button_chrome(primitives, &self.common, bounds, theme);
        }
        let side = bounds.width().min(bounds.height()).clamp(8.0, 16.0);
        let rect = inset_rect(
            bounds,
            (bounds.width() - side) * 0.5,
            (bounds.height() - side) * 0.5,
        );
        let icon = if !self.common.state.disabled
            && (self.common.state.hovered || self.common.state.pressed)
        {
            self.hover_icon.as_ref().unwrap_or(&self.icon)
        } else {
            &self.icon
        };
        icon.append_paint(primitives, self.common.id, rect);
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
    fn icon_button_uses_single_semantic_chrome_layer_for_interaction_states() {
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
        let hover_tokens = crate::widgets::resolve_widget_visual_tokens(
            &theme,
            widget.common.style,
            widget.common.state,
        );

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
        let pressed_tokens = crate::widgets::resolve_widget_visual_tokens(
            &theme,
            widget.common.style,
            widget.common.state,
        );

        assert_eq!(
            chrome_colors(&hover),
            (hover_tokens.fill, hover_tokens.border)
        );
        assert_eq!(
            chrome_colors(&pressed),
            (pressed_tokens.fill, pressed_tokens.border)
        );
        assert_ne!(hover_tokens.fill, pressed_tokens.fill);
    }

    #[test]
    fn bare_icon_button_paints_icon_without_button_chrome() {
        let icon = SvgIcon::from_svg(
            r##"<svg viewBox="0 0 16 16" xmlns="http://www.w3.org/2000/svg">
  <path fill="#ffffff" d="M6 4h4v8H6z"/>
</svg>"##,
        )
        .expect("valid icon");
        let bounds = Rect::from_min_size(
            Point::new(0.0, 0.0),
            crate::layout::Vector2::new(16.0, 18.0),
        );
        let widget = IconButtonWidget::new(
            102,
            icon,
            WidgetSizing::fixed(crate::layout::Vector2::new(16.0, 18.0)),
        )
        .bare();
        let mut primitives = Vec::new();

        widget.append_paint(
            &mut primitives,
            bounds,
            &Default::default(),
            &ThemeTokens::default(),
        );

        assert!(
            primitives
                .iter()
                .any(|primitive| matches!(primitive, PaintPrimitive::Svg(_))),
            "bare icon button should still paint the icon"
        );
        assert!(
            !primitives.iter().any(|primitive| matches!(
                primitive,
                PaintPrimitive::FillPolygon(_) | PaintPrimitive::StrokePolygon(_)
            )),
            "bare icon button should not paint button chrome"
        );
    }

    #[test]
    fn bare_icon_button_can_swap_fill_icon_on_hover_without_extra_chrome() {
        let idle_icon = SvgIcon::from_svg(
            r##"<svg viewBox="0 0 16 16" xmlns="http://www.w3.org/2000/svg">
  <circle fill="#999999" cx="8" cy="8" r="4"/>
</svg>"##,
        )
        .expect("valid idle icon");
        let hover_icon = SvgIcon::from_svg(
            r##"<svg viewBox="0 0 16 16" xmlns="http://www.w3.org/2000/svg">
  <circle fill="#ff6655" cx="8" cy="8" r="4"/>
</svg>"##,
        )
        .expect("valid hover icon");
        let bounds = Rect::from_min_size(
            Point::new(0.0, 0.0),
            crate::layout::Vector2::new(22.0, 18.0),
        );
        let theme = ThemeTokens::default();
        let mut widget = IconButtonWidget::new(
            103,
            idle_icon,
            WidgetSizing::fixed(crate::layout::Vector2::new(22.0, 18.0)),
        )
        .bare()
        .with_hover_icon(hover_icon);
        let mut idle = Vec::new();
        widget.append_paint(&mut idle, bounds, &Default::default(), &theme);

        widget.handle_input(
            bounds,
            WidgetInput::PointerMove {
                position: Point::new(11.0, 9.0),
            },
        );
        let mut hovered = Vec::new();
        widget.append_paint(&mut hovered, bounds, &Default::default(), &theme);

        let idle_svg = idle
            .iter()
            .find_map(|primitive| match primitive {
                PaintPrimitive::Svg(svg) => Some(&svg.document),
                _ => None,
            })
            .expect("idle icon paint");
        let hover_svg = hovered
            .iter()
            .find_map(|primitive| match primitive {
                PaintPrimitive::Svg(svg) => Some(&svg.document),
                _ => None,
            })
            .expect("hover icon paint");
        assert_ne!(idle_svg, hover_svg);
        assert!(!hovered.iter().any(|primitive| matches!(
            primitive,
            PaintPrimitive::FillPolygon(_) | PaintPrimitive::StrokePolygon(_)
        )));
    }

    fn chrome_colors(
        primitives: &[PaintPrimitive],
    ) -> (crate::gui::types::Rgba8, crate::gui::types::Rgba8) {
        let fills = primitives
            .iter()
            .filter_map(|primitive| match primitive {
                PaintPrimitive::FillPolygon(fill) => Some(fill.color),
                _ => None,
            })
            .collect::<Vec<_>>();
        let strokes = primitives
            .iter()
            .filter_map(|primitive| match primitive {
                PaintPrimitive::StrokePolygon(stroke) => Some(stroke.color),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(fills.len(), 1, "icon button should paint one chrome fill");
        assert_eq!(
            strokes.len(),
            1,
            "icon button should paint one chrome border"
        );
        (fills[0], strokes[0])
    }
}

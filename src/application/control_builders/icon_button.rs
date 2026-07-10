use std::sync::OnceLock;

use crate::{
    application::{MappedWidget, ViewNode, primary_style, view_node_from_widget},
    gui::svg::SvgIcon,
    runtime::WidgetMessageMapper,
    widgets::{ButtonMessage, IconButtonWidget, WidgetProminence, WidgetSizing, WidgetStyle},
};

const CLOSE_ICON_SVG: &str = r##"<svg viewBox="0 0 16 16" xmlns="http://www.w3.org/2000/svg">
  <path fill="#eeeeee" d="M4.2 3.1 8 6.9l3.8-3.8 1.1 1.1L9.1 8l3.8 3.8-1.1 1.1L8 9.1l-3.8 3.8-1.1-1.1L6.9 8 3.1 4.2z"/>
</svg>"##;

const DISCLOSURE_OPEN_ICON_SVG: &str = r##"<svg viewBox="0 0 16 16" xmlns="http://www.w3.org/2000/svg">
  <path fill="#eeeeee" d="M3.5 5.6 8 10.1l4.5-4.5 1.1 1.1L8 12.3 2.4 6.7z"/>
</svg>"##;

const DISCLOSURE_CLOSED_ICON_SVG: &str = r##"<svg viewBox="0 0 16 16" xmlns="http://www.w3.org/2000/svg">
  <path fill="#eeeeee" d="M5.6 2.4 11.2 8l-5.6 5.6-1.1-1.1L9 8 4.5 3.5z"/>
</svg>"##;

/// Builder for compact SVG icon buttons.
pub struct IconButtonBuilder {
    icon: SvgIcon,
    style: Option<WidgetStyle>,
    enabled: bool,
    active: bool,
    bare: bool,
}

impl IconButtonBuilder {
    /// Apply an explicit widget style before binding this icon button.
    pub fn style(mut self, style: WidgetStyle) -> Self {
        self.style = Some(style);
        self
    }

    /// Use the accent tone and strong prominence.
    pub fn primary(self) -> Self {
        self.style(primary_style())
    }

    /// Use a lower-prominence treatment.
    pub fn subtle(mut self) -> Self {
        let mut style = self.style.unwrap_or_default();
        style.prominence = WidgetProminence::Subtle;
        self.style = Some(style);
        self
    }

    /// Set whether this button can be activated.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Set whether this button should paint as active.
    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    /// Paint only the retained icon while preserving hit testing and activation.
    pub fn bare(mut self) -> Self {
        self.bare = true;
        self
    }

    /// Build a passive icon button view without host messages.
    pub fn passive<Message: 'static>(self) -> ViewNode<Message> {
        let (widget, style) = self.into_widget_and_style();
        let mut node = view_node_from_widget(widget);
        node.style = style;
        node
    }

    /// Emit one cloned host message when activated.
    pub fn message<Message>(self, message: Message) -> ViewNode<Message>
    where
        Message: Clone + Send + Sync + 'static,
    {
        let (widget, style) = self.into_widget_and_style();
        let mut node = view_node_from_widget(MappedWidget::new(
            widget,
            WidgetMessageMapper::icon_button_message(message),
        ));
        node.style = style;
        node
    }

    /// Emit a mapped host message when activated.
    pub fn mapped<Message: 'static>(
        self,
        map: impl Fn(ButtonMessage) -> Message + Send + Sync + 'static,
    ) -> ViewNode<Message> {
        let (widget, style) = self.into_widget_and_style();
        let mut node = view_node_from_widget(MappedWidget::new(
            widget,
            WidgetMessageMapper::icon_button(map),
        ));
        node.style = style;
        node
    }

    fn into_widget_and_style(self) -> (IconButtonWidget, Option<WidgetStyle>) {
        let mut widget = IconButtonWidget::new(
            0,
            self.icon,
            WidgetSizing::fixed(crate::layout::Vector2::new(28.0, 24.0)),
        );
        if self.bare {
            widget = widget.bare();
        }
        widget.common.state.disabled = !self.enabled;
        widget.common.state.active = self.active;
        (widget, self.style)
    }
}

/// Build a compact SVG icon button.
pub fn icon_button(icon: SvgIcon) -> IconButtonBuilder {
    IconButtonBuilder {
        icon,
        style: None,
        enabled: true,
        active: false,
        bare: false,
    }
}

/// Build a standard compact close button.
pub fn close_button() -> IconButtonBuilder {
    icon_button(cached_icon(&CLOSE_ICON, CLOSE_ICON_SVG, "close"))
}

/// Build a standard compact disclosure button.
///
/// Pass `true` when the controlled section is expanded and `false` when it is
/// collapsed.
pub fn disclosure_button(expanded: bool) -> IconButtonBuilder {
    let (cache, svg, name) = if expanded {
        (
            &DISCLOSURE_OPEN_ICON,
            DISCLOSURE_OPEN_ICON_SVG,
            "open disclosure",
        )
    } else {
        (
            &DISCLOSURE_CLOSED_ICON,
            DISCLOSURE_CLOSED_ICON_SVG,
            "closed disclosure",
        )
    };
    icon_button(cached_icon(cache, svg, name))
}

static CLOSE_ICON: OnceLock<SvgIcon> = OnceLock::new();
static DISCLOSURE_OPEN_ICON: OnceLock<SvgIcon> = OnceLock::new();
static DISCLOSURE_CLOSED_ICON: OnceLock<SvgIcon> = OnceLock::new();

fn cached_icon(cache: &'static OnceLock<SvgIcon>, svg: &'static str, _name: &str) -> SvgIcon {
    cache
        .get_or_init(|| SvgIcon::from_svg(svg).unwrap_or_else(SvgIcon::empty))
        .clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        application::IntoView,
        gui::types::{Point, Rect},
        layout::{LayoutOutput, Vector2},
        runtime::{PaintPrimitive, UiSurface},
        widgets::{PointerButton, Widget, WidgetInput},
    };

    #[test]
    fn standard_icon_buttons_parse_and_paint_retained_svg() {
        for builder in [
            close_button(),
            disclosure_button(false),
            disclosure_button(true),
        ] {
            let widget = IconButtonWidget::new(
                101,
                builder.icon,
                WidgetSizing::fixed(Vector2::new(24.0, 20.0)),
            );
            let mut primitives = Vec::new();
            widget.append_paint(
                &mut primitives,
                Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(24.0, 20.0)),
                &LayoutOutput::default(),
                &Default::default(),
            );
            assert!(
                primitives
                    .iter()
                    .any(|primitive| matches!(primitive, PaintPrimitive::Svg(_))),
                "standard icon button should paint a retained SVG icon"
            );
        }
    }

    #[test]
    fn standard_icon_buttons_route_activation_messages() {
        let mut widget = IconButtonWidget::new(
            101,
            close_button().icon,
            WidgetSizing::fixed(Vector2::new(24.0, 20.0)),
        );
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(24.0, 20.0));
        widget.handle_input(
            bounds,
            WidgetInput::PointerPress {
                position: Point::new(12.0, 10.0),
                button: PointerButton::Primary,
                modifiers: Default::default(),
            },
        );
        let output = widget.handle_input(
            bounds,
            WidgetInput::PointerRelease {
                position: Point::new(12.0, 10.0),
                button: PointerButton::Primary,
                modifiers: Default::default(),
            },
        );
        assert!(output.is_some());
    }

    #[test]
    fn icon_button_builder_passive_paints_without_host_message() {
        let frame = UiSurface::new(
            disclosure_button(false)
                .subtle()
                .passive::<()>()
                .size(24.0, 20.0)
                .into_node(),
        )
        .frame(
            Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(24.0, 20.0)),
            &Default::default(),
        );

        assert!(
            frame
                .paint_plan
                .primitives
                .iter()
                .any(|primitive| matches!(primitive, PaintPrimitive::Svg(_))),
            "passive icon button should paint a retained SVG icon"
        );
    }
}

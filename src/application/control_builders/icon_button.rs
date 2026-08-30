use std::sync::OnceLock;

use crate::{
    application::{MappedWidget, ViewNode, primary_style, view_node_from_widget},
    gui::{svg::SvgIcon, types::Rect},
    layout::{LayoutOutput, Vector2},
    runtime::PaintPrimitive,
    runtime::WidgetMessageMapper,
    theme::ThemeTokens,
    widgets::{
        ButtonMessage, FocusBehavior, IconButtonWidget, Widget, WidgetCapabilities, WidgetCommon,
        WidgetInput, WidgetOutput, WidgetPointerMotion, WidgetPointerMotionRevision,
        WidgetProminence, WidgetSemantics, WidgetSemanticsRevision, WidgetSizing, WidgetStyle,
    },
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
    hover_icon: Option<SvgIcon>,
    focus: Option<FocusBehavior>,
    label: Option<String>,
}

/// Application-only semantic decorator for labeled icon buttons.
///
/// Keeping the label in this wrapper preserves source compatibility for
/// existing public `IconButtonWidget` struct literals.
#[derive(Clone, Debug)]
struct LabeledIconButtonWidget {
    widget: IconButtonWidget,
    label: String,
}

impl LabeledIconButtonWidget {
    fn new(widget: IconButtonWidget, label: String) -> Self {
        Self { widget, label }
    }
}

impl WidgetSemantics for LabeledIconButtonWidget {
    fn revision(&self) -> WidgetSemanticsRevision {
        WidgetSemanticsRevision::exact((
            self.label.clone(),
            self.widget.common.focus,
            self.widget.common.state.selected,
            self.widget.common.state.disabled,
            self.widget.common.state.read_only,
        ))
    }

    fn automation_role(&self) -> crate::gui::automation::AutomationRole {
        crate::gui::automation::AutomationRole::Button
    }

    fn automation_label(&self) -> Option<String> {
        Some(self.label.clone())
    }
}

impl WidgetPointerMotion for LabeledIconButtonWidget {
    fn revision(&self) -> WidgetPointerMotionRevision {
        WidgetPointerMotion::revision(&self.widget)
    }

    fn accepts_pointer_move(&self) -> bool {
        WidgetPointerMotion::accepts_pointer_move(&self.widget)
    }
}

impl Widget for LabeledIconButtonWidget {
    fn common(&self) -> &WidgetCommon {
        self.widget.common()
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        self.widget.common_mut()
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        self.widget.handle_input(bounds, input)
    }

    fn synchronize_from_previous(&mut self, previous: &dyn Widget) {
        if let Some(previous) = previous.as_any().downcast_ref::<Self>() {
            self.widget.synchronize_from_previous(&previous.widget);
        }
    }

    fn capabilities(&self) -> WidgetCapabilities<'_> {
        WidgetCapabilities::new()
            .semantics(self)
            .pointer_motion(self)
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        layout: &LayoutOutput,
        theme: &ThemeTokens,
    ) {
        self.widget.append_paint(primitives, bounds, layout, theme);
    }
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

    /// Override how this icon button participates in keyboard/pointer focus.
    pub fn focus(mut self, focus: FocusBehavior) -> Self {
        self.focus = Some(focus);
        self
    }

    /// Set the exact label exposed through the icon button's automation semantics.
    ///
    /// This semantic label is independent of any visual hover tooltip attached
    /// to the resulting view.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Paint only the retained icon while preserving hit testing and activation.
    pub fn bare(mut self) -> Self {
        self.bare = true;
        self
    }

    /// Swap to a retained icon variant while hovered or pressed.
    pub fn hover_icon(mut self, icon: SvgIcon) -> Self {
        self.hover_icon = Some(icon);
        self
    }

    /// Build a passive icon button view without host messages.
    pub fn passive<Message: 'static>(self) -> ViewNode<Message> {
        let (widget, label, style) = self.into_widget_and_style();
        let mut node = match label {
            Some(label) => view_node_from_widget(LabeledIconButtonWidget::new(widget, label)),
            None => view_node_from_widget(widget),
        };
        node.style = style;
        node
    }

    /// Emit one cloned host message when activated.
    pub fn message<Message>(self, message: Message) -> ViewNode<Message>
    where
        Message: Clone + 'static,
    {
        let (widget, label, style) = self.into_widget_and_style();
        let messages = WidgetMessageMapper::icon_button_message(message);
        let mut node = match label {
            Some(label) => view_node_from_widget(MappedWidget::new(
                LabeledIconButtonWidget::new(widget, label),
                messages,
            )),
            None => view_node_from_widget(MappedWidget::new(widget, messages)),
        };
        node.style = style;
        node
    }

    /// Emit a mapped host message when activated.
    pub fn mapped<Message: 'static>(
        self,
        map: impl Fn(ButtonMessage) -> Message + 'static,
    ) -> ViewNode<Message> {
        let (widget, label, style) = self.into_widget_and_style();
        let messages = WidgetMessageMapper::icon_button(map);
        let mut node = match label {
            Some(label) => view_node_from_widget(MappedWidget::new(
                LabeledIconButtonWidget::new(widget, label),
                messages,
            )),
            None => view_node_from_widget(MappedWidget::new(widget, messages)),
        };
        node.style = style;
        node
    }

    fn into_widget_and_style(self) -> (IconButtonWidget, Option<String>, Option<WidgetStyle>) {
        let mut widget =
            IconButtonWidget::new(0, self.icon, WidgetSizing::fixed(Vector2::new(28.0, 24.0)));
        if self.bare {
            widget = widget.bare();
        }
        if let Some(hover_icon) = self.hover_icon {
            widget = widget.with_hover_icon(hover_icon);
        }
        if let Some(focus) = self.focus {
            widget.common.focus = focus;
        }
        widget.common.state.disabled = !self.enabled;
        widget.common.state.active = self.active;
        (widget, self.label, self.style)
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
        hover_icon: None,
        focus: None,
        label: None,
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
        gui::{
            input::InputTimestamp,
            types::{Point, Rect},
        },
        layout::{LayoutOutput, Vector2},
        runtime::{PaintPrimitive, UiSurface},
        widgets::{
            ButtonMessage, InteractionProvenance, KeyboardModifiers, PointerButton,
            PointerModifiers, Widget, WidgetInput, WidgetKey,
        },
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
        let mut widget = LabeledIconButtonWidget::new(
            IconButtonWidget::new(
                101,
                close_button().icon,
                WidgetSizing::fixed(Vector2::new(24.0, 20.0)),
            ),
            "Close".into(),
        );
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(24.0, 20.0));
        widget.handle_input(
            bounds,
            WidgetInput::PointerPress {
                position: Point::new(12.0, 10.0),
                button: PointerButton::Primary,
                modifiers: PointerModifiers {
                    command: true,
                    ..Default::default()
                },
                timestamp: Some(InputTimestamp::capture()),
            },
        );
        let release_modifiers = PointerModifiers {
            shift: true,
            alt: true,
            ..Default::default()
        };
        let release_timestamp = InputTimestamp::capture();
        let output = widget.handle_input(
            bounds,
            WidgetInput::PointerRelease {
                position: Point::new(12.0, 10.0),
                button: PointerButton::Primary,
                modifiers: release_modifiers,
                timestamp: Some(release_timestamp),
            },
        );
        assert_eq!(
            output.and_then(|output| output.typed_cloned::<ButtonMessage>()),
            Some(ButtonMessage::ActivateWithModifiers {
                provenance: InteractionProvenance::Pointer {
                    modifiers: release_modifiers,
                    timestamp: Some(release_timestamp),
                    sequence_range: None,
                },
            })
        );
    }

    #[test]
    fn icon_button_keyboard_activation_keeps_key_timestamp_and_synthetic_sources() {
        let icon = SvgIcon::from_svg(
            r##"<svg viewBox="0 0 16 16" xmlns="http://www.w3.org/2000/svg"><rect fill="#fff" x="4" y="4" width="8" height="8"/></svg>"##,
        )
        .expect("valid icon");
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(24.0, 20.0));
        let mut widget = LabeledIconButtonWidget::new(
            IconButtonWidget::new(
                109,
                icon.clone(),
                WidgetSizing::fixed(Vector2::new(24.0, 20.0)),
            ),
            "Activate icon button".into(),
        );
        widget.handle_input(bounds, WidgetInput::FocusChanged(true));

        for key in [WidgetKey::Enter, WidgetKey::Space] {
            let timestamp = InputTimestamp::capture();
            assert_eq!(
                widget
                    .handle_input(
                        bounds,
                        WidgetInput::KeyPress {
                            key,
                            modifiers: KeyboardModifiers::default(),
                            repeat: false,
                            timestamp: Some(timestamp),
                        },
                    )
                    .and_then(|output| output.typed_cloned::<ButtonMessage>()),
                Some(ButtonMessage::Activate {
                    provenance: InteractionProvenance::Keyboard {
                        timestamp: Some(timestamp),
                    },
                })
            );
        }

        let mut synthetic = LabeledIconButtonWidget::new(
            IconButtonWidget::new(110, icon, WidgetSizing::fixed(Vector2::new(24.0, 20.0))),
            "Synthetic icon button".into(),
        );
        assert_eq!(
            synthetic.handle_input(bounds, WidgetInput::primary_press(Point::new(12.0, 10.0)),),
            None
        );
        assert_eq!(
            synthetic
                .handle_input(bounds, WidgetInput::primary_release(Point::new(12.0, 10.0)),)
                .and_then(|output| output.typed_cloned::<ButtonMessage>()),
            Some(ButtonMessage::ActivateWithModifiers {
                provenance: InteractionProvenance::Pointer {
                    modifiers: PointerModifiers::default(),
                    timestamp: None,
                    sequence_range: None,
                },
            })
        );
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

//! Reusable button primitive.

mod builders;
mod input;
mod model;
mod paint;

use crate::gui::svg::{SvgIcon, SvgIconTintCache};
use crate::gui::types::Rect;
use crate::layout::LayoutOutput;
use crate::runtime::{PaintPrimitive, PaintText};
use crate::theme::ThemeTokens;

use super::support::{
    WidgetCommon,
    revision::{
        CommonInteractionRevision, CommonPaintRevision, common_geometry, common_interaction,
        common_paint, exact_revision,
    },
};
use crate::widgets::TextAlign;
use crate::widgets::contract::{
    FocusBehavior, Widget, WidgetCapabilities, WidgetId, WidgetRevision, WidgetSemantics,
    WidgetSemanticsRevision, WidgetSizing,
};
use crate::widgets::interaction::{ButtonMessage, WidgetInput, WidgetOutput};

pub use model::{ButtonProps, ButtonState};

/// Public button primitive.
#[derive(Clone, Debug, PartialEq)]
pub struct ButtonWidget {
    /// Shared widget contract.
    pub common: WidgetCommon,
    /// Immutable user-facing button configuration.
    pub props: ButtonProps,
    /// Mutable interaction state owned by the button.
    pub state: ButtonState,
    /// Optional retained SVG painted at the trailing edge of the button.
    pub trailing_icon: Option<SvgIcon>,
    /// Optional monochrome icon source tinted from the resolved foreground.
    pub trailing_icon_tint_cache: Option<&'static SvgIconTintCache>,
}

/// Named construction fields for a [`ButtonWidget`].
#[derive(Clone, Debug, PartialEq)]
pub struct ButtonWidgetParts {
    /// Stable widget id used by layout, paint, and input routing.
    pub id: WidgetId,
    /// User-facing button label.
    pub label: PaintText,
    /// Intrinsic sizing contract for the button.
    pub sizing: WidgetSizing,
}

impl ButtonWidget {
    /// Build a button descriptor from named parts.
    pub fn from_parts(parts: ButtonWidgetParts) -> Self {
        let mut common = WidgetCommon::new(parts.id, parts.sizing);
        common.focus = FocusBehavior::Keyboard;
        Self {
            common,
            props: ButtonProps {
                label: parts.label,
                trailing_label: None,
                text_align: TextAlign::Center,
                secondary_click: false,
                drag: false,
                hover_chrome_only: false,
            },
            state: ButtonState::default(),
            trailing_icon: None,
            trailing_icon_tint_cache: None,
        }
    }

    /// Build a button descriptor with keyboard focus and activation semantics.
    pub fn new(id: WidgetId, label: impl Into<PaintText>, sizing: WidgetSizing) -> Self {
        Self::from_parts(ButtonWidgetParts {
            id,
            label: label.into(),
            sizing,
        })
    }

    /// Enable secondary/right-click activation messages for this button.
    pub fn with_secondary_click(mut self) -> Self {
        self.props.secondary_click = true;
        self
    }

    /// Enable primary-pointer drag lifecycle messages from the button surface.
    pub fn with_drag(mut self) -> Self {
        self.props.drag = true;
        self
    }

    /// Paint button chrome only while hovered, pressed, or focused.
    pub fn with_hover_chrome_only(mut self) -> Self {
        self.props.hover_chrome_only = true;
        self
    }

    /// Add passive trailing text while preserving the main label storage.
    pub fn with_trailing_label(mut self, label: impl Into<PaintText>) -> Self {
        self.props.trailing_label = Some(label.into());
        self
    }

    /// Add a retained SVG icon at the trailing edge without using a text glyph.
    pub fn with_trailing_icon(mut self, icon: SvgIcon) -> Self {
        self.trailing_icon = Some(icon);
        self.trailing_icon_tint_cache = None;
        self
    }

    /// Add a monochrome retained SVG source whose color follows button state.
    pub fn with_trailing_icon_tint_cache(mut self, cache: &'static SvgIconTintCache) -> Self {
        self.trailing_icon = None;
        self.trailing_icon_tint_cache = Some(cache);
        self
    }

    /// Route one backend-neutral interaction into the button.
    ///
    /// The button emits [`ButtonMessage::Activate { provenance }`] when a
    /// primary press is released inside bounds or when the focused widget
    /// receives Enter/Space.
    pub fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<ButtonMessage> {
        input::handle_button_input(self, bounds, input)
    }
}

impl WidgetSemantics for ButtonWidget {
    fn revision(&self) -> WidgetSemanticsRevision {
        WidgetSemanticsRevision::exact((
            self.props.label.clone(),
            self.props.trailing_label.clone(),
            self.common.state.selected,
            self.common.state.disabled,
            self.common.state.read_only,
        ))
    }

    fn automation_role(&self) -> crate::gui::automation::AutomationRole {
        crate::gui::automation::AutomationRole::Button
    }

    fn automation_label(&self) -> Option<String> {
        Some(match self.props.trailing_label.as_ref() {
            Some(trailing) => format!("{} {}", self.props.label, trailing),
            None => self.props.label.as_str().to_owned(),
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ButtonPaintRevision {
    common: CommonPaintRevision,
    label: PaintText,
    trailing_label: Option<PaintText>,
    text_align: TextAlign,
    hover_chrome_only: bool,
    active: bool,
    selected: bool,
    disabled: bool,
    automation_active: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ButtonInteractionRevision {
    common: CommonInteractionRevision,
    secondary_click: bool,
    drag: bool,
    active: bool,
    paints_state_layers: bool,
    suppresses_container_hover: bool,
    disabled: bool,
    read_only: bool,
}

impl ButtonWidget {
    fn exact_revision(&self) -> Option<WidgetRevision> {
        // Retained SVGs may contain backend-owned or opaque paint state. Keep
        // ordinary icon buttons conservative until their representation has a
        // typed equality contract of its own.
        if self.trailing_icon.is_some() || self.trailing_icon_tint_cache.is_some() {
            return None;
        }

        exact_revision(
            common_geometry(&self.common),
            ButtonPaintRevision {
                common: common_paint(&self.common),
                label: self.props.label.clone(),
                trailing_label: self.props.trailing_label.clone(),
                text_align: self.props.text_align,
                hover_chrome_only: self.props.hover_chrome_only,
                active: self.common.state.active,
                selected: self.common.state.selected,
                disabled: self.common.state.disabled,
                automation_active: self.common.state.automation_active,
            },
            ButtonInteractionRevision {
                common: common_interaction(&self.common),
                secondary_click: self.props.secondary_click,
                drag: self.props.drag,
                active: self.common.state.active,
                paints_state_layers: self.common.paint.paints_state_layers,
                suppresses_container_hover: self.common.paint.suppresses_container_hover,
                disabled: self.common.state.disabled,
                read_only: self.common.state.read_only,
            },
        )
    }
}

impl Widget for ButtonWidget {
    fn revision(&self) -> WidgetRevision {
        self.exact_revision()
            .unwrap_or_else(WidgetRevision::conservative)
    }

    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        ButtonWidget::handle_input(self, bounds, input).map(WidgetOutput::typed)
    }

    fn synchronize_from_previous(&mut self, previous: &dyn Widget) {
        let Some(previous) = previous.as_any().downcast_ref::<Self>() else {
            return;
        };

        // Runtime-owned interaction state follows the retained widget across
        // declarative projections; fresh semantic state and props remain
        // authoritative on `self`.
        self.common.state.hovered = previous.common.state.hovered;
        self.common.state.pressed = previous.common.state.pressed;
        self.common.state.focused = previous.common.state.focused;
        self.state.armed = previous.state.armed;
        self.state.dragged = previous.state.dragged;
        self.state.press_position = previous.state.press_position;
    }

    fn accepts_pointer_move(&self) -> bool {
        false
    }

    fn capabilities(&self) -> WidgetCapabilities<'_> {
        WidgetCapabilities::new().semantics(self)
    }

    fn needs_state_synchronization(&self) -> bool {
        true
    }

    fn set_text_align(&mut self, align: TextAlign) -> bool {
        self.props.text_align = align;
        true
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        theme: &ThemeTokens,
    ) {
        paint::push_button_widget_paint(primitives, self, bounds, theme);
    }
}

#[cfg(test)]
mod tests;

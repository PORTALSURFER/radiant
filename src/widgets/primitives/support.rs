//! Shared widget support types plus non-interactive public primitives.

use crate::gui::types::Rect;
use crate::layout::LayoutNode;

use super::scrollbar::ScrollbarAxis;
use crate::widgets::contract::{
    FocusBehavior, PaintBounds, WidgetId, WidgetKind, WidgetProminence, WidgetSizing, WidgetState,
    WidgetStyle, WidgetTone,
};
use crate::widgets::interaction::{
    ListItemMessage, PointerButton, WidgetInput, WidgetKey, WidgetOutput,
};

/// Shared contract carried by every public widget descriptor.
#[derive(Clone, Debug, PartialEq)]
pub struct WidgetCommon {
    /// Stable widget identifier.
    pub id: WidgetId,
    /// Primitive taxonomy entry.
    pub kind: WidgetKind,
    /// Intrinsic sizing contract exposed to layout containers.
    pub sizing: WidgetSizing,
    /// Focus participation contract.
    pub focus: FocusBehavior,
    /// Paint responsibilities for this widget.
    pub paint: crate::widgets::contract::PaintContract,
    /// Shared style vocabulary independent from any app theme.
    pub style: WidgetStyle,
    /// Current interaction and visual state.
    pub state: WidgetState,
    /// Semantic event families this widget may emit.
    pub emitted_messages: Vec<crate::widgets::contract::WidgetMessageKind>,
}

impl WidgetCommon {
    /// Build a shared widget contract with neutral defaults.
    pub fn new(id: WidgetId, kind: WidgetKind, sizing: WidgetSizing) -> Self {
        Self {
            id,
            kind,
            sizing,
            focus: FocusBehavior::None,
            paint: Default::default(),
            style: WidgetStyle::default(),
            state: WidgetState::default(),
            emitted_messages: Vec::new(),
        }
    }

    /// Project this widget into the current public layout leaf representation.
    pub fn layout_node(&self) -> LayoutNode {
        self.sizing.layout_node(self.id)
    }
}

/// Text wrapping behavior for text-like widgets.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TextWrap {
    /// Keep text on one line and clip overflow.
    None,
    /// Wrap text to additional lines inside the assigned rect.
    Word,
}

/// Public label/text primitive.
#[derive(Clone, Debug, PartialEq)]
pub struct TextWidget {
    /// Shared widget contract.
    pub common: WidgetCommon,
    /// Displayed text content.
    pub text: String,
    /// Wrapping policy used for intrinsic sizing and paint.
    pub wrap: TextWrap,
}

impl TextWidget {
    /// Build a label/text widget with a preferred intrinsic size.
    pub fn new(id: WidgetId, text: impl Into<String>, sizing: WidgetSizing) -> Self {
        let mut common = WidgetCommon::new(id, WidgetKind::Text, sizing);
        common.paint.paints_focus = false;
        Self {
            common,
            text: text.into(),
            wrap: TextWrap::None,
        }
    }
}

/// Public list-row or list-item primitive.
#[derive(Clone, Debug, PartialEq)]
pub struct ListItemWidget {
    /// Shared widget contract.
    pub common: WidgetCommon,
    /// Primary row label.
    pub label: String,
    /// Optional secondary text.
    pub detail: Option<String>,
}

impl ListItemWidget {
    /// Build a list-item descriptor that can be focused, selected, and invoked.
    pub fn new(id: WidgetId, label: impl Into<String>, sizing: WidgetSizing) -> Self {
        let mut common = WidgetCommon::new(id, WidgetKind::ListItem, sizing);
        common.focus = FocusBehavior::Keyboard;
        common
            .emitted_messages
            .push(crate::widgets::contract::WidgetMessageKind::ItemInvoked);
        Self {
            common,
            label: label.into(),
            detail: None,
        }
    }

    /// Route one backend-neutral interaction into the list item.
    pub fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<ListItemMessage> {
        if self.common.state.disabled {
            return None;
        }

        match input {
            WidgetInput::PointerMove { position } => {
                self.common.state.hovered = bounds.contains(position);
                None
            }
            WidgetInput::PointerPress {
                position,
                button: PointerButton::Primary,
            } if bounds.contains(position) => {
                self.common.state.pressed = true;
                None
            }
            WidgetInput::PointerRelease {
                position,
                button: PointerButton::Primary,
            } => {
                let was_pressed = self.common.state.pressed;
                self.common.state.pressed = false;
                (was_pressed && bounds.contains(position)).then_some(ListItemMessage::Invoked)
            }
            WidgetInput::FocusChanged(focused) => {
                self.common.state.focused = focused;
                None
            }
            WidgetInput::KeyPress(key)
                if self.common.state.focused && activate_on_keyboard(key) =>
            {
                Some(ListItemMessage::Invoked)
            }
            _ => None,
        }
    }
}

/// Public card/panel primitive for grouped content surfaces.
#[derive(Clone, Debug, PartialEq)]
pub struct CardWidget {
    /// Shared widget contract.
    pub common: WidgetCommon,
}

impl CardWidget {
    /// Build a non-interactive card descriptor with neutral panel styling.
    pub fn new(id: WidgetId, sizing: WidgetSizing) -> Self {
        let mut common = WidgetCommon::new(id, WidgetKind::Card, sizing);
        common.paint.paints_focus = false;
        common.style = WidgetStyle {
            tone: WidgetTone::Neutral,
            prominence: WidgetProminence::Subtle,
        };
        Self { common }
    }
}

/// Public canvas/custom-paint primitive.
#[derive(Clone, Debug, PartialEq)]
pub struct CanvasWidget {
    /// Shared widget contract.
    pub common: WidgetCommon,
}

impl CanvasWidget {
    /// Build a canvas descriptor for custom paint and routed pointer/keyboard input.
    pub fn new(id: WidgetId, sizing: WidgetSizing) -> Self {
        let mut common = WidgetCommon::new(id, WidgetKind::Canvas, sizing);
        common.focus = FocusBehavior::Pointer;
        common.paint.bounds = PaintBounds::AllowOverflow;
        common.style = WidgetStyle {
            tone: WidgetTone::Neutral,
            prominence: WidgetProminence::Subtle,
        };
        common
            .emitted_messages
            .push(crate::widgets::contract::WidgetMessageKind::CanvasInput);
        Self { common }
    }
}

/// Union over the first-class public widget primitives.
#[derive(Clone, Debug, PartialEq)]
pub enum WidgetSpec {
    /// Non-interactive text or label content.
    Text(TextWidget),
    /// Momentary action control.
    Button(super::button::ButtonWidget),
    /// Boolean or multi-state toggle control.
    Toggle(super::toggle::ToggleWidget),
    /// Editable text field.
    TextInput(super::text_input::TextInputWidget),
    /// Scroll affordance.
    Scrollbar(super::scrollbar::ScrollbarWidget),
    /// Focusable row/item primitive.
    ListItem(ListItemWidget),
    /// Compact badge or pill primitive.
    Badge(super::badge::BadgeWidget),
    /// Non-interactive card or panel surface.
    Card(CardWidget),
    /// Custom paint/input surface.
    Canvas(CanvasWidget),
}

impl WidgetSpec {
    /// Return the shared widget contract for this primitive.
    pub fn common(&self) -> &WidgetCommon {
        match self {
            Self::Text(widget) => &widget.common,
            Self::Button(widget) => &widget.common,
            Self::Toggle(widget) => &widget.common,
            Self::TextInput(widget) => &widget.common,
            Self::Scrollbar(widget) => &widget.common,
            Self::ListItem(widget) => &widget.common,
            Self::Badge(widget) => &widget.common,
            Self::Card(widget) => &widget.common,
            Self::Canvas(widget) => &widget.common,
        }
    }

    /// Return the stable widget id.
    pub fn id(&self) -> WidgetId {
        self.common().id
    }

    /// Return the primitive taxonomy entry.
    pub fn kind(&self) -> WidgetKind {
        self.common().kind
    }

    /// Project the widget into a public layout leaf.
    pub fn layout_node(&self) -> LayoutNode {
        self.common().layout_node()
    }

    /// Route one backend-neutral interaction into the concrete widget.
    ///
    /// Non-interactive primitives ignore input and return `None`.
    pub fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        match self {
            Self::Button(widget) => widget.handle_input(bounds, input).map(WidgetOutput::Button),
            Self::Toggle(widget) => widget.handle_input(bounds, input).map(WidgetOutput::Toggle),
            Self::TextInput(widget) => widget
                .handle_input(bounds, input)
                .map(WidgetOutput::TextInput),
            Self::Scrollbar(widget) => widget
                .handle_input(bounds, input)
                .map(WidgetOutput::Scrollbar),
            Self::Badge(widget) => widget.handle_input(bounds, input).map(WidgetOutput::Badge),
            Self::ListItem(widget) => widget
                .handle_input(bounds, input)
                .map(WidgetOutput::ListItem),
            Self::Text(_) | Self::Card(_) | Self::Canvas(_) => None,
        }
    }
}

pub(super) fn activate_on_keyboard(key: WidgetKey) -> bool {
    matches!(key, WidgetKey::Enter | WidgetKey::Space)
}

pub(super) fn clamp_fraction(value: f32) -> f32 {
    value.clamp(0.0, 1.0)
}

pub(super) fn leading_arrow_for_axis(axis: ScrollbarAxis) -> WidgetKey {
    match axis {
        ScrollbarAxis::Horizontal => WidgetKey::ArrowLeft,
        ScrollbarAxis::Vertical => WidgetKey::ArrowUp,
    }
}

pub(super) fn trailing_arrow_for_axis(axis: ScrollbarAxis) -> WidgetKey {
    match axis {
        ScrollbarAxis::Horizontal => WidgetKey::ArrowRight,
        ScrollbarAxis::Vertical => WidgetKey::ArrowDown,
    }
}

//! Shared widget support types plus non-interactive public primitives.

use crate::gui::types::{ImageRgba, Rect};
use crate::layout::LayoutNode;
use std::sync::Arc;

use super::scrollbar::ScrollbarAxis;
use crate::widgets::contract::{
    FocusBehavior, PaintBounds, WidgetId, WidgetKind, WidgetProminence, WidgetSizing, WidgetState,
    WidgetStyle, WidgetTone,
};
use crate::widgets::interaction::{
    CanvasMessage, ListItemMessage, PointerButton, SelectableMessage, WidgetInput, WidgetKey,
    WidgetOutput,
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

/// Immutable public properties for a reusable selectable surface.
#[derive(Clone, Debug, PartialEq)]
pub struct SelectableProps {
    /// User-visible selectable label.
    pub label: String,
}

/// Public selectable primitive for cards, rows, tiles, and options.
#[derive(Clone, Debug, PartialEq)]
pub struct SelectableWidget {
    /// Shared widget contract.
    pub common: WidgetCommon,
    /// Immutable user-facing selectable configuration.
    pub props: SelectableProps,
}

impl SelectableWidget {
    /// Build a selectable descriptor with the provided selected state.
    pub fn new(
        id: WidgetId,
        label: impl Into<String>,
        selected: bool,
        sizing: WidgetSizing,
    ) -> Self {
        let mut common = WidgetCommon::new(id, WidgetKind::Selectable, sizing);
        common.focus = FocusBehavior::Keyboard;
        common.state.selected = selected;
        common
            .emitted_messages
            .push(crate::widgets::contract::WidgetMessageKind::ValueChanged);
        Self {
            common,
            props: SelectableProps {
                label: label.into(),
            },
        }
    }

    /// Route one backend-neutral interaction into the selectable.
    pub fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<SelectableMessage> {
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
                (was_pressed && bounds.contains(position)).then(|| self.toggle_selected())
            }
            WidgetInput::FocusChanged(focused) => {
                self.common.state.focused = focused;
                None
            }
            WidgetInput::KeyPress(key)
                if self.common.state.focused && activate_on_keyboard(key) =>
            {
                Some(self.toggle_selected())
            }
            _ => None,
        }
    }

    fn toggle_selected(&mut self) -> SelectableMessage {
        self.common.state.selected = !self.common.state.selected;
        SelectableMessage::SelectionChanged {
            selected: self.common.state.selected,
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

/// Immutable public properties for a reusable image widget.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImageProps {
    /// Shared RGBA image payload.
    pub image: Arc<ImageRgba>,
}

/// Public image primitive for raster content.
#[derive(Clone, Debug, PartialEq)]
pub struct ImageWidget {
    /// Shared widget contract.
    pub common: WidgetCommon,
    /// Immutable image configuration.
    pub props: ImageProps,
}

impl ImageWidget {
    /// Build a non-interactive image descriptor that reuses shared pixel storage.
    pub fn new(id: WidgetId, image: Arc<ImageRgba>, sizing: WidgetSizing) -> Self {
        let mut common = WidgetCommon::new(id, WidgetKind::Image, sizing);
        common.paint.paints_focus = false;
        common.paint.paints_state_layers = false;
        Self {
            common,
            props: ImageProps { image },
        }
    }
}

/// Public canvas/custom-paint primitive.
#[derive(Clone, Debug, PartialEq)]
pub struct CanvasWidget {
    /// Shared widget contract.
    pub common: WidgetCommon,
    /// Optional retained-surface metadata supplied by the host.
    pub retained: Option<RetainedSurfaceDescriptor>,
}

/// Product-neutral metadata for a host-retained custom surface.
///
/// The descriptor lets a host attach stable cache identity, revision, and dirty
/// mask information to a canvas without moving product model state into
/// Radiant. Native backends can use this to avoid unnecessary full-surface
/// recomputation while still treating the actual retained paint as host-owned.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct RetainedSurfaceDescriptor {
    /// Stable host-defined retained surface key.
    pub key: u64,
    /// Monotonic host-defined revision for this retained surface.
    pub revision: u64,
    /// Host-defined dirty segment bitmask for the latest projection.
    pub dirty_mask: u64,
    /// Whether the host-retained surface has dynamic paint that must be
    /// re-rendered whenever the runtime is asked to repaint it.
    pub volatile: bool,
}

impl CanvasWidget {
    /// Build a canvas descriptor for custom paint and routed pointer/keyboard input.
    pub fn new(id: WidgetId, sizing: WidgetSizing) -> Self {
        let mut common = WidgetCommon::new(id, WidgetKind::Canvas, sizing);
        common.focus = FocusBehavior::Keyboard;
        common.paint.bounds = PaintBounds::AllowOverflow;
        common.style = WidgetStyle {
            tone: WidgetTone::Neutral,
            prominence: WidgetProminence::Subtle,
        };
        common
            .emitted_messages
            .push(crate::widgets::contract::WidgetMessageKind::CanvasInput);
        Self {
            common,
            retained: None,
        }
    }

    /// Attach retained-surface metadata to this custom canvas.
    pub fn with_retained_surface(mut self, descriptor: RetainedSurfaceDescriptor) -> Self {
        self.retained = Some(descriptor);
        self
    }

    /// Route one backend-neutral interaction into the custom surface.
    pub fn handle_input(&mut self, _bounds: Rect, input: WidgetInput) -> Option<CanvasMessage> {
        (!self.common.state.disabled).then_some(CanvasMessage::Input { input })
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
    /// Selectable content surface.
    Selectable(SelectableWidget),
    /// Compact badge or pill primitive.
    Badge(super::badge::BadgeWidget),
    /// Non-interactive card or panel surface.
    Card(CardWidget),
    /// Non-interactive raster image surface.
    Image(ImageWidget),
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
            Self::Selectable(widget) => &widget.common,
            Self::Badge(widget) => &widget.common,
            Self::Card(widget) => &widget.common,
            Self::Image(widget) => &widget.common,
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
            Self::Selectable(widget) => widget
                .handle_input(bounds, input)
                .map(WidgetOutput::Selectable),
            Self::Canvas(widget) => widget.handle_input(bounds, input).map(WidgetOutput::Canvas),
            Self::Text(_) | Self::Card(_) | Self::Image(_) => None,
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

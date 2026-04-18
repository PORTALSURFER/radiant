//! Public primitive widget descriptors for `radiant::widgets`.

use super::contract::{
    FocusBehavior, PaintBounds, PaintContract, WidgetId, WidgetKind, WidgetMessageKind,
    WidgetProminence, WidgetSizing, WidgetState, WidgetStyle, WidgetTone,
};
use crate::layout::LayoutNode;

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
    pub paint: PaintContract,
    /// Shared style vocabulary independent from any app theme.
    pub style: WidgetStyle,
    /// Current interaction and visual state.
    pub state: WidgetState,
    /// Semantic event families this widget may emit.
    pub emitted_messages: Vec<WidgetMessageKind>,
}

impl WidgetCommon {
    /// Build a shared widget contract with neutral defaults.
    pub fn new(id: WidgetId, kind: WidgetKind, sizing: WidgetSizing) -> Self {
        Self {
            id,
            kind,
            sizing,
            focus: FocusBehavior::None,
            paint: PaintContract::default(),
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

/// Public button primitive.
#[derive(Clone, Debug, PartialEq)]
pub struct ButtonWidget {
    /// Shared widget contract.
    pub common: WidgetCommon,
    /// Button label.
    pub label: String,
}

impl ButtonWidget {
    /// Build a button descriptor with keyboard focus and activation semantics.
    pub fn new(id: WidgetId, label: impl Into<String>, sizing: WidgetSizing) -> Self {
        let mut common = WidgetCommon::new(id, WidgetKind::Button, sizing);
        common.focus = FocusBehavior::Keyboard;
        common.emitted_messages.push(WidgetMessageKind::Activate);
        Self {
            common,
            label: label.into(),
        }
    }
}

/// Public toggle primitive.
#[derive(Clone, Debug, PartialEq)]
pub struct ToggleWidget {
    /// Shared widget contract.
    pub common: WidgetCommon,
    /// Toggle label.
    pub label: String,
}

impl ToggleWidget {
    /// Build a toggle descriptor with value-change semantics.
    pub fn new(id: WidgetId, label: impl Into<String>, sizing: WidgetSizing) -> Self {
        let mut common = WidgetCommon::new(id, WidgetKind::Toggle, sizing);
        common.focus = FocusBehavior::Keyboard;
        common
            .emitted_messages
            .push(WidgetMessageKind::ValueChanged);
        Self {
            common,
            label: label.into(),
        }
    }
}

/// Public text-input primitive.
#[derive(Clone, Debug, PartialEq)]
pub struct TextInputWidget {
    /// Shared widget contract.
    pub common: WidgetCommon,
    /// Current value displayed in the field.
    pub value: String,
    /// Optional placeholder shown when the value is empty.
    pub placeholder: Option<String>,
    /// Whether the field accepts multiple lines.
    pub multiline: bool,
}

impl TextInputWidget {
    /// Build a text-input descriptor with edit semantics.
    pub fn new(id: WidgetId, value: impl Into<String>, sizing: WidgetSizing) -> Self {
        let mut common = WidgetCommon::new(id, WidgetKind::TextInput, sizing);
        common.focus = FocusBehavior::Keyboard;
        common.emitted_messages.push(WidgetMessageKind::TextEdited);
        Self {
            common,
            value: value.into(),
            placeholder: None,
            multiline: false,
        }
    }
}

/// Scrollbar orientation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ScrollbarAxis {
    /// Horizontal scroll direction.
    Horizontal,
    /// Vertical scroll direction.
    Vertical,
}

/// Public scrollbar primitive.
#[derive(Clone, Debug, PartialEq)]
pub struct ScrollbarWidget {
    /// Shared widget contract.
    pub common: WidgetCommon,
    /// Scroll direction represented by the scrollbar.
    pub axis: ScrollbarAxis,
    /// Fraction of the full content currently visible inside the viewport.
    pub viewport_fraction: f32,
    /// Fractional viewport start position.
    pub offset_fraction: f32,
}

impl ScrollbarWidget {
    /// Build a scrollbar descriptor with drag/page request semantics.
    pub fn new(id: WidgetId, axis: ScrollbarAxis, sizing: WidgetSizing) -> Self {
        let mut common = WidgetCommon::new(id, WidgetKind::Scrollbar, sizing);
        common.focus = FocusBehavior::Pointer;
        common
            .emitted_messages
            .push(WidgetMessageKind::ScrollRequested);
        common.paint.bounds = PaintBounds::ClipToRect;
        Self {
            common,
            axis,
            viewport_fraction: 1.0,
            offset_fraction: 0.0,
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
        common.emitted_messages.push(WidgetMessageKind::ItemInvoked);
        Self {
            common,
            label: label.into(),
            detail: None,
        }
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
        common.emitted_messages.push(WidgetMessageKind::CanvasInput);
        Self { common }
    }
}

/// Union over the first-class public widget primitives.
#[derive(Clone, Debug, PartialEq)]
pub enum WidgetSpec {
    /// Non-interactive text or label content.
    Text(TextWidget),
    /// Momentary action control.
    Button(ButtonWidget),
    /// Boolean or multi-state toggle control.
    Toggle(ToggleWidget),
    /// Editable text field.
    TextInput(TextInputWidget),
    /// Scroll affordance.
    Scrollbar(ScrollbarWidget),
    /// Focusable row/item primitive.
    ListItem(ListItemWidget),
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
}

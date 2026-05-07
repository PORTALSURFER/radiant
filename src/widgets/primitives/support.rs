//! Shared widget support types plus non-interactive public primitives.

use crate::gui::types::{ImageRgba, Rect};
use crate::layout::{LayoutNode, LayoutOutput};
use crate::runtime::{
    PaintPrimitive, push_badge_widget_paint, push_button_widget_paint, push_canvas_widget_paint,
    push_card_widget_paint, push_drag_handle_widget_paint, push_image_widget_paint,
    push_list_item_widget_paint, push_scrollbar_widget_paint, push_selectable_widget_paint,
    push_text_input_widget_paint, push_text_widget_paint, push_toggle_widget_paint,
};
use crate::theme::ThemeTokens;
use std::sync::Arc;

use super::scrollbar::ScrollbarAxis;
use crate::widgets::contract::{
    FocusBehavior, PaintBounds, Widget, WidgetId, WidgetProminence, WidgetSizing, WidgetState,
    WidgetStyle, WidgetTone,
};
use crate::widgets::interaction::{
    CanvasMessage, DragHandleMessage, ListItemMessage, PointerButton, SelectableMessage,
    WidgetInput, WidgetKey, WidgetOutput,
};

/// Shared contract carried by every public widget descriptor.
#[derive(Clone, Debug, PartialEq)]
pub struct WidgetCommon {
    /// Stable widget identifier.
    pub id: WidgetId,
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
}

impl WidgetCommon {
    /// Build a shared widget contract with neutral defaults.
    pub fn new(id: WidgetId, sizing: WidgetSizing) -> Self {
        Self {
            id,
            sizing,
            focus: FocusBehavior::None,
            paint: Default::default(),
            style: WidgetStyle::default(),
            state: WidgetState::default(),
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
        let mut common = WidgetCommon::new(id, sizing);
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
        let mut common = WidgetCommon::new(id, sizing);
        common.focus = FocusBehavior::Keyboard;
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

/// Public drag handle primitive for pointer-driven reordering.
#[derive(Clone, Debug, PartialEq)]
pub struct DragHandleWidget {
    /// Shared widget contract.
    pub common: WidgetCommon,
}

impl DragHandleWidget {
    /// Build a compact handle that emits drag lifecycle messages.
    pub fn new(id: WidgetId, sizing: WidgetSizing) -> Self {
        let mut common = WidgetCommon::new(id, sizing);
        common.focus = FocusBehavior::Pointer;
        Self { common }
    }

    /// Route one backend-neutral interaction into the handle.
    pub fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<DragHandleMessage> {
        if self.common.state.disabled {
            return None;
        }

        match input {
            WidgetInput::PointerMove { position } => {
                self.common.state.hovered = bounds.contains(position);
                self.common
                    .state
                    .pressed
                    .then_some(DragHandleMessage::Moved { position })
            }
            WidgetInput::PointerPress {
                position,
                button: PointerButton::Primary,
            } if bounds.contains(position) => {
                self.common.state.pressed = true;
                self.common.state.active = true;
                Some(DragHandleMessage::Started { position })
            }
            WidgetInput::PointerRelease {
                position,
                button: PointerButton::Primary,
            } => {
                self.common.state.pressed = false;
                self.common.state.active = false;
                Some(DragHandleMessage::Ended { position })
            }
            WidgetInput::FocusChanged(focused) => {
                self.common.state.focused = focused;
                None
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
        let mut common = WidgetCommon::new(id, sizing);
        common.focus = FocusBehavior::Keyboard;
        common.state.selected = selected;
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
        let mut common = WidgetCommon::new(id, sizing);
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
        let mut common = WidgetCommon::new(id, sizing);
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
        let mut common = WidgetCommon::new(id, sizing);
        common.focus = FocusBehavior::Keyboard;
        common.paint.bounds = PaintBounds::AllowOverflow;
        common.style = WidgetStyle {
            tone: WidgetTone::Neutral,
            prominence: WidgetProminence::Subtle,
        };
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

macro_rules! impl_widget_common {
    ($widget:ty) => {
        fn common(&self) -> &WidgetCommon {
            &self.common
        }

        fn common_mut(&mut self) -> &mut WidgetCommon {
            &mut self.common
        }
    };
}

impl Widget for TextWidget {
    impl_widget_common!(TextWidget);

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
        push_text_widget_paint(primitives, self, bounds, theme);
    }
}

impl Widget for super::button::ButtonWidget {
    impl_widget_common!(super::button::ButtonWidget);

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        super::button::ButtonWidget::handle_input(self, bounds, input).map(WidgetOutput::Button)
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        theme: &ThemeTokens,
    ) {
        push_button_widget_paint(primitives, self, bounds, theme);
    }
}

impl Widget for super::toggle::ToggleWidget {
    impl_widget_common!(super::toggle::ToggleWidget);

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        super::toggle::ToggleWidget::handle_input(self, bounds, input).map(WidgetOutput::Toggle)
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        theme: &ThemeTokens,
    ) {
        push_toggle_widget_paint(primitives, self, bounds, theme);
    }
}

impl Widget for super::text_input::TextInputWidget {
    impl_widget_common!(super::text_input::TextInputWidget);

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        super::text_input::TextInputWidget::handle_input(self, bounds, input)
            .map(WidgetOutput::TextInput)
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        theme: &ThemeTokens,
    ) {
        push_text_input_widget_paint(primitives, self, bounds, theme);
    }
}

impl Widget for super::scrollbar::ScrollbarWidget {
    impl_widget_common!(super::scrollbar::ScrollbarWidget);

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        super::scrollbar::ScrollbarWidget::handle_input(self, bounds, input)
            .map(WidgetOutput::Scrollbar)
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        theme: &ThemeTokens,
    ) {
        push_scrollbar_widget_paint(primitives, self, bounds, theme);
    }
}

impl Widget for DragHandleWidget {
    impl_widget_common!(DragHandleWidget);

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        DragHandleWidget::handle_input(self, bounds, input).map(WidgetOutput::DragHandle)
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        theme: &ThemeTokens,
    ) {
        push_drag_handle_widget_paint(primitives, self, bounds, theme);
    }
}

impl Widget for ListItemWidget {
    impl_widget_common!(ListItemWidget);

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        ListItemWidget::handle_input(self, bounds, input).map(WidgetOutput::ListItem)
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        theme: &ThemeTokens,
    ) {
        push_list_item_widget_paint(primitives, self, bounds, theme);
    }
}

impl Widget for SelectableWidget {
    impl_widget_common!(SelectableWidget);

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        SelectableWidget::handle_input(self, bounds, input).map(WidgetOutput::Selectable)
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        theme: &ThemeTokens,
    ) {
        push_selectable_widget_paint(primitives, self, bounds, theme);
    }
}

impl Widget for super::badge::BadgeWidget {
    impl_widget_common!(super::badge::BadgeWidget);

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        super::badge::BadgeWidget::handle_input(self, bounds, input).map(WidgetOutput::Badge)
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        theme: &ThemeTokens,
    ) {
        push_badge_widget_paint(primitives, self, bounds, theme);
    }
}

impl Widget for CardWidget {
    impl_widget_common!(CardWidget);

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
        push_card_widget_paint(primitives, self, bounds, theme);
    }
}

impl Widget for ImageWidget {
    impl_widget_common!(ImageWidget);

    fn handle_input(&mut self, _bounds: Rect, _input: WidgetInput) -> Option<WidgetOutput> {
        None
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        _theme: &ThemeTokens,
    ) {
        push_image_widget_paint(primitives, self, bounds);
    }
}

impl Widget for CanvasWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        CanvasWidget::handle_input(self, bounds, input).map(WidgetOutput::Canvas)
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        _theme: &ThemeTokens,
    ) {
        push_canvas_widget_paint(primitives, self, bounds);
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

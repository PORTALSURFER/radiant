//! Shared widget support types plus non-interactive public primitives.

use crate::gui::types::{ImageRgba, Point, Rect, Vector2};
use crate::layout::{LayoutNode, LayoutOutput};
use crate::runtime::{
    PaintCustomSurface, PaintFillPolygon, PaintFillRect, PaintImage, PaintPrimitive,
    PaintStrokePolygon, PaintStrokePolyline, PaintStrokeRect, PaintTextAlign, PaintTextInput,
    blend_color, button_font_size, diagonal_cut_rect_points, input_font_size, inset_rect,
    optical_centered_baseline, push_axis_stroke, push_text_run, text_font_size,
};
use crate::theme::ThemeTokens;
use std::sync::Arc;

use super::scrollbar::{ScrollbarAxis, ScrollbarWidget};
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

fn push_text_widget_paint(
    primitives: &mut Vec<PaintPrimitive>,
    text: &TextWidget,
    bounds: Rect,
    theme: &ThemeTokens,
) {
    push_text_run(
        primitives,
        text.common.id,
        text.text.clone(),
        bounds,
        optical_centered_baseline(bounds, text_font_size(bounds)),
        theme.text_primary,
        PaintTextAlign::Left,
        text.wrap,
        text_font_size(bounds),
    );
}

fn push_button_chrome(
    primitives: &mut Vec<PaintPrimitive>,
    common: &WidgetCommon,
    bounds: Rect,
    theme: &ThemeTokens,
) {
    let tokens = crate::widgets::resolve_widget_visual_tokens(theme, common.style, common.state);
    let points = diagonal_cut_rect_points(bounds);
    primitives.push(PaintPrimitive::FillPolygon(PaintFillPolygon {
        widget_id: common.id,
        points: points.clone(),
        color: tokens.fill,
    }));
    primitives.push(PaintPrimitive::StrokePolygon(PaintStrokePolygon {
        widget_id: common.id,
        points,
        color: tokens.border,
        width: 1.0,
    }));
    if common.state.focused && common.paint.paints_focus {
        primitives.push(PaintPrimitive::StrokePolygon(PaintStrokePolygon {
            widget_id: common.id,
            points: diagonal_cut_rect_points(inset_rect(bounds, -1.0, -1.0)),
            color: tokens.emphasis,
            width: 1.0,
        }));
    }
}

fn push_control_chrome(
    primitives: &mut Vec<PaintPrimitive>,
    common: &WidgetCommon,
    bounds: Rect,
    theme: &ThemeTokens,
) {
    let tokens = crate::widgets::resolve_widget_visual_tokens(theme, common.style, common.state);
    primitives.push(PaintPrimitive::FillRect(PaintFillRect {
        widget_id: common.id,
        rect: bounds,
        color: tokens.fill,
    }));
    primitives.push(PaintPrimitive::StrokeRect(PaintStrokeRect {
        widget_id: common.id,
        rect: bounds,
        color: tokens.border,
        width: 1.0,
    }));
    if common.state.focused && common.paint.paints_focus {
        primitives.push(PaintPrimitive::StrokeRect(PaintStrokeRect {
            widget_id: common.id,
            rect: inset_rect(bounds, -1.0, -1.0),
            color: tokens.emphasis,
            width: 1.0,
        }));
    }
}

fn push_button_widget_paint(
    primitives: &mut Vec<PaintPrimitive>,
    button: &super::button::ButtonWidget,
    bounds: Rect,
    theme: &ThemeTokens,
) {
    push_button_chrome(primitives, &button.common, bounds, theme);
    push_text_run(
        primitives,
        button.common.id,
        button.props.label.clone(),
        inset_rect(bounds, 8.0, 4.0),
        optical_centered_baseline(inset_rect(bounds, 8.0, 4.0), button_font_size(bounds)),
        crate::widgets::resolve_widget_visual_tokens(
            theme,
            button.common.style,
            button.common.state,
        )
        .foreground,
        PaintTextAlign::Center,
        TextWrap::None,
        button_font_size(bounds),
    );
}

fn push_checkbox_chrome(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: WidgetId,
    bounds: Rect,
    theme: &ThemeTokens,
    state: WidgetState,
    checked: bool,
) {
    let side = bounds.width().min(bounds.height()).max(0.0);
    let bounds = Rect::from_min_size(
        Point::new(
            bounds.min.x + (bounds.width() - side) * 0.5,
            bounds.min.y + (bounds.height() - side) * 0.5,
        ),
        Vector2::new(side, side),
    );
    primitives.push(PaintPrimitive::FillRect(PaintFillRect {
        widget_id,
        rect: bounds,
        color: if state.pressed {
            theme.bg_tertiary
        } else if state.hovered {
            theme.surface_raised
        } else {
            theme.surface_base
        },
    }));
    primitives.push(PaintPrimitive::StrokeRect(PaintStrokeRect {
        widget_id,
        rect: bounds,
        color: if checked || state.pressed || state.hovered {
            theme.accent_danger
        } else {
            theme.border_emphasis
        },
        width: 1.0,
    }));
    if checked {
        primitives.push(PaintPrimitive::StrokePolyline(PaintStrokePolyline {
            widget_id,
            points: vec![
                Point::new(bounds.min.x + side * 0.25, bounds.min.y + side * 0.55),
                Point::new(bounds.min.x + side * 0.43, bounds.min.y + side * 0.72),
                Point::new(bounds.min.x + side * 0.76, bounds.min.y + side * 0.30),
            ],
            color: theme.accent_danger,
            width: 2.0,
        }));
    }
}

fn push_toggle_widget_paint(
    primitives: &mut Vec<PaintPrimitive>,
    toggle: &super::toggle::ToggleWidget,
    bounds: Rect,
    theme: &ThemeTokens,
) {
    let tokens = crate::widgets::resolve_widget_visual_tokens(
        theme,
        toggle.common.style,
        toggle.common.state,
    );
    if toggle.props.label.is_empty() {
        push_checkbox_chrome(
            primitives,
            toggle.common.id,
            bounds,
            theme,
            toggle.common.state,
            toggle.state.checked,
        );
    } else {
        push_control_chrome(primitives, &toggle.common, bounds, theme);
        push_text_run(
            primitives,
            toggle.common.id,
            toggle.props.label.clone(),
            inset_rect(bounds, 8.0, 4.0),
            optical_centered_baseline(inset_rect(bounds, 8.0, 4.0), text_font_size(bounds)),
            tokens.foreground,
            PaintTextAlign::Left,
            TextWrap::None,
            text_font_size(bounds),
        );
    }
}

fn push_text_input_chrome(
    primitives: &mut Vec<PaintPrimitive>,
    common: &WidgetCommon,
    bounds: Rect,
    theme: &ThemeTokens,
) {
    let tokens = crate::widgets::resolve_widget_visual_tokens(theme, common.style, common.state);
    let fill = if common.state.disabled {
        tokens.fill
    } else if common.state.hovered {
        blend_color(
            theme.bg_primary,
            theme.surface_raised,
            theme.state_hover_strong,
        )
    } else {
        theme.bg_primary
    };
    primitives.push(PaintPrimitive::FillRect(PaintFillRect {
        widget_id: common.id,
        rect: bounds,
        color: fill,
    }));
    primitives.push(PaintPrimitive::StrokeRect(PaintStrokeRect {
        widget_id: common.id,
        rect: bounds,
        color: tokens.border,
        width: 1.0,
    }));
    if common.state.focused && common.paint.paints_focus {
        primitives.push(PaintPrimitive::StrokeRect(PaintStrokeRect {
            widget_id: common.id,
            rect: inset_rect(bounds, -1.0, -1.0),
            color: tokens.emphasis,
            width: 1.0,
        }));
    }
}

fn push_text_input_widget_paint(
    primitives: &mut Vec<PaintPrimitive>,
    input: &super::text_input::TextInputWidget,
    bounds: Rect,
    theme: &ThemeTokens,
) {
    push_text_input_chrome(primitives, &input.common, bounds, theme);
    let rect = inset_rect(bounds, 16.0, 4.0);
    let font_size = input_font_size(bounds);
    primitives.push(PaintPrimitive::TextInput(PaintTextInput {
        widget_id: input.common.id,
        rect,
        placeholder: input.props.placeholder.clone(),
        state: input.state.clone(),
        font_size,
        baseline: optical_centered_baseline(rect, font_size),
        color: crate::widgets::resolve_widget_visual_tokens(
            theme,
            input.common.style,
            input.common.state,
        )
        .foreground,
        placeholder_color: theme.text_muted,
        selection_color: theme.grid_strong,
        caret_color: theme.accent_danger,
        focused: input.common.state.focused,
    }));
}

fn push_scrollbar_widget_paint(
    primitives: &mut Vec<PaintPrimitive>,
    scrollbar: &ScrollbarWidget,
    bounds: Rect,
    theme: &ThemeTokens,
) {
    let tokens = crate::widgets::resolve_widget_visual_tokens(
        theme,
        scrollbar.common.style,
        scrollbar.common.state,
    );
    primitives.push(PaintPrimitive::FillRect(PaintFillRect {
        widget_id: scrollbar.common.id,
        rect: bounds,
        color: theme.surface_base,
    }));
    primitives.push(PaintPrimitive::FillRect(PaintFillRect {
        widget_id: scrollbar.common.id,
        rect: scrollbar.thumb_rect(bounds),
        color: tokens.emphasis,
    }));
    push_axis_stroke(
        primitives,
        scrollbar.common.id,
        bounds,
        theme.grid_soft,
        scrollbar.props.axis == ScrollbarAxis::Horizontal,
    );
}

fn push_drag_handle_widget_paint(
    primitives: &mut Vec<PaintPrimitive>,
    handle: &DragHandleWidget,
    bounds: Rect,
    theme: &ThemeTokens,
) {
    if !handle.common.paint.paints_state_layers {
        return;
    }
    let tokens = crate::widgets::resolve_widget_visual_tokens(
        theme,
        handle.common.style,
        handle.common.state,
    );
    let color = if handle.common.state.pressed {
        theme.accent_danger
    } else if handle.common.state.hovered {
        tokens.emphasis
    } else {
        theme.text_muted
    };
    let center_y = bounds.min.y + bounds.height() * 0.5;
    for y in [center_y - 5.0, center_y, center_y + 5.0] {
        primitives.push(PaintPrimitive::StrokePolyline(PaintStrokePolyline {
            widget_id: handle.common.id,
            points: vec![
                Point::new(bounds.min.x + bounds.width() * 0.25, y),
                Point::new(bounds.max.x - bounds.width() * 0.25, y),
            ],
            color,
            width: if handle.common.state.pressed {
                2.0
            } else {
                1.25
            },
        }));
    }
    if handle.common.state.hovered || handle.common.state.pressed {
        primitives.push(PaintPrimitive::StrokeRect(PaintStrokeRect {
            widget_id: handle.common.id,
            rect: inset_rect(bounds, 2.0, 2.0),
            color,
            width: 1.0,
        }));
    }
}

fn push_list_item_widget_paint(
    primitives: &mut Vec<PaintPrimitive>,
    item: &ListItemWidget,
    bounds: Rect,
    theme: &ThemeTokens,
) {
    push_control_chrome(primitives, &item.common, bounds, theme);
    push_text_run(
        primitives,
        item.common.id,
        item.label.clone(),
        inset_rect(bounds, 8.0, 3.0),
        optical_centered_baseline(inset_rect(bounds, 8.0, 3.0), text_font_size(bounds)),
        crate::widgets::resolve_widget_visual_tokens(theme, item.common.style, item.common.state)
            .foreground,
        PaintTextAlign::Left,
        TextWrap::None,
        text_font_size(bounds),
    );
    if let Some(detail) = &item.detail {
        push_text_run(
            primitives,
            item.common.id,
            detail.clone(),
            inset_rect(bounds, bounds.width() * 0.5, 3.0),
            optical_centered_baseline(
                inset_rect(bounds, bounds.width() * 0.5, 3.0),
                text_font_size(bounds),
            ),
            theme.text_muted,
            PaintTextAlign::Right,
            TextWrap::None,
            text_font_size(bounds),
        );
    }
}

fn push_selectable_widget_paint(
    primitives: &mut Vec<PaintPrimitive>,
    selectable: &SelectableWidget,
    bounds: Rect,
    theme: &ThemeTokens,
) {
    push_control_chrome(primitives, &selectable.common, bounds, theme);
    push_text_run(
        primitives,
        selectable.common.id,
        selectable.props.label.clone(),
        inset_rect(bounds, 8.0, 3.0),
        optical_centered_baseline(inset_rect(bounds, 8.0, 3.0), text_font_size(bounds)),
        crate::widgets::resolve_widget_visual_tokens(
            theme,
            selectable.common.style,
            selectable.common.state,
        )
        .foreground,
        PaintTextAlign::Left,
        TextWrap::None,
        text_font_size(bounds),
    );
}

fn push_badge_widget_paint(
    primitives: &mut Vec<PaintPrimitive>,
    badge: &super::badge::BadgeWidget,
    bounds: Rect,
    theme: &ThemeTokens,
) {
    push_control_chrome(primitives, &badge.common, bounds, theme);
    push_text_run(
        primitives,
        badge.common.id,
        badge.props.label.clone(),
        inset_rect(bounds, 8.0, 3.0),
        optical_centered_baseline(inset_rect(bounds, 8.0, 3.0), button_font_size(bounds)),
        crate::widgets::resolve_widget_visual_tokens(theme, badge.common.style, badge.common.state)
            .foreground,
        PaintTextAlign::Center,
        TextWrap::None,
        button_font_size(bounds),
    );
}

fn push_card_widget_paint(
    primitives: &mut Vec<PaintPrimitive>,
    card: &CardWidget,
    bounds: Rect,
    theme: &ThemeTokens,
) {
    push_control_chrome(primitives, &card.common, bounds, theme);
}

fn push_image_widget_paint(
    primitives: &mut Vec<PaintPrimitive>,
    image: &ImageWidget,
    bounds: Rect,
) {
    primitives.push(PaintPrimitive::Image(PaintImage {
        widget_id: image.common.id,
        rect: bounds,
        image: Arc::clone(&image.props.image),
    }));
}

fn push_canvas_widget_paint(
    primitives: &mut Vec<PaintPrimitive>,
    canvas: &CanvasWidget,
    bounds: Rect,
) {
    primitives.push(PaintPrimitive::CustomSurface(PaintCustomSurface {
        widget_id: canvas.common.id,
        rect: bounds,
        bounds: canvas.common.paint.bounds,
        retained: canvas.retained,
    }));
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

//! Backend-neutral paint plans emitted from generic Radiant surfaces.

use crate::{
    gui::types::{ImageRgba, Point, Rect, Rgba8},
    layout::{LayoutOutput, NodeId, OverflowPolicy},
    theme::ThemeTokens,
    widgets::{
        BadgeWidget, ButtonWidget, CanvasWidget, CardWidget, DragHandleWidget, ImageWidget,
        ListItemWidget, PaintBounds, RetainedSurfaceDescriptor, ScrollbarAxis, ScrollbarWidget,
        SelectableWidget, TextInputState, TextInputWidget, TextWidget, TextWrap, ToggleWidget,
        WidgetCommon, WidgetId, WidgetState, WidgetStyle, resolve_widget_visual_tokens,
    },
};
use std::sync::Arc;

/// Horizontal alignment for generic text paint primitives.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PaintTextAlign {
    /// Align text to the left edge of the assigned text rectangle.
    Left,
    /// Align text to the center of the assigned text rectangle.
    Center,
    /// Align text to the right edge of the assigned text rectangle.
    Right,
}

/// Filled rectangle primitive in logical surface coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PaintFillRect {
    /// Widget that produced this primitive.
    pub widget_id: WidgetId,
    /// Rectangle to fill.
    pub rect: Rect,
    /// Fill color.
    pub color: Rgba8,
}

/// Stroked rectangle primitive in logical surface coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PaintStrokeRect {
    /// Widget that produced this primitive.
    pub widget_id: WidgetId,
    /// Rectangle to stroke.
    pub rect: Rect,
    /// Stroke color.
    pub color: Rgba8,
    /// Stroke width in logical pixels.
    pub width: f32,
}

/// Filled polygon primitive in logical surface coordinates.
#[derive(Clone, Debug, PartialEq)]
pub struct PaintFillPolygon {
    /// Widget or node that produced this primitive.
    pub widget_id: WidgetId,
    /// Polygon points in clockwise or counter-clockwise order.
    pub points: Vec<Point>,
    /// Fill color.
    pub color: Rgba8,
}

/// Stroked polygon primitive in logical surface coordinates.
#[derive(Clone, Debug, PartialEq)]
pub struct PaintStrokePolygon {
    /// Widget or node that produced this primitive.
    pub widget_id: WidgetId,
    /// Polygon points in clockwise or counter-clockwise order.
    pub points: Vec<Point>,
    /// Stroke color.
    pub color: Rgba8,
    /// Stroke width in logical pixels.
    pub width: f32,
}

/// Stroked open polyline primitive in logical surface coordinates.
#[derive(Clone, Debug, PartialEq)]
pub struct PaintStrokePolyline {
    /// Widget or node that produced this primitive.
    pub widget_id: WidgetId,
    /// Connected line points in paint order.
    pub points: Vec<Point>,
    /// Stroke color.
    pub color: Rgba8,
    /// Stroke width in logical pixels.
    pub width: f32,
}

/// Single-line text primitive in logical surface coordinates.
#[derive(Clone, Debug, PartialEq)]
pub struct PaintTextRun {
    /// Widget that produced this text run.
    pub widget_id: WidgetId,
    /// Text content to paint.
    pub text: String,
    /// Text layout rectangle.
    pub rect: Rect,
    /// Font size in logical pixels per em.
    pub font_size: f32,
    /// Optional baseline measured from the text rectangle top edge.
    pub baseline: Option<f32>,
    /// Text color.
    pub color: Rgba8,
    /// Horizontal alignment inside the text rectangle.
    pub align: PaintTextAlign,
    /// Wrapping policy requested by the owning widget.
    pub wrap: TextWrap,
}

/// Floating overlay panel used for drag previews and transient popups.
#[derive(Clone, Debug, PartialEq)]
pub struct PaintOverlayPanel {
    /// Stable overlay identifier.
    pub widget_id: WidgetId,
    /// Overlay rectangle in logical surface coordinates.
    pub rect: Rect,
    /// Optional text label to paint inside the panel.
    pub label: Option<String>,
    /// Panel style.
    pub style: WidgetStyle,
}

/// Single-line text-input primitive with native caret and selection state.
#[derive(Clone, Debug, PartialEq)]
pub struct PaintTextInput {
    /// Widget that produced this text field.
    pub widget_id: WidgetId,
    /// Text layout rectangle inside the control chrome.
    pub rect: Rect,
    /// Optional placeholder shown when the value is empty.
    pub placeholder: Option<String>,
    /// Current text input state.
    pub state: TextInputState,
    /// Font size in logical pixels per em.
    pub font_size: f32,
    /// Optional baseline measured from the text rectangle top edge.
    pub baseline: Option<f32>,
    /// Value text color.
    pub color: Rgba8,
    /// Placeholder text color.
    pub placeholder_color: Rgba8,
    /// Selection fill color.
    pub selection_color: Rgba8,
    /// Block caret fill color.
    pub caret_color: Rgba8,
    /// Whether the field currently owns keyboard focus.
    pub focused: bool,
}

/// Placeholder primitive for widgets whose paint is intentionally host-defined.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PaintCustomSurface {
    /// Widget that owns this custom surface.
    pub widget_id: WidgetId,
    /// Assigned widget rectangle.
    pub rect: Rect,
    /// Whether the custom paint is clipped to the assigned rectangle.
    pub bounds: PaintBounds,
    /// Optional retained-surface metadata supplied by the host.
    pub retained: Option<RetainedSurfaceDescriptor>,
}

/// Textured RGBA image primitive in logical surface coordinates.
#[derive(Clone, Debug, PartialEq)]
pub struct PaintImage {
    /// Widget that produced this image primitive.
    pub widget_id: WidgetId,
    /// Destination rectangle.
    pub rect: Rect,
    /// Shared RGBA image payload.
    pub image: Arc<ImageRgba>,
}

/// Begin clipping subsequent paint primitives to a rectangle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PaintClipStart {
    /// Container node that owns this clip.
    pub node_id: NodeId,
    /// Clip rectangle in logical surface coordinates.
    pub rect: Rect,
}

/// End the most recent paint clip.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PaintClipEnd {
    /// Container node that owns the matching clip.
    pub node_id: NodeId,
}

/// One backend-neutral primitive emitted by a generic surface projection.
#[derive(Clone, Debug, PartialEq)]
pub enum PaintPrimitive {
    /// Begin a rectangular clip.
    ClipStart(PaintClipStart),
    /// End the current clip.
    ClipEnd(PaintClipEnd),
    /// Fill a rectangle.
    FillRect(PaintFillRect),
    /// Stroke a rectangle.
    StrokeRect(PaintStrokeRect),
    /// Fill a polygon.
    FillPolygon(PaintFillPolygon),
    /// Stroke a polygon.
    StrokePolygon(PaintStrokePolygon),
    /// Stroke an open polyline.
    StrokePolyline(PaintStrokePolyline),
    /// Paint one text run.
    Text(PaintTextRun),
    /// Paint a floating overlay panel above normal layout content.
    OverlayPanel(PaintOverlayPanel),
    /// Paint a single-line text input value, selection, and caret.
    TextInput(PaintTextInput),
    /// Paint an RGBA image stretched into one destination rectangle.
    Image(PaintImage),
    /// Reserve a host-painted custom surface.
    CustomSurface(PaintCustomSurface),
}

pub(super) fn push_clip_start(primitives: &mut Vec<PaintPrimitive>, node_id: NodeId, rect: Rect) {
    primitives.push(PaintPrimitive::ClipStart(PaintClipStart { node_id, rect }));
}

pub(super) fn push_clip_end(primitives: &mut Vec<PaintPrimitive>, node_id: NodeId) {
    primitives.push(PaintPrimitive::ClipEnd(PaintClipEnd { node_id }));
}

pub(super) fn push_scroll_affordance(
    primitives: &mut Vec<PaintPrimitive>,
    node_id: NodeId,
    content_id: NodeId,
    layout: &LayoutOutput,
    theme: &ThemeTokens,
) {
    let Some(viewport) = layout.rects.get(&node_id).copied() else {
        return;
    };
    let Some(content) = layout.rects.get(&content_id).copied() else {
        return;
    };
    let Some(overflow) = layout.overflow_flags.get(&node_id) else {
        return;
    };
    if overflow.policy != OverflowPolicy::Scroll || !overflow.y {
        return;
    }

    let viewport_h = viewport.height().max(0.0);
    let content_h = content.height().max(viewport_h);
    if viewport_h <= 0.0 || content_h <= viewport_h {
        return;
    }

    let track_w = 3.0;
    let y_inset = 6.0;
    let track_x = viewport.max.x - track_w;
    let track = Rect::from_min_max(
        Point::new(track_x, viewport.min.y + y_inset),
        Point::new(track_x + track_w, viewport.max.y - y_inset),
    );
    let max_scroll = (content_h - viewport_h).max(1.0);
    let scroll_y = (viewport.min.y - content.min.y).clamp(0.0, max_scroll);
    let thumb_h = ((viewport_h / content_h) * track.height()).clamp(24.0, track.height());
    let thumb_y = track.min.y + ((track.height() - thumb_h) * (scroll_y / max_scroll));
    let thumb = Rect::from_min_size(
        Point::new(track.min.x, thumb_y),
        crate::gui::types::Vector2::new(track.width(), thumb_h),
    );

    primitives.push(PaintPrimitive::FillRect(PaintFillRect {
        widget_id: node_id,
        rect: thumb,
        color: theme.grid_strong,
    }));
}

pub(super) fn scroll_content_clip_rect(
    node_id: NodeId,
    layout: &LayoutOutput,
    viewport: Rect,
) -> Rect {
    let Some(overflow) = layout.overflow_flags.get(&node_id) else {
        return viewport;
    };
    if overflow.policy != OverflowPolicy::Scroll || !overflow.y {
        return viewport;
    }
    viewport
}

pub(super) fn push_container_chrome(
    primitives: &mut Vec<PaintPrimitive>,
    node_id: NodeId,
    layout: &LayoutOutput,
    theme: &ThemeTokens,
    style: WidgetStyle,
    state: WidgetState,
) {
    let Some(bounds) = layout.rects.get(&node_id).copied() else {
        return;
    };
    let base_tokens = resolve_widget_visual_tokens(theme, style, WidgetState::default());
    let tokens = if state.hovered {
        base_tokens
    } else {
        resolve_widget_visual_tokens(theme, style, state)
    };
    let fill = if state.hovered {
        blend_color(
            base_tokens.fill,
            theme.surface_overlay,
            theme.state_hover_strong,
        )
    } else {
        tokens.fill
    };
    primitives.push(PaintPrimitive::FillRect(PaintFillRect {
        widget_id: node_id,
        rect: bounds,
        color: fill,
    }));
    if state.hovered {
        let marker_height = (bounds.height() - 16.0).max(8.0).min(bounds.height());
        primitives.push(PaintPrimitive::FillRect(PaintFillRect {
            widget_id: node_id,
            rect: Rect::from_min_size(
                Point::new(
                    bounds.min.x + 1.0,
                    bounds.min.y + (bounds.height() - marker_height) * 0.5,
                ),
                crate::gui::types::Vector2::new(3.0, marker_height),
            ),
            color: theme.accent_danger,
        }));
    }
    primitives.push(PaintPrimitive::StrokeRect(PaintStrokeRect {
        widget_id: node_id,
        rect: bounds,
        color: tokens.border,
        width: 1.0,
    }));
}

pub(super) fn push_overlay_panel(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: WidgetId,
    rect: Rect,
    label: Option<String>,
    theme: &ThemeTokens,
    style: WidgetStyle,
) {
    let mut state = WidgetState {
        active: true,
        ..WidgetState::default()
    };
    if label.is_none() {
        state.selected = true;
    }
    let tokens = resolve_widget_visual_tokens(theme, style, state);
    if label.is_some() {
        let shadow = Rect::from_min_max(
            Point::new(rect.min.x + 4.0, rect.min.y + 6.0),
            Point::new(rect.max.x + 4.0, rect.max.y + 6.0),
        );
        primitives.push(PaintPrimitive::FillRect(PaintFillRect {
            widget_id,
            rect: shadow,
            color: Rgba8 {
                r: 0,
                g: 0,
                b: 0,
                a: 96,
            },
        }));
        primitives.push(PaintPrimitive::FillRect(PaintFillRect {
            widget_id,
            rect,
            color: blend_color(tokens.fill, theme.surface_overlay, 0.30),
        }));
        primitives.push(PaintPrimitive::StrokeRect(PaintStrokeRect {
            widget_id,
            rect,
            color: tokens.emphasis,
            width: 1.0,
        }));
        if let Some(label) = label {
            push_text_run(
                primitives,
                widget_id,
                label,
                inset_rect(rect, 48.0, 4.0),
                optical_centered_baseline(inset_rect(rect, 48.0, 4.0), text_font_size(rect)),
                theme.text_primary,
                PaintTextAlign::Left,
                TextWrap::None,
                text_font_size(rect),
            );
        }
    } else {
        primitives.push(PaintPrimitive::FillRect(PaintFillRect {
            widget_id,
            rect,
            color: tokens.emphasis,
        }));
    }
}

/// Deterministic backend-neutral paint output for a generic [`crate::runtime::UiSurface`].
#[derive(Clone, Debug, PartialEq)]
pub struct SurfacePaintPlan {
    /// Clear color a backend may use before replaying primitives.
    pub clear_color: Rgba8,
    /// Primitives in declarative surface tree order.
    pub primitives: Vec<PaintPrimitive>,
}

/// Backend-neutral renderer contract for generic Radiant paint plans.
///
/// A renderer consumes the deterministic [`SurfacePaintPlan`] emitted by a
/// [`crate::runtime::View`] or [`crate::runtime::SurfaceRuntime`]. Renderer
/// implementations own backend resources and frame submission policy; Radiant
/// only defines the replayable paint-plan boundary.
pub trait Renderer {
    /// Backend-specific error type.
    type Error;

    /// Render one backend-neutral paint plan.
    fn render(&mut self, plan: &SurfacePaintPlan) -> Result<(), Self::Error>;
}

impl SurfacePaintPlan {
    /// Build an empty paint plan for the provided theme.
    pub fn empty(theme: &ThemeTokens) -> Self {
        Self {
            clear_color: theme.clear_color,
            primitives: Vec::new(),
        }
    }
}

pub(crate) fn push_text_widget_paint(
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

pub(crate) fn push_button_widget_paint(
    primitives: &mut Vec<PaintPrimitive>,
    button: &ButtonWidget,
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
        resolve_widget_visual_tokens(theme, button.common.style, button.common.state).foreground,
        PaintTextAlign::Center,
        TextWrap::None,
        button_font_size(bounds),
    );
}

pub(crate) fn push_toggle_widget_paint(
    primitives: &mut Vec<PaintPrimitive>,
    toggle: &ToggleWidget,
    bounds: Rect,
    theme: &ThemeTokens,
) {
    let tokens = resolve_widget_visual_tokens(theme, toggle.common.style, toggle.common.state);
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

pub(crate) fn push_text_input_widget_paint(
    primitives: &mut Vec<PaintPrimitive>,
    input: &TextInputWidget,
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
        color: resolve_widget_visual_tokens(theme, input.common.style, input.common.state)
            .foreground,
        placeholder_color: theme.text_muted,
        selection_color: theme.grid_strong,
        caret_color: theme.accent_danger,
        focused: input.common.state.focused,
    }));
}

pub(crate) fn push_scrollbar_widget_paint(
    primitives: &mut Vec<PaintPrimitive>,
    scrollbar: &ScrollbarWidget,
    bounds: Rect,
    theme: &ThemeTokens,
) {
    let tokens =
        resolve_widget_visual_tokens(theme, scrollbar.common.style, scrollbar.common.state);
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
    if scrollbar.props.axis == ScrollbarAxis::Horizontal {
        push_axis_stroke(
            primitives,
            scrollbar.common.id,
            bounds,
            theme.grid_soft,
            true,
        );
    } else {
        push_axis_stroke(
            primitives,
            scrollbar.common.id,
            bounds,
            theme.grid_soft,
            false,
        );
    }
}

pub(crate) fn push_drag_handle_widget_paint(
    primitives: &mut Vec<PaintPrimitive>,
    handle: &DragHandleWidget,
    bounds: Rect,
    theme: &ThemeTokens,
) {
    if !handle.common.paint.paints_state_layers {
        return;
    }
    let tokens = resolve_widget_visual_tokens(theme, handle.common.style, handle.common.state);
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

pub(crate) fn push_list_item_widget_paint(
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
        resolve_widget_visual_tokens(theme, item.common.style, item.common.state).foreground,
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

pub(crate) fn push_selectable_widget_paint(
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
        resolve_widget_visual_tokens(theme, selectable.common.style, selectable.common.state)
            .foreground,
        PaintTextAlign::Left,
        TextWrap::None,
        text_font_size(bounds),
    );
}

pub(crate) fn push_badge_widget_paint(
    primitives: &mut Vec<PaintPrimitive>,
    badge: &BadgeWidget,
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
        resolve_widget_visual_tokens(theme, badge.common.style, badge.common.state).foreground,
        PaintTextAlign::Center,
        TextWrap::None,
        button_font_size(bounds),
    );
}

pub(crate) fn push_card_widget_paint(
    primitives: &mut Vec<PaintPrimitive>,
    card: &CardWidget,
    bounds: Rect,
    theme: &ThemeTokens,
) {
    push_control_chrome(primitives, &card.common, bounds, theme);
}

pub(crate) fn push_image_widget_paint(
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

pub(crate) fn push_canvas_widget_paint(
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
        crate::gui::types::Vector2::new(side, side),
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

fn push_button_chrome(
    primitives: &mut Vec<PaintPrimitive>,
    common: &WidgetCommon,
    bounds: Rect,
    theme: &ThemeTokens,
) {
    let tokens = resolve_widget_visual_tokens(theme, common.style, common.state);
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
    let tokens = resolve_widget_visual_tokens(theme, common.style, common.state);
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

fn push_text_input_chrome(
    primitives: &mut Vec<PaintPrimitive>,
    common: &WidgetCommon,
    bounds: Rect,
    theme: &ThemeTokens,
) {
    let tokens = resolve_widget_visual_tokens(theme, common.style, common.state);
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

fn blend_color(from: Rgba8, to: Rgba8, amount: f32) -> Rgba8 {
    let amount = amount.clamp(0.0, 1.0);
    Rgba8 {
        r: blend_channel(from.r, to.r, amount),
        g: blend_channel(from.g, to.g, amount),
        b: blend_channel(from.b, to.b, amount),
        a: blend_channel(from.a, to.a, amount),
    }
}

fn blend_channel(from: u8, to: u8, amount: f32) -> u8 {
    ((from as f32) + (((to as f32) - (from as f32)) * amount)).round() as u8
}

fn diagonal_cut_rect_points(rect: Rect) -> Vec<Point> {
    let cut = (rect.height().min(rect.width()) * 0.18).clamp(4.0, 8.0);
    vec![
        Point::new(rect.min.x, rect.min.y),
        Point::new(rect.max.x, rect.min.y),
        Point::new(rect.max.x, rect.max.y - cut),
        Point::new(rect.max.x - cut, rect.max.y),
        Point::new(rect.min.x, rect.max.y),
    ]
}

fn push_text_run(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: WidgetId,
    text: String,
    rect: Rect,
    baseline: Option<f32>,
    color: Rgba8,
    align: PaintTextAlign,
    wrap: TextWrap,
    font_size: f32,
) {
    primitives.push(PaintPrimitive::Text(PaintTextRun {
        widget_id,
        text,
        rect,
        font_size,
        baseline,
        color,
        align,
        wrap,
    }));
}

fn text_font_size(rect: Rect) -> f32 {
    if rect.height() >= 38.0 {
        18.0
    } else if rect.height() >= 28.0 {
        14.0
    } else {
        13.0
    }
}

fn button_font_size(rect: Rect) -> f32 {
    if rect.height() >= 48.0 {
        16.0
    } else if rect.height() >= 36.0 {
        14.0
    } else {
        13.0
    }
}

fn input_font_size(rect: Rect) -> f32 {
    if rect.height() >= 42.0 { 15.0 } else { 13.0 }
}

fn optical_centered_baseline(rect: Rect, font_size: f32) -> Option<f32> {
    Some((rect.height() * 0.5 + font_size * 0.35).max(0.0))
}

fn push_axis_stroke(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: WidgetId,
    bounds: Rect,
    color: Rgba8,
    horizontal: bool,
) {
    let rect = if horizontal {
        Rect::from_min_max(
            Point::new(bounds.min.x, bounds.min.y + bounds.height() * 0.5),
            Point::new(bounds.max.x, bounds.min.y + bounds.height() * 0.5),
        )
    } else {
        Rect::from_min_max(
            Point::new(bounds.min.x + bounds.width() * 0.5, bounds.min.y),
            Point::new(bounds.min.x + bounds.width() * 0.5, bounds.max.y),
        )
    };
    primitives.push(PaintPrimitive::StrokeRect(PaintStrokeRect {
        widget_id,
        rect,
        color,
        width: 1.0,
    }));
}

fn inset_rect(rect: Rect, x: f32, y: f32) -> Rect {
    Rect::from_min_max(
        Point::new(rect.min.x + x, rect.min.y + y),
        Point::new(rect.max.x - x, rect.max.y - y),
    )
}

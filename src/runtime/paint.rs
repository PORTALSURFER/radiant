//! Backend-neutral paint plans emitted from generic Radiant surfaces.

use crate::{
    gui::types::{ImageRgba, Point, Rect, Rgba8},
    layout::{LayoutOutput, NodeId, OverflowPolicy},
    theme::ThemeTokens,
    widgets::{
        PaintBounds, RetainedSurfaceDescriptor, ScrollbarAxis, TextWrap, WidgetId, WidgetSpec,
        WidgetState, WidgetStyle, resolve_widget_visual_tokens,
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
    /// Paint one text run.
    Text(PaintTextRun),
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

    let gutter = scroll_gutter_width();
    let gutter_rect = Rect::from_min_max(
        Point::new(viewport.max.x - gutter, viewport.min.y),
        Point::new(viewport.max.x, viewport.max.y),
    );
    primitives.push(PaintPrimitive::FillRect(PaintFillRect {
        widget_id: node_id,
        rect: gutter_rect,
        color: theme.bg_secondary,
    }));
    primitives.push(PaintPrimitive::StrokeRect(PaintStrokeRect {
        widget_id: node_id,
        rect: Rect::from_min_max(
            gutter_rect.min,
            Point::new(gutter_rect.min.x, gutter_rect.max.y),
        ),
        color: theme.grid_soft,
        width: 1.0,
    }));

    let track_w = 4.0;
    let inset = 4.0;
    let track_x = gutter_rect.min.x + (gutter - track_w) * 0.5;
    let track = Rect::from_min_max(
        Point::new(track_x, viewport.min.y + inset),
        Point::new(track_x + track_w, viewport.max.y - inset),
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
        rect: track,
        color: theme.grid_soft,
    }));
    primitives.push(PaintPrimitive::FillRect(PaintFillRect {
        widget_id: node_id,
        rect: thumb,
        color: theme.border_emphasis,
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
    Rect::from_min_max(
        viewport.min,
        Point::new(
            (viewport.max.x - scroll_gutter_width()).max(viewport.min.x),
            viewport.max.y,
        ),
    )
}

fn scroll_gutter_width() -> f32 {
    14.0
}

pub(super) fn push_container_chrome(
    primitives: &mut Vec<PaintPrimitive>,
    node_id: NodeId,
    layout: &LayoutOutput,
    theme: &ThemeTokens,
    style: WidgetStyle,
) {
    let Some(bounds) = layout.rects.get(&node_id).copied() else {
        return;
    };
    let tokens = resolve_widget_visual_tokens(theme, style, WidgetState::default());
    primitives.push(PaintPrimitive::FillRect(PaintFillRect {
        widget_id: node_id,
        rect: bounds,
        color: tokens.fill,
    }));
    primitives.push(PaintPrimitive::StrokeRect(PaintStrokeRect {
        widget_id: node_id,
        rect: bounds,
        color: tokens.border,
        width: 1.0,
    }));
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

pub(super) fn push_widget_paint(
    primitives: &mut Vec<PaintPrimitive>,
    widget: &WidgetSpec,
    layout: &LayoutOutput,
    theme: &ThemeTokens,
) {
    let Some(bounds) = layout.rects.get(&widget.id()).copied() else {
        return;
    };

    match widget {
        WidgetSpec::Text(text) => {
            push_text_run(
                primitives,
                widget.id(),
                text.text.clone(),
                bounds,
                text.common.sizing.baseline,
                theme.text_primary,
                PaintTextAlign::Left,
                text.wrap,
            );
        }
        WidgetSpec::Button(button) => {
            push_button_chrome(primitives, widget, bounds, theme);
            push_text_run(
                primitives,
                widget.id(),
                button.props.label.clone(),
                inset_rect(bounds, 8.0, 4.0),
                button.common.sizing.baseline,
                resolve_widget_visual_tokens(theme, button.common.style, button.common.state)
                    .foreground,
                PaintTextAlign::Center,
                TextWrap::None,
            );
        }
        WidgetSpec::Toggle(toggle) => {
            let tokens =
                resolve_widget_visual_tokens(theme, toggle.common.style, toggle.common.state);
            if toggle.props.label.is_empty() {
                push_checkbox_chrome(primitives, widget.id(), bounds, theme, toggle.state.checked);
            } else {
                push_control_chrome(primitives, widget, bounds, theme);
                push_text_run(
                    primitives,
                    widget.id(),
                    toggle.props.label.clone(),
                    inset_rect(bounds, 8.0, 4.0),
                    toggle.common.sizing.baseline,
                    tokens.foreground,
                    PaintTextAlign::Left,
                    TextWrap::None,
                );
            }
        }
        WidgetSpec::TextInput(input) => {
            push_control_chrome(primitives, widget, bounds, theme);
            let (value, color) = if input.state.value.is_empty() {
                (
                    input.props.placeholder.clone().unwrap_or_default(),
                    theme.text_muted,
                )
            } else {
                (
                    input.state.value.clone(),
                    resolve_widget_visual_tokens(theme, input.common.style, input.common.state)
                        .foreground,
                )
            };
            push_text_run(
                primitives,
                widget.id(),
                value,
                inset_rect(bounds, 16.0, 4.0),
                input.common.sizing.baseline,
                color,
                PaintTextAlign::Left,
                TextWrap::None,
            );
        }
        WidgetSpec::Scrollbar(scrollbar) => {
            let tokens =
                resolve_widget_visual_tokens(theme, scrollbar.common.style, scrollbar.common.state);
            primitives.push(PaintPrimitive::FillRect(PaintFillRect {
                widget_id: widget.id(),
                rect: bounds,
                color: theme.surface_base,
            }));
            primitives.push(PaintPrimitive::FillRect(PaintFillRect {
                widget_id: widget.id(),
                rect: scrollbar.thumb_rect(bounds),
                color: tokens.emphasis,
            }));
            if scrollbar.props.axis == ScrollbarAxis::Horizontal {
                push_axis_stroke(primitives, widget.id(), bounds, theme.grid_soft, true);
            } else {
                push_axis_stroke(primitives, widget.id(), bounds, theme.grid_soft, false);
            }
        }
        WidgetSpec::ListItem(item) => {
            push_control_chrome(primitives, widget, bounds, theme);
            push_text_run(
                primitives,
                widget.id(),
                item.label.clone(),
                inset_rect(bounds, 8.0, 3.0),
                item.common.sizing.baseline,
                resolve_widget_visual_tokens(theme, item.common.style, item.common.state)
                    .foreground,
                PaintTextAlign::Left,
                TextWrap::None,
            );
            if let Some(detail) = &item.detail {
                push_text_run(
                    primitives,
                    widget.id(),
                    detail.clone(),
                    inset_rect(bounds, bounds.width() * 0.5, 3.0),
                    item.common.sizing.baseline,
                    theme.text_muted,
                    PaintTextAlign::Right,
                    TextWrap::None,
                );
            }
        }
        WidgetSpec::Selectable(selectable) => {
            push_control_chrome(primitives, widget, bounds, theme);
            push_text_run(
                primitives,
                widget.id(),
                selectable.props.label.clone(),
                inset_rect(bounds, 8.0, 3.0),
                selectable.common.sizing.baseline,
                resolve_widget_visual_tokens(
                    theme,
                    selectable.common.style,
                    selectable.common.state,
                )
                .foreground,
                PaintTextAlign::Left,
                TextWrap::None,
            );
        }
        WidgetSpec::Badge(badge) => {
            push_control_chrome(primitives, widget, bounds, theme);
            push_text_run(
                primitives,
                widget.id(),
                badge.props.label.clone(),
                inset_rect(bounds, 8.0, 3.0),
                badge.common.sizing.baseline,
                resolve_widget_visual_tokens(theme, badge.common.style, badge.common.state)
                    .foreground,
                PaintTextAlign::Center,
                TextWrap::None,
            );
        }
        WidgetSpec::Card(_) => {
            push_control_chrome(primitives, widget, bounds, theme);
        }
        WidgetSpec::Image(image) => {
            primitives.push(PaintPrimitive::Image(PaintImage {
                widget_id: widget.id(),
                rect: bounds,
                image: Arc::clone(&image.props.image),
            }));
        }
        WidgetSpec::Canvas(canvas) => {
            primitives.push(PaintPrimitive::CustomSurface(PaintCustomSurface {
                widget_id: widget.id(),
                rect: bounds,
                bounds: canvas.common.paint.bounds,
                retained: canvas.retained,
            }));
        }
    }
}

fn push_checkbox_chrome(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: WidgetId,
    bounds: Rect,
    theme: &ThemeTokens,
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
        color: theme.surface_base,
    }));
    primitives.push(PaintPrimitive::StrokeRect(PaintStrokeRect {
        widget_id,
        rect: bounds,
        color: if checked {
            theme.accent_danger
        } else {
            theme.border_emphasis
        },
        width: 1.0,
    }));
    if checked {
        primitives.push(PaintPrimitive::FillRect(PaintFillRect {
            widget_id,
            rect: inset_rect(bounds, side * 0.32, side * 0.32),
            color: theme.accent_danger,
        }));
    }
}

fn push_button_chrome(
    primitives: &mut Vec<PaintPrimitive>,
    widget: &WidgetSpec,
    bounds: Rect,
    theme: &ThemeTokens,
) {
    let common = widget.common();
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
    widget: &WidgetSpec,
    bounds: Rect,
    theme: &ThemeTokens,
) {
    let common = widget.common();
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
) {
    primitives.push(PaintPrimitive::Text(PaintTextRun {
        widget_id,
        text,
        rect,
        font_size: 13.0,
        baseline,
        color,
        align,
        wrap,
    }));
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

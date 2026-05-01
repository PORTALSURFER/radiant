//! Backend-neutral paint plans emitted from generic Radiant surfaces.

use crate::{
    gui::types::{Point, Rect, Rgba8},
    layout::LayoutOutput,
    theme::ThemeTokens,
    widgets::{
        PaintBounds, ScrollbarAxis, TextWrap, WidgetId, WidgetSpec, resolve_widget_visual_tokens,
    },
};

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
}

/// One backend-neutral primitive emitted by a generic surface projection.
#[derive(Clone, Debug, PartialEq)]
pub enum PaintPrimitive {
    /// Fill a rectangle.
    FillRect(PaintFillRect),
    /// Stroke a rectangle.
    StrokeRect(PaintStrokeRect),
    /// Paint one text run.
    Text(PaintTextRun),
    /// Reserve a host-painted custom surface.
    CustomSurface(PaintCustomSurface),
}

/// Deterministic backend-neutral paint output for a generic [`crate::runtime::UiSurface`].
#[derive(Clone, Debug, PartialEq)]
pub struct SurfacePaintPlan {
    /// Clear color a backend may use before replaying primitives.
    pub clear_color: Rgba8,
    /// Primitives in declarative surface tree order.
    pub primitives: Vec<PaintPrimitive>,
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
            push_control_chrome(primitives, widget, bounds, theme);
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
            push_control_chrome(primitives, widget, bounds, theme);
            push_text_run(
                primitives,
                widget.id(),
                toggle.props.label.clone(),
                inset_rect(bounds, 8.0, 4.0),
                toggle.common.sizing.baseline,
                resolve_widget_visual_tokens(theme, toggle.common.style, toggle.common.state)
                    .foreground,
                PaintTextAlign::Left,
                TextWrap::None,
            );
        }
        WidgetSpec::TextInput(input) => {
            push_control_chrome(primitives, widget, bounds, theme);
            push_text_run(
                primitives,
                widget.id(),
                input.state.value.clone(),
                inset_rect(bounds, 8.0, 4.0),
                input.common.sizing.baseline,
                resolve_widget_visual_tokens(theme, input.common.style, input.common.state)
                    .foreground,
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
        WidgetSpec::Canvas(canvas) => {
            primitives.push(PaintPrimitive::CustomSurface(PaintCustomSurface {
                widget_id: widget.id(),
                rect: bounds,
                bounds: canvas.common.paint.bounds,
            }));
        }
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

//! Passive color marker primitive for swatches, list indicators, and legends.

use crate::gui::types::{Point, Rect, Rgba8};
use crate::layout::{LayoutOutput, Vector2};
use crate::runtime::{PaintFillRect, PaintPrimitive};
use crate::theme::ThemeTokens;
use crate::widgets::contract::{FocusBehavior, PaintBounds, Widget, WidgetId, WidgetSizing};
use crate::widgets::interaction::{WidgetInput, WidgetOutput};
use crate::widgets::primitives::support::WidgetCommon;

/// Horizontal alignment for a color marker inside its assigned bounds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ColorMarkerAlign {
    /// Align to the left edge.
    Left,
    /// Center inside the assigned bounds.
    Center,
    /// Align to the right edge.
    #[default]
    Right,
}

/// Immutable color-marker paint configuration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ColorMarkerProps {
    /// Marker fill color. `None` paints nothing.
    pub color: Option<Rgba8>,
    /// Preferred side length in logical pixels.
    pub side: u8,
    /// Horizontal inset from the selected edge.
    pub inset: u8,
    /// Horizontal marker alignment.
    pub align: ColorMarkerAlign,
}

/// Named construction fields for [`ColorMarkerWidget`].
#[derive(Clone, Debug, PartialEq)]
pub struct ColorMarkerWidgetParts {
    /// Stable widget identity used by layout and paint.
    pub id: WidgetId,
    /// Intrinsic color-marker sizing contract.
    pub sizing: WidgetSizing,
    /// Marker paint configuration.
    pub props: ColorMarkerProps,
}

/// Passive color marker widget.
#[derive(Clone, Debug, PartialEq)]
pub struct ColorMarkerWidget {
    /// Shared widget contract.
    pub common: WidgetCommon,
    /// Immutable marker paint configuration.
    pub props: ColorMarkerProps,
}

impl ColorMarkerProps {
    /// Build default marker configuration for an optional color.
    pub fn new(color: Option<Rgba8>) -> Self {
        Self {
            color,
            side: 10,
            inset: 4,
            align: ColorMarkerAlign::Right,
        }
    }

    /// Set the preferred marker side length.
    pub fn side(mut self, side: u8) -> Self {
        self.side = side;
        self
    }

    /// Set the horizontal edge inset.
    pub fn inset(mut self, inset: u8) -> Self {
        self.inset = inset;
        self
    }

    /// Set horizontal alignment.
    pub fn align(mut self, align: ColorMarkerAlign) -> Self {
        self.align = align;
        self
    }

    /// Return the marker paint rectangle inside `bounds`.
    pub fn rect_in(self, bounds: Rect) -> Option<Rect> {
        marker_rect(bounds, self)
    }
}

impl ColorMarkerWidget {
    /// Build a color marker from named construction fields.
    pub fn from_parts(parts: ColorMarkerWidgetParts) -> Self {
        let mut common = WidgetCommon::new(parts.id, parts.sizing);
        common.focus = FocusBehavior::None;
        common.paint.bounds = PaintBounds::ClipToRect;
        common.paint.paints_focus = false;
        common.paint.paints_state_layers = false;
        Self {
            common,
            props: parts.props,
        }
    }

    /// Build a fill-style color marker with a generated runtime id.
    pub fn new(color: Option<Rgba8>) -> Self {
        Self::from_parts(ColorMarkerWidgetParts {
            id: 0,
            sizing: WidgetSizing::fixed(Vector2::new(1.0, 1.0)),
            props: ColorMarkerProps::new(color),
        })
    }

    /// Set the preferred marker side length.
    pub fn with_side(mut self, side: u8) -> Self {
        self.props.side = side;
        self
    }

    /// Set the horizontal edge inset.
    pub fn with_inset(mut self, inset: u8) -> Self {
        self.props.inset = inset;
        self
    }

    /// Set horizontal alignment.
    pub fn with_align(mut self, align: ColorMarkerAlign) -> Self {
        self.props.align = align;
        self
    }
}

impl Widget for ColorMarkerWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, _bounds: Rect, _input: WidgetInput) -> Option<WidgetOutput> {
        None
    }

    fn needs_state_synchronization(&self) -> bool {
        false
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        _theme: &ThemeTokens,
    ) {
        let Some(color) = self.props.color else {
            return;
        };
        let Some(rect) = self.props.rect_in(bounds) else {
            return;
        };
        primitives.push(PaintPrimitive::FillRect(PaintFillRect {
            widget_id: self.common.id,
            rect,
            color,
        }));
    }
}

fn marker_rect(bounds: Rect, props: ColorMarkerProps) -> Option<Rect> {
    if !bounds.has_finite_positive_area() || props.side == 0 {
        return None;
    }
    let side = (props.side as f32).min(bounds.width()).min(bounds.height());
    if side <= 0.0 {
        return None;
    }
    let inset = props.inset as f32;
    let x = match props.align {
        ColorMarkerAlign::Left => (bounds.min.x + inset).min(bounds.max.x - side),
        ColorMarkerAlign::Center => bounds.min.x + (bounds.width() - side) * 0.5,
        ColorMarkerAlign::Right => (bounds.max.x - side - inset).max(bounds.min.x),
    };
    let y = bounds.min.y + (bounds.height() - side) * 0.5;
    Some(Rect::from_min_max(
        Point::new(x, y),
        Point::new(x + side, y + side),
    ))
}

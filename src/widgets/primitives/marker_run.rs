//! Passive repeated marker primitive for compact status and rating indicators.

use crate::gui::types::{Point, Rect, Rgba8};
use crate::layout::{LayoutOutput, Vector2};
use crate::runtime::{PaintFillRect, PaintPrimitive};
use crate::theme::ThemeTokens;
use crate::widgets::contract::{FocusBehavior, PaintBounds, Widget, WidgetId, WidgetSizing};
use crate::widgets::interaction::{WidgetInput, WidgetOutput};
use crate::widgets::primitives::support::WidgetCommon;

/// Horizontal alignment for a marker run inside its assigned bounds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum MarkerRunAlign {
    /// Align the run to the left edge.
    Left,
    /// Center the run inside the assigned bounds.
    Center,
    /// Align the run to the right edge.
    #[default]
    Right,
}

/// Immutable marker-run paint configuration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MarkerRunProps {
    /// Marker fill color. `None` paints nothing.
    pub color: Option<Rgba8>,
    /// Number of markers to paint.
    pub count: u8,
    /// Preferred side length in logical pixels.
    pub side: u8,
    /// Gap between markers in logical pixels.
    pub gap: u8,
    /// Horizontal inset from the selected edge.
    pub inset: u8,
    /// Horizontal run alignment.
    pub align: MarkerRunAlign,
}

/// Immutable per-color marker-run paint configuration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColorMarkerRunProps {
    /// Fill colors, one per marker.
    pub colors: Vec<Rgba8>,
    /// Preferred side length in logical pixels.
    pub side: u8,
    /// Gap between markers in logical pixels.
    pub gap: u8,
    /// Horizontal inset from the selected edge.
    pub inset: u8,
    /// Horizontal run alignment.
    pub align: MarkerRunAlign,
}

/// Named construction fields for [`MarkerRunWidget`].
#[derive(Clone, Debug, PartialEq)]
pub struct MarkerRunWidgetParts {
    /// Stable widget identity used by layout and paint.
    pub id: WidgetId,
    /// Intrinsic marker-run sizing contract.
    pub sizing: WidgetSizing,
    /// Marker paint configuration.
    pub props: MarkerRunProps,
}

/// Named construction fields for [`ColorMarkerRunWidget`].
#[derive(Clone, Debug, PartialEq)]
pub struct ColorMarkerRunWidgetParts {
    /// Stable widget identity used by layout and paint.
    pub id: WidgetId,
    /// Intrinsic marker-run sizing contract.
    pub sizing: WidgetSizing,
    /// Marker paint configuration.
    pub props: ColorMarkerRunProps,
}

/// Passive repeated marker widget.
#[derive(Clone, Debug, PartialEq)]
pub struct MarkerRunWidget {
    /// Shared widget contract.
    pub common: WidgetCommon,
    /// Immutable marker paint configuration.
    pub props: MarkerRunProps,
}

/// Passive per-color marker widget.
#[derive(Clone, Debug, PartialEq)]
pub struct ColorMarkerRunWidget {
    /// Shared widget contract.
    pub common: WidgetCommon,
    /// Immutable marker paint configuration.
    pub props: ColorMarkerRunProps,
}

impl MarkerRunProps {
    /// Build default marker-run configuration for an optional color and count.
    pub fn new(color: Option<Rgba8>, count: u8) -> Self {
        Self {
            color,
            count,
            side: 6,
            gap: 4,
            inset: 4,
            align: MarkerRunAlign::Right,
        }
    }
}

impl ColorMarkerRunProps {
    /// Build default marker-run configuration from a per-marker color list.
    pub fn new(colors: Vec<Rgba8>) -> Self {
        Self {
            colors,
            side: 6,
            gap: 4,
            inset: 4,
            align: MarkerRunAlign::Right,
        }
    }
}

impl MarkerRunWidget {
    /// Build a marker run from named construction fields.
    pub fn from_parts(parts: MarkerRunWidgetParts) -> Self {
        Self {
            common: marker_run_common(parts.id, parts.sizing),
            props: parts.props,
        }
    }

    /// Build a fill-style marker run with a generated runtime id.
    pub fn new(color: Option<Rgba8>, count: u8) -> Self {
        Self::from_parts(MarkerRunWidgetParts {
            id: 0,
            sizing: WidgetSizing::fixed(Vector2::new(1.0, 1.0)),
            props: MarkerRunProps::new(color, count),
        })
    }

    /// Set the preferred marker side length.
    pub fn with_side(mut self, side: u8) -> Self {
        self.props.side = side;
        self
    }

    /// Set the gap between markers.
    pub fn with_gap(mut self, gap: u8) -> Self {
        self.props.gap = gap;
        self
    }

    /// Set the horizontal edge inset.
    pub fn with_inset(mut self, inset: u8) -> Self {
        self.props.inset = inset;
        self
    }

    /// Set horizontal alignment.
    pub fn with_align(mut self, align: MarkerRunAlign) -> Self {
        self.props.align = align;
        self
    }
}

impl ColorMarkerRunWidget {
    /// Build a marker run from named construction fields.
    pub fn from_parts(parts: ColorMarkerRunWidgetParts) -> Self {
        Self {
            common: marker_run_common(parts.id, parts.sizing),
            props: parts.props,
        }
    }

    /// Build a marker run with one marker per color.
    pub fn new(colors: Vec<Rgba8>) -> Self {
        Self::from_parts(ColorMarkerRunWidgetParts {
            id: 0,
            sizing: WidgetSizing::fixed(Vector2::new(1.0, 1.0)),
            props: ColorMarkerRunProps::new(colors),
        })
    }

    /// Set the preferred marker side length.
    pub fn with_side(mut self, side: u8) -> Self {
        self.props.side = side;
        self
    }

    /// Set the gap between markers.
    pub fn with_gap(mut self, gap: u8) -> Self {
        self.props.gap = gap;
        self
    }

    /// Set the horizontal edge inset.
    pub fn with_inset(mut self, inset: u8) -> Self {
        self.props.inset = inset;
        self
    }

    /// Set horizontal alignment.
    pub fn with_align(mut self, align: MarkerRunAlign) -> Self {
        self.props.align = align;
        self
    }
}

impl Widget for MarkerRunWidget {
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
        for_each_marker_rect(
            bounds,
            self.props.count as usize,
            marker_geometry(
                self.props.side,
                self.props.gap,
                self.props.inset,
                self.props.align,
            ),
            |_, rect| {
                primitives.push(PaintPrimitive::FillRect(PaintFillRect {
                    widget_id: self.common.id,
                    rect,
                    color,
                }));
            },
        );
    }
}

impl Widget for ColorMarkerRunWidget {
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
        for_each_marker_rect(
            bounds,
            self.props.colors.len(),
            marker_geometry(
                self.props.side,
                self.props.gap,
                self.props.inset,
                self.props.align,
            ),
            |index, rect| {
                if let Some(color) = self.props.colors.get(index) {
                    primitives.push(PaintPrimitive::FillRect(PaintFillRect {
                        widget_id: self.common.id,
                        rect,
                        color: *color,
                    }));
                }
            },
        );
    }
}

#[derive(Clone, Copy)]
struct MarkerRunGeometry {
    side: u8,
    gap: u8,
    inset: u8,
    align: MarkerRunAlign,
}

fn marker_run_common(id: WidgetId, sizing: WidgetSizing) -> WidgetCommon {
    let mut common = WidgetCommon::new(id, sizing);
    common.focus = FocusBehavior::None;
    common.paint.bounds = PaintBounds::ClipToRect;
    common.paint.paints_focus = false;
    common.paint.paints_state_layers = false;
    common
}

fn marker_geometry(side: u8, gap: u8, inset: u8, align: MarkerRunAlign) -> MarkerRunGeometry {
    MarkerRunGeometry {
        side,
        gap,
        inset,
        align,
    }
}

fn for_each_marker_rect(
    bounds: Rect,
    count: usize,
    geometry: MarkerRunGeometry,
    mut push: impl FnMut(usize, Rect),
) {
    if !bounds.has_finite_positive_area() || count == 0 || geometry.side == 0 {
        return;
    }

    let side = (geometry.side as f32)
        .min(bounds.width())
        .min(bounds.height());
    if side <= 0.0 {
        return;
    }

    let gap = geometry.gap as f32;
    let total_width = count as f32 * side + count.saturating_sub(1) as f32 * gap;
    let start_x = marker_start_x(bounds, geometry.align, total_width, geometry.inset as f32);
    let y = bounds.min.y + (bounds.height() - side) * 0.5;
    for index in 0..count {
        let x = start_x + index as f32 * (side + gap);
        push(
            index,
            Rect::from_min_max(Point::new(x, y), Point::new(x + side, y + side)),
        );
    }
}

fn marker_start_x(bounds: Rect, align: MarkerRunAlign, total_width: f32, inset: f32) -> f32 {
    match align {
        MarkerRunAlign::Left => (bounds.min.x + inset).min(bounds.max.x - total_width),
        MarkerRunAlign::Center => bounds.min.x + (bounds.width() - total_width) * 0.5,
        MarkerRunAlign::Right => (bounds.max.x - total_width - inset).max(bounds.min.x),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WHITE: Rgba8 = Rgba8 {
        r: 255,
        g: 255,
        b: 255,
        a: 255,
    };

    fn bounds() -> Rect {
        Rect::from_min_max(Point::new(10.0, 20.0), Point::new(110.0, 40.0))
    }

    fn marker_rects(bounds: Rect, props: MarkerRunProps) -> Vec<Rect> {
        let mut rects = Vec::new();
        for_each_marker_rect(
            bounds,
            props.count as usize,
            marker_geometry(props.side, props.gap, props.inset, props.align),
            |_, rect| rects.push(rect),
        );
        rects
    }

    #[test]
    fn right_aligned_marker_run_respects_gap_and_inset() {
        let rects = marker_rects(
            bounds(),
            MarkerRunProps {
                count: 3,
                side: 5,
                gap: 4,
                inset: 4,
                align: MarkerRunAlign::Right,
                color: Some(WHITE),
            },
        );

        assert_eq!(
            rects,
            vec![
                Rect::from_min_max(Point::new(83.0, 27.5), Point::new(88.0, 32.5)),
                Rect::from_min_max(Point::new(92.0, 27.5), Point::new(97.0, 32.5)),
                Rect::from_min_max(Point::new(101.0, 27.5), Point::new(106.0, 32.5)),
            ]
        );
    }

    #[test]
    fn empty_or_transparent_marker_runs_paint_no_rects() {
        assert!(marker_rects(bounds(), MarkerRunProps::new(Some(WHITE), 0)).is_empty());

        let widget = MarkerRunWidget::new(None, 3);
        let mut primitives = Vec::new();
        widget.append_paint(
            &mut primitives,
            bounds(),
            &LayoutOutput::default(),
            &ThemeTokens::default(),
        );
        assert!(primitives.is_empty());
    }

    #[test]
    fn color_marker_run_can_paint_one_marker_per_color() {
        const RED: Rgba8 = Rgba8 {
            r: 255,
            g: 0,
            b: 0,
            a: 255,
        };
        const BLUE: Rgba8 = Rgba8 {
            r: 0,
            g: 0,
            b: 255,
            a: 255,
        };

        let widget = ColorMarkerRunWidget::new(vec![RED, BLUE])
            .with_side(5)
            .with_gap(4)
            .with_inset(4);
        let mut primitives = Vec::new();
        widget.append_paint(
            &mut primitives,
            bounds(),
            &LayoutOutput::default(),
            &ThemeTokens::default(),
        );

        let colors = primitives
            .iter()
            .filter_map(|primitive| match primitive {
                PaintPrimitive::FillRect(fill) => Some(fill.color),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(colors, vec![RED, BLUE]);
    }
}

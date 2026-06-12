use crate::{
    gui::types::Rect,
    layout::{LayoutOutput, Vector2},
    runtime::PaintPrimitive,
    theme::ThemeTokens,
    widgets::{
        contract::{FocusBehavior, PaintBounds, Widget, WidgetId, WidgetSizing},
        interaction::{WidgetInput, WidgetOutput},
        primitives::support::WidgetCommon,
    },
};

use super::{
    model::{
        ColorMarkerRunProps, ColorMarkerRunWidgetParts, MarkerRunAlign, MarkerRunProps,
        MarkerRunWidgetParts,
    },
    paint::{append_color_marker_run_paint, append_marker_run_paint},
};

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

impl MarkerRunWidget {
    /// Build a marker run from named construction fields.
    pub fn from_parts(parts: MarkerRunWidgetParts) -> Self {
        Self {
            common: marker_run_common(parts.id, parts.sizing),
            props: parts.props,
        }
    }

    /// Build a fill-style marker run with a generated runtime id.
    pub fn new(color: Option<crate::gui::types::Rgba8>, count: u8) -> Self {
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
    pub fn new(colors: Vec<crate::gui::types::Rgba8>) -> Self {
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
        append_marker_run_paint(primitives, self.common.id, bounds, self.props);
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
        append_color_marker_run_paint(primitives, self.common.id, bounds, &self.props);
    }
}

fn marker_run_common(id: WidgetId, sizing: WidgetSizing) -> WidgetCommon {
    let mut common = WidgetCommon::new(id, sizing);
    common.focus = FocusBehavior::None;
    common.paint.bounds = PaintBounds::ClipToRect;
    common.paint.paints_focus = false;
    common.paint.paints_state_layers = false;
    common
}

use crate::{
    gui::types::Rgba8,
    widgets::contract::{WidgetId, WidgetSizing},
};

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

/// Named construction fields for [`crate::widgets::MarkerRunWidget`].
#[derive(Clone, Debug, PartialEq)]
pub struct MarkerRunWidgetParts {
    /// Stable widget identity used by layout and paint.
    pub id: WidgetId,
    /// Intrinsic marker-run sizing contract.
    pub sizing: WidgetSizing,
    /// Marker paint configuration.
    pub props: MarkerRunProps,
}

/// Named construction fields for [`crate::widgets::ColorMarkerRunWidget`].
#[derive(Clone, Debug, PartialEq)]
pub struct ColorMarkerRunWidgetParts {
    /// Stable widget identity used by layout and paint.
    pub id: WidgetId,
    /// Intrinsic marker-run sizing contract.
    pub sizing: WidgetSizing,
    /// Marker paint configuration.
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

use crate::runtime::PaintText;
use crate::widgets::primitives::ColorMarkerProps;

/// Immutable public properties for a reusable selectable surface.
#[derive(Clone, Debug, PartialEq)]
pub struct SelectableProps {
    /// User-visible selectable label.
    pub label: PaintText,
    /// Optional passive color marker painted inside the selectable bounds.
    pub color_marker: Option<ColorMarkerProps>,
}

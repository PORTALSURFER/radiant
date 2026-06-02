//! Geometry, paint, SVG, and image prelude exports.

pub use crate::gui::{
    paint::{BorderSides, horizontal_line_rect, vertical_line_rect},
    svg::{SvgIcon, SvgIconTintCache, svg_with_current_color},
    types::{ImageRgba, ImageRgbaError, Point, Rect, Rgba8, Vector2},
};

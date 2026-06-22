//! Retained SVG icons for Vello-backed vector rendering.
//!
//! The public surface is [`SvgIcon`](crate::gui::svg::SvgIcon). Parser details
//! stay behind Radiant paint primitives so application widgets can embed SVG
//! assets without constructing backend scenes directly.

mod hit_test;
mod icon;
mod model;
mod parser;

#[cfg(test)]
#[path = "svg/tests.rs"]
mod tests;

pub use hit_test::point_in_svg_shapes;
pub use icon::{SvgIcon, SvgIconTintCache, SvgIconTintPalette, svg_with_current_color};
pub use model::{SvgDocument, SvgShape};
pub use parser::parse_svg_document;

use model::SvgFillRule;

//! Passive repeated marker primitive for compact status and rating indicators.

mod geometry;
mod model;
mod paint;
mod widget;

#[cfg(test)]
mod tests;

pub use model::{
    ColorMarkerRunProps, ColorMarkerRunWidgetParts, MarkerRunAlign, MarkerRunProps,
    MarkerRunWidgetParts,
};
pub use widget::{ColorMarkerRunWidget, MarkerRunWidget};

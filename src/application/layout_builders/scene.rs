//! Declarative root scene builder for base content plus overlay layers.

mod layer;
mod overlays;
mod root;

#[cfg(test)]
mod tests;

pub use overlays::{Overlays, overlays};
pub use root::{Scene, scene};

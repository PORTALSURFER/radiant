//! Surface paint projection for runtime nodes.

mod capacity;
mod context;
mod nodes;
mod scroll;

pub(in crate::runtime) use capacity::{clear_paint_plan_for_layout, empty_paint_plan_for_layout};
pub(super) use context::SurfacePaintContext;

#[cfg(test)]
#[path = "paint/tests.rs"]
mod tests;

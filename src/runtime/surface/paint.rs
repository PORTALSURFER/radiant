//! Surface paint projection for runtime nodes.

mod capacity;
mod context;
mod nodes;

pub(in crate::runtime) use capacity::estimated_paint_primitive_capacity;
pub(super) use context::SurfacePaintContext;

#[cfg(test)]
#[path = "paint/tests.rs"]
mod tests;

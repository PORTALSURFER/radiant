//! Surface paint projection for runtime nodes.

mod context;
mod nodes;

pub(super) use context::SurfacePaintContext;

#[cfg(test)]
#[path = "paint/tests.rs"]
mod tests;

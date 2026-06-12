use super::NormalizedRange;

mod model;
mod navigation;
mod projection;
mod ratio;
mod scope;

pub use model::IndexViewport;
pub use scope::IndexViewportScope;

#[cfg(test)]
#[path = "index_viewport/tests.rs"]
mod tests;

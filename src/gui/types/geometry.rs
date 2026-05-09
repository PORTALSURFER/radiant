//! Backend-neutral geometry types.

mod point;
mod rect;
mod vector;

pub use point::Point;
pub use rect::Rect;
pub use vector::Vector2;

#[cfg(test)]
mod tests;

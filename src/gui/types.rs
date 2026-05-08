//! Backend-neutral geometry, color, and image buffer types.

mod color;
mod geometry;
mod image;

pub use color::Rgba8;
pub use geometry::{Point, Rect, Vector2};
pub use image::ImageRgba;

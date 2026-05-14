//! Keyboard and pointer input primitives used by hotkeys and GUI backends.

mod key;
mod pointer;

pub use key::{KeyCode, KeyPress};
pub use pointer::logical_point_to_u16_coords;

//! Backend-neutral widget input contracts.

mod event;
mod keyboard;
mod pointer;
mod text_edit;

pub use event::WidgetInput;
pub use keyboard::WidgetKey;
pub use pointer::{PointerButton, PointerModifiers};
pub use text_edit::TextEditCommand;

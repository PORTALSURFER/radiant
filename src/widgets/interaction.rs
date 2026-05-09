//! Backend-neutral widget interaction events and emitted messages.

mod input;
mod messages;
mod output;

pub use input::{PointerButton, TextEditCommand, WidgetInput, WidgetKey};
pub use messages::{
    BadgeMessage, ButtonMessage, CanvasMessage, DragHandleMessage, GpuSurfaceMessage,
    ListItemMessage, ScrollbarMessage, SelectableMessage, TextInputMessage, ToggleMessage,
};
pub use output::{CustomWidgetOutput, WidgetOutput};

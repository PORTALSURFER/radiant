//! Backend-neutral widget interaction events and emitted messages.

mod canvas_gesture;
mod input;
mod messages;
mod output;

pub use canvas_gesture::{CanvasGestureEvent, CanvasGestureState, CanvasPointer};
pub use input::{PointerButton, PointerModifiers, TextEditCommand, WidgetInput, WidgetKey};
pub use messages::{
    BadgeMessage, ButtonMessage, CanvasMessage, DragHandleMessage, GpuSurfaceMessage,
    InteractiveRowMessage, ListItemMessage, ScrollbarMessage, SelectableMessage, SliderMessage,
    TextInputMessage, ToggleMessage,
};
pub use output::{CustomWidgetOutput, WidgetOutput};

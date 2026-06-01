//! Backend-neutral widget interaction events and emitted messages.

mod activation;
mod canvas_gesture;
mod cursor;
mod input;
mod messages;
mod output;

pub use activation::{ActivationInputPolicy, ActivationInputResult, handle_activation_input};
pub use canvas_gesture::{CanvasGestureEvent, CanvasGestureState, CanvasPointer};
pub use cursor::WidgetCursor;
pub use input::{PointerButton, PointerModifiers, TextEditCommand, WidgetInput, WidgetKey};
pub use messages::{
    BadgeMessage, ButtonMessage, CanvasMessage, DragHandleMessage, DragHandlePhase,
    GpuSurfaceMessage, InteractiveRowMessage, ListItemMessage, PointerShieldMessage,
    ScrollbarMessage, SelectableMessage, SliderMessage, TextInputMessage, ToggleMessage,
};
pub use output::{CustomWidgetOutput, WidgetOutput};

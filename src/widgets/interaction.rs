//! Backend-neutral widget interaction events and emitted messages.

mod activation;
mod canvas_gesture;
mod cursor;
mod edit;
mod input;
mod messages;
mod output;
mod provenance;

pub use activation::{ActivationInputPolicy, ActivationInputResult, handle_activation_input};
pub use canvas_gesture::{
    CanvasGestureEvent, CanvasGestureMetadata, CanvasGestureState, CanvasPointer,
};
pub use cursor::WidgetCursor;
pub use edit::{EditEvent, EditPhase, EditTransaction};
pub use input::{PointerButton, PointerModifiers, TextEditCommand, WidgetInput, WidgetKey};
pub use messages::{
    BadgeMessage, ButtonMessage, CanvasMessage, DragHandleMessage, DragHandleMetadata,
    DragHandlePhase, GpuSurfaceMessage, InteractiveRowMessage, InteractiveRowMetadata,
    KnobAutomationEvent, KnobEditBatch, KnobKeyboardGesture, KnobKeyboardMetadata, KnobMessage,
    KnobPointerMetadata, KnobWheelGesture, KnobWheelMetadata, ListItemMessage,
    PointerShieldMessage, RenderCanvasMessage, ScrollbarMessage, SelectableMessage,
    SliderEditBatch, SliderMessage, TextInputMessage, TextInputMessageKind, TextInputMessageParts,
    ToggleMessage,
};
pub use output::{CustomWidgetOutput, WidgetOutput};
pub use provenance::{InteractionProvenance, InteractionSource};

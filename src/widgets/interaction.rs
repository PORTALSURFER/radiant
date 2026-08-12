//! Backend-neutral widget interaction events and emitted messages.

mod activation;
mod canvas_gesture;
mod cursor;
mod edit;
mod format;
mod input;
mod messages;
mod numeric_adjustment;
mod numeric_edit;
mod numeric_input;
mod numeric_ownership;
mod numeric_policy;
mod numeric_step_modifiers;
mod output;
mod provenance;
mod text_input_revision;
mod value;

pub use activation::{ActivationInputPolicy, ActivationInputResult, handle_activation_input};
pub use canvas_gesture::{
    CanvasGestureEvent, CanvasGestureMetadata, CanvasGestureState, CanvasPointer,
};
pub use cursor::WidgetCursor;
pub use edit::{EditEvent, EditPhase, EditTransaction};
pub use format::{DecimalSeparator, ValueFormat, ValueFormatError, ValueFormatKind};
pub(crate) use input::CompositionSelectionState;
pub use input::{
    CompositionPhase, CompositionRange, CompositionRangeError, CompositionSample,
    CompositionSampleError, CompositionStartContext, KeyboardModifiers, PointerButton,
    PointerModifiers, TextEditCommand, WHEEL_LINE_EQUIVALENCE_PIXELS, WheelDelta, WheelDeltaError,
    WheelPhase, WheelSample, WheelSampleError, WidgetInput, WidgetKey,
};
pub use messages::{
    BadgeMessage, ButtonMessage, CanvasMessage, DragHandleMessage, DragHandleMetadata,
    DragHandlePhase, GpuSurfaceMessage, InteractiveRowMessage, InteractiveRowMetadata,
    KnobAutomationEvent, KnobEditBatch, KnobKeyboardGesture, KnobKeyboardMetadata, KnobMessage,
    KnobPointerMetadata, KnobWheelGesture, KnobWheelMetadata, ListItemMessage,
    PointerShieldMessage, RenderCanvasMessage, ScrollbarMessage, SelectableMessage,
    SliderEditBatch, SliderMessage, TextInputMessage, TextInputMessageKind, TextInputMessageParts,
    ToggleMessage,
};
pub use numeric_adjustment::{NumericAdjustment, NumericStep, NumericStepDirection};
pub use numeric_edit::NumericEditSession;
pub use numeric_input::{
    NumericAccessibilityAction, NumericAccessibilityBlockOwner, NumericAccessibilityOutcome,
    NumericAccessibilityRejectedReason, NumericInputConstructionError, NumericInputEditBatch,
    NumericInputInteraction, NumericInputInteractionBatch, NumericScrubActivation,
    NumericScrubAttempt, NumericScrubPolicy, NumericStepAttempt, NumericWheelAttempt,
    NumericWheelPolicy,
};
pub(crate) use numeric_ownership::{NumericInteractionGate, NumericInteractionOwner};
pub use numeric_policy::{NumericCodec, NumericParseResult};
pub use numeric_step_modifiers::{KeyboardModifier, NumericStepModifiers};
pub use output::{CustomWidgetOutput, WidgetOutput};
pub use provenance::{InteractionProvenance, InteractionSource};
pub use text_input_revision::TextInputRevision;
pub use value::{ValueMapping, ValueMappingError, ValueMappingKind};

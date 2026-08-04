//! Widget contract, primitive, and interaction prelude exports.

pub use crate::widgets::{
    ActivationInputPolicy, ActivationInputResult, ColorMarkerAlign, ColorMarkerProps,
    DragHandleMessage, DragHandleMetadata, DragHandlePhase, FocusBehavior, InteractiveRowMessage,
    InteractiveRowMetadata, InteractiveRowPointerMotion, KnobAutomationEvent, KnobKeyboardGesture,
    KnobKeyboardMetadata, KnobMessage, KnobWheelGesture, KnobWheelMetadata, PointerButton,
    PointerCapturePolicy, PointerShieldMessage, ProgressBarMessage, ProgressBarMode, ScrollbarAxis,
    ScrollbarMessage, SliderMessage, TextAlign, TextBackgroundRole, TextColorRole, TextInputChrome,
    TextInputEditResult, TextInputState, TextWrap, Widget, WidgetCommon, WidgetCursor, WidgetInput,
    WidgetKey, WidgetOutput, WidgetPaintContext, WidgetProminence, WidgetSizing, WidgetState,
    WidgetStyle, WidgetTone, WidgetVisualCue, WidgetVisualTokens, handle_activation_input,
    stable_widget_id, stable_widget_id_u64,
};

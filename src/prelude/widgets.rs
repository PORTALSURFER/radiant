//! Widget contract, primitive, and interaction prelude exports.

pub use crate::widgets::{
    ActivationInputPolicy, ActivationInputResult, ColorMarkerAlign, ColorMarkerProps,
    DragHandleMessage, DragHandlePhase, FocusBehavior, InteractiveRowMessage,
    InteractiveRowPointerMotion, InteractiveRowVisualStateParts, PointerButton,
    PointerCapturePolicy, PointerShieldMessage, ProgressBarMessage, ProgressBarMode, ScrollbarAxis,
    ScrollbarMessage, SliderMessage, TextAlign, TextBackgroundRole, TextColorRole, TextInputChrome,
    TextInputEditResult, TextInputState, TextWrap, Widget, WidgetCommon, WidgetCursor, WidgetInput,
    WidgetKey, WidgetOutput, WidgetProminence, WidgetSizing, WidgetState, WidgetStyle, WidgetTone,
    handle_activation_input, stable_widget_id, stable_widget_id_u64,
};

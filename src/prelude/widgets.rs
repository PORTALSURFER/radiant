//! Widget contract, primitive, and interaction prelude exports.

pub use crate::widgets::{
    ActivationInputPolicy, ActivationInputResult, BadgeWidgetParts, ButtonWidgetParts,
    CanvasGestureEvent, CanvasGestureState, CanvasPointer, CanvasWidgetParts, CardWidgetParts,
    ColorMarkerAlign, ColorMarkerProps, ColorMarkerWidget, ColorMarkerWidgetParts,
    DragHandleMessage, DragHandlePhase, DragHandleWidgetParts, EmbeddedInteractiveRowWidget,
    FeedbackOverlayEdge, FeedbackOverlayProgress, FeedbackOverlayProps, FeedbackOverlayWidget,
    FeedbackOverlayWidgetParts, FocusBehavior, GpuSurfaceMessage, GpuSurfaceParts,
    GpuSurfaceWidget, IconButtonWidget, IconButtonWidgetParts, ImageWidgetParts,
    InteractiveRowMessage, InteractiveRowPointerMotion, InteractiveRowVisualStateParts,
    InteractiveRowWidget, InteractiveRowWidgetParts, ListItemWidgetParts, MarkerRunAlign,
    MarkerRunProps, MarkerRunWidget, MarkerRunWidgetParts, PointerButton, PointerShieldMessage,
    PointerShieldWidget, PointerShieldWidgetParts, ProgressBarMessage, ProgressBarMode,
    ProgressBarProps, ProgressBarWidget, ProgressBarWidgetParts, ScrollbarAxis, ScrollbarMessage,
    ScrollbarWidgetParts, SelectableWidgetParts, SliderMessage, SliderWidget, SliderWidgetParts,
    TextAlign, TextBackgroundRole, TextColorRole, TextInputChrome, TextInputEditResult,
    TextInputState, TextInputWidgetParts, TextWidgetParts, TextWrap, ToggleWidgetParts, Widget,
    WidgetCommon, WidgetCursor, WidgetInput, WidgetKey, WidgetOutput, WidgetProminence,
    WidgetSizing, WidgetSizingParts, WidgetState, WidgetStyle, WidgetTone, WidgetVisualTokens,
    handle_activation_input, resolve_widget_visual_tokens, stable_widget_id, stable_widget_id_u64,
};

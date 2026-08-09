//! First-class public widget contracts for `radiant`.
//!
//! `radiant::layout` owns container placement. `radiant::widgets` owns the leaf
//! and tightly-bounded composite vocabulary that applications place inside those
//! containers.
//!
//! This module is intentionally additive and design-focused:
//! - it documents the shared sizing, focus, and paint contracts
//! - it lets the generic runtime project reusable widgets into paint data
//! - it keeps widget variation in concrete widget implementations
//!
//! Native runtime adapters can layer on top of this vocabulary without changing
//! the public widget contracts.
//!
//! # Example
//!
//! ```
//! use radiant::{
//!     layout::{
//!         ContainerKind, ContainerPolicy, LayoutNode, Point, Rect, SlotChild, SlotParams,
//!         Vector2, layout_tree,
//!     },
//!     widgets::{ButtonWidget, ButtonWidgetParts, TextWidget, TextWidgetParts, WidgetSizing},
//! };
//!
//! let title = TextWidget::from_parts(TextWidgetParts {
//!     id: 10,
//!     text: "Items".into(),
//!     sizing: WidgetSizing::fixed(Vector2::new(80.0, 20.0)).with_baseline(14.0),
//! });
//! let add_button = ButtonWidget::from_parts(ButtonWidgetParts {
//!     id: 11,
//!     label: "Import".into(),
//!     sizing: WidgetSizing::fixed(Vector2::new(96.0, 28.0)),
//! });
//!
//! let layout = LayoutNode::container(
//!     1,
//!     ContainerPolicy {
//!         kind: ContainerKind::Row,
//!         spacing: 8.0,
//!         ..ContainerPolicy::default()
//!     },
//!     vec![
//!         SlotChild::new(SlotParams::fill(), title.common.layout_node()),
//!         SlotChild::new(SlotParams::fill(), add_button.common.layout_node()),
//!     ],
//! );
//!
//! let output = layout_tree(
//!     &layout,
//!     Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(240.0, 32.0)),
//! );
//!
//! assert!(output.rects.contains_key(&title.common.id));
//! assert!(output.rects.contains_key(&add_button.common.id));
//! ```

mod contract;
pub mod interaction;
mod primitives;
mod theme;

pub(crate) use contract::WidgetRevisionComponents;
pub use contract::{
    FocusBehavior, FocusLossDecision, PaintBounds, PaintContract, PointerCapturePolicy,
    WIDGET_CAPABILITIES_CONTRACT_VERSION, Widget, WidgetCapabilities, WidgetId, WidgetPaintContext,
    WidgetProminence, WidgetRevision, WidgetSemantics, WidgetSemanticsRevision, WidgetSizing,
    WidgetSizingParts, WidgetState, WidgetStyle, WidgetTone, stable_widget_id,
    stable_widget_id_u64,
};
pub use interaction::{
    ActivationInputPolicy, ActivationInputResult, BadgeMessage, ButtonMessage, CanvasGestureEvent,
    CanvasGestureMetadata, CanvasGestureState, CanvasMessage, CanvasPointer, CustomWidgetOutput,
    DecimalSeparator, DragHandleMessage, DragHandleMetadata, DragHandlePhase, EditEvent, EditPhase,
    EditTransaction, GpuSurfaceMessage, InteractionProvenance, InteractionSource,
    InteractiveRowMessage, InteractiveRowMetadata, KeyboardModifier, KeyboardModifiers,
    KnobAutomationEvent, KnobEditBatch, KnobKeyboardGesture, KnobKeyboardMetadata, KnobMessage,
    KnobPointerMetadata, KnobWheelGesture, KnobWheelMetadata, ListItemMessage, NumericAdjustment,
    NumericCodec, NumericEditSession, NumericInputConstructionError, NumericInputEditBatch,
    NumericInputInteraction, NumericInputInteractionBatch, NumericParseResult, NumericStep,
    NumericStepAttempt, NumericStepDirection, NumericStepModifiers, PointerButton,
    PointerModifiers, PointerShieldMessage, RenderCanvasMessage, ScrollbarMessage,
    SelectableMessage, SliderEditBatch, SliderMessage, TextEditCommand, TextInputMessage,
    TextInputMessageKind, TextInputMessageParts, TextInputRevision, ToggleMessage, ValueFormat,
    ValueFormatError, ValueFormatKind, ValueMapping, ValueMappingError, ValueMappingKind,
    WidgetCursor, WidgetInput, WidgetKey, WidgetOutput, handle_activation_input,
};
pub(crate) use primitives::NumericInputWidget;
pub use primitives::{
    BadgeProps, BadgeState, BadgeWidget, BadgeWidgetParts, ButtonProps, ButtonState, ButtonWidget,
    ButtonWidgetParts, CanvasWidget, CanvasWidgetParts, CardWidget, CardWidgetParts,
    ColorMarkerAlign, ColorMarkerProps, ColorMarkerRunProps, ColorMarkerRunWidget,
    ColorMarkerRunWidgetParts, ColorMarkerWidget, ColorMarkerWidgetParts, DragHandleWidget,
    DragHandleWidgetParts, EmbeddedInteractiveRowWidget, FeedbackOverlayEdge,
    FeedbackOverlayProgress, FeedbackOverlayProps, FeedbackOverlayWidget,
    FeedbackOverlayWidgetParts, GpuSurfaceParts, GpuSurfaceWidget, IconButtonWidget,
    IconButtonWidgetParts, ImageProps, ImageWidget, ImageWidgetParts, InteractiveRowActions,
    InteractiveRowLocalActions, InteractiveRowPointerMotion, InteractiveRowProps,
    InteractiveRowVisualStateParts, InteractiveRowWidget, InteractiveRowWidgetParts, KnobProps,
    KnobState, KnobWidget, KnobWidgetParts, ListItemWidget, ListItemWidgetParts, MarkerRunAlign,
    MarkerRunProps, MarkerRunWidget, MarkerRunWidgetParts, PointerShieldProps, PointerShieldWidget,
    PointerShieldWidgetParts, ProgressBarMessage, ProgressBarMode, ProgressBarProps,
    ProgressBarWidget, ProgressBarWidgetParts, RenderCanvasParts, RenderCanvasWidget,
    RetainedSurfaceDescriptor, ScrollbarAxis, ScrollbarProps, ScrollbarState, ScrollbarWidget,
    ScrollbarWidgetParts, SelectableProps, SelectableWidget, SelectableWidgetParts, SliderProps,
    SliderState, SliderWidget, SliderWidgetParts, TextAlign, TextBackgroundRole, TextColorRole,
    TextInputChrome, TextInputEditResult, TextInputProps, TextInputState, TextInputWidget,
    TextInputWidgetParts, TextWidget, TextWidgetParts, TextWrap, ToggleProps, ToggleState,
    ToggleWidget, ToggleWidgetParts, WidgetCommon,
};
pub(crate) use primitives::{RetainedKnobWidget, RetainedSliderWidget};
pub use theme::{WidgetVisualCue, WidgetVisualTokens, resolve_widget_visual_tokens};

//! # Radiant
//!
//! Radiant is a macOS-first Rust GUI library for building native desktop
//! applications, with a cross-platform design goal. It is
//! application-independent: your application owns its state and domain
//! behavior, while Radiant provides declarative views, application updates,
//! layout, input, focus, styling, rendering, and runtime integration. The
//! public API grows from a small app to an explicitly hosted UI while keeping
//! platform-specific details behind runtime boundaries.
//!
//! ## Start with an app
//!
//! The [`prelude`] is the normal starting point for applications. The
//! application and view builders keep a first window concise:
//!
//! ```no_run
//! use radiant::prelude::*;
//!
//! fn main() -> radiant::Result {
//!     radiant::window("Radiant Hello World").run(text("Hello, world!"))
//! }
//! ```
//!
//! ## Find your next layer
//!
//! - **Prelude, applications, and views:** begin with [`prelude`], then use
//!   [`application`] to define stateful apps and view projections.
//! - **Widgets and composition:** compose [`widgets`], [`layout`], and
//!   [`theme`] contracts when an app needs reusable controls, containers, or
//!   visual tokens beyond the builder defaults.
//! - **Runtime and custom hosting:** advanced embedders can use [`runtime`],
//!   [`runtime::RuntimeBridge`], [`runtime::UiSurface`],
//!   [`runtime::SurfaceNode`], and [`runtime::NativeRunOptions`] to own the
//!   host boundary while keeping the same declarative model.
//!
//! The application builders and explicit runtime objects are supported parts of
//! one API; the latter are not a separate framework.
//!
//! ## Design and API guides
//!
//! The repository guides extend this API reference:
//!
//! - [README and installation overview](https://github.com/PORTALSURFER/radiant#readme)
//! - [API guide](https://github.com/PORTALSURFER/radiant/blob/main/docs/API.md)
//! - [Architecture map](https://github.com/PORTALSURFER/radiant/blob/main/docs/ARCHITECTURE.md)
//! - [Design direction](https://github.com/PORTALSURFER/radiant/blob/main/docs/DESIGN_DIRECTION.md)
//! - [Long-term target](https://github.com/PORTALSURFER/radiant/blob/main/docs/TARGET.md)
//! - [Timer host migration](https://github.com/PORTALSURFER/radiant/blob/main/docs/migrations/TIMER_API_MIGRATION.md)
//!
//! The maintained [examples](https://github.com/PORTALSURFER/radiant/tree/main/examples)
//! provide progressively richer application patterns, including
//! [`hello_world`](https://github.com/PORTALSURFER/radiant/blob/main/examples/hello_world.rs),
//! [`counter`](https://github.com/PORTALSURFER/radiant/blob/main/examples/counter.rs),
//! [`widget_gallery`](https://github.com/PORTALSURFER/radiant/blob/main/examples/widget_gallery.rs),
//! and custom-host-oriented [`generic_native`](https://github.com/PORTALSURFER/radiant/blob/main/examples/generic_native.rs).
//!
//! Generic host-facing modules include [`gui_runtime`] for native runtime
//! adapters, [`runtime`] for the declarative view/message bridge, and the
//! reusable [`layout`], [`widgets`], and [`theme`] contracts.

/// Readable application and view builder implementation.
pub mod application;
/// Shared environment-flag parsing helpers used by runtime internals.
#[cfg(test)]
extern crate radiant_native_test_support;

mod env_flags;
/// Internal marker for values owned by one UI runtime.
mod ui_affinity;
pub(crate) use ui_affinity::UiAffinity;
/// Reusable static guardrails for Radiant host application tests.
pub mod guardrails;
/// Backend-agnostic GUI primitives.
pub mod gui;
/// Common imports for Radiant apps.
pub mod prelude;
/// Stable public slot-based layout API.
pub mod layout {
    pub use crate::gui::layout_core::{
        Constraints, ConstraintsParts, ContainerKind, ContainerNode, ContainerNodeParts,
        ContainerPolicy, ContainerStateDeclaration, ContainerStateId, Controlled, CrossAlign,
        DebugPrimitiveKind, FloatingLayerHorizontalOverflow, FloatingLayerPolicy,
        FloatingLayerVerticalOverflow, GridPolicy, Insets, LAYOUT_CAPABILITIES_CONTRACT_VERSION,
        LAYOUT_CAPABILITIES_PROJECTION_CONTRACT_VERSION,
        LAYOUT_CAPABILITIES_STATE_CONTRACT_VERSION, LayoutCapabilities,
        LayoutContainerStateContext, LayoutDebugOptions, LayoutDebugPrimitive, LayoutDiagnostic,
        LayoutDiagnosticCode, LayoutDragSource, LayoutDropTarget, LayoutEngine, LayoutEventContext,
        LayoutGestures, LayoutHitRegion, LayoutHitRegionDeclarationError,
        LayoutHitRegionDiagnostics, LayoutHitRegionId, LayoutHitTarget, LayoutInput,
        LayoutInteraction, LayoutInteractionCapabilities, LayoutInteractionRevision, LayoutNode,
        LayoutOmissionReason, LayoutOutput, LayoutPolicy, LayoutPolicyOmissionReason,
        LayoutPolicyPlacementError, LayoutState, LayoutStats, LayoutTargetIdentity, MainAlign,
        MeasureChildError, MeasureChildren, MeasureChildrenError, NodeId, OverflowInfo,
        OverflowPolicy, PlaceChildren, PlaceChildrenError, Point, Rect, ScrollAlignment,
        ScrollAxis, ScrollAxisLock, ScrollDeclaration, ScrollEdge, ScrollPolicy, ScrollRequest,
        ScrollRuntimeState, ScrollTarget, ScrollbarPlacement, ScrollbarVisibility, SizeHint,
        SizeModeCross, SizeModeMain, SlotChild, SlotChildParts, SlotParams, SplitPaneAxis,
        SplitPaneCollapsePolicy, SplitPanePolicy, StackedRowRectsParts, SwitchBreakpoint,
        VIRTUAL_LAYOUT_MAX_QUERY_ENTRIES, Vector2, VirtualLayoutBoundsConfidence,
        VirtualLayoutBudget, VirtualLayoutCoordinateSpace, VirtualLayoutDeferredReason,
        VirtualLayoutDiagnostic, VirtualLayoutDiagnosticCode, VirtualLayoutDiagnostics,
        VirtualLayoutExtent, VirtualLayoutExtentCandidate, VirtualLayoutExtentKind,
        VirtualLayoutFenceField, VirtualLayoutFenceFields, VirtualLayoutInputError,
        VirtualLayoutItem, VirtualLayoutItemCandidate, VirtualLayoutItemKey,
        VirtualLayoutItemKeyCandidate, VirtualLayoutOverscan, VirtualLayoutPolicy,
        VirtualLayoutPolicyDecision, VirtualLayoutPolicyIdentity, VirtualLayoutQueryExecutor,
        VirtualLayoutQueryFence, VirtualLayoutQueryInput, VirtualLayoutQueryInputParts,
        VirtualLayoutQueryOutcome, VirtualLayoutQueryResult, VirtualLayoutQuerySink,
        VirtualLayoutSinkError, VirtualLayoutUnavailableReason, VirtualLayoutVisibility,
        VirtualWindowInfo, VirtualizationAxis, VirtualizationPolicy, WidgetNode, WidgetNodeParts,
        WrapPolicy, WritingDirection, fixed_width_group_width,
        fixed_width_item_extent_for_available_width, fixed_width_row_rects_end,
        fixed_width_row_rects_end_into, fixed_width_row_rects_start,
        fixed_width_row_rects_start_into, grouped_fixed_width_row_width, layout_tree,
        layout_tree_with_direction, layout_tree_with_state, stacked_row_rects,
        stacked_row_rects_from_parts, stacked_row_rects_into, stacked_row_rects_into_from_parts,
        visible_suffix_widths, visible_suffix_widths_into,
    };
    pub(crate) use crate::gui::layout_core::{
        supports_layout_capabilities_contract, supports_layout_input_contract,
        supports_layout_state_input_contract,
    };
}
/// Shared runtime host implementations.
pub mod gui_runtime;
/// Generic declarative view/message runtime surface for new hosts.
pub mod runtime;
/// Generic theme tokens for reusable Radiant widgets and containers.
pub mod theme;
/// Stable public widget contracts.
pub mod widgets;

// Typed pointer/gesture ingress is available from the crate root as well as
// `gui::pointer_ingress`; it intentionally remains outside the common prelude.
pub use gui::pointer_ingress::{
    DeviceKind, GestureIngress, GestureIngressDisposition, GestureIngressError, GestureKind,
    GesturePhase, GestureUnit, InputDeviceId, InvalidPointerButtons, InvalidPointerIdentity,
    InvalidPointerPressure, InvalidPointerTilt, PointerButtons, PointerContactId, PointerEvent,
    PointerIngress, PointerIngressAdmission, PointerIngressDisposition, PointerIngressError,
    PointerPhase, PointerPressure, PointerSequenceToken, PointerTilt,
};

pub use application::{
    DEFAULT_COLUMN_SPACING, DEFAULT_GRID_GAP, DEFAULT_ROW_SPACING,
    DEFAULT_STYLED_CONTAINER_PADDING, Layer, LayerInputPolicy, Result, app, window,
};

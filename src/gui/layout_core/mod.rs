//! Stable slot-based layout primitives for `radiant`.
//!
//! This module exposes a deterministic two-pass measure/layout engine that is
//! independent from the current native shell. Applications describe a layout
//! tree with [`LayoutNode`](crate::layout::LayoutNode) values, configure parent-owned slot behavior with
//! [`SlotParams`](crate::layout::SlotParams), and then run
//! [`layout_tree`](crate::layout::layout_tree) or [`LayoutEngine`](crate::layout::LayoutEngine) to produce
//! a [`LayoutOutput`](crate::gui::layout_core::LayoutOutput).
//!
//! The layout flow is intentionally explicit:
//! - widgets provide intrinsic size hints through [`WidgetNode`](crate::layout::WidgetNode)
//! - containers own child placement through [`ContainerPolicy`](crate::layout::ContainerPolicy)
//! - slots express the contract between a parent and one child
//! - the engine measures bottom-up and assigns final rectangles top-down
//!
//! Baseline container policies:
//! - [`ContainerKind::Row`](crate::layout::ContainerKind::Row)
//! - [`ContainerKind::Column`](crate::layout::ContainerKind::Column)
//! - [`ContainerKind::Stack`](crate::layout::ContainerKind::Stack)
//! - [`ContainerKind::PaddingBox`](crate::layout::ContainerKind::PaddingBox)
//! - [`ContainerKind::AlignBox`](crate::layout::ContainerKind::AlignBox)
//! - [`ContainerKind::AspectBox`](crate::layout::ContainerKind::AspectBox)
//! - [`ContainerKind::Grid`](crate::layout::ContainerKind::Grid)
//! - [`ContainerKind::ScrollView`](crate::layout::ContainerKind::ScrollView)
//! - [`ContainerKind::Wrap`](crate::layout::ContainerKind::Wrap)
//! - [`ContainerKind::SwitchLayout`](crate::layout::ContainerKind::SwitchLayout)
//! - [`ContainerKind::FloatingLayer`](crate::layout::ContainerKind::FloatingLayer)
//! - [`ContainerKind::SplitPane`](crate::layout::ContainerKind::SplitPane)
//!
//! Qualified [`LayoutCapabilities`](crate::layout::LayoutCapabilities) and
//! [`LayoutInteraction`](crate::layout::LayoutInteraction) registration is
//! available for backend-neutral containers, including exact or conservative
//! revision evidence and normalized hit-region declarations. Production
//! runtime projection exposes read-only [`LayoutHitTarget`](crate::layout::LayoutHitTarget) values;
//! version-3 capabilities may receive typed pointer input while version 4 adds
//! optional runtime-owned typed container state; the runtime owns routing and
//! capture for both. State slots are UI-local and bounded, and are distinct
//! from [`LayoutState`](crate::layout::LayoutState) scroll offsets.
//! Static `split_pane` construction is available through the application
//! builders. Runtime-owned ratio mode additionally projects one clipped
//! built-in divider target when its resolved divider is positive; primary
//! pointer drags use controller-owned capture and the mounted ratio state.
//! Static and controlled-ratio modes remain non-interactive. Semantic/keyboard
//! behavior and the target `VirtualLayoutPolicy` remain future runtime work.
//!
//! # Example
//!
//! ```
//! use radiant::layout::{
//!     ContainerKind, ContainerNodeParts, ContainerPolicy, LayoutNode, Point, Rect, SlotChild,
//!     SlotChildParts, SlotParams, Vector2, WidgetNodeParts, layout_tree,
//! };
//!
//! let root = LayoutNode::container_from_parts(ContainerNodeParts {
//!     id: 1,
//!     policy: ContainerPolicy {
//!         kind: ContainerKind::Row,
//!         spacing: 8.0,
//!         ..ContainerPolicy::default()
//!     },
//!     children: vec![
//!         SlotChild::from_parts(SlotChildParts {
//!             slot: SlotParams::fill(),
//!             child: LayoutNode::widget_from_parts(WidgetNodeParts {
//!                 id: 2,
//!                 intrinsic: Vector2::new(40.0, 20.0),
//!             }),
//!         }),
//!         SlotChild::from_parts(SlotChildParts {
//!             slot: SlotParams::fill(),
//!             child: LayoutNode::widget_from_parts(WidgetNodeParts {
//!                 id: 3,
//!                 intrinsic: Vector2::new(40.0, 20.0),
//!             }),
//!         }),
//!     ],
//! });
//!
//! let output = layout_tree(
//!     &root,
//!     Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(200.0, 40.0)),
//! );
//!
//! assert!(output.rects.contains_key(&2));
//! assert!(output.rects.contains_key(&3));
//! ```

mod capabilities;
mod constraints;
mod controlled;
mod engine;
mod model;
mod policy;
mod row_helpers;
mod scroll;
mod split_pane_interaction;
mod split_pane_state;
mod tree;
mod validated_geometry;
mod virtual_layout;

pub use crate::gui::panel::{SplitPaneAxis, SplitPaneCollapsePolicy};
pub use crate::gui::types::{Point, Rect, Vector2};
pub use capabilities::{
    ContainerStateDeclaration, ContainerStateId, LAYOUT_CAPABILITIES_CONTRACT_VERSION,
    LAYOUT_CAPABILITIES_PROJECTION_CONTRACT_VERSION, LAYOUT_CAPABILITIES_STATE_CONTRACT_VERSION,
    LayoutCapabilities, LayoutContainerStateContext, LayoutDragSource, LayoutDropTarget,
    LayoutEventContext, LayoutGestures, LayoutHitRegion, LayoutHitRegionDeclarationError,
    LayoutHitRegionDiagnostics, LayoutHitRegionId, LayoutHitTarget, LayoutInput, LayoutInteraction,
    LayoutInteractionCapabilities, LayoutInteractionRevision, LayoutTargetIdentity,
};
pub(crate) use capabilities::{
    MountedContainerStateId, MountedContainerStateRead, supports_layout_capabilities_contract,
    supports_layout_input_contract, supports_layout_state_input_contract,
};
pub use constraints::{Constraints, ConstraintsParts};
pub use controlled::Controlled;
pub use engine::{
    DebugPrimitiveKind, LayoutDebugOptions, LayoutDebugPrimitive, LayoutDiagnostic,
    LayoutDiagnosticCode, LayoutEngine, LayoutOutput, LayoutState, LayoutStats, OverflowInfo,
    ScrollRuntimeState, VirtualWindowInfo, layout_tree, layout_tree_with_direction,
    layout_tree_with_state,
};
pub(crate) use engine::{
    LayoutAuthorityEvidence, LayoutContainerStateReadSource, LayoutInputEvidence,
    LayoutStateAuthorityOwner, MountedLayoutSourceAuthorityOwner, PreparedLayoutPass,
    RootLayoutAuthorityOwner,
};
pub use model::{
    ContainerKind, ContainerPolicy, CrossAlign, FloatingLayerHorizontalOverflow,
    FloatingLayerPolicy, FloatingLayerVerticalOverflow, GridPolicy, Insets, MainAlign,
    OverflowPolicy, SizeModeCross, SizeModeMain, SlotParams, SplitPanePolicy, SwitchBreakpoint,
    VirtualizationAxis, VirtualizationPolicy, WrapPolicy, WritingDirection,
};
pub use policy::{
    LayoutOmissionReason, LayoutPolicy, LayoutPolicyOmissionReason, LayoutPolicyPlacementError,
    MeasureChildError, MeasureChildren, MeasureChildrenError, PlaceChildren, PlaceChildrenError,
    SizeHint,
};
pub use row_helpers::{
    StackedLayoutCursor, StackedLayoutItem, StackedRowRectsParts, fixed_width_group_width,
    fixed_width_item_extent_for_available_width, fixed_width_row_rects_end,
    fixed_width_row_rects_end_into, fixed_width_row_rects_start, fixed_width_row_rects_start_into,
    grouped_fixed_width_row_width, stacked_row_rects, stacked_row_rects_from_parts,
    stacked_row_rects_into, stacked_row_rects_into_from_parts, visible_suffix_widths,
    visible_suffix_widths_into,
};
pub use scroll::{
    ScrollAlignment, ScrollAxis, ScrollAxisLock, ScrollDeclaration, ScrollEdge, ScrollPolicy,
    ScrollRequest, ScrollTarget, ScrollbarPlacement, ScrollbarVisibility, resolve_scroll_alignment,
};
pub(crate) use split_pane_interaction::{
    SPLIT_PANE_DIVIDER_REGION_ID, SplitPaneCaptureWitness, SplitPaneDividerDescriptor,
    SplitPaneRatioAdjustment, apply_split_pane_ratio_delta, runtime_owned_split_pane_capabilities,
    runtime_owned_split_pane_capabilities_with_ratio_settled,
};
pub(crate) use split_pane_state::{
    SplitPaneRuntimeMode, SplitPaneRuntimeOwnership, SplitPaneRuntimePolicyRevision,
    SplitPaneRuntimeState, SplitPaneRuntimeStateInput, sanitize_runtime_ratio,
};
pub use tree::{
    ContainerNode, ContainerNodeParts, LayoutNode, NodeId, SlotChild, SlotChildParts, WidgetNode,
    WidgetNodeParts,
};
pub(crate) use virtual_layout::VirtualLayoutSemanticDeferredReason;
pub use virtual_layout::{
    VIRTUAL_LAYOUT_MAX_QUERY_ENTRIES, VirtualLayoutBoundsConfidence, VirtualLayoutBudget,
    VirtualLayoutCoordinateSpace, VirtualLayoutDeferredReason, VirtualLayoutDiagnostic,
    VirtualLayoutDiagnosticCode, VirtualLayoutDiagnostics, VirtualLayoutExtent,
    VirtualLayoutExtentCandidate, VirtualLayoutExtentKind, VirtualLayoutFenceField,
    VirtualLayoutFenceFields, VirtualLayoutInputError, VirtualLayoutItem,
    VirtualLayoutItemCandidate, VirtualLayoutItemKey, VirtualLayoutItemKeyCandidate,
    VirtualLayoutOverscan, VirtualLayoutPolicy, VirtualLayoutPolicyDecision,
    VirtualLayoutPolicyIdentity, VirtualLayoutQueryExecutor, VirtualLayoutQueryFence,
    VirtualLayoutQueryInput, VirtualLayoutQueryInputParts, VirtualLayoutQueryOutcome,
    VirtualLayoutQueryResult, VirtualLayoutQuerySink, VirtualLayoutSinkError,
    VirtualLayoutUnavailableReason, VirtualLayoutVisibility,
};
pub(crate) use virtual_layout::{
    VirtualLayoutBatchProjector, VirtualLayoutCompletion, VirtualLayoutLifecycleAdapter,
    VirtualLayoutMaterializationError, VirtualLayoutMaterializationReentry,
    VirtualLayoutMaterializationStore, VirtualLayoutPin, VirtualLayoutPinReason,
    VirtualLayoutProjectionEvidence, VirtualLayoutProjectionKind, VirtualLayoutRetainReason,
    VirtualLayoutSemanticEntry, VirtualLayoutSemanticProjection,
    VirtualLayoutSemanticProjectionAuthority, VirtualLayoutSemanticProjectionBatch,
    VirtualLayoutSemanticProvider, VirtualLayoutSemanticQueryOutcome, VirtualLayoutSemanticRange,
    VirtualLayoutSemanticRangeProvider, VirtualLayoutSemanticRangeProviderOutcome,
    VirtualLayoutSemanticRangeQueryOutcome, VirtualLayoutSemanticRangeRequest,
    VirtualLayoutSemanticRejectedReason, VirtualLayoutSemanticRequest,
    VirtualLayoutSemanticTransformWitness, VirtualLayoutSemanticUnavailableReason,
    VirtualLayoutSlotIdentity, VirtualLayoutWindowCoordinator,
};

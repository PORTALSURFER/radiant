//! `radiant`: reusable GUI primitives and runtimes for host applications.
//!
//! Radiant exposes one public API with progressive control. Applications can
//! start with [`prelude`] for readable window, app, and view
//! builders, then name [`runtime`], [`widgets`],
//! [`layout`], and [`theme`] objects when they need
//! more explicit control. All of those entry points lower into the same generic
//! declarative UI tree and native Vello backend without depending on host-shaped
//! shell DTOs.
//!
//! A minimal app starts through the prelude:
//!
//! ```no_run
//! use radiant::prelude::*;
//!
//! fn main() -> radiant::Result {
//!     radiant::window("Radiant Hello World").run(text("Hello, world!"))
//! }
//! ```
//!
//! For more explicit control, use the same model through [`runtime::RuntimeBridge`],
//! [`runtime::UiSurface`], [`runtime::SurfaceNode`], and
//! [`runtime::NativeRunOptions`]. Those APIs are supported host control
//! surfaces, not a separate framework.
//!
//! Start with `README.md`, then use `docs/API.md` for the checked public API
//! boundary and lifecycle model, `docs/ARCHITECTURE.md` for ownership boundaries,
//! and `docs/TARGET.md` for the long-term standalone GUI library direction. The
//! checked `hello_world`, `counter`, `generic_native`, `widget_gallery`,
//! `waveform_view`, and `timeline_editor` examples cover application patterns
//! across the target areas.
//!
//! Generic host-facing modules:
//! - [`layout`]: stable slot-based layout primitives
//! - [`widgets`]: first-class reusable widget contracts
//! - [`gui_runtime`]: backend runtimes and scheduling
//! - [`runtime`]: generic declarative view/message bridge for new hosts
//! - [`theme`]: reusable visual tokens for generic widgets and containers

/// Readable application and view builder implementation.
mod application;
/// Shared environment-flag parsing helpers used by runtime internals.
mod env_flags;
/// Backend-agnostic GUI primitives.
pub mod gui;
/// Common imports for Radiant apps.
pub mod prelude;
/// Stable public slot-based layout API.
pub mod layout {
    pub use crate::gui::layout_core::{
        Constraints, ConstraintsParts, ContainerKind, ContainerNode, ContainerNodeParts,
        ContainerPolicy, CrossAlign, DebugPrimitiveKind, GridPolicy, Insets, LayoutDebugOptions,
        LayoutDebugPrimitive, LayoutDiagnostic, LayoutDiagnosticCode, LayoutEngine, LayoutNode,
        LayoutOutput, LayoutState, LayoutStats, MainAlign, NodeId, OverflowInfo, OverflowPolicy,
        Point, Rect, SizeModeCross, SizeModeMain, SlotChild, SlotChildParts, SlotParams,
        StackedRowRectsParts, SwitchBreakpoint, Vector2, VirtualWindowInfo, VirtualizationAxis,
        VirtualizationPolicy, WidgetNode, WidgetNodeParts, WrapPolicy, fixed_width_group_width,
        fixed_width_item_extent_for_available_width, fixed_width_row_rects_end,
        fixed_width_row_rects_end_into, fixed_width_row_rects_start,
        fixed_width_row_rects_start_into, grouped_fixed_width_row_width, layout_tree,
        layout_tree_with_state, stacked_row_rects, stacked_row_rects_from_parts,
        stacked_row_rects_into, stacked_row_rects_into_from_parts, visible_suffix_widths,
        visible_suffix_widths_into,
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

pub use application::{
    DEFAULT_COLUMN_SPACING, DEFAULT_GRID_GAP, DEFAULT_ROW_SPACING,
    DEFAULT_STYLED_CONTAINER_PADDING, Result, app, window,
};

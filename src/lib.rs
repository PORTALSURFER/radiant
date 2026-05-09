//! `radiant`: reusable GUI primitives and runtimes for host applications.
//!
//! Radiant exposes one public API with progressive control. Applications can
//! start with [`prelude`] for readable window, app, and view
//! builders, then name [`runtime`], [`widgets`],
//! [`layout`], and [`theme`] objects when they need
//! more explicit control. All of those entry points lower into the same generic
//! declarative UI tree and native Vello backend without depending on host-shaped
//! shell DTOs. See the checked `hello_world`, `counter`, and `generic_native`
//! examples for application patterns.
//! See `docs/API.md` for the checked public API boundary and lifecycle model.
//!
//! Generic host-facing modules:
//! - [`layout`]: stable slot-based layout primitives
//! - [`widgets`]: first-class reusable widget contracts
//! - [`gui_runtime`]: backend runtimes and scheduling
//! - [`runtime`]: generic declarative view/message bridge for new hosts
//! - [`theme`]: reusable visual tokens for generic widgets and containers

#![allow(clippy::too_many_arguments)]

/// Readable application and view builder implementation.
mod application;
/// Shared environment-flag parsing helpers used by runtime internals.
mod env_flags;
/// Backend-agnostic GUI primitives.
pub mod gui;
/// Stable public slot-based layout API.
pub mod layout {
    pub use crate::gui::layout_core::*;
}
/// Shared runtime host implementations.
pub mod gui_runtime;
/// Generic declarative view/message runtime surface for new hosts.
pub mod runtime;
/// Generic theme tokens for reusable Radiant widgets and containers.
pub mod theme;
/// Stable public widget contracts.
pub mod widgets;

/// Common imports for Radiant apps.
pub mod prelude {
    pub use crate::Result;
    pub use crate::application::{
        BadgeBuilder, ButtonBuilder, DetailsColumn, DetailsRow, DetailsSort, DragHandleBuilder,
        DynamicWidget, IntoView, MappedWidget, MenuItem, PropertyRow, RetainedCanvasBuilder,
        RunnableStatefulApp, SelectableBuilder, SortDirection, StateAction, StateView,
        StatefulAppBuilder, StatefulAppWithView, Subscription, TextInputBuilder, ToggleBuilder,
        TreeListItem, UpdateContext, View, ViewNode, WidgetView, WidgetViewContext, WindowBuilder,
        app, badge, badge_mapped, badge_message, button, button_mapped, button_message, canvas,
        card, checkbox, column, column_key, context_menu_overlay, custom_widget,
        custom_widget_mapped, drag_handle, drag_handle_mapped, drop_marker, gpu_surface,
        gpu_surface_input, grid, grid_with_gaps, image, list, list_row, menu, overlay_panel,
        passive_button, passive_text_input, passive_toggle, property_panel, retained_canvas,
        retained_canvas_with, row, row_key, scroll, scroll_column, selectable, selectable_mapped,
        selectable_property_panel, selectable_sortable_details_list, sortable_details_list, spacer,
        stack, text, text_input, text_input_mapped, toggle, toggle_mapped, tree_list,
        tree_list_with_drag, virtual_list, virtual_scroll, widget, window,
    };
    pub use crate::runtime::{
        Command, GpuHoverCursor, GpuSignalSummary, GpuSignalSummaryBucket, GpuSignalSummaryLevel,
        GpuSurfaceCapabilities, GpuSurfaceContent, GpuSurfaceOverlay, ResourceKey, ResourceLoad,
        ResourceLoadState, ResourceSlot, WindowManifest, WindowSpec,
    };
    pub use crate::widgets::{
        DragHandleMessage, GpuSurfaceMessage, GpuSurfaceWidget, Widget, WidgetOutput,
        WidgetProminence, WidgetStyle, WidgetTone,
    };
}

pub use application::{Result, app, window};

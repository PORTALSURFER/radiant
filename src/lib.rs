//! `radiant`: reusable GUI primitives and runtimes for host applications.
//!
//! Radiant exposes one public API with progressive control. Applications can
//! start with [`prelude`](crate::prelude) for readable window, app, and view
//! builders, then name [`runtime`](crate::runtime), [`widgets`](crate::widgets),
//! [`layout`](crate::layout), and [`theme`](crate::theme) objects when they need
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

#![allow(clippy::collapsible_if)]
#![allow(clippy::derivable_impls)]
#![allow(clippy::double_ended_iterator_last)]
#![allow(clippy::field_reassign_with_default)]
#![allow(clippy::if_same_then_else)]
#![allow(clippy::into_iter_on_ref)]
#![allow(clippy::manual_clamp)]
#![allow(clippy::manual_is_multiple_of)]
#![allow(clippy::needless_borrow)]
#![allow(clippy::question_mark)]
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
        ButtonBuilder, DetailsColumn, DetailsRow, DetailsSort, DragHandleBuilder, DynamicWidget,
        IntoView, MappedWidget, RetainedCanvasBuilder, RunnableStatefulApp, SortDirection,
        StateAction, StateView, StatefulAppBuilder, StatefulAppWithView, Subscription,
        TextInputBuilder, ToggleBuilder, TreeListItem, UpdateContext, View, ViewNode, WidgetView,
        WidgetViewContext, WindowBuilder, app, button, button_mapped, button_message, canvas,
        checkbox, column, column_key, custom_widget, drag_handle, drag_handle_mapped, drop_marker,
        gpu_surface, image, list, list_row, overlay_panel, passive_button, passive_text_input,
        passive_toggle, retained_canvas, retained_canvas_with, row, row_key, scroll, scroll_column,
        selectable_sortable_details_list, sortable_details_list, spacer, stack, text, text_input,
        text_input_mapped, toggle, toggle_mapped, tree_list, tree_list_with_drag, widget, window,
    };
    pub use crate::runtime::{
        Command, GpuHoverCursor, GpuSignalSummary, GpuSignalSummaryBucket, GpuSignalSummaryLevel,
        GpuSurfaceCapabilities, GpuSurfaceContent, GpuSurfaceOverlay,
    };
    pub use crate::widgets::{
        DragHandleMessage, GpuSurfaceWidget, Widget, WidgetOutput, WidgetProminence, WidgetStyle,
        WidgetTone,
    };
}

pub use application::{Result, app, window};

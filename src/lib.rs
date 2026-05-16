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
        BadgeBuilder, ButtonBuilder, DEFAULT_COLUMN_SPACING, DEFAULT_GRID_GAP, DEFAULT_ROW_SPACING,
        DEFAULT_STYLED_CONTAINER_PADDING, DetailsColumn, DetailsRow, DetailsSort,
        DragHandleBuilder, DynamicWidget, IconButtonBuilder, InteractiveRowBuilder, IntoView,
        KeyedLatestTasks, KeyedTaskCompletion, LatestTask, MappedWidget, MenuItem, PropertyRow,
        RetainedCanvasBuilder, RunnableStatefulApp, ScrollbarBuilder, SelectableBuilder,
        SliderBuilder, SortDirection, StateAction, StateView, StatefulAppBuilder,
        StatefulAppWithView, Subscription, TaskCompletion, TaskTicket, TextInputBuilder,
        ToggleBuilder, TreeListItem, UpdateContext, View, ViewNode, WidgetView, WidgetViewContext,
        WindowBuilder, app, badge, badge_mapped, badge_message, button, button_mapped,
        button_message, canvas, card, checkbox, column, column_key, context_menu_overlay,
        custom_widget, custom_widget_mapped, drag_handle, drag_handle_mapped, drag_preview,
        drag_preview_sized, drop_marker, gpu_surface, gpu_surface_input, grid, grid_with_gaps,
        icon_button, image, interactive_row, list, list_row, list_row_id, menu, overlay_panel,
        passive_button, passive_text_input, passive_toggle, property_panel, retained_canvas,
        retained_canvas_with, row, row_key, scroll, scroll_column, scrollbar, selectable,
        selectable_mapped, selectable_property_panel, selectable_sortable_details_list, slider,
        slider_mapped, sortable_details_list, spacer, stack, text, text_input, text_input_mapped,
        toggle, toggle_mapped, tree_list, tree_list_with_drag, virtual_list, virtual_list_window,
        virtual_scroll, widget, window,
    };
    pub use crate::gui::types::{ImageRgba, ImageRgbaError, Point, Rect, Rgba8, Vector2};
    pub use crate::gui::{
        chrome::{ContentViewChrome, StatusSegments},
        feedback::{StatusLineEntry, StatusLineLog},
        focus::FocusSurface,
        input::{KeyCode, KeyPress},
        invalidation::{
            InvalidationMask, RetainedSegment, RetainedSegmentKind, RetainedSegmentMask,
            RetainedSegmentPlan, RetainedSegmentRevisions,
        },
        list::{
            VirtualListController, VirtualListWindow, VirtualListWindowRequest,
            resolve_virtual_list_window,
        },
        range::IndexViewport,
        shortcuts::{ShortcutGesture, ShortcutLayer, ShortcutModifier, ShortcutResolution},
        svg::SvgIcon,
    };
    pub use crate::layout::LayoutOutput;
    pub use crate::runtime::{
        Command, ConfirmDialogRequest, ConfirmationButtons, ConfirmationLevel,
        ConfirmationResponse, EmbeddedFont, ExternalDragEffect, ExternalDragOutcome,
        ExternalDragPayload, ExternalDragPreview, ExternalDragRequest, FileDialogFilter,
        FileDialogRequest, GpuSignalGainPreview, GpuSignalRenderShape, GpuSignalSummary,
        GpuSignalSummaryBucket, GpuSignalSummaryLevel, GpuSurfaceCapabilities, GpuSurfaceContent,
        GpuSurfaceContentError, GpuSurfaceLineStyle, GpuSurfaceOverlay, GpuSurfaceRuntimeOverlays,
        NativeGenericRunError, NativeGenericRunReport, NativePopupOptions, NativeRunOptions,
        NativeRunOptionsError, NativeWindowMode, PaintFillPath, PaintFillRect, PaintFillRule,
        PaintImage, PaintPath, PaintPathCommand, PaintPrimitive, PaintStrokeRect, PaintSvg,
        PaintSvgDocument, PaintTextAlign, PaintTextRun, PaintTransform, PlatformCompletion,
        PlatformRequest, PlatformResponse, RepaintScope, ResourceCompletion, ResourceKey,
        ResourceLoad, ResourceLoadState, ResourceRequest, ResourceSlot, RuntimeRunReport,
        ScrollUpdate, SurfaceFrame, SurfacePaintPlan, SvgParseError, TransientOverlayContext,
        WindowManifest, WindowManifestError, WindowSpec, WindowSpecError,
    };
    pub use crate::theme::ThemeTokens;
    pub use crate::widgets::{
        CanvasGestureEvent, CanvasGestureState, CanvasPointer, DragHandleMessage, FocusBehavior,
        GpuSurfaceMessage, GpuSurfaceWidget, IconButtonWidget, InteractiveRowMessage,
        InteractiveRowWidget, PointerButton, ScrollbarAxis, ScrollbarMessage, SliderMessage,
        SliderWidget, TextAlign, TextInputEditResult, TextInputState, TextWrap, Widget,
        WidgetCommon, WidgetInput, WidgetKey, WidgetOutput, WidgetProminence, WidgetSizing,
        WidgetState, WidgetStyle, WidgetTone, WidgetVisualTokens, resolve_widget_visual_tokens,
    };
}

pub use application::{
    DEFAULT_COLUMN_SPACING, DEFAULT_GRID_GAP, DEFAULT_ROW_SPACING,
    DEFAULT_STYLED_CONTAINER_PADDING, Result, app, window,
};

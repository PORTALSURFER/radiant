//! Readable Radiant application and view builders.
//!
//! This module is a convenience layer over [`crate::runtime`]. It generates
//! deterministic widget ids, supplies default widget sizing, and lowers every
//! view into the existing [`UiSurface`](crate::runtime::UiSurface) tree.

const ROOT_KEY_SCOPE: u64 = 0xcbf2_9ce4_8422_2325;

/// Default content padding for styled Radiant application containers.
pub const DEFAULT_STYLED_CONTAINER_PADDING: f32 = 4.0;

/// Result type used by native launch helpers.
pub type Result<T = ()> = std::result::Result<T, String>;

mod view_node;
pub(in crate::application) use view_node::ViewNodeKind;
pub use view_node::{Layer, LayerInputPolicy, ViewNode};

/// Application view node type used by builder functions.
pub type View<Message = ()> = ViewNode<Message>;

/// Application view node type for direct state-callback apps.
pub type StateView<State> = View<StateAction<State>>;

mod state;
pub(in crate::application) use state::OptionalBaseline;
pub use state::StateAction;
mod runtime;
pub(in crate::application) use runtime::{
    AppBridge, AppBridgeLifecycle, AppUpdate, FrameMessageActivity, FrameRepaintSource,
    PendingFrameRepaint, StateCallback, StateDragCallback, StateStringCallback,
};
pub use runtime::{
    CancellationToken, KeyedLatestTasks, KeyedTaskCompletion, LatestTask, Subscription,
    TaskCompletion, TaskTicket, UpdateContext,
};
mod presentation;
pub use presentation::{FrameClock, Presentation, TransientOverlay, presentation};
mod repaint_policy;
pub use repaint_policy::RepaintPolicy;
mod launch;
pub use launch::{
    IntoView, RunnableStatefulApp, StatefulAppBuilder, StatefulAppWithView, WindowBuilder, app,
    window,
};
mod widget_view;
pub use widget_view::{
    DynamicWidget, DynamicWidgetParts, MappedWidget, MappedWidgetParts, WidgetView,
    WidgetViewContext,
};
mod tree_list;
pub use tree_list::{TreeListItem, TreeListItemParts, tree_list, tree_list_with_drag};
mod details_list;
pub use details_list::{
    CompactDetailsAnchoredCellParts, CompactDetailsHeaderCellIds, DetailsColumn,
    DetailsColumnDragFeedback, DetailsColumnParts, DetailsColumnPlacement,
    DetailsColumnReorderDrag, DetailsColumnResizeDrag, DetailsColumnWidthUpdate, DetailsRow,
    DetailsRowParts, DetailsSort, DetailsSortParts, SortDirection,
    compact_details_anchored_cell_from_parts, compact_details_cell, compact_details_header_row,
    compact_details_row, compact_resizable_details_header_cell,
    compact_resizable_details_header_cell_with_ids, details_column_drag_content_left,
    details_column_drag_feedback, details_column_reorder_index, details_sort_label,
    reorder_details_columns_by_id, selectable_sortable_details_list, sortable_details_list,
    update_details_column_reorder_drag, update_details_column_resize_drag,
};
mod property_panel;
pub use property_panel::{
    PropertyRow, PropertyRowParts, property_panel, property_rows, selectable_property_panel,
};
mod form_row;
pub use form_row::{FormRowParts, form_row, form_row_from_parts};
mod status_bar;
pub use status_bar::{StatusBarParts, status_bar, status_bar_from_parts};
mod option_list;
pub use option_list::{
    CompactOptionListAnchoredParts, CompactOptionListFloatingAboveParts, CompactOptionListItem,
    CompactOptionListParts, compact_option_list, compact_option_list_anchored,
    compact_option_list_floating_above, compact_option_list_from_parts,
};
mod panel_section;
pub use panel_section::{
    PanelSectionGeometry, PanelSectionLayerParts, PanelSectionParts,
    closeable_panel_section_from_parts, closeable_panel_section_layer_from_parts, panel_section,
    panel_section_from_parts, panel_section_layer_from_parts,
};
mod labeled_control;
pub use labeled_control::{
    LabeledControlParts, labeled_control, labeled_control_control_offset,
    labeled_control_control_offset_for, labeled_control_from_parts,
};
mod menu;
pub use menu::{
    ContextMenuOverlayParts, DismissibleContextMenuParts, MenuCommand, MenuCommandParts, MenuItem,
    MenuItemParts, MenuParts, MessageContextMenuOverlayParts, MessageMenuParts,
    MessageMenuWidthPolicy, context_menu_overlay, context_menu_overlay_from_parts,
    dismissible_context_menu, dismissible_context_menu_auto_width,
    dismissible_context_menu_from_parts, dismissible_context_menu_with_width,
    dismissible_context_menu_with_width_policy, menu, menu_from_parts, menu_height,
    message_context_menu_overlay, message_context_menu_overlay_auto_width,
    message_context_menu_overlay_from_parts, message_context_menu_overlay_with_width,
    message_context_menu_overlay_with_width_policy, message_menu, message_menu_from_parts,
    message_menu_height,
};
mod retained_canvas;
pub use retained_canvas::{RetainedCanvasBuilder, retained_canvas, retained_canvas_with};
mod builders;
pub use builders::{
    GpuSurfaceConfiguredParts, GpuSurfaceInputParts, canvas, card, custom_widget,
    custom_widget_direct, custom_widget_mapped, empty, gpu_surface,
    gpu_surface_configured_from_parts, gpu_surface_from_parts, gpu_surface_input,
    gpu_surface_input_from_parts, image, passive_badge, passive_button, passive_text_input,
    passive_toggle, spacer, text, text_line, widget,
};
pub(in crate::application) use builders::{
    danger_style, default_badge_sizing, default_button_sizing, default_canvas_sizing,
    default_drag_handle_sizing, default_selectable_sizing, default_slider_sizing,
    default_text_input_sizing, default_toggle_sizing, primary_style, view_node_from_widget,
};
mod control_builders;
pub use control_builders::{
    ActionRowBuilder, BadgeBuilder, ButtonBuilder, ColorMarkerBuilder, ColorMarkerRunBuilder,
    DEFAULT_ACTION_ROW_HEIGHT, DragHandleBuilder, DropdownBuilder, DropdownBuilderNeedsToggle,
    DropdownMenuOverlayBelowParts, DropdownOption, DropdownOptionParts, DropdownOptionSelection,
    DropdownParts, DropdownTriggerBuilder, DropdownTriggerBuilderNeedsToggle, DropdownTriggerParts,
    FeedbackOverlayBuilder, IconButtonBuilder, InteractiveBadgeBuilder, InteractiveRowActions,
    InteractiveRowBuilder, InteractiveRowUnderlayBuilder, MarkerRunBuilder, PointerShieldBuilder,
    ProgressBarBuilder, ScrollbarBuilder, SelectableBuilder, SliderBuilder, TextInputBuilder,
    ToggleBuilder, action_row, badge, badge_mapped, badge_message, button, button_mapped,
    button_message, checkbox, close_button, color_marker, determinate_progress_bar,
    disclosure_button, drag_handle, drag_handle_mapped, dropdown, dropdown_from_parts,
    dropdown_height, dropdown_menu, dropdown_menu_height, dropdown_menu_overlay,
    dropdown_menu_overlay_below, dropdown_menu_overlay_below_from_parts,
    dropdown_menu_overlay_below_labeled_control,
    dropdown_menu_overlay_below_stacked_labeled_control, dropdown_menu_overlay_below_trigger,
    dropdown_option, dropdown_trigger, dropdown_trigger_from_parts, dropdown_trigger_height,
    feedback_overlay, icon_button, indeterminate_progress_bar, interactive_badge, interactive_row,
    interactive_row_underlay, marker_run, marker_run_colors, pointer_drop_shield,
    pointer_move_shield, pointer_shield, progress_bar, progress_bar_for_snapshot, scrollbar,
    selectable, selectable_mapped, slider, slider_mapped, state_dropdown, text_input,
    text_input_mapped, toggle, toggle_mapped,
};
mod layout_builders;
pub use crate::runtime::LayerKind;
pub use layout_builders::{
    AnchoredLayerParts, BoundedScrollColumnParts, CenteredLayerParts, DEFAULT_COLUMN_SPACING,
    DEFAULT_GRID_GAP, DEFAULT_ROW_SPACING, FloatingLayerAnchorParts, FloatingLayerPlacement,
    LayerHorizontalAnchor, LayerVerticalAnchor, OverlayStack, Scene, VirtualListBuilder,
    anchored_layer, anchored_layer_from_parts, bounded_scroll_column,
    bounded_scroll_column_from_parts, centered_layer, centered_layer_from_parts, column,
    column_key, dismiss_layer, dismissible_overlay, drag_preview, drag_preview_sized, drop_marker,
    floating_layer, floating_layer_above, floating_layer_around_from_parts, floating_layer_below,
    floating_layer_with_input, grid, grid_with_gaps, input_overlay, input_underlay, list, list_row,
    list_row_id, local_drop_marker, overlay_panel, overlay_stack, resizable, row, row_key, scene,
    scroll, scroll_column, stack, stack_layers, virtual_list, virtual_list_window,
    virtual_list_window_body, virtual_list_window_change_for_scroll, virtual_list_windowed,
    virtual_scroll, virtual_tree_list_window, wrap,
};
mod ids;
pub(in crate::application) use ids::{IdGenerator, scoped_key_id};

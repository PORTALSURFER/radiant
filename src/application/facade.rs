//! Focused public export facades for the application subsystem.
//!
//! These modules mirror the application prelude's API roles. The root facade
//! keeps `crate::application::<item>` available through explicit grouped
//! re-exports so ownership stays visible at the export site.

mod controls;
mod details;
mod layout;
mod menus;
mod overlays;
mod panels;
mod runtime;
mod surfaces;
mod view;

pub use controls::{
    ActionRowBuilder, BadgeBuilder, ButtonBuilder, ButtonRowParts, ColorMarkerBuilder,
    DropdownBuilder, DropdownBuilderNeedsToggle, DropdownOption, DropdownOptionParts,
    DropdownOptionSelection, DropdownParts, DropdownTriggerBuilder,
    DropdownTriggerBuilderNeedsToggle, DropdownTriggerParts, IconButtonBuilder,
    InteractiveBadgeBuilder, InteractiveRowActions, InteractiveRowBuilder,
    InteractiveRowUnderlayBuilder, PointerTarget, PointerTargetBuilder, ProgressBarBuilder,
    SelectableBuilder, SliderBuilder, TextInputBuilder, TextInputWithClearButtonBuilder,
    ToggleBuilder, TreeRowBuilder, TreeRowDragDropState, TreeRowMessageBuilder, action_row, badge,
    badge_mapped, badge_message, button, button_mapped, button_message, button_row,
    button_row_from_parts, checkbox, close_button, color_marker, determinate_progress_bar,
    disclosure_button, dropdown, dropdown_from_parts, dropdown_height, dropdown_menu,
    dropdown_menu_height, dropdown_trigger, dropdown_trigger_from_parts, dropdown_trigger_height,
    icon_button, indeterminate_progress_bar, interactive_badge, interactive_row,
    interactive_row_underlay, pointer_drop_target, pointer_move_target, pointer_target,
    progress_bar, progress_bar_for_snapshot, row_actions, selectable, selectable_mapped, slider,
    slider_mapped, text_input, text_input_clear_button_id, text_input_mapped, toggle,
    toggle_mapped, tree_row,
};
pub use details::{
    CompactDetailsAnchoredCellParts, CompactDetailsHeaderCellIds, CompactOptionListAnchoredParts,
    CompactOptionListFloatingAboveParts, CompactOptionListItem, CompactOptionListParts,
    DetailsColumn, DetailsColumnDragFeedback, DetailsColumnParts, DetailsColumnPlacement,
    DetailsColumnReorderDrag, DetailsColumnResizeDrag, DetailsColumnWidthUpdate, DetailsRow,
    DetailsRowParts, DetailsSort, DetailsSortParts, SortDirection, TreeListItem, TreeListItemParts,
    compact_details_anchored_cell_from_parts, compact_details_cell, compact_details_header_row,
    compact_details_row, compact_option_list, compact_option_list_anchored,
    compact_option_list_anchored_with_activation, compact_option_list_anchored_with_interaction,
    compact_option_list_floating_above, compact_option_list_from_parts,
    compact_option_list_from_parts_with_activation,
    compact_option_list_from_parts_with_interaction, compact_resizable_details_header_cell,
    compact_resizable_details_header_cell_with_ids, details_column_drag_content_left,
    details_column_drag_feedback, details_column_reorder_index, details_sort_label,
    message_selectable_sortable_details_list, message_sortable_details_list, message_tree_list,
    message_tree_list_with_drag, reorder_details_columns_by_id,
    reorder_visible_details_columns_by_id, update_details_column_reorder_drag,
    update_details_column_resize_drag, update_visible_details_column_reorder_drag,
};
pub use layout::{
    BoundedScrollColumnParts, Children, ColorMarkerRunBuilder, DEFAULT_ACTION_ROW_HEIGHT,
    DEFAULT_COLUMN_SPACING, DEFAULT_GRID_GAP, DEFAULT_ROW_SPACING, Layer, LayerInputPolicy,
    LayerKind, MarkerRunBuilder, MaterializedVirtualListBuilder, OverlayStack, Overlays, Scene,
    ToolbarAlignment, ToolbarParts, VirtualListBuilder, VirtualTreeListBuilder,
    WorkspaceShellBuilder, bounded_scroll_column, bounded_scroll_column_from_parts, children,
    column, column_key, fixed_slot_if, fixed_slot_opt, grid, grid_with_gaps, list, list_row,
    list_row_id, local_drop_marker, marker_run, marker_run_colors, overlay_stack, overlays,
    resizable, row, row_key, scene, scroll, scroll_column, stack, stack_layers, toolbar,
    toolbar_from_parts, virtual_list_materialized_windowed, virtual_list_window,
    virtual_list_window_body, virtual_list_window_change_for_scroll, virtual_list_windowed,
    virtual_scroll, virtual_tree_list_window, virtual_tree_list_windowed, workspace_shell, wrap,
};
pub use menus::{
    DismissibleContextMenuParts, MenuCommand, MenuCommandParts, MessageContextMenuOverlayParts,
    MessageMenuParts, MessageMenuWidthPolicy, dismissible_context_menu,
    dismissible_context_menu_auto_width, dismissible_context_menu_from_parts,
    dismissible_context_menu_with_width, dismissible_context_menu_with_width_policy, menu_height,
    message_context_menu_overlay, message_context_menu_overlay_auto_width,
    message_context_menu_overlay_from_parts, message_context_menu_overlay_with_width,
    message_context_menu_overlay_with_width_policy, message_menu, message_menu_from_parts,
    message_menu_height,
};
pub use overlays::{
    AnchoredLayerParts, CenteredLayerParts, DragHandleBuilder, DropdownMenuOverlayBelowParts,
    FeedbackOverlayBuilder, FloatingLayerAnchorParts, FloatingLayerPlacement,
    LayerHorizontalAnchor, LayerVerticalAnchor, PointerShieldBuilder, anchored_layer,
    anchored_layer_from_parts, centered_layer, centered_layer_from_parts, dismiss_layer,
    dismissible_overlay, drag_handle, drag_handle_mapped, drag_preview, drag_preview_sized,
    drop_marker, dropdown_menu_overlay, dropdown_menu_overlay_below,
    dropdown_menu_overlay_below_from_parts, dropdown_menu_overlay_below_labeled_control,
    dropdown_menu_overlay_below_stacked_labeled_control, dropdown_menu_overlay_below_trigger,
    feedback_overlay, floating_layer, floating_layer_above, floating_layer_around_from_parts,
    floating_layer_below, floating_layer_with_input, input_overlay, input_underlay, overlay_panel,
    pointer_drop_shield, pointer_move_shield, pointer_shield,
};
pub use panels::{
    FormRowParts, LabeledControlParts, PanelSectionGeometry, PanelSectionHeaderParts,
    PanelSectionLayerParts, PanelSectionParts, PropertyRow, PropertyRowParts, StatusBarParts,
    closeable_dialog_layer, closeable_panel_section_from_parts,
    closeable_panel_section_layer_from_parts, dialog_layer, form_row, form_row_from_parts,
    labeled_control, labeled_control_control_offset, labeled_control_control_offset_for,
    labeled_control_from_parts, message_selectable_property_panel, panel_section,
    panel_section_from_header_parts, panel_section_from_parts, panel_section_layer_from_parts,
    panel_section_resize_header, property_panel, property_rows, status_bar, status_bar_from_parts,
};
pub use runtime::{
    CancellationToken, FrameClock, KeyedLatestTasks, KeyedTaskCompletion, LatestTask, Presentation,
    RepaintPolicy, ResourceTaskTicket, ResourceTasks, Subscription, TaskCompletion, TaskTicket,
    TransientOverlay, UiUpdateContext, presentation,
};
pub use surfaces::{
    DynamicWidget, DynamicWidgetParts, GpuSurfaceConfiguredParts, GpuSurfaceInputParts,
    RetainedCanvasBuilder, ScrollbarBuilder, canvas, card, custom_widget, custom_widget_direct,
    custom_widget_mapped, empty, gpu_surface, gpu_surface_configured_from_parts,
    gpu_surface_from_parts, gpu_surface_input, gpu_surface_input_from_parts, image, passive_badge,
    passive_button, passive_text_input, passive_toggle, retained_canvas, retained_canvas_with,
    scrollbar, spacer, text, text_line, widget,
};
pub use view::{
    IntoView, MappedWidget, MappedWidgetParts, RunnableStatefulApp, StatefulAppBuilder,
    StatefulAppWithView, ViewNode, WidgetView, WidgetViewContext, WindowBuilder, app, window,
};

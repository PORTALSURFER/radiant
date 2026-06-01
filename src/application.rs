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
pub use view_node::ViewNode;
pub(in crate::application) use view_node::ViewNodeKind;

/// Application view node type used by builder functions.
pub type View<Message = ()> = ViewNode<Message>;

/// Application view node type for direct state-callback apps.
pub type StateView<State> = View<StateAction<State>>;

mod state;
pub(in crate::application) use state::OptionalBaseline;
pub use state::StateAction;
mod runtime;
pub(in crate::application) use runtime::{
    AppBridge, AppBridgeLifecycle, AppUpdate, StateCallback, StateDragCallback, StateStringCallback,
};
pub use runtime::{
    CancellationToken, KeyedLatestTasks, KeyedTaskCompletion, LatestTask, Subscription,
    TaskCompletion, TaskTicket, UpdateContext,
};
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
    DetailsColumn, DetailsColumnParts, DetailsColumnPlacement, DetailsColumnReorderDrag,
    DetailsColumnResizeDrag, DetailsRow, DetailsRowParts, DetailsSort, DetailsSortParts,
    SortDirection, compact_details_header_row, compact_details_row,
    compact_resizable_details_header_cell, details_column_drag_content_left,
    details_column_reorder_index, details_sort_label, reorder_details_columns_by_id,
    selectable_sortable_details_list, sortable_details_list,
};
mod property_panel;
pub use property_panel::{
    PropertyRow, PropertyRowParts, property_panel, selectable_property_panel,
};
mod panel_section;
pub use panel_section::{PanelSectionParts, panel_section, panel_section_from_parts};
mod labeled_control;
pub use labeled_control::{
    LabeledControlParts, labeled_control, labeled_control_control_offset,
    labeled_control_control_offset_for, labeled_control_from_parts,
};
mod menu;
pub use menu::{
    ContextMenuOverlayParts, DismissibleContextMenuParts, MenuCommand, MenuCommandParts, MenuItem,
    MenuItemParts, MenuParts, MessageMenuParts, context_menu_overlay,
    context_menu_overlay_from_parts, dismissible_context_menu, dismissible_context_menu_from_parts,
    dismissible_context_menu_with_width, menu, menu_from_parts, menu_height, message_menu,
    message_menu_from_parts, message_menu_height,
};
mod retained_canvas;
pub use retained_canvas::{RetainedCanvasBuilder, retained_canvas, retained_canvas_with};
mod builders;
pub use builders::{
    GpuSurfaceInputParts, canvas, card, custom_widget, custom_widget_mapped, gpu_surface,
    gpu_surface_from_parts, gpu_surface_input, gpu_surface_input_from_parts, image, passive_badge,
    passive_button, passive_text_input, passive_toggle, spacer, text, widget,
};
pub(in crate::application) use builders::{
    danger_style, default_badge_sizing, default_button_sizing, default_canvas_sizing,
    default_drag_handle_sizing, default_selectable_sizing, default_slider_sizing,
    default_text_input_sizing, default_toggle_sizing, primary_style, view_node_from_widget,
};
mod control_builders;
pub use control_builders::{
    ActionRowBuilder, BadgeBuilder, ButtonBuilder, ColorMarkerBuilder, DEFAULT_ACTION_ROW_HEIGHT,
    DragHandleBuilder, DropdownBuilder, DropdownBuilderNeedsToggle, DropdownMenuOverlayBelowParts,
    DropdownOption, DropdownOptionParts, DropdownOptionSelection, DropdownParts,
    DropdownTriggerBuilder, DropdownTriggerBuilderNeedsToggle, DropdownTriggerParts,
    FeedbackOverlayBuilder, IconButtonBuilder, InteractiveRowBuilder, MarkerRunBuilder,
    PointerShieldBuilder, ProgressBarBuilder, ScrollbarBuilder, SelectableBuilder, SliderBuilder,
    TextInputBuilder, ToggleBuilder, action_row, badge, badge_mapped, badge_message, button,
    button_mapped, button_message, checkbox, color_marker, determinate_progress_bar, drag_handle,
    drag_handle_mapped, dropdown, dropdown_from_parts, dropdown_height, dropdown_menu,
    dropdown_menu_height, dropdown_menu_overlay, dropdown_menu_overlay_below,
    dropdown_menu_overlay_below_from_parts, dropdown_menu_overlay_below_trigger, dropdown_option,
    dropdown_trigger, dropdown_trigger_from_parts, dropdown_trigger_height, feedback_overlay,
    icon_button, indeterminate_progress_bar, interactive_row, marker_run, pointer_drop_shield,
    pointer_move_shield, pointer_shield, progress_bar, progress_bar_for_snapshot, scrollbar,
    selectable, selectable_mapped, slider, slider_mapped, state_dropdown, text_input,
    text_input_mapped, toggle, toggle_mapped,
};
mod layout_builders;
pub use layout_builders::{
    AnchoredLayerParts, CenteredLayerParts, DEFAULT_COLUMN_SPACING, DEFAULT_GRID_GAP,
    DEFAULT_ROW_SPACING, FloatingLayerAnchorParts, FloatingLayerPlacement, LayerHorizontalAnchor,
    LayerVerticalAnchor, anchored_layer, anchored_layer_from_parts, centered_layer,
    centered_layer_from_parts, column, column_key, dismiss_layer, drag_preview, drag_preview_sized,
    drop_marker, floating_layer, floating_layer_above, floating_layer_around_from_parts,
    floating_layer_below, floating_layer_with_input, grid, grid_with_gaps, input_overlay, list,
    list_row, list_row_id, overlay_panel, row, row_key, scroll, scroll_column, stack, virtual_list,
    virtual_list_window, virtual_scroll, wrap,
};
mod ids;
pub(in crate::application) use ids::{IdGenerator, scoped_key_id};

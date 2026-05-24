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
pub use launch::*;
mod widget_view;
pub use widget_view::{
    DynamicWidget, DynamicWidgetParts, MappedWidget, MappedWidgetParts, WidgetView,
    WidgetViewContext,
};
mod tree_list;
pub use tree_list::{TreeListItem, TreeListItemParts, tree_list, tree_list_with_drag};
mod details_list;
pub use details_list::{
    DetailsColumn, DetailsColumnParts, DetailsRow, DetailsRowParts, DetailsSort, DetailsSortParts,
    SortDirection, selectable_sortable_details_list, sortable_details_list,
};
mod property_panel;
pub use property_panel::{
    PropertyRow, PropertyRowParts, property_panel, selectable_property_panel,
};
mod menu;
pub use menu::{
    ContextMenuOverlayParts, MenuItem, MenuItemParts, MenuParts, context_menu_overlay,
    context_menu_overlay_from_parts, menu, menu_from_parts,
};
mod retained_canvas;
pub use retained_canvas::{RetainedCanvasBuilder, retained_canvas, retained_canvas_with};
mod builders;
pub use builders::{
    GpuSurfaceInputParts, canvas, card, custom_widget, custom_widget_mapped, gpu_surface,
    gpu_surface_from_parts, gpu_surface_input, gpu_surface_input_from_parts, image, passive_button,
    passive_text_input, passive_toggle, spacer, text, widget,
};
pub(in crate::application) use builders::{
    danger_style, default_badge_sizing, default_button_sizing, default_canvas_sizing,
    default_drag_handle_sizing, default_selectable_sizing, default_slider_sizing,
    default_text_input_sizing, default_toggle_sizing, primary_style, view_node_from_widget,
};
mod control_builders;
pub use control_builders::{
    BadgeBuilder, ButtonBuilder, DragHandleBuilder, DropdownBuilder, DropdownBuilderNeedsToggle,
    DropdownOption, DropdownOptionParts, DropdownOptionSelection, DropdownParts, IconButtonBuilder,
    InteractiveRowBuilder, ScrollbarBuilder, SelectableBuilder, SliderBuilder, TextInputBuilder,
    ToggleBuilder, badge, badge_mapped, badge_message, button, button_mapped, button_message,
    checkbox, drag_handle, drag_handle_mapped, dropdown, dropdown_from_parts, dropdown_height,
    dropdown_menu, dropdown_menu_height, dropdown_menu_overlay, dropdown_option, icon_button,
    interactive_row, scrollbar, selectable, selectable_mapped, slider, slider_mapped,
    state_dropdown, text_input, text_input_mapped, toggle, toggle_mapped,
};
mod layout_builders;
pub use layout_builders::*;
mod ids;
pub(in crate::application) use ids::{IdGenerator, scoped_key_id};

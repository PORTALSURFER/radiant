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

/// Application view node type used by builder functions.
pub type View<Message = ()> = view_node::ViewNode<Message>;

mod state;
pub(in crate::application) use state::OptionalBaseline;
pub(crate) mod runtime;
mod text_content;
pub(in crate::application) use runtime::{
    AppBridge, AppBridgeLifecycle, AppUpdate, FrameMessageActivity, FrameRepaintSource,
    PendingFrameRepaint,
};
mod builders;
mod details_list;
mod form_row;
mod labeled_control;
mod launch;
mod menu;
mod option_list;
mod panel_section;
mod presentation;
mod property_panel;
mod repaint_policy;
mod retained_canvas;
mod status_bar;
mod tree_list;
mod widget_view;
pub(in crate::application) use builders::{
    danger_style, default_badge_sizing, default_button_sizing, default_canvas_sizing,
    default_drag_handle_sizing, default_selectable_sizing, default_slider_sizing,
    default_text_input_sizing, default_toggle_sizing, primary_style, view_node_from_widget,
};
mod control_builders;
mod ids;
mod layout_builders;
pub(in crate::application) use ids::{IdGenerator, scoped_key_id};
mod facade;
pub use facade::*;

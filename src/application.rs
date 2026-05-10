//! Readable Radiant application and view builders.
//!
//! This module is a convenience layer over [`crate::runtime`]. It generates
//! deterministic widget ids, supplies default widget sizing, and lowers every
//! view into the existing [`UiSurface`](crate::runtime::UiSurface) tree.

use crate::{
    layout::{CrossAlign, Insets, MainAlign, NodeId},
    runtime::SurfaceNode,
    widgets::{TextAlign, TextWrap, WidgetProminence, WidgetSizing, WidgetStyle, WidgetTone},
};
use std::sync::Arc;

const ROOT_KEY_SCOPE: u64 = 0xcbf2_9ce4_8422_2325;

/// Result type used by native launch helpers.
pub type Result<T = ()> = std::result::Result<T, String>;

/// Application view node type used by builder functions.
pub type View<Message = ()> = ViewNode<Message>;

/// Application view node type for direct state-callback apps.
pub type StateView<State> = View<StateAction<State>>;

mod state;
pub(in crate::application) use state::OptionalBaseline;
pub use state::StateAction;
mod runtime;
pub(in crate::application) use runtime::{
    AppAnimation, AppBridge, AppCloseRequested, AppFrameMessage, AppRuntime, AppShortcuts,
    AppShutdown, AppStartup, AppSubscriptions, AppUpdate, RetainedPainter, StateCallback,
    StateDragCallback, StateStringCallback,
};
pub use runtime::{Subscription, UpdateContext};
mod launch;
pub use launch::*;
mod widget_view;
pub use widget_view::{DynamicWidget, MappedWidget, WidgetView, WidgetViewContext};
include!("application/view_node.rs");
include!("application/tree_list.rs");
include!("application/details_list.rs");
include!("application/property_panel.rs");
mod menu;
pub use menu::{MenuItem, context_menu_overlay, menu};
mod retained_canvas;
pub use retained_canvas::{RetainedCanvasBuilder, retained_canvas, retained_canvas_with};
mod builders;
pub use builders::{
    canvas, card, custom_widget, custom_widget_mapped, gpu_surface, gpu_surface_input, image,
    passive_button, passive_text_input, passive_toggle, spacer, text, widget,
};
pub(in crate::application) use builders::{
    danger_style, default_badge_sizing, default_button_sizing, default_canvas_sizing,
    default_drag_handle_sizing, default_selectable_sizing, default_text_input_sizing,
    default_toggle_sizing, primary_style, view_node_from_widget,
};
mod control_builders;
pub use control_builders::*;
mod layout_builders;
pub use layout_builders::*;
mod ids;
pub(in crate::application) use ids::{IdGenerator, scoped_key_id};

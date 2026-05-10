//! Readable Radiant application and view builders.
//!
//! This module is a convenience layer over [`crate::runtime`]. It generates
//! deterministic widget ids, supplies default widget sizing, and lowers every
//! view into the existing [`UiSurface`](crate::runtime::UiSurface) tree.

use crate::{
    gui::types::{ImageRgba, Point, Rect},
    layout::{CrossAlign, Insets, MainAlign, NodeId, Vector2},
    runtime::{GpuSurfaceContent, SurfaceNode, WidgetMessageMapper},
    widgets::{
        ButtonWidget, CanvasWidget, CardWidget, GpuSurfaceMessage, GpuSurfaceWidget, ImageWidget,
        RetainedSurfaceDescriptor, TextAlign, TextInputWidget, TextWidget, TextWrap, ToggleWidget,
        Widget, WidgetOutput, WidgetProminence, WidgetSizing, WidgetStyle, WidgetTone,
    },
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
include!("application/menu.rs");
include!("application/retained_canvas.rs");
include!("application/builders.rs");
mod control_builders;
pub use control_builders::*;
include!("application/layout_builders.rs");
mod ids;
pub(in crate::application) use ids::{IdGenerator, scoped_key_id};

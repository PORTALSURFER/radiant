//! Generic declarative view-tree types for message-driven Radiant hosts.

mod builders;
mod dispatch;
mod focus;
mod frame;
mod input;
mod layout;
mod lookup;
mod node;
mod paint;
mod path;
mod projection;
mod state_sync;
mod traversal;
mod view;
mod widget;

pub use frame::SurfaceFrame;
pub(in crate::runtime) use input::WidgetDispatchResult;
pub(in crate::runtime) use layout::SurfaceRuntimeProjection;
pub use node::{
    LayerKind, SurfaceChild, SurfaceContainer, SurfaceFloatingLayer, SurfaceLayer, SurfaceNode,
    SurfaceOverlay, SurfaceScene,
};
pub(in crate::runtime) use paint::{clear_paint_plan_for_layout, empty_paint_plan_for_layout};
pub(in crate::runtime) use path::{ClipAncestors, WidgetPath};
pub(in crate::runtime) use state_sync::WidgetStateSyncPolicy;
pub(in crate::runtime) use traversal::{
    SurfaceContainerTraversalRecord, SurfaceTraversalIndex, SurfaceTraversalStats,
    SurfaceWidgetTraversalRecord,
};
pub use widget::{
    MessageMapper, NativeFileDropMessageMapper, ScrollMessageMapper, SurfaceWidget,
    WidgetMessageMapper,
};

pub(in crate::runtime) use crate::widgets::WidgetId;

/// Top-level immutable UI surface projected by a generic Radiant host.
pub struct UiSurface<Message> {
    root: SurfaceNode<Message>,
}

/// Public declarative view snapshot alias for host applications.
///
/// `View<Message>` is the framework vocabulary for the top-level immutable UI
/// projection. It is an alias for [`UiSurface`] so existing code keeps the same
/// storage, cloning, layout, input, and paint behavior.
pub type View<Message> = UiSurface<Message>;

/// Public declarative element tree alias for host applications.
///
/// `Element<Message>` is the framework vocabulary for one node in a projected
/// view tree. It is an alias for [`SurfaceNode`] to keep identity and layout
/// behavior exactly shared with the existing runtime surface.
pub type Element<Message> = SurfaceNode<Message>;

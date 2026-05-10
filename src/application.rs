//! Readable Radiant application and view builders.
//!
//! This module is a convenience layer over [`crate::runtime`]. It generates
//! deterministic widget ids, supplies default widget sizing, and lowers every
//! view into the existing [`UiSurface`](crate::runtime::UiSurface) tree.

use crate::{
    gui::types::{ImageRgba, Point, Rect},
    layout::{
        ContainerKind, ContainerPolicy, CrossAlign, GridPolicy, Insets, MainAlign, NodeId,
        SizeModeCross, SizeModeMain, SlotParams, Vector2, VirtualizationAxis, VirtualizationPolicy,
    },
    runtime::{GpuSurfaceContent, SurfaceChild, SurfaceNode, WidgetMessageMapper},
    widgets::{
        ButtonWidget, CanvasWidget, CardWidget, GpuSurfaceMessage, GpuSurfaceWidget, ImageWidget,
        RetainedSurfaceDescriptor, TextAlign, TextInputWidget, TextWidget, TextWrap, ToggleWidget,
        Widget, WidgetOutput, WidgetProminence, WidgetSizing, WidgetStyle, WidgetTone,
    },
};
use std::{collections::HashSet, sync::Arc};

const ROOT_KEY_SCOPE: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// Result type used by native launch helpers.
pub type Result<T = ()> = std::result::Result<T, String>;

/// Application view node type used by builder functions.
pub type View<Message = ()> = ViewNode<Message>;

/// Application view node type for direct state-callback apps.
pub type StateView<State> = View<StateAction<State>>;

include!("application/state.rs");
mod runtime;
pub(in crate::application) use runtime::{
    AppAnimation, AppBridge, AppCloseRequested, AppFrameMessage, AppRuntime, AppShortcuts,
    AppShutdown, AppStartup, AppSubscriptions, AppUpdate, RetainedPainter, StateCallback,
    StateDragCallback, StateStringCallback,
};
pub use runtime::{Subscription, UpdateContext};
mod launch;
pub use launch::*;
include!("application/widget_view.rs");
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
include!("application/ids.rs");

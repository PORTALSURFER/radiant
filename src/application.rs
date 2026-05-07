//! Readable Radiant application and view builders.
//!
//! This module is a convenience layer over [`crate::runtime`]. It generates
//! deterministic widget ids, supplies default widget sizing, and lowers every
//! view into the existing [`UiSurface`](crate::runtime::UiSurface) tree.

use crate::{
    gui::types::ImageRgba,
    gui_runtime::NativeRunOptions,
    layout::{
        ContainerKind, ContainerPolicy, CrossAlign, Insets, MainAlign, NodeId, SizeModeCross,
        SizeModeMain, SlotParams, Vector2,
    },
    runtime::{
        Command, RuntimeBridge, SurfaceChild, SurfaceNode, UiSurface, WidgetMessageMapper,
        declarative_command_runtime_bridge, run_native_vello_runtime,
    },
    widgets::{
        ButtonWidget, CanvasWidget, DragHandleWidget, ImageWidget, TextInputWidget, TextWidget,
        TextWrap, ToggleWidget, Widget, WidgetOutput, WidgetProminence, WidgetSizing, WidgetStyle,
        WidgetTone,
    },
};
use std::{collections::HashSet, marker::PhantomData, sync::Arc};

const ROOT_KEY_SCOPE: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// Result type used by native launch helpers.
pub type Result<T = ()> = std::result::Result<T, String>;

/// Application view node type used by builder functions.
pub type View<Message = ()> = ViewNode<Message>;

/// Application view node type for direct state-callback apps.
pub type StateView<State> = View<StateAction<State>>;

include!("application/state.rs");
include!("application/launch.rs");
include!("application/widget_view.rs");
include!("application/view_node.rs");
include!("application/tree_list.rs");
include!("application/details_list.rs");
include!("application/builders.rs");
include!("application/ids.rs");

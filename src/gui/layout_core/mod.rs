//! Strict hierarchical slot-based layout primitives for `radiant`.
//!
//! This module provides a deterministic two-pass measure/layout engine used by
//! the native shell adapter and future retained UI containers.

pub(crate) mod constraints;
pub(crate) mod engine;
pub(crate) mod model;
pub(crate) mod tree;

pub(crate) use constraints::Constraints;
pub(crate) use engine::{LayoutDebugOptions, LayoutEngine, LayoutState, layout_tree};
pub(crate) use model::{
    ContainerKind, ContainerPolicy, CrossAlign, Insets, MainAlign, OverflowPolicy, SizeModeCross,
    SizeModeMain, SlotParams,
};
pub(crate) use tree::{LayoutNode, SlotChild};

//! Unit tests for ScrollView virtualization behavior.

use super::{
    DebugPrimitiveKind, LayoutDebugOptions, LayoutDiagnosticCode, LayoutEngine, LayoutState,
    layout_tree_with_state,
};
use crate::gui::layout_core::constraints::Constraints;
use crate::gui::layout_core::model::{
    ContainerKind, ContainerPolicy, OverflowPolicy, SizeModeCross, SizeModeMain, SlotParams,
    VirtualizationAxis, VirtualizationPolicy, WrapPolicy,
};
use crate::gui::layout_core::tree::{LayoutNode, SlotChild};
use crate::gui::types::{Point, Rect, Vector2};

#[path = "virtualization_tests/alignment.rs"]
mod alignment;
#[path = "virtualization_tests/cache.rs"]
mod cache;
#[path = "virtualization_tests/diagnostics.rs"]
mod diagnostics;
#[path = "virtualization_tests/equivalence.rs"]
mod equivalence;
#[path = "virtualization_tests/helpers.rs"]
mod helpers;
#[path = "virtualization_tests/performance.rs"]
mod performance;

pub(super) use helpers::{fixed_virtualized_scroll_root, scroll_with_content};

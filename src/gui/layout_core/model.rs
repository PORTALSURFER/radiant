//! Public policy model for the slot-based layout engine.

mod alignment;
mod container;
mod slot;
mod virtualization;

pub use alignment::{CrossAlign, MainAlign, OverflowPolicy, SizeModeCross, SizeModeMain};
pub use container::{
    ContainerKind, ContainerPolicy, FloatingLayerPolicy, GridPolicy, SwitchBreakpoint, WrapPolicy,
};
pub use slot::{Insets, SlotParams};
pub use virtualization::{VirtualizationAxis, VirtualizationPolicy};

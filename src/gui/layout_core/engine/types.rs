//! Shared data types for layout output, diagnostics, and debug rendering.

mod debug;
mod diagnostics;
mod output;
mod state;
mod stats;
mod virtualization;

pub use debug::{DebugPrimitiveKind, LayoutDebugOptions, LayoutDebugPrimitive};
pub use diagnostics::{LayoutDiagnostic, LayoutDiagnosticCode};
pub use output::LayoutOutput;
pub use state::LayoutState;
pub use stats::LayoutStats;
pub use virtualization::{OverflowInfo, VirtualWindowInfo};

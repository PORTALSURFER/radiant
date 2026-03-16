//! Backend-neutral native shell model used by the Vello runtime.
//!
//! The design mirrors a retained view tree (inspired by Xilem): build a
//! deterministic layout tree, run hit testing against that tree, then derive
//! backend-neutral paint primitives (shapes + text runs).

mod layout;
mod layout_adapter;
mod layout_runtime;
mod paint;
#[cfg(test)]
mod shots;
mod state;
mod style;
#[cfg(test)]
mod tests;

pub(crate) use layout::ShellLayout;
pub(crate) use layout::ShellNodeKind;
pub(crate) use layout_runtime::{ShellLayoutDirtyKind, ShellLayoutRuntime};
pub(crate) use paint::{NativeViewFrame, Primitive, TextAlign, TextRun};
pub(crate) use state::{
    ChromeMotionOverlayFingerprint, CursorMoveEffect, NativeShellState, StateOverlayFingerprint,
    StaticFrameSegment, StaticFrameSegments, TextFieldVisualState,
    WaveformMotionOverlayFingerprint,
};
pub(crate) use style::StyleTokens;

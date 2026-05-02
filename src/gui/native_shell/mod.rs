//! Backend-neutral native shell model used by the Vello runtime.
//!
//! The design mirrors a retained view tree (inspired by Xilem): build a
//! deterministic layout tree, run hit testing against that tree, then derive
//! backend-neutral paint primitives (shapes + text runs).

#[path = "../../../../../src/app_core/native_shell/composition/browser_chrome_surface.rs"]
mod browser_chrome_surface;
mod layout;
mod layout_adapter;
mod layout_runtime;
mod paint;
#[path = "../../../../../src/app_core/native_shell/composition/sidebar_surface.rs"]
mod sidebar_surface;
mod state;
#[path = "../../../../../src/app_core/native_shell/composition/status_surface.rs"]
mod status_surface;
mod style;
#[cfg(test)]
#[path = "../../../../../src/app_core/native_shell/composition/tests/mod.rs"]
mod tests;
#[path = "../../../../../src/app_core/native_shell/composition/top_bar_surface.rs"]
mod top_bar_surface;
#[path = "../../../../../src/app_core/native_shell/composition/waveform_header_surface.rs"]
mod waveform_header_surface;
#[path = "../../../../../src/app_core/native_shell/composition/waveform_toolbar_surface.rs"]
mod waveform_toolbar_surface;

pub(crate) use layout::ShellLayout;
pub(crate) use layout::ShellNodeKind;
#[cfg(test)]
pub(crate) use layout_adapter::BROWSER_BANDS_ROOT_ID;
pub(crate) use layout_adapter::{
    WaveformPixelSnap, compute_waveform_slice_preview_rects, waveform_plot_x_for_micros,
    waveform_view_window_from_bounds,
};
pub(crate) use layout_runtime::{
    ShellLayoutDirtyKind, ShellLayoutRuntime, ShellLayoutTreeKind,
    dirty_segments_for_layout_subtree,
};
pub(crate) use state::{
    ChromeMotionOverlayFingerprint, CursorMoveEffect, FocusOverlayFingerprint,
    HoverOverlayFingerprint, ModalOverlayFingerprint, NativeShellState, StaticFrameSegment,
    StaticFrameSegments, TextFieldVisualState, WaveformMotionOverlayFingerprint,
    WaveformToolbarHoverHint,
};
pub(crate) use style::StyleTokens;

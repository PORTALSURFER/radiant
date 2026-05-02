#[cfg(all(test, feature = "legacy-shell"))]
pub(in crate::gui_runtime::native_vello) use crate::compat::legacy_shell::PreviewBridge;
pub(in crate::gui_runtime::native_vello) use crate::compat::legacy_shell::{
    AppModel, DirtySegments, FrameBuildResult, KeyPress, NativeAppBridge, NativeMotionModel,
    NativeRunReport, NativeRuntimeArtifacts, SegmentRevisions, UiAction,
};
pub(in crate::gui_runtime::native_vello) use crate::gui::{
    input::KeyCode,
    native_shell::{
        ChromeMotionOverlayFingerprint, CursorMoveEffect, NativeShellState, ShellLayout,
        ShellLayoutRuntime, ShellNodeKind, StaticFrameSegment, StaticFrameSegments, StyleTokens,
        TextFieldVisualState, WaveformMotionOverlayFingerprint,
    },
    paint::{PaintFrame as NativeViewFrame, Primitive},
};
pub(in crate::gui_runtime::native_vello) use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};
pub(in crate::gui_runtime::native_vello) use vello::{
    kurbo::{Circle, Point as KurboPoint},
    peniko::{Gradient, ImageAlphaType, ImageData, ImageFormat},
};
pub(in crate::gui_runtime::native_vello) use winit::{
    event::MouseScrollDelta, keyboard::ModifiersState, window::CursorIcon,
};

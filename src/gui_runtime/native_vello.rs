//! Native `winit + vello` runtime preview used for backend selection rollout.

#![cfg_attr(not(feature = "legacy-shell"), allow(dead_code))]

use super::{NativeRunOptions, WindowIconRgba};
#[cfg(all(test, feature = "legacy-shell"))]
use crate::compat::legacy_shell::PreviewBridge;
#[cfg(feature = "legacy-shell")]
use crate::compat::legacy_shell::{
    AppModel, DirtySegments, FrameBuildResult, KeyPress, NativeAppBridge, NativeMotionModel,
    NativeRunReport, NativeRuntimeArtifacts, SegmentRevisions, UiAction,
};
#[cfg(feature = "legacy-shell")]
use crate::gui::input::KeyCode;
use crate::gui::{
    input::key_code_from_winit,
    paint::{TextAlign, TextRun},
    types::{Point, Rect as UiRect, Rgba8, Vector2},
};
#[cfg(feature = "legacy-shell")]
use crate::gui::{
    native_shell::{
        ChromeMotionOverlayFingerprint, CursorMoveEffect, NativeShellState, ShellLayout,
        ShellLayoutRuntime, ShellNodeKind, StaticFrameSegment, StaticFrameSegments, StyleTokens,
        TextFieldVisualState, WaveformMotionOverlayFingerprint,
    },
    paint::{PaintFrame as NativeViewFrame, Primitive},
};
use crate::runtime::{PaintPrimitive, PaintTextAlign, RuntimeBridge, SurfaceRuntime};
use crate::theme::ThemeTokens;
use crate::widgets::{PointerButton, WidgetId, WidgetInput, WidgetKey};
use skrifa::{
    MetadataProvider,
    instance::{LocationRef, Size as FontSize},
};
use std::{
    collections::{HashMap, VecDeque},
    path::PathBuf,
    sync::Arc,
    time::Instant,
};
#[cfg(feature = "legacy-shell")]
use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};
use tracing::{error, info, warn};
use vello::util::{RenderContext, RenderSurface};
use vello::{
    AaConfig, AaSupport, Glyph, RenderParams, Renderer, RendererOptions, Scene,
    kurbo::{Affine, Rect as KurboRect},
    peniko::{Blob, Color, Fill, FontData},
    wgpu,
};
#[cfg(feature = "legacy-shell")]
use vello::{
    kurbo::{Circle, Point as KurboPoint},
    peniko::{Gradient, ImageAlphaType, ImageData, ImageFormat},
};
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalSize, Size},
    event::{ElementState, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{Key, NamedKey, PhysicalKey},
    window::{Icon, Window, WindowAttributes, WindowId},
};
#[cfg(feature = "legacy-shell")]
use winit::{event::MouseScrollDelta, keyboard::ModifiersState, window::CursorIcon};

mod generic_runtime;
#[cfg(feature = "legacy-shell")]
mod input;
#[cfg(feature = "legacy-shell")]
mod legacy_shell_config;
#[cfg(feature = "legacy-shell")]
mod legacy_shell_runner;
#[cfg(feature = "legacy-shell")]
mod legacy_shell_runtime;
#[cfg(feature = "legacy-shell")]
mod profiling;
#[cfg(feature = "legacy-shell")]
mod runtime_actions;
#[cfg(feature = "legacy-shell")]
mod runtime_events;
#[cfg(feature = "legacy-shell")]
mod runtime_input;
#[cfg(feature = "legacy-shell")]
mod runtime_render;
#[cfg(feature = "legacy-shell")]
mod runtime_startup;
#[cfg(feature = "legacy-shell")]
mod runtime_state;
#[cfg(feature = "legacy-shell")]
mod scene_cache;
#[cfg(feature = "legacy-shell")]
mod scene_rebuild;
mod startup;
#[cfg(feature = "legacy-shell")]
#[path = "../../../../src/app_core/native_shell/composition/runtime/text_entry/mod.rs"]
mod text_bpm;
#[cfg(feature = "legacy-shell")]
mod text_edit;
mod text_renderer;
#[cfg(feature = "legacy-shell")]
mod text_runtime;

#[cfg(feature = "legacy-shell")]
use self::{
    input::*, profiling::*, runtime_state::*, scene_cache::*, scene_rebuild::*, startup::*,
    text_bpm::*, text_edit::*, text_renderer::*,
};
#[cfg(not(feature = "legacy-shell"))]
use self::{startup::*, text_renderer::*};
#[cfg(feature = "legacy-shell")]
pub(in crate::gui_runtime::native_vello) use legacy_shell_runner::NativeVelloRunner;
#[cfg(feature = "legacy-shell")]
pub(in crate::gui_runtime::native_vello) use legacy_shell_config::*;
#[cfg(feature = "legacy-shell")]
pub(crate) use legacy_shell_runtime::run_legacy_shell_vello_app_with_artifacts;

pub use self::{
    generic_runtime::{
        NativeGenericRunReport, NativeGenericRuntimeArtifacts, run_native_vello_runtime,
        run_native_vello_runtime_with_artifacts,
    },
    startup::NativeStartupTimingArtifact,
};

/// High-refresh surface present-mode preference order for animation-heavy playback UI.
const HIGH_REFRESH_PRESENT_MODE_CANDIDATES: [wgpu::PresentMode; 3] = [
    wgpu::PresentMode::Mailbox,
    wgpu::PresentMode::Immediate,
    wgpu::PresentMode::AutoVsync,
];
/// Standard present-mode preference order for non-high-refresh UI.
const STANDARD_PRESENT_MODE_CANDIDATES: [wgpu::PresentMode; 1] = [wgpu::PresentMode::AutoVsync];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RuntimeUserEvent {
    RepaintRequested,
}

/// Return the ordered present-mode fallback chain for the configured frame target.
fn present_mode_candidates(target_fps: u32) -> &'static [wgpu::PresentMode] {
    if target_fps >= 120 {
        &HIGH_REFRESH_PRESENT_MODE_CANDIDATES
    } else {
        &STANDARD_PRESENT_MODE_CANDIDATES
    }
}

fn select_present_mode(
    target_fps: u32,
    supported_present_modes: &[wgpu::PresentMode],
) -> wgpu::PresentMode {
    present_mode_candidates(target_fps)
        .iter()
        .copied()
        .find(|mode| present_mode_is_supported(*mode, supported_present_modes))
        .or_else(|| supported_present_modes.first().copied())
        .unwrap_or(wgpu::PresentMode::Fifo)
}

fn present_mode_is_supported(
    present_mode: wgpu::PresentMode,
    supported_present_modes: &[wgpu::PresentMode],
) -> bool {
    matches!(
        present_mode,
        wgpu::PresentMode::AutoVsync | wgpu::PresentMode::AutoNoVsync
    ) || supported_present_modes.contains(&present_mode)
}

/// Build renderer startup options for the native shell's fixed AA strategy.
///
/// The native runtime currently renders every frame with [`AaConfig::Area`], so
/// startup should avoid compiling MSAA shader variants that will never be used.
fn startup_renderer_options() -> RendererOptions {
    RendererOptions {
        antialiasing_support: AaSupport::area_only(),
        ..RendererOptions::default()
    }
}

#[cfg(all(test, feature = "legacy-shell"))]
mod tests;

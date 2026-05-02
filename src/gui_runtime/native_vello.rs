//! Native `winit + vello` runtime preview used for backend selection rollout.

#![cfg_attr(not(feature = "legacy-shell"), allow(dead_code))]

use super::{NativeRunOptions, WindowIconRgba};
use crate::gui::{
    input::key_code_from_winit,
    paint::{TextAlign, TextRun},
    types::{Point, Rect as UiRect, Rgba8, Vector2},
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
use tracing::{error, info, warn};
use vello::util::{RenderContext, RenderSurface};
use vello::{
    AaConfig, AaSupport, Glyph, RenderParams, Renderer, RendererOptions, Scene,
    kurbo::{Affine, Rect as KurboRect},
    peniko::{Blob, Color, Fill, FontData},
    wgpu,
};
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalSize, Size},
    event::{ElementState, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{Key, NamedKey, PhysicalKey},
    window::{Icon, Window, WindowAttributes, WindowId},
};

mod generic_runtime;
#[cfg(feature = "legacy-shell")]
mod input;
#[cfg(feature = "legacy-shell")]
mod legacy_shell_config;
#[cfg(feature = "legacy-shell")]
mod legacy_shell_prelude;
#[cfg(feature = "legacy-shell")]
mod legacy_shell_runner;
#[cfg(feature = "legacy-shell")]
mod legacy_shell_runtime;
#[cfg(feature = "legacy-shell")]
mod profiling;
#[cfg(feature = "legacy-shell")]
mod runtime_actions;
mod runtime_config;
mod runtime_event;
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
    input::*, legacy_shell_prelude::*, profiling::*, runtime_state::*, scene_cache::*,
    scene_rebuild::*, startup::*, text_bpm::*, text_edit::*, text_renderer::*,
};
#[cfg(not(feature = "legacy-shell"))]
use self::{startup::*, text_renderer::*};
#[cfg(feature = "legacy-shell")]
pub(in crate::gui_runtime::native_vello) use legacy_shell_runner::NativeVelloRunner;
#[cfg(feature = "legacy-shell")]
pub(in crate::gui_runtime::native_vello) use legacy_shell_config::*;
pub(in crate::gui_runtime::native_vello) use runtime_config::*;
pub(in crate::gui_runtime::native_vello) use runtime_event::RuntimeUserEvent;
#[cfg(feature = "legacy-shell")]
pub(crate) use legacy_shell_runtime::run_legacy_shell_vello_app_with_artifacts;

pub use self::{
    generic_runtime::{
        NativeGenericRunReport, NativeGenericRuntimeArtifacts, run_native_vello_runtime,
        run_native_vello_runtime_with_artifacts,
    },
    startup::NativeStartupTimingArtifact,
};

#[cfg(all(test, feature = "legacy-shell"))]
mod tests;

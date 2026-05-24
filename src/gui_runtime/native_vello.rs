//! Native `winit + vello` runtime for generic `RuntimeBridge` hosts.

use super::{
    NativeGpuBackend, NativePopupOptions, NativeRunOptions, NativeRunOptionsError,
    NativeTextOptions, NativeWindowMode, WindowIconRgba,
};
use crate::gui::{
    paint::{PaintFrame, Primitive, TextAlign},
    types::{Point, Rect as UiRect, Rgba8, Vector2},
};
use crate::runtime::{
    PaintFillRule, PaintPath, PaintPathCommand, PaintPrimitive, PaintSvg, PaintTextAlign,
    PaintTextInput, PaintTransform, RuntimeBridge,
};
use crate::widgets::{RetainedSurfaceDescriptor, TextEditCommand, WidgetKey};
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use tracing::{error, info, warn};
use vello::util::{RenderContext, RenderSurface};
use vello::{
    AaConfig, RenderParams, Renderer, Scene,
    kurbo::{Affine, BezPath, Circle, Point as KurboPoint},
    peniko::{BlendMode, Blob, Fill, Gradient, ImageAlphaType, ImageData, ImageFormat},
    wgpu,
};
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalPosition, LogicalSize, Position, Size},
    event::{ElementState, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{Key, NamedKey, PhysicalKey},
    window::{Window, WindowAttributes, WindowId, WindowLevel},
};

mod generic_runtime;
mod runtime_config;
mod runtime_event;
mod startup;
mod text_edit;
mod text_renderer;

use self::{startup::*, text_renderer::*};
pub(in crate::gui_runtime::native_vello) use runtime_config::{
    select_present_mode, startup_renderer_options,
};
pub(in crate::gui_runtime::native_vello) use runtime_event::RuntimeUserEvent;

pub use self::{
    generic_runtime::{
        NativeGenericRunError, NativeGenericRunReport, NativeGenericRuntimeArtifacts,
        run_native_vello_runtime, run_native_vello_runtime_with_artifacts,
    },
    startup::NativeStartupTimingArtifact,
};

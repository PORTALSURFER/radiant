use super::*;
use crate::{
    layout::{ContainerKind, ContainerPolicy, LayoutDebugOptions, Rect, SlotParams},
    runtime::{
        Command, GpuSignalSummary, GpuSurfaceCapabilities, GpuSurfaceContent, GpuSurfaceLineStyle,
        GpuSurfaceOverlay, GpuSurfaceRuntimeOverlays, PaintGpuSurface, PaintPrimitive,
        SurfaceChild, SurfaceNode, UiSurface, WidgetMessageMapper,
    },
    widgets::{
        ButtonWidget, CanvasMessage, DragHandleWidget, PointerButton, ScrollbarAxis,
        ScrollbarMessage, ScrollbarWidget, TextInputMessage, TextInputWidget, Widget, WidgetCommon,
        WidgetId, WidgetInput, WidgetOutput, WidgetSizing,
    },
};
use winit::{dpi::Position, window::WindowLevel};

#[cfg(test)]
#[path = "tests/event_routing.rs"]
mod event_routing;
#[path = "tests/fixtures.rs"]
mod fixtures;
#[cfg(test)]
#[path = "tests/gpu_surface_runtime.rs"]
mod gpu_surface_runtime;
#[cfg(test)]
#[path = "tests/pointer_motion.rs"]
mod pointer_motion;
#[path = "tests/runtime_core.rs"]
mod runtime_core;
#[cfg(test)]
#[path = "tests/scene_cache.rs"]
mod scene_cache;
#[path = "tests/timing.rs"]
mod timing;
#[path = "tests/window_policy.rs"]
mod window_policy;
use fixtures::*;

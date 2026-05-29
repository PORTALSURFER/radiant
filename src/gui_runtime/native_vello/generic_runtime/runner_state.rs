//! Focused state groups owned by the generic native Vello runner.

use super::PendingGpuSurfaceWheel;
use crate::gui::types::Point;
use crate::gui::types::Vector2;
use crate::gui_runtime::native_vello::startup::StartupTimingProfile;
use crate::widgets::WidgetCursor;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use vello::{
    Renderer,
    util::{RenderContext, RenderSurface},
};
use winit::{
    dpi::PhysicalSize,
    keyboard::ModifiersState,
    window::{Window, WindowId},
};

#[derive(Default)]
pub(super) struct NativeRunnerWindowState {
    pub(super) id: Option<WindowId>,
    pub(super) window: Option<Arc<Window>>,
    pub(super) render_ctx: Option<RenderContext>,
    pub(super) render_surface: Option<RenderSurface<'static>>,
    pub(super) renderer: Option<Renderer>,
    pub(super) native_dpi_scale: crate::theme::DpiScale,
    pub(super) dpi_scale: crate::theme::DpiScale,
    pub(super) dpi_scale_override: Option<crate::theme::DpiScale>,
}

pub(super) struct NativeRunnerInputState {
    pub(super) last_cursor: Option<Point>,
    pub(super) native_cursor: Option<WidgetCursor>,
    pub(super) clipboard: Option<arboard::Clipboard>,
    pub(super) modifiers: ModifiersState,
    pub(super) last_navigation_key_repeat: Option<Instant>,
    pub(super) pending_gpu_surface_wheel: Option<PendingGpuSurfaceWheel>,
}

impl Default for NativeRunnerInputState {
    fn default() -> Self {
        Self {
            last_cursor: None,
            native_cursor: None,
            clipboard: arboard::Clipboard::new().ok(),
            modifiers: ModifiersState::default(),
            last_navigation_key_repeat: None,
            pending_gpu_surface_wheel: None,
        }
    }
}

pub(super) struct NativeRunnerTimingState {
    pub(super) redraw_requested: bool,
    pub(super) startup_timing: StartupTimingProfile,
    pub(super) first_frame_presented: bool,
    pub(super) animation_origin: Instant,
    pub(super) last_redraw: Instant,
    pub(super) last_timed_frame_drain: Instant,
    pub(super) deferred_surface_refresh: bool,
    pub(super) deferred_scene_rebuild: bool,
    pub(super) deferred_scene_rebuild_requires_encode: bool,
    pub(super) deferred_auxiliary_window_sync: bool,
    pub(super) last_interactive_scene_rebuild: Instant,
    pub(super) pending_surface_resize: Option<PhysicalSize<u32>>,
    pub(super) pending_viewport_resize: Option<Vector2>,
    pub(super) surface_resize_applied_this_frame: bool,
}

impl Default for NativeRunnerTimingState {
    fn default() -> Self {
        let now = Instant::now();
        Self {
            redraw_requested: false,
            startup_timing: StartupTimingProfile::new(),
            first_frame_presented: false,
            animation_origin: now,
            last_redraw: now,
            last_timed_frame_drain: now,
            deferred_surface_refresh: false,
            deferred_scene_rebuild: false,
            deferred_scene_rebuild_requires_encode: false,
            deferred_auxiliary_window_sync: false,
            last_interactive_scene_rebuild: now - Duration::from_secs(1),
            pending_surface_resize: None,
            pending_viewport_resize: None,
            surface_resize_applied_this_frame: false,
        }
    }
}

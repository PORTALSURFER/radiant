//! Native `winit + vello` runtime preview used for backend selection rollout.

use super::{NativeRunOptions, WindowIconRgba};
use crate::app::{AppModel, FrameBuildResult, NativeAppBridge, UiAction};
use crate::gui::{
    input::{KeyCode, key_code_from_winit},
    native_shell::{
        NativeShellState, NativeViewFrame, Primitive, ShellLayout, ShellNodeKind, StyleTokens,
        TextAlign, TextRun,
    },
    types::{Point, Rect as UiRect, Rgba8, Vector2},
};
use skrifa::{
    MetadataProvider,
    instance::{LocationRef, Size as FontSize},
};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};
use vello::util::{RenderContext, RenderSurface};
use vello::{
    AaConfig, Glyph, RenderParams, Renderer, RendererOptions, Scene,
    kurbo::{Affine, Circle, Rect as KurboRect},
    peniko::{Blob, Color, Fill, FontData},
    wgpu,
};
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalSize, Size},
    event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{Key, ModifiersState, NamedKey, PhysicalKey},
    window::{Icon, Window, WindowAttributes, WindowId},
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum TextInputTarget {
    #[default]
    None,
    BrowserSearch,
    FolderSearch,
    PromptInput,
}

#[derive(Clone, Copy, Debug, Default)]
struct NativeVelloFrameState {
    layout_dirty: bool,
    scene_dirty: bool,
    model_dirty: bool,
}

impl NativeVelloFrameState {
    fn mark_layout_dirty(&mut self) {
        self.layout_dirty = true;
        self.scene_dirty = true;
    }

    fn mark_scene_dirty(&mut self) {
        self.scene_dirty = true;
    }

    fn clear_layout_dirty(&mut self) {
        self.layout_dirty = false;
    }

    fn mark_model_dirty(&mut self) {
        self.model_dirty = true;
    }

    fn take_scene(&mut self) -> bool {
        let dirty = self.scene_dirty;
        self.scene_dirty = false;
        dirty
    }

    fn take_model(&mut self) -> bool {
        let dirty = self.model_dirty;
        self.model_dirty = false;
        dirty
    }
}

struct NativeVelloRunner<B: NativeAppBridge> {
    options: NativeRunOptions,
    bridge: B,
    model: AppModel,
    window_id: Option<WindowId>,
    window: Option<Arc<Window>>,
    render_ctx: Option<RenderContext>,
    render_surface: Option<RenderSurface<'static>>,
    renderer: Option<Renderer>,
    frame_cache: NativeViewFrame,
    scene: Scene,
    text_renderer: NativeTextRenderer,
    style_cache: Option<StyleTokens>,
    frame_state: NativeVelloFrameState,
    shell_layout: Option<ShellLayout>,
    shell_state: NativeShellState,
    clear_color: Rgba8,
    last_cursor: Option<Point>,
    modifiers: ModifiersState,
    text_input_target: TextInputTarget,
    last_redraw: Instant,
}

impl<B: NativeAppBridge> NativeVelloRunner<B> {
    fn new(options: NativeRunOptions, bridge: B) -> Self {
        Self {
            options,
            bridge,
            model: AppModel::default(),
            window_id: None,
            window: None,
            render_ctx: None,
            render_surface: None,
            renderer: None,
            frame_cache: NativeViewFrame {
                clear_color: Rgba8 {
                    r: 0,
                    g: 0,
                    b: 0,
                    a: 255,
                },
                primitives: Vec::new(),
                text_runs: Vec::new(),
            },
            scene: Scene::new(),
            text_renderer: NativeTextRenderer::new(),
            style_cache: None,
            frame_state: NativeVelloFrameState {
                model_dirty: true,
                ..NativeVelloFrameState::default()
            },
            shell_layout: None,
            shell_state: NativeShellState::new(),
            clear_color: Rgba8 {
                r: 0,
                g: 0,
                b: 0,
                a: 255,
            },
            last_cursor: None,
            modifiers: ModifiersState::default(),
            text_input_target: TextInputTarget::None,
            last_redraw: Instant::now(),
        }
    }

    fn ui_scale_factor(&self) -> f32 {
        self.window
            .as_ref()
            .map(|window| {
                let scale = window.scale_factor() as f32;
                scale.clamp(1.0, 3.0)
            })
            .unwrap_or(1.0)
    }

    fn build_window_attributes(&self) -> WindowAttributes {
        let mut attrs = Window::default_attributes()
            .with_title(self.options.title.clone())
            .with_maximized(self.options.maximized);
        if let Some([w, h]) = self.options.inner_size {
            attrs = attrs.with_inner_size(Size::Logical(LogicalSize::new(w as f64, h as f64)));
        }
        if let Some([w, h]) = self.options.min_inner_size {
            attrs = attrs.with_min_inner_size(Size::Logical(LogicalSize::new(w as f64, h as f64)));
        }
        if let Some(icon) = self.options.icon.as_ref().and_then(icon_from_rgba) {
            attrs = attrs.with_window_icon(Some(icon));
        }
        #[cfg(target_os = "windows")]
        {
            use winit::platform::windows::WindowAttributesExtWindows;
            attrs = attrs.with_drag_and_drop(true);
        }
        attrs
    }

    fn initialize_runtime(&mut self, event_loop: &ActiveEventLoop) {
        let window = match event_loop.create_window(self.build_window_attributes()) {
            Ok(window) => Arc::new(window),
            Err(err) => {
                eprintln!("Failed to create native vello window: {err}");
                event_loop.exit();
                return;
            }
        };
        let mut render_ctx = RenderContext::new();
        let size = window.inner_size();
        let width = size.width.max(1);
        let height = size.height.max(1);
        let render_surface = match pollster::block_on(render_ctx.create_surface(
            window.clone(),
            width,
            height,
            wgpu::PresentMode::AutoVsync,
        )) {
            Ok(surface) => surface,
            Err(err) => {
                eprintln!("Failed to create native vello surface: {err}");
                event_loop.exit();
                return;
            }
        };
        let dev_handle = &render_ctx.devices[render_surface.dev_id];
        let renderer = match Renderer::new(&dev_handle.device, RendererOptions::default()) {
            Ok(renderer) => renderer,
            Err(err) => {
                eprintln!("Failed to create native vello renderer: {err}");
                event_loop.exit();
                return;
            }
        };

        self.window_id = Some(window.id());
        self.window = Some(window);
        self.render_ctx = Some(render_ctx);
        self.render_surface = Some(render_surface);
        self.renderer = Some(renderer);
        self.frame_state.mark_layout_dirty();
        self.rebuild_scene_if_needed();
        self.last_redraw = Instant::now();
    }

    fn rebuild_layout(&mut self) {
        let Some(surface) = self.render_surface.as_ref() else {
            return;
        };

        let viewport = Vector2::new(surface.config.width as f32, surface.config.height as f32);
        let style = StyleTokens::for_viewport_with_scale(viewport.x, self.ui_scale_factor());
        self.style_cache = Some(style);
        self.shell_layout = Some(ShellLayout::build_with_style(viewport, &style));
        self.frame_state.clear_layout_dirty();
    }

    fn build_style_for_layout(layout: &ShellLayout) -> StyleTokens {
        StyleTokens::for_viewport_with_scale(layout.root.rect.width(), layout.ui_scale)
    }

    fn cached_style_for_layout(&self, layout: &ShellLayout) -> StyleTokens {
        self.style_cache
            .unwrap_or_else(|| Self::build_style_for_layout(layout))
    }

    fn rebuild_scene_if_needed(&mut self) {
        if self.shell_layout.is_none() || self.frame_state.layout_dirty {
            self.rebuild_layout();
        }
        if !self.frame_state.take_scene() {
            return;
        }
        self.rebuild_scene();
    }

    fn rebuild_scene_and_request_redraw(&mut self) {
        self.frame_state.mark_scene_dirty();
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }

    fn rebuild_scene_for_tick(&mut self) {
        self.frame_state.mark_scene_dirty();
        self.rebuild_scene_if_needed();
    }

    fn rebuild_scene(&mut self) {
        self.scene.reset();
        let should_refresh_model = self.frame_state.take_model()
            || self.shell_state.is_transport_running();
        if should_refresh_model {
            self.model = self.bridge.pull_model();
            self.shell_state.sync_from_model(&self.model);
            self.sync_text_input_target();
        }
        let Some(layout) = self.shell_layout.as_ref() else {
            return;
        };
        let style = self.cached_style_for_layout(layout);
        self.shell_state.build_frame_with_style_into(
            layout,
            &style,
            &self.model,
            &mut self.frame_cache,
        );
        self.clear_color = self.frame_cache.clear_color;
        let frame_result = FrameBuildResult {
            primitive_count: self.frame_cache.primitives.len(),
            text_run_count: self.frame_cache.text_runs.len(),
            needs_animation: self.shell_state.needs_animation(),
        };
        self.bridge.on_frame_result(frame_result);
        for primitive in self.frame_cache.primitives.iter() {
            match primitive {
                Primitive::Rect(fill) => {
                    self.scene.fill(
                        Fill::NonZero,
                        Affine::IDENTITY,
                        color_from_rgba(fill.color),
                        None,
                        &to_kurbo_rect(fill.rect),
                    );
                }
                Primitive::Circle(fill) => {
                    self.scene.fill(
                        Fill::NonZero,
                        Affine::IDENTITY,
                        color_from_rgba(fill.color),
                        None,
                        &Circle::new(
                            (fill.center.x as f64, fill.center.y as f64),
                            fill.radius as f64,
                        ),
                    );
                }
            }
        }
        self.text_renderer
            .draw_text_runs(&mut self.scene, &self.frame_cache.text_runs);
    }

    fn redraw(&mut self, event_loop: &ActiveEventLoop) {
        let now = Instant::now();
        let delta = (now - self.last_redraw).as_secs_f32();
        self.last_redraw = now;
        let Some(layout) = self.shell_layout.as_ref() else {
            return;
        };
        let style = self.cached_style_for_layout(layout);
        self.shell_state.tick_with_style(delta, &style);
        self.rebuild_scene_for_tick();

        let window = self.window.as_ref().cloned();
        let (Some(window), Some(render_ctx), Some(surface), Some(renderer)) = (
            window,
            self.render_ctx.as_mut(),
            self.render_surface.as_mut(),
            self.renderer.as_mut(),
        ) else {
            return;
        };
        let dev_handle = &render_ctx.devices[surface.dev_id];
        let surface_texture = match surface.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(err) => {
                match err {
                    wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated => {
                        let size = window.inner_size();
                        render_ctx.resize_surface(surface, size.width.max(1), size.height.max(1));
                        self.frame_state.mark_layout_dirty();
                        self.rebuild_scene_if_needed();
                        window.request_redraw();
                    }
                    wgpu::SurfaceError::OutOfMemory => event_loop.exit(),
                    wgpu::SurfaceError::Timeout => {}
                    wgpu::SurfaceError::Other => {}
                }
                return;
            }
        };
        let surface_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let render_result = renderer.render_to_texture(
            &dev_handle.device,
            &dev_handle.queue,
            &self.scene,
            &surface.target_view,
            &RenderParams {
                base_color: color_from_rgba(self.clear_color),
                width: surface.config.width,
                height: surface.config.height,
                antialiasing_method: AaConfig::Area,
            },
        );
        if let Err(err) = render_result {
            eprintln!("Native vello render failed: {err}");
            event_loop.exit();
            return;
        }
        let mut encoder =
            dev_handle
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("native_vello_present_blit"),
                });
        surface.blitter.copy(
            &dev_handle.device,
            &mut encoder,
            &surface.target_view,
            &surface_view,
        );
        dev_handle.queue.submit(std::iter::once(encoder.finish()));
        surface_texture.present();
    }

    fn sync_text_input_target(&mut self) {
        if self.model.confirm_prompt.visible && self.model.confirm_prompt.input_value.is_some() {
            self.text_input_target = TextInputTarget::PromptInput;
        } else if self.text_input_target == TextInputTarget::PromptInput {
            self.text_input_target = TextInputTarget::None;
        }
    }

    fn current_text_value(&self) -> Option<String> {
        match self.text_input_target {
            TextInputTarget::None => None,
            TextInputTarget::BrowserSearch => Some(self.model.browser.search_query.clone()),
            TextInputTarget::FolderSearch => Some(self.model.sources.folder_search_query.clone()),
            TextInputTarget::PromptInput => self.model.confirm_prompt.input_value.clone(),
        }
    }

    fn set_text_value(&mut self, value: String) -> bool {
        let action = match self.text_input_target {
            TextInputTarget::None => return false,
            TextInputTarget::BrowserSearch => UiAction::SetBrowserSearch { query: value },
            TextInputTarget::FolderSearch => UiAction::SetFolderSearch { query: value },
            TextInputTarget::PromptInput => UiAction::SetPromptInput { value },
        };
        self.emit_model_action(action);
        true
    }

    fn append_text(&mut self, appended: &str) -> bool {
        if appended.is_empty() {
            return false;
        }
        let Some(mut value) = self.current_text_value() else {
            return false;
        };
        value.push_str(appended);
        self.set_text_value(value)
    }

    fn emit_model_action(&mut self, action: UiAction) {
        self.frame_state.mark_model_dirty();
        self.bridge.on_action(action);
    }

    fn backspace_text(&mut self) -> bool {
        let Some(mut value) = self.current_text_value() else {
            return false;
        };
        if value.pop().is_none() {
            return false;
        }
        self.set_text_value(value)
    }

    fn update_text_target_after_action(&mut self, action: &UiAction) {
        match action {
            UiAction::FocusBrowserSearch => self.text_input_target = TextInputTarget::BrowserSearch,
            UiAction::FocusFolderSearch => self.text_input_target = TextInputTarget::FolderSearch,
            UiAction::ConfirmPrompt | UiAction::CancelPrompt => {
                self.text_input_target = TextInputTarget::None;
            }
            _ => {}
        }
    }
}

impl<B: NativeAppBridge> ApplicationHandler for NativeVelloRunner<B> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            self.initialize_runtime(event_loop);
            if let Some(window) = self.window.as_ref() {
                window.request_redraw();
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if Some(window_id) != self.window_id {
            return;
        }
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::ScaleFactorChanged { .. } => {
                self.frame_state.mark_layout_dirty();
                self.frame_state.mark_model_dirty();
                self.rebuild_scene_and_request_redraw();
            }
            WindowEvent::Resized(size) => {
                let window = self.window.as_ref().cloned();
                if size.width > 0
                    && size.height > 0
                    && let (Some(render_ctx), Some(surface), Some(_window)) = (
                        self.render_ctx.as_ref(),
                        self.render_surface.as_mut(),
                        window,
                    )
                {
                    render_ctx.resize_surface(surface, size.width, size.height);
                    self.frame_state.mark_layout_dirty();
                    self.frame_state.mark_model_dirty();
                    self.rebuild_scene_and_request_redraw();
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let point = Point::new(position.x as f32, position.y as f32);
                self.last_cursor = Some(point);
                let _window = self.window.as_ref().cloned();
                if let Some(layout) = self.shell_layout.as_ref()
                    && self.shell_state.handle_cursor_move(layout, point)
                    && let Some(_window) = _window
                {
                    self.rebuild_scene_and_request_redraw();
                }
            }
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state: ElementState::Pressed,
                ..
            } => {
                let _window = self.window.as_ref().cloned();
                if let (Some(point), Some(layout), Some(_window)) =
                    (self.last_cursor, self.shell_layout.as_ref(), _window)
                {
                    self.text_input_target = TextInputTarget::None;
                    let mut handled = false;
                    if self
                        .shell_state
                        .prompt_input_at_point(layout, &self.model, point)
                    {
                        self.text_input_target = TextInputTarget::PromptInput;
                        handled = true;
                    } else if let Some(action) = action_from_pointer(
                        layout,
                        &self.model,
                        &mut self.shell_state,
                        point,
                        self.modifiers,
                    ) {
                        self.update_text_target_after_action(&action);
                        self.emit_model_action(action);
                        handled = true;
                    } else if self.shell_state.handle_primary_click(layout, point)
                        && let Some(column) = layout.column_at_point(point)
                    {
                        self.emit_model_action(UiAction::SelectColumn { index: column });
                        handled = true;
                    }
                    if handled {
                        self.rebuild_scene_and_request_redraw();
                    }
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                if let (Some(point), Some(layout)) = (self.last_cursor, self.shell_layout.as_ref())
                {
                    let style = self.cached_style_for_layout(layout);
                    if let Some(delta) =
                        browser_wheel_row_delta(layout, &self.model, point, &style, delta)
                    {
                        self.emit_model_action(UiAction::MoveBrowserFocus { delta });
                        self.rebuild_scene_and_request_redraw();
                    }
                }
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.modifiers = modifiers.state();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state == ElementState::Pressed && !event.repeat {
                    let mut handled = false;
                    if matches!(event.logical_key, Key::Named(NamedKey::Escape)) {
                        if self.model.confirm_prompt.visible {
                            self.emit_model_action(UiAction::CancelPrompt);
                            self.text_input_target = TextInputTarget::None;
                            handled = true;
                        } else if self.text_input_target != TextInputTarget::None {
                            self.text_input_target = TextInputTarget::None;
                            handled = true;
                        }
                    }
                    if !handled && matches!(event.logical_key, Key::Named(NamedKey::Backspace)) {
                        handled = self.backspace_text();
                    }
                    if !handled
                        && matches!(event.logical_key, Key::Named(NamedKey::Enter))
                        && matches!(
                            self.text_input_target,
                            TextInputTarget::BrowserSearch | TextInputTarget::FolderSearch
                        )
                    {
                        self.text_input_target = TextInputTarget::None;
                        handled = true;
                    }
                    if !handled
                        && self.text_input_target != TextInputTarget::None
                        && !self.modifiers.control_key()
                        && !self.modifiers.super_key()
                        && !self.modifiers.alt_key()
                        && let Some(text) = event.text.as_ref()
                    {
                        let appended: String = text.chars().filter(|ch| !ch.is_control()).collect();
                        if !appended.is_empty() {
                            handled = self.append_text(&appended);
                        }
                    }
                    if !handled
                        && let PhysicalKey::Code(code) = event.physical_key
                        && let Some(key) = key_code_from_winit(code)
                    {
                        handled = if self.model.confirm_prompt.visible {
                            false
                        } else {
                            self.shell_state.handle_key(key)
                        };
                        if let Some(action) = action_from_key(key, self.modifiers, &self.model) {
                            self.update_text_target_after_action(&action);
                            self.emit_model_action(action);
                            handled = true;
                        }
                    }
                    if handled {
                        self.rebuild_scene_and_request_redraw();
                    }
                }
            }
            WindowEvent::RedrawRequested => self.redraw(event_loop),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.shell_state.needs_animation() {
            if let Some(window) = self.window.as_ref() {
                window.request_redraw();
            }
            event_loop.set_control_flow(ControlFlow::WaitUntil(
                Instant::now() + Duration::from_millis(16),
            ));
        } else {
            event_loop.set_control_flow(ControlFlow::Wait);
        }
    }
}

fn action_from_key(key: KeyCode, modifiers: ModifiersState, model: &AppModel) -> Option<UiAction> {
    if model.confirm_prompt.visible {
        let confirm_enabled = model
            .confirm_prompt
            .input_error
            .as_ref()
            .is_none_or(|error| error.trim().is_empty());
        return match key {
            KeyCode::Enter if confirm_enabled => Some(UiAction::ConfirmPrompt),
            KeyCode::C => Some(UiAction::CancelPrompt),
            _ => None,
        };
    }
    let shift = modifiers.shift_key();
    let command = modifiers.control_key() || modifiers.super_key();
    match key {
        KeyCode::ArrowLeft => Some(UiAction::MoveColumn { delta: -1 }),
        KeyCode::ArrowRight => Some(UiAction::MoveColumn { delta: 1 }),
        KeyCode::ArrowUp => {
            if shift && command {
                Some(UiAction::AddRangeBrowserSelectionFromFocus { delta: -1 })
            } else if shift {
                Some(UiAction::ExtendBrowserSelectionFromFocus { delta: -1 })
            } else {
                Some(UiAction::MoveBrowserFocus { delta: -1 })
            }
        }
        KeyCode::ArrowDown => {
            if shift && command {
                Some(UiAction::AddRangeBrowserSelectionFromFocus { delta: 1 })
            } else if shift {
                Some(UiAction::ExtendBrowserSelectionFromFocus { delta: 1 })
            } else {
                Some(UiAction::MoveBrowserFocus { delta: 1 })
            }
        }
        KeyCode::Num1 => Some(UiAction::SelectColumn { index: 0 }),
        KeyCode::Num2 => Some(UiAction::SelectColumn { index: 1 }),
        KeyCode::Num3 => Some(UiAction::SelectColumn { index: 2 }),
        KeyCode::A => Some(UiAction::SelectAllBrowserRows),
        KeyCode::B => Some(UiAction::StartNewFolder),
        KeyCode::C => Some(UiAction::ClearWaveformSelection),
        KeyCode::D => Some(UiAction::DeleteBrowserSelection),
        KeyCode::Enter => Some(UiAction::ToggleTransport),
        KeyCode::F => Some(UiAction::FocusBrowserSearch),
        KeyCode::G => Some(UiAction::DeleteFocusedFolder),
        KeyCode::I => Some(UiAction::StartBrowserRename),
        KeyCode::L => Some(UiAction::ToggleLoopPlayback),
        KeyCode::M => Some(UiAction::ZoomWaveformToSelection),
        KeyCode::N => Some(UiAction::TagBrowserSelection {
            target: crate::app::BrowserTagTarget::Neutral,
        }),
        KeyCode::OpenBracket => Some(UiAction::ZoomWaveform {
            zoom_in: false,
            steps: 1,
        }),
        KeyCode::P => model
            .progress_overlay
            .cancelable
            .then_some(UiAction::CancelProgress),
        KeyCode::CloseBracket => Some(UiAction::ZoomWaveform {
            zoom_in: true,
            steps: 1,
        }),
        KeyCode::Slash => Some(UiAction::ZoomWaveformFull),
        KeyCode::Quote => Some(UiAction::FocusFolderSearch),
        KeyCode::R => Some(UiAction::Redo),
        KeyCode::S => Some(UiAction::FocusSourcesPanel),
        KeyCode::T => Some(UiAction::ToggleFocusedBrowserRowSelection),
        KeyCode::U => Some(UiAction::Undo),
        KeyCode::W => Some(UiAction::FocusWaveformPanel),
        KeyCode::X => Some(UiAction::TagBrowserSelection {
            target: crate::app::BrowserTagTarget::Trash,
        }),
        KeyCode::Y => Some(UiAction::TagBrowserSelection {
            target: crate::app::BrowserTagTarget::Keep,
        }),
        KeyCode::Z => Some(UiAction::StartFolderRename),
        _ => None,
    }
}

fn action_from_pointer(
    layout: &ShellLayout,
    model: &AppModel,
    shell_state: &mut NativeShellState,
    point: Point,
    modifiers: ModifiersState,
) -> Option<UiAction> {
    if let Some(action) = shell_state.prompt_action_at_point(layout, model, point) {
        return Some(action);
    }
    if let Some(action) = shell_state.progress_action_at_point(layout, model, point) {
        return Some(action);
    }
    if let Some(action) = shell_state.update_action_at_point(layout, model, point) {
        return Some(action);
    }
    if let Some(action) = shell_state.browser_tab_action_at_point(layout, point) {
        return Some(action);
    }
    if let Some(action) = shell_state.map_sample_action_at_point(layout, model, point) {
        return Some(action);
    }
    if let Some(action) = shell_state.browser_action_at_point(layout, model, point) {
        return Some(action);
    }
    if let Some(action) = shell_state.source_action_at_point(layout, model, point) {
        return Some(action);
    }
    if let Some(visible_row) = shell_state.browser_row_at_point(layout, model, point) {
        let shift = modifiers.shift_key();
        let command = modifiers.control_key() || modifiers.super_key();
        return Some(if shift && command {
            UiAction::AddRangeBrowserSelection { visible_row }
        } else if shift {
            UiAction::ExtendBrowserSelectionToRow { visible_row }
        } else if command {
            UiAction::ToggleBrowserRowSelection { visible_row }
        } else {
            UiAction::FocusBrowserRow { visible_row }
        });
    }
    if let Some(index) = shell_state.folder_row_at_point(layout, model, point) {
        return Some(UiAction::FocusFolderRow { index });
    }

    let hit = layout.hit_test(point)?;
    match hit {
        ShellNodeKind::Sidebar => shell_state
            .source_row_at_point(layout, model, point)
            .map_or(Some(UiAction::FocusSourcesPanel), |index| {
                Some(UiAction::SelectSourceRow { index })
            }),
        ShellNodeKind::WaveformCard => {
            let inner = layout.waveform_plot;
            let width = inner.width().max(1.0);
            let ratio = ((point.x - inner.min.x) / width).clamp(0.0, 1.0);
            let position_milli = ratio_to_milli(ratio);
            let shift = modifiers.shift_key();
            let command = modifiers.control_key() || modifiers.super_key();
            if shift {
                Some(UiAction::SetWaveformSelectionRange {
                    start_milli: waveform_anchor_milli(model),
                    end_milli: position_milli,
                })
            } else if command {
                Some(UiAction::SetWaveformCursor { position_milli })
            } else {
                Some(UiAction::SeekWaveform { position_milli })
            }
        }
        ShellNodeKind::TopBar => Some(UiAction::ToggleTransport),
        ShellNodeKind::Content
        | ShellNodeKind::BrowserPanel
        | ShellNodeKind::BrowserTabs
        | ShellNodeKind::BrowserTable => Some(UiAction::FocusBrowserPanel),
        ShellNodeKind::StatusBar => Some(UiAction::FocusLoadedSampleInBrowser),
        _ => None,
    }
}

fn ratio_to_milli(ratio: f32) -> u16 {
    (ratio.clamp(0.0, 1.0) * 1000.0).round() as u16
}

fn waveform_anchor_milli(model: &AppModel) -> u16 {
    model
        .waveform
        .selection_milli
        .map(|selection| selection.start_milli)
        .or(model.waveform.cursor_milli)
        .or(model.waveform.playhead_milli)
        .unwrap_or(0)
}

fn browser_wheel_row_delta(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
    style: &StyleTokens,
    delta: MouseScrollDelta,
) -> Option<i8> {
    if model.map.active || !layout.browser_rows.contains(point) {
        return None;
    }
    let row_stride = (style.sizing.browser_row_height + style.sizing.browser_row_gap).max(1.0);
    let raw = match delta {
        MouseScrollDelta::LineDelta(_, y) => y,
        MouseScrollDelta::PixelDelta(position) => (position.y as f32) / row_stride,
    };
    let mut steps = raw.round();
    if steps.abs() < 1.0 {
        steps = raw.signum();
    }
    if steps == 0.0 {
        return None;
    }
    let clamped = if steps > 1.0 { steps.min(i8::MAX as f32) } else { steps.max(i8::MIN as f32) };
    Some(clamped as i8)
}

#[derive(Clone, Debug)]
struct GlyphLayout {
    id: u32,
    x: f32,
}

#[derive(Clone, Debug)]
struct TextLayout {
    width: f32,
    glyphs: Vec<GlyphLayout>,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct TextLayoutKey {
    text: String,
    font_size_bits: u32,
}

const TEXT_LAYOUT_CACHE_CAPACITY: usize = 2_048;

#[derive(Clone)]
struct LoadedFont {
    font: FontData,
}

struct NativeTextRenderer {
    loaded_font: Option<LoadedFont>,
    layout_cache: HashMap<TextLayoutKey, TextLayout>,
}

impl NativeTextRenderer {
    fn new() -> Self {
        let loaded_font = load_native_font().map(|font| LoadedFont { font });
        if loaded_font.is_none() {
            eprintln!(
                "Native vello text renderer: no fallback font found; text runs will be skipped"
            );
        }
        Self {
            loaded_font,
            layout_cache: HashMap::with_capacity(TEXT_LAYOUT_CACHE_CAPACITY / 2),
        }
    }

    fn draw_text_runs(&mut self, scene: &mut Scene, text_runs: &[TextRun]) {
        let Some(font) = self.loaded_font.clone() else {
            return;
        };
        for run in text_runs {
            if run.text.is_empty() || run.font_size <= 0.0 {
                continue;
            }
            let Some(layout) = self.layout_for(&font, &run.text, run.font_size) else {
                continue;
            };
            let mut origin_x = run.position.x;
            if let Some(max_width) = run.max_width {
                let extra = (max_width - layout.width).max(0.0);
                origin_x += match run.align {
                    TextAlign::Left => 0.0,
                    TextAlign::Center => extra * 0.5,
                    TextAlign::Right => extra,
                };
            }
            let clip_width = run.max_width.unwrap_or(f32::INFINITY);
            let baseline = run.position.y + run.font_size;
            let glyph_iter = layout
                .glyphs
                .iter()
                .take_while(|glyph| glyph.x <= clip_width)
                .map(|glyph| Glyph {
                    id: glyph.id,
                    x: origin_x + glyph.x,
                    y: baseline,
                });
            scene
                .draw_glyphs(&font.font)
                .font_size(run.font_size)
                .brush(color_from_rgba(run.color))
                .draw(Fill::NonZero, glyph_iter);
        }
    }

    fn layout_for<'a>(
        &'a mut self,
        font: &LoadedFont,
        text: &str,
        font_size: f32,
    ) -> Option<&'a TextLayout> {
        let key = TextLayoutKey {
            text: text.to_string(),
            font_size_bits: font_size.to_bits(),
        };
        if self.layout_cache.len() >= TEXT_LAYOUT_CACHE_CAPACITY {
            self.layout_cache.clear();
        }
        if !self.layout_cache.contains_key(&key) {
            let layout = Self::compute_layout(font, text, font_size)?;
            self.layout_cache.insert(key.clone(), layout);
        }
        self.layout_cache.get(&key)
    }

    fn compute_layout(font: &LoadedFont, text: &str, font_size: f32) -> Option<TextLayout> {
        let font_ref =
            skrifa::FontRef::from_index(font.font.data.as_ref(), font.font.index).ok()?;
        let charmap = font_ref.charmap();
        let metrics = font_ref.glyph_metrics(FontSize::new(font_size), LocationRef::default());
        let fallback_glyph = charmap.map('?');

        let mut x = 0.0_f32;
        let mut glyphs = Vec::with_capacity(text.len());
        for ch in text.chars() {
            if ch == '\n' || ch == '\r' {
                break;
            }
            if ch == '\t' {
                x += font_size * 2.0;
                continue;
            }
            if ch == ' ' {
                x += font_size * 0.33;
                continue;
            }
            if ch.is_control() {
                continue;
            }
            let glyph_id = charmap.map(ch).or(fallback_glyph);
            let Some(glyph_id) = glyph_id else {
                x += font_size * 0.5;
                continue;
            };
            glyphs.push(GlyphLayout {
                id: glyph_id.to_u32(),
                x,
            });
            let advance = metrics
                .advance_width(glyph_id)
                .unwrap_or(font_size * 0.55)
                .max(0.0);
            x += advance;
        }

        Some(TextLayout { width: x, glyphs })
    }
}

fn load_native_font() -> Option<FontData> {
    for path in native_font_candidates() {
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        return Some(FontData::new(Blob::from(bytes), 0));
    }
    None
}

fn native_font_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(path) = std::env::var("SEMPAL_NATIVE_FONT_PATH") {
        candidates.push(PathBuf::from(path));
    }

    #[cfg(target_os = "windows")]
    {
        if let Ok(windir) = std::env::var("WINDIR") {
            let base = PathBuf::from(windir).join("Fonts");
            candidates.push(base.join("segoeui.ttf"));
            candidates.push(base.join("arial.ttf"));
            candidates.push(base.join("consola.ttf"));
        }
    }
    #[cfg(target_os = "macos")]
    {
        candidates.push(PathBuf::from("/System/Library/Fonts/SFNS.ttf"));
        candidates.push(PathBuf::from(
            "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
        ));
        candidates.push(PathBuf::from("/Library/Fonts/Arial.ttf"));
    }
    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    {
        candidates.push(PathBuf::from(
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        ));
        candidates.push(PathBuf::from("/usr/share/fonts/dejavu/DejaVuSans.ttf"));
        candidates.push(PathBuf::from("/usr/share/fonts/TTF/DejaVuSans.ttf"));
        candidates.push(PathBuf::from(
            "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
        ));
    }

    candidates
}

fn to_kurbo_rect(rect: UiRect) -> KurboRect {
    KurboRect::new(
        rect.min.x as f64,
        rect.min.y as f64,
        rect.max.x as f64,
        rect.max.y as f64,
    )
}

fn color_from_rgba(color: Rgba8) -> Color {
    Color::from_rgba8(color.r, color.g, color.b, color.a)
}

fn icon_from_rgba(icon: &WindowIconRgba) -> Option<Icon> {
    Icon::from_rgba(icon.rgba.clone(), icon.width, icon.height).ok()
}

#[derive(Default)]
struct PreviewBridge;

impl NativeAppBridge for PreviewBridge {
    fn pull_model(&mut self) -> AppModel {
        AppModel::default()
    }
}

/// Run the native Vello backend window with a host-provided app bridge.
pub fn run_native_vello_app<B: NativeAppBridge>(
    options: NativeRunOptions,
    bridge: B,
) -> Result<(), String> {
    let event_loop = EventLoop::new().map_err(|err| err.to_string())?;
    let mut runner = NativeVelloRunner::new(options, bridge);
    let run_result = event_loop
        .run_app(&mut runner)
        .map_err(|err| err.to_string());
    runner.bridge.on_exit();
    run_result
}

/// Run the experimental native Vello backend window for backend-selection testing.
///
/// This preview path now renders an interactive backend-neutral shell model with
/// Vello primitives and exercises native input hit-testing without `egui`.
pub fn run_native_vello_preview(options: NativeRunOptions) -> Result<(), String> {
    run_native_vello_app(options, PreviewBridge)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{
        BrowserPanelModel, ColumnModel, MapPanelModel, MapPointModel, SourcesPanelModel,
        UpdatePanelModel, UpdateStatusModel, WaveformPanelModel,
    };
    use crate::gui::types::Vector2;
    use winit::event::MouseScrollDelta;

    #[test]
    fn key_bindings_emit_waveform_zoom_actions() {
        let model = AppModel::default();
        assert_eq!(
            action_from_key(KeyCode::OpenBracket, ModifiersState::default(), &model),
            Some(UiAction::ZoomWaveform {
                zoom_in: false,
                steps: 1,
            })
        );
        assert_eq!(
            action_from_key(KeyCode::CloseBracket, ModifiersState::default(), &model),
            Some(UiAction::ZoomWaveform {
                zoom_in: true,
                steps: 1,
            })
        );
        assert_eq!(
            action_from_key(KeyCode::M, ModifiersState::default(), &model),
            Some(UiAction::ZoomWaveformToSelection)
        );
        assert_eq!(
            action_from_key(KeyCode::C, ModifiersState::default(), &model),
            Some(UiAction::ClearWaveformSelection)
        );
        assert_eq!(
            action_from_key(KeyCode::Slash, ModifiersState::default(), &model),
            Some(UiAction::ZoomWaveformFull)
        );
    }

    #[test]
    fn key_bindings_emit_browser_actions() {
        let model = AppModel::default();
        assert_eq!(
            action_from_key(KeyCode::D, ModifiersState::default(), &model),
            Some(UiAction::DeleteBrowserSelection)
        );
        assert_eq!(
            action_from_key(KeyCode::I, ModifiersState::default(), &model),
            Some(UiAction::StartBrowserRename)
        );
        assert_eq!(
            action_from_key(KeyCode::N, ModifiersState::default(), &model),
            Some(UiAction::TagBrowserSelection {
                target: crate::app::BrowserTagTarget::Neutral
            })
        );
        assert_eq!(
            action_from_key(KeyCode::X, ModifiersState::default(), &model),
            Some(UiAction::TagBrowserSelection {
                target: crate::app::BrowserTagTarget::Trash
            })
        );
    }

    #[test]
    fn key_bindings_emit_folder_actions() {
        let model = AppModel::default();
        assert_eq!(
            action_from_key(KeyCode::B, ModifiersState::default(), &model),
            Some(UiAction::StartNewFolder)
        );
        assert_eq!(
            action_from_key(KeyCode::G, ModifiersState::default(), &model),
            Some(UiAction::DeleteFocusedFolder)
        );
        assert_eq!(
            action_from_key(KeyCode::Quote, ModifiersState::default(), &model),
            Some(UiAction::FocusFolderSearch)
        );
        assert_eq!(
            action_from_key(KeyCode::Z, ModifiersState::default(), &model),
            Some(UiAction::StartFolderRename)
        );
    }

    #[test]
    fn prompt_visible_routes_enter_and_cancel_keys() {
        let mut model = AppModel::default();
        model.confirm_prompt.visible = true;
        assert_eq!(
            action_from_key(KeyCode::Enter, ModifiersState::default(), &model),
            Some(UiAction::ConfirmPrompt)
        );
        assert_eq!(
            action_from_key(KeyCode::C, ModifiersState::default(), &model),
            Some(UiAction::CancelPrompt)
        );
        assert_eq!(
            action_from_key(KeyCode::W, ModifiersState::default(), &model),
            None
        );

        model.confirm_prompt.input_error = Some(String::from("Folder already exists"));
        assert_eq!(
            action_from_key(KeyCode::Enter, ModifiersState::default(), &model),
            None
        );
    }

    #[test]
    fn waveform_click_modifiers_route_expected_actions() {
        let layout = ShellLayout::build(Vector2::new(1200.0, 800.0));
        let mut shell_state = NativeShellState::new();
        let point = Point::new(
            layout.waveform_card.min.x + layout.waveform_card.width() * 0.5,
            layout.waveform_card.min.y + layout.waveform_card.height() * 0.5,
        );
        let model = AppModel {
            columns: [
                ColumnModel::new("Trash", 0),
                ColumnModel::new("Neutral", 0),
                ColumnModel::new("Keep", 0),
            ],
            sources: SourcesPanelModel::default(),
            browser: BrowserPanelModel::default(),
            waveform: WaveformPanelModel {
                selection_milli: Some(crate::app::NormalizedRangeModel::new(120, 360)),
                cursor_milli: Some(220),
                playhead_milli: Some(260),
                ..WaveformPanelModel::default()
            },
            ..AppModel::default()
        };

        assert_eq!(
            action_from_pointer(
                &layout,
                &model,
                &mut shell_state,
                point,
                ModifiersState::default(),
            ),
            Some(UiAction::SeekWaveform {
                position_milli: 500
            })
        );

        assert_eq!(
            action_from_pointer(
                &layout,
                &model,
                &mut shell_state,
                point,
                ModifiersState::CONTROL,
            ),
            Some(UiAction::SetWaveformCursor {
                position_milli: 500
            })
        );

        assert_eq!(
            action_from_pointer(
                &layout,
                &model,
                &mut shell_state,
                point,
                ModifiersState::SHIFT,
            ),
            Some(UiAction::SetWaveformSelectionRange {
                start_milli: 120,
                end_milli: 500,
            })
        );
    }

    #[test]
    fn waveform_anchor_prefers_selection_then_cursor_then_playhead() {
        let mut model = AppModel::default();
        assert_eq!(waveform_anchor_milli(&model), 0);

        model.waveform.playhead_milli = Some(333);
        assert_eq!(waveform_anchor_milli(&model), 333);

        model.waveform.cursor_milli = Some(222);
        assert_eq!(waveform_anchor_milli(&model), 222);

        model.waveform.selection_milli = Some(crate::app::NormalizedRangeModel::new(111, 444));
        assert_eq!(waveform_anchor_milli(&model), 111);
    }

    #[test]
    fn browser_tab_clicks_route_to_tab_actions() {
        let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
        let mut shell_state = NativeShellState::new();
        let model = AppModel::default();
        let map_tab_point = Point::new(
            layout.browser_tabs.min.x + (layout.browser_tabs.width() * 0.75),
            layout.browser_tabs.min.y + (layout.browser_tabs.height() * 0.5),
        );
        assert_eq!(
            action_from_pointer(
                &layout,
                &model,
                &mut shell_state,
                map_tab_point,
                ModifiersState::default(),
            ),
            Some(UiAction::SetBrowserTab { map: true })
        );

        let list_tab_point = Point::new(
            layout.browser_tabs.min.x + (layout.browser_tabs.width() * 0.25),
            layout.browser_tabs.min.y + (layout.browser_tabs.height() * 0.5),
        );
        assert_eq!(
            action_from_pointer(
                &layout,
                &model,
                &mut shell_state,
                list_tab_point,
                ModifiersState::default(),
            ),
            Some(UiAction::SetBrowserTab { map: false })
        );
    }

    #[test]
    fn map_point_click_routes_to_focus_map_sample() {
        let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
        let mut shell_state = NativeShellState::new();
        let point = Point::new(
            layout.browser_rows.min.x + (layout.browser_rows.width() * 0.5),
            layout.browser_rows.min.y + (layout.browser_rows.height() * 0.5),
        );
        let model = AppModel {
            map: MapPanelModel {
                active: true,
                summary: String::from("1 point"),
                legend_label: String::from("Render: points"),
                selection_label: String::from("Selection: source::kick.wav"),
                hover_label: String::from("Hover: source::kick.wav"),
                cluster_label: String::from("Clusters: 1"),
                viewport_label: String::from("zoom 1.00x | pan (0, 0)"),
                error: None,
                render_mode: crate::app::MapRenderModeModel::Points,
                points: vec![MapPointModel {
                    sample_id: String::from("source::kick.wav"),
                    x_milli: 500,
                    y_milli: 500,
                    cluster_id: Some(1),
                    selected: true,
                    focused: true,
                }],
            },
            ..AppModel::default()
        };
        assert_eq!(
            action_from_pointer(
                &layout,
                &model,
                &mut shell_state,
                point,
                ModifiersState::default(),
            ),
            Some(UiAction::FocusMapSample {
                sample_id: String::from("source::kick.wav")
            })
        );
    }

    #[test]
    fn update_button_click_routes_update_check_action() {
        let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
        let mut shell_state = NativeShellState::new();
        let model = AppModel {
            update: UpdatePanelModel {
                status: UpdateStatusModel::Idle,
                status_label: String::from("Updates: idle"),
                action_hint_label: String::from("Action: check"),
                release_notes_label: String::new(),
                available_tag: None,
                available_url: None,
                last_error: None,
            },
            ..AppModel::default()
        };
        let button_point = Point::new(
            layout.top_bar_action_cluster.max.x - 18.0,
            layout.top_bar_title_row.min.y + (layout.top_bar_title_row.height() * 0.5),
        );
        assert_eq!(
            action_from_pointer(
                &layout,
                &model,
                &mut shell_state,
                button_point,
                ModifiersState::default(),
            ),
            Some(UiAction::CheckForUpdates)
        );
    }

    #[test]
    fn browser_wheel_delta_is_bounded_and_directional() {
        let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
        let style = StyleTokens::for_viewport_width(layout.root.rect.width());
        let mut model = AppModel::default();
        model.map.active = false;
        let point = Point::new(
            layout.browser_rows.min.x + 10.0,
            layout.browser_rows.min.y + 10.0,
        );

        assert_eq!(
            browser_wheel_row_delta(
                &layout,
                &model,
                point,
                &style,
                MouseScrollDelta::LineDelta(0.0, 3.0),
            ),
            Some(3)
        );
        assert_eq!(
            browser_wheel_row_delta(
                &layout,
                &model,
                point,
                &style,
                MouseScrollDelta::LineDelta(0.0, 0.0)
            ),
            None
        );
    }
}

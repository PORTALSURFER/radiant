//! Generic `RuntimeBridge` native Vello runner.

use super::*;

struct GenericSharedPixelBytes(Arc<[u8]>);

impl AsRef<[u8]> for GenericSharedPixelBytes {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

/// Run a generic [`RuntimeBridge`] through the native Vello backend.
///
/// This entrypoint is intentionally narrower than the compatibility
/// shell runner: it renders public `UiSurface` paint primitives, routes pointer
/// and keyboard input into projected widget ids, reduces host-defined messages,
/// and requests redraws when the surface changes.
pub fn run_native_vello_runtime<Bridge, Message>(
    options: NativeRunOptions,
    bridge: Bridge,
) -> Result<(), String>
where
    Bridge: RuntimeBridge<Message> + 'static,
    Message: 'static,
{
    run_native_vello_runtime_with_artifacts(options, bridge).result
}

/// Run a generic [`RuntimeBridge`] through native Vello and return runtime artifacts.
pub fn run_native_vello_runtime_with_artifacts<Bridge, Message>(
    options: NativeRunOptions,
    bridge: Bridge,
) -> NativeGenericRunReport
where
    Bridge: RuntimeBridge<Message> + 'static,
    Message: 'static,
{
    info!("radiant generic native vello: creating event loop");
    let run_started = Instant::now();
    let event_loop = match EventLoop::<RuntimeUserEvent>::with_user_event().build() {
        Ok(event_loop) => event_loop,
        Err(err) => {
            return NativeGenericRunReport {
                artifacts: NativeGenericRuntimeArtifacts::default(),
                result: Err(err.to_string()),
            };
        }
    };
    let viewport = initial_viewport(&options);
    let mut runner = GenericNativeVelloRunner::new(options, bridge, viewport);
    let run_result = event_loop
        .run_app(&mut runner)
        .map_err(|err| err.to_string());
    let elapsed = run_started.elapsed();
    match &run_result {
        Ok(_) => info!(
            "radiant generic native vello: event loop ended in {} ms",
            elapsed.as_millis()
        ),
        Err(err) => warn!(
            "radiant generic native vello: event loop returned error in {} ms: {}",
            elapsed.as_millis(),
            err
        ),
    }
    NativeGenericRunReport {
        artifacts: NativeGenericRuntimeArtifacts {
            startup_timing: runner.startup_timing.export_artifact(),
        },
        result: run_result,
    }
}

/// Structured runtime artifacts exported after one generic native run completes.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct NativeGenericRuntimeArtifacts {
    /// Native startup timing artifact captured for this run, when startup began.
    pub startup_timing: Option<NativeStartupTimingArtifact>,
}

/// Result plus structured artifacts returned by one generic native runtime execution.
pub type NativeGenericRunReport =
    crate::gui_runtime::RuntimeRunReport<NativeGenericRuntimeArtifacts>;

fn initial_viewport(options: &NativeRunOptions) -> Vector2 {
    let [width, height] = options.inner_size.unwrap_or([1280.0, 720.0]);
    Vector2::new(width.max(1.0), height.max(1.0))
}

pub(in crate::gui_runtime::native_vello) struct GenericNativeRuntimeCore<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(in crate::gui_runtime::native_vello) runtime: SurfaceRuntime<Bridge, Message>,
    theme: ThemeTokens,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(in crate::gui_runtime::native_vello) struct GenericRouteOutcome {
    pub(in crate::gui_runtime::native_vello) routed: bool,
    pub(in crate::gui_runtime::native_vello) repaint_requested: bool,
}

impl GenericRouteOutcome {
    fn needs_redraw(self) -> bool {
        self.routed || self.repaint_requested
    }
}

impl<Bridge, Message> GenericNativeRuntimeCore<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(in crate::gui_runtime::native_vello) fn new(bridge: Bridge, viewport: Vector2) -> Self {
        Self {
            runtime: SurfaceRuntime::new(bridge, viewport),
            theme: ThemeTokens::default(),
        }
    }

    fn set_viewport(&mut self, viewport: Vector2) {
        let _ = self
            .runtime
            .dispatch_event(crate::runtime::Event::Resize { viewport });
    }

    fn paint_plan(&self) -> crate::runtime::SurfacePaintPlan {
        self.runtime.paint_plan(&self.theme)
    }

    fn route_outcome(&mut self, routed: bool) -> GenericRouteOutcome {
        GenericRouteOutcome {
            routed,
            repaint_requested: self.runtime.take_repaint_requested(),
        }
    }

    pub(in crate::gui_runtime::native_vello) fn route_pointer_move(
        &mut self,
        position: Point,
    ) -> GenericRouteOutcome {
        let routed = self
            .runtime
            .dispatch_event(crate::runtime::Event::PointerMove { position })
            .is_some();
        self.route_outcome(routed)
    }

    pub(in crate::gui_runtime::native_vello) fn route_pointer_press(
        &mut self,
        position: Point,
        button: PointerButton,
    ) -> GenericRouteOutcome {
        let routed = self
            .runtime
            .dispatch_event(crate::runtime::Event::PointerPress { position, button })
            .is_some();
        self.route_outcome(routed)
    }

    pub(in crate::gui_runtime::native_vello) fn route_pointer_release(
        &mut self,
        position: Point,
        button: PointerButton,
    ) -> GenericRouteOutcome {
        let routed = self
            .runtime
            .dispatch_event(crate::runtime::Event::PointerRelease { position, button })
            .is_some();
        self.route_outcome(routed)
    }

    pub(in crate::gui_runtime::native_vello) fn route_key_press(
        &mut self,
        key: WidgetKey,
    ) -> GenericRouteOutcome {
        let routed = self
            .runtime
            .dispatch_event(crate::runtime::Event::KeyPress(key))
            .is_some();
        self.route_outcome(routed)
    }

    pub(in crate::gui_runtime::native_vello) fn route_character(
        &mut self,
        character: char,
    ) -> GenericRouteOutcome {
        let routed = self
            .runtime
            .dispatch_event(crate::runtime::Event::Character(character))
            .is_some();
        self.route_outcome(routed)
    }
}

struct GenericNativeVelloRunner<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    options: NativeRunOptions,
    core: GenericNativeRuntimeCore<Bridge, Message>,
    window_id: Option<WindowId>,
    window: Option<Arc<Window>>,
    render_ctx: Option<RenderContext>,
    render_surface: Option<RenderSurface<'static>>,
    renderer: Option<Renderer>,
    text_renderer: NativeTextRenderer,
    scene: Scene,
    last_cursor: Option<Point>,
    redraw_requested: bool,
    startup_timing: StartupTimingProfile,
    first_frame_presented: bool,
    last_redraw: Instant,
}

impl<Bridge, Message> GenericNativeVelloRunner<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    fn new(options: NativeRunOptions, bridge: Bridge, viewport: Vector2) -> Self {
        Self {
            options,
            core: GenericNativeRuntimeCore::new(bridge, viewport),
            window_id: None,
            window: None,
            render_ctx: None,
            render_surface: None,
            renderer: None,
            text_renderer: NativeTextRenderer::new(),
            scene: Scene::new(),
            last_cursor: None,
            redraw_requested: false,
            startup_timing: StartupTimingProfile::new(),
            first_frame_presented: false,
            last_redraw: Instant::now(),
        }
    }

    fn initialize_runtime(&mut self, event_loop: &ActiveEventLoop) {
        info!("radiant generic native vello: initializing runtime window and surface");
        self.startup_timing.mark_init_started();
        let window = match event_loop.create_window(generic_window_attributes(&self.options)) {
            Ok(window) => Arc::new(window),
            Err(err) => {
                error!(
                    "radiant generic native vello: failed to create window: {:?}",
                    err
                );
                event_loop.exit();
                return;
            }
        };
        self.startup_timing.mark_window_created();
        self.window_id = Some(window.id());
        self.window = Some(Arc::clone(&window));

        let mut render_ctx = RenderContext::new();
        let size = window.inner_size();
        let width = size.width.max(1);
        let height = size.height.max(1);
        self.core
            .set_viewport(Vector2::new(width as f32, height as f32));
        let surface = match render_ctx.instance.create_surface(window.clone()) {
            Ok(surface) => surface,
            Err(err) => {
                error!(
                    "radiant generic native vello: failed to create wgpu surface: {:?}",
                    err
                );
                event_loop.exit();
                return;
            }
        };
        self.startup_timing.mark_wgpu_surface_created();
        let Some(dev_id) = pollster::block_on(render_ctx.device(Some(&surface))) else {
            error!("radiant generic native vello: no compatible render device found");
            event_loop.exit();
            return;
        };
        self.startup_timing.mark_wgpu_device_ready();
        let supported_present_modes = surface
            .get_capabilities(render_ctx.devices[dev_id].adapter())
            .present_modes;
        let present_mode = select_present_mode(self.options.target_fps, &supported_present_modes);
        let render_surface = match pollster::block_on(render_ctx.create_render_surface(
            surface,
            width,
            height,
            present_mode,
        )) {
            Ok(render_surface) => render_surface,
            Err(err) => {
                error!(
                    "radiant generic native vello: failed to create render surface: {:?}",
                    err
                );
                event_loop.exit();
                return;
            }
        };
        self.startup_timing.mark_surface_ready();
        let dev_handle = &render_ctx.devices[render_surface.dev_id];
        self.startup_timing.mark_renderer_started();
        let renderer = match Renderer::new(&dev_handle.device, startup_renderer_options()) {
            Ok(renderer) => renderer,
            Err(err) => {
                error!(
                    "radiant generic native vello: failed to create renderer: {:?}",
                    err
                );
                event_loop.exit();
                return;
            }
        };
        self.startup_timing.mark_renderer_ready();
        self.render_ctx = Some(render_ctx);
        self.render_surface = Some(render_surface);
        self.renderer = Some(renderer);
        self.rebuild_scene();
        self.startup_timing.mark_first_scene_ready();
        window.set_visible(true);
        self.startup_timing.mark_window_revealed();
        self.last_redraw = Instant::now();
        self.request_redraw_if_needed();
    }

    fn request_redraw_if_needed(&mut self) {
        if self.redraw_requested {
            return;
        }
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
            self.redraw_requested = true;
        }
    }

    fn rebuild_scene(&mut self) {
        let plan = self.core.paint_plan();
        encode_surface_paint_plan_to_scene(&plan, &mut self.scene, &mut self.text_renderer);
    }

    fn resize_surface(&mut self, size: winit::dpi::PhysicalSize<u32>) {
        if size.width == 0 || size.height == 0 {
            return;
        }
        if let (Some(render_ctx), Some(surface)) =
            (self.render_ctx.as_ref(), self.render_surface.as_mut())
        {
            render_ctx.resize_surface(surface, size.width, size.height);
            self.core
                .set_viewport(Vector2::new(size.width as f32, size.height as f32));
            self.rebuild_scene();
            self.request_redraw_if_needed();
        }
    }

    fn handle_route_outcome(&mut self, outcome: GenericRouteOutcome) {
        if outcome.needs_redraw() {
            self.rebuild_scene();
            self.request_redraw_if_needed();
        }
    }

    fn handle_keyboard_event(&mut self, event: winit::event::KeyEvent) {
        if event.state != ElementState::Pressed || event.repeat {
            return;
        }
        let mut route_outcome = GenericRouteOutcome::default();
        if let Some(text) = event.text.as_ref() {
            for character in text.chars().filter(|character| !character.is_control()) {
                let outcome = self.core.route_character(character);
                route_outcome.routed |= outcome.routed;
                route_outcome.repaint_requested |= outcome.repaint_requested;
            }
        }
        if matches!(event.logical_key, Key::Named(NamedKey::Backspace)) {
            let outcome = self.core.route_key_press(WidgetKey::Backspace);
            route_outcome.routed |= outcome.routed;
            route_outcome.repaint_requested |= outcome.repaint_requested;
        }
        if matches!(event.logical_key, Key::Named(NamedKey::Delete)) {
            let outcome = self.core.route_key_press(WidgetKey::Delete);
            route_outcome.routed |= outcome.routed;
            route_outcome.repaint_requested |= outcome.repaint_requested;
        }
        if let PhysicalKey::Code(code) = event.physical_key
            && let Some(key) = key_code_from_winit(code).and_then(WidgetKey::from_key_code)
        {
            let outcome = self.core.route_key_press(key);
            route_outcome.routed |= outcome.routed;
            route_outcome.repaint_requested |= outcome.repaint_requested;
        }
        self.handle_route_outcome(route_outcome);
    }

    fn redraw(&mut self, event_loop: &ActiveEventLoop) {
        self.redraw_requested = false;
        let Some(window) = self.window.as_ref() else {
            return;
        };
        let Some(dev_id) = self.render_surface.as_ref().map(|surface| surface.dev_id) else {
            return;
        };
        let surface_texture = {
            let Some(surface) = self.render_surface.as_mut() else {
                return;
            };
            match surface.surface.get_current_texture() {
                Ok(frame) => frame,
                Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                    let size = window.inner_size();
                    self.resize_surface(size);
                    return;
                }
                Err(wgpu::SurfaceError::OutOfMemory) => {
                    error!("radiant generic native vello: out of memory acquiring surface");
                    event_loop.exit();
                    return;
                }
                Err(err) => {
                    warn!(
                        "radiant generic native vello: non-fatal surface acquire error: {:?}",
                        err
                    );
                    return;
                }
            }
        };
        let Some(surface) = self.render_surface.as_mut() else {
            return;
        };
        let Some(render_ctx) = self.render_ctx.as_ref() else {
            return;
        };
        let Some(renderer) = self.renderer.as_mut() else {
            return;
        };
        let dev_handle = &render_ctx.devices[dev_id];
        let surface_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let plan = self.core.paint_plan();
        let render_result = renderer.render_to_texture(
            &dev_handle.device,
            &dev_handle.queue,
            &self.scene,
            &surface.target_view,
            &RenderParams {
                base_color: color_from_rgba(plan.clear_color),
                width: surface.config.width,
                height: surface.config.height,
                antialiasing_method: AaConfig::Area,
            },
        );
        if let Err(err) = render_result {
            error!(
                "radiant generic native vello: render_to_texture failed: {:?}",
                err
            );
            event_loop.exit();
            return;
        }
        let mut encoder =
            dev_handle
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("generic_native_vello_present_blit"),
                });
        surface.blitter.copy(
            &dev_handle.device,
            &mut encoder,
            &surface.target_view,
            &surface_view,
        );
        dev_handle.queue.submit(std::iter::once(encoder.finish()));
        surface_texture.present();
        if !self.first_frame_presented {
            self.first_frame_presented = true;
            self.startup_timing.mark_first_presented();
            self.startup_timing.maybe_emit_summary();
        }
    }
}

impl<Bridge, Message> ApplicationHandler<RuntimeUserEvent>
    for GenericNativeVelloRunner<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            self.initialize_runtime(event_loop);
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
            WindowEvent::Resized(size) => self.resize_surface(size),
            WindowEvent::ScaleFactorChanged { .. } => self.request_redraw_if_needed(),
            WindowEvent::CursorMoved { position, .. } => {
                let position = Point::new(position.x as f32, position.y as f32);
                self.last_cursor = Some(position);
                let outcome = self.core.route_pointer_move(position);
                self.handle_route_outcome(outcome);
            }
            WindowEvent::CursorLeft { .. } => {
                self.last_cursor = None;
            }
            WindowEvent::MouseInput { button, state, .. } => {
                let Some(position) = self.last_cursor else {
                    return;
                };
                let Some(button) = pointer_button_from_winit(button) else {
                    return;
                };
                let routed = match state {
                    ElementState::Pressed => self.core.route_pointer_press(position, button),
                    ElementState::Released => self.core.route_pointer_release(position, button),
                };
                self.handle_route_outcome(routed);
            }
            WindowEvent::KeyboardInput { event, .. } => self.handle_keyboard_event(event),
            WindowEvent::RedrawRequested => self.redraw(event_loop),
            _ => {}
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: RuntimeUserEvent) {
        match event {
            RuntimeUserEvent::RepaintRequested => {
                self.rebuild_scene();
                self.request_redraw_if_needed();
            }
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        event_loop.set_control_flow(ControlFlow::Wait);
    }
}

fn pointer_button_from_winit(button: MouseButton) -> Option<PointerButton> {
    Some(match button {
        MouseButton::Left => PointerButton::Primary,
        MouseButton::Right => PointerButton::Secondary,
        MouseButton::Middle => PointerButton::Auxiliary,
        _ => return None,
    })
}

fn generic_window_attributes(options: &NativeRunOptions) -> WindowAttributes {
    let mut attrs = Window::default_attributes()
        .with_title(options.title.clone())
        .with_maximized(options.maximized)
        .with_decorations(options.decorations);
    if let Some([w, h]) = options.inner_size {
        attrs = attrs.with_inner_size(Size::Logical(LogicalSize::new(w as f64, h as f64)));
    }
    if let Some([w, h]) = options.min_inner_size {
        attrs = attrs.with_min_inner_size(Size::Logical(LogicalSize::new(w as f64, h as f64)));
    }
    if let Some(icon) = options.icon.as_ref().and_then(icon_from_rgba) {
        attrs = attrs.with_window_icon(Some(icon));
    }
    #[cfg(target_os = "windows")]
    {
        use winit::platform::windows::WindowAttributesExtWindows;
        attrs = attrs.with_drag_and_drop(true);
    }
    attrs
}

pub(in crate::gui_runtime::native_vello) fn encode_surface_paint_plan_to_scene(
    plan: &crate::runtime::SurfacePaintPlan,
    scene: &mut Scene,
    text_renderer: &mut NativeTextRenderer,
) {
    scene.reset();
    let mut text_runs = Vec::new();
    for primitive in &plan.primitives {
        match primitive {
            PaintPrimitive::FillRect(fill) => {
                scene.fill(
                    Fill::NonZero,
                    Affine::IDENTITY,
                    color_from_rgba(fill.color),
                    None,
                    &to_kurbo_rect(fill.rect),
                );
            }
            PaintPrimitive::StrokeRect(stroke) => {
                scene.stroke(
                    &vello::kurbo::Stroke::new(stroke.width as f64),
                    Affine::IDENTITY,
                    color_from_rgba(stroke.color),
                    None,
                    &to_kurbo_rect(stroke.rect),
                );
            }
            PaintPrimitive::Text(text) => {
                let align = match text.align {
                    PaintTextAlign::Left => TextAlign::Left,
                    PaintTextAlign::Center => TextAlign::Center,
                    PaintTextAlign::Right => TextAlign::Right,
                };
                let baseline_offset = text.baseline.unwrap_or(text.font_size);
                text_runs.push(TextRun {
                    text: text.text.clone(),
                    position: Point::new(
                        text.rect.min.x,
                        text.rect.min.y + baseline_offset - text.font_size,
                    ),
                    font_size: text.font_size,
                    color: text.color,
                    max_width: Some(text.rect.width().max(0.0)),
                    align,
                });
            }
            PaintPrimitive::Image(draw) => {
                let (Ok(width), Ok(height)) = (
                    u32::try_from(draw.image.width),
                    u32::try_from(draw.image.height),
                ) else {
                    continue;
                };
                if width == 0
                    || height == 0
                    || draw.rect.width() <= 0.0
                    || draw.rect.height() <= 0.0
                {
                    continue;
                }
                let image_data = ImageData {
                    data: Blob::new(Arc::new(GenericSharedPixelBytes(Arc::clone(
                        &draw.image.pixels,
                    )))),
                    format: ImageFormat::Rgba8,
                    alpha_type: ImageAlphaType::Alpha,
                    width,
                    height,
                };
                let transform = Affine::translate((draw.rect.min.x as f64, draw.rect.min.y as f64))
                    * Affine::scale_non_uniform(
                        draw.rect.width() as f64 / f64::from(width),
                        draw.rect.height() as f64 / f64::from(height),
                    );
                scene.draw_image(&image_data, transform);
            }
            PaintPrimitive::CustomSurface(custom) => {
                scene.stroke(
                    &vello::kurbo::Stroke::new(1.0),
                    Affine::IDENTITY,
                    color_from_rgba(Rgba8 {
                        r: 96,
                        g: 96,
                        b: 96,
                        a: 255,
                    }),
                    None,
                    &to_kurbo_rect(custom.rect),
                );
            }
        }
    }
    text_renderer.draw_text_runs(scene, &text_runs);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        layout::{ContainerKind, ContainerPolicy, SlotParams},
        runtime::{Command, SurfaceChild, SurfaceNode, UiSurface, WidgetMessageMapper},
        widgets::{ButtonWidget, TextInputMessage, TextInputWidget, WidgetSizing, WidgetSpec},
    };

    #[derive(Clone, Debug, PartialEq, Eq)]
    enum DemoMessage {
        Increment,
        Rename(String),
    }

    #[derive(Default)]
    struct DemoState {
        count: usize,
        name: String,
    }

    #[test]
    fn generic_core_routes_pointer_and_key_input_to_host_messages() {
        let bridge = demo_bridge();
        let mut core = GenericNativeRuntimeCore::new(bridge, Vector2::new(320.0, 40.0));
        let button_point = core
            .runtime
            .layout()
            .rects
            .get(&11)
            .map(|rect| Point::new(rect.min.x + 2.0, rect.min.y + 2.0))
            .expect("button should be laid out");

        assert!(
            core.route_pointer_press(button_point, PointerButton::Primary)
                .routed
        );
        assert!(
            core.route_pointer_release(button_point, PointerButton::Primary)
                .routed
        );
        assert_eq!(core.runtime.bridge().state.count, 1);

        let input_point = core
            .runtime
            .layout()
            .rects
            .get(&12)
            .map(|rect| Point::new(rect.min.x + 2.0, rect.min.y + 2.0))
            .expect("text input should be laid out");
        assert!(
            core.route_pointer_press(input_point, PointerButton::Primary)
                .routed
        );
        assert!(core.route_character('R').routed);
        assert!(core.route_key_press(WidgetKey::Enter).routed);
        assert_eq!(core.runtime.bridge().state.name, "R");
    }

    #[test]
    fn generic_core_drains_command_repaint_requests_after_routing() {
        let bridge = RepaintBridge::default();
        let mut core = GenericNativeRuntimeCore::new(bridge, Vector2::new(320.0, 40.0));
        let button_point = core
            .runtime
            .layout()
            .rects
            .get(&11)
            .map(|rect| Point::new(rect.min.x + 2.0, rect.min.y + 2.0))
            .expect("button should be laid out");

        assert!(
            core.route_pointer_press(button_point, PointerButton::Primary)
                .routed
        );
        let outcome = core.route_pointer_release(button_point, PointerButton::Primary);

        assert!(outcome.routed);
        assert!(outcome.repaint_requested);
        assert!(!core.runtime.repaint_requested());
        assert_eq!(core.runtime.bridge().state.count, 1);
    }

    #[test]
    fn generic_paint_plan_encodes_to_vello_scene() {
        let bridge = demo_bridge();
        let core = GenericNativeRuntimeCore::new(bridge, Vector2::new(320.0, 40.0));
        let mut scene = Scene::new();
        let mut text_renderer = NativeTextRenderer::new();

        encode_surface_paint_plan_to_scene(&core.paint_plan(), &mut scene, &mut text_renderer);
    }

    #[derive(Default)]
    struct DemoBridge {
        state: DemoState,
    }

    impl RuntimeBridge<DemoMessage> for DemoBridge {
        fn project_surface(&mut self) -> Arc<UiSurface<DemoMessage>> {
            demo_surface(&self.state)
        }

        fn reduce_message(&mut self, message: DemoMessage) {
            match message {
                DemoMessage::Increment => self.state.count += 1,
                DemoMessage::Rename(name) => self.state.name = name,
            }
        }
    }

    fn demo_bridge() -> DemoBridge {
        DemoBridge::default()
    }

    #[derive(Default)]
    struct RepaintBridge {
        state: DemoState,
    }

    impl RuntimeBridge<DemoMessage> for RepaintBridge {
        fn project_surface(&mut self) -> Arc<UiSurface<DemoMessage>> {
            demo_surface(&self.state)
        }

        fn update(&mut self, message: DemoMessage) -> Command<DemoMessage> {
            match message {
                DemoMessage::Increment => {
                    self.state.count += 1;
                    Command::request_repaint()
                }
                DemoMessage::Rename(name) => {
                    self.state.name = name;
                    Command::none()
                }
            }
        }
    }

    fn demo_surface(state: &DemoState) -> Arc<UiSurface<DemoMessage>> {
        let button = WidgetSpec::Button(ButtonWidget::new(
            11,
            "Increment",
            WidgetSizing::fixed(Vector2::new(96.0, 28.0)),
        ));
        let input = WidgetSpec::TextInput(TextInputWidget::new(
            12,
            state.name.clone(),
            WidgetSizing::fixed(Vector2::new(120.0, 28.0)),
        ));
        Arc::new(UiSurface::new(SurfaceNode::container(
            1,
            ContainerPolicy {
                kind: ContainerKind::Row,
                spacing: 8.0,
                ..ContainerPolicy::default()
            },
            vec![
                SurfaceChild::new(
                    SlotParams::fill(),
                    SurfaceNode::widget(
                        button,
                        WidgetMessageMapper::button(|_| DemoMessage::Increment),
                    ),
                ),
                SurfaceChild::new(
                    SlotParams::fill(),
                    SurfaceNode::widget(
                        input,
                        WidgetMessageMapper::text_input(|message| match message {
                            TextInputMessage::Changed { value }
                            | TextInputMessage::Submitted { value } => DemoMessage::Rename(value),
                        }),
                    ),
                ),
            ],
        )))
    }
}

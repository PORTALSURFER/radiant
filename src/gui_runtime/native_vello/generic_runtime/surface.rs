//! Window, surface, and renderer setup for the generic native Vello runner.

use super::*;

impl<Bridge, Message> GenericNativeVelloRunner<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(super) fn initialize_runtime(&mut self, event_loop: &ActiveEventLoop) {
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

    pub(super) fn resize_surface(&mut self, size: winit::dpi::PhysicalSize<u32>) {
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
}

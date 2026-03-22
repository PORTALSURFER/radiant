use super::super::*;

impl<B: NativeAppBridge> NativeVelloRunner<B> {
    pub(in crate::gui_runtime::native_vello) fn new(options: NativeRunOptions, bridge: B) -> Self {
        let target_fps = options.target_fps.max(1);
        let frame_interval_ns = (1_000_000_000u64 / target_fps as u64).max(1);
        let target_frame_interval = Duration::from_nanos(frame_interval_ns);
        let focus_animation_interval =
            Duration::from_nanos((1_000_000_000u64 / FOCUS_PULSE_HZ).max(1));
        let idle_status_refresh_interval =
            Duration::from_nanos(1_000_000_000u64 / IDLE_STATUS_REFRESH_HZ.max(1));
        let cursor_activity_redraw_interval =
            Duration::from_nanos(1_000_000_000u64 / CURSOR_ACTIVITY_REDRAW_HZ.max(1));
        let startup_clear_color = Self::startup_placeholder_clear_color();
        let incremental_frame_pipeline =
            crate::env_flags::env_var_truthy(INCREMENTAL_FRAME_PIPELINE_ENV);
        info!(
            "radiant native vello runner created: title={} target_fps={} maximized={} has_icon={}",
            options.title,
            options.target_fps,
            options.maximized,
            options.icon.is_some()
        );
        Self {
            options,
            bridge,
            repaint_event_pending: Arc::new(AtomicBool::new(false)),
            incremental_frame_pipeline,
            model: Arc::new(AppModel::default()),
            window_id: None,
            window: None,
            render_ctx: None,
            render_surface: None,
            renderer: None,
            redraw_requested: false,
            frame_cache: NativeViewFrame {
                clear_color: startup_clear_color,
                primitives: Vec::new(),
                text_runs: Vec::new(),
            },
            static_segment_frame_cache: StaticFrameSegments::default(),
            static_segment_graph: StaticSegmentStateGraph::default(),
            static_segment_scene_cache: StaticSegmentSceneCache::default(),
            state_overlay_frame_cache: NativeViewFrame {
                clear_color: startup_clear_color,
                primitives: Vec::new(),
                text_runs: Vec::new(),
            },
            waveform_motion_overlay_frame_cache: NativeViewFrame {
                clear_color: startup_clear_color,
                primitives: Vec::new(),
                text_runs: Vec::new(),
            },
            chrome_motion_overlay_frame_cache: NativeViewFrame {
                clear_color: startup_clear_color,
                primitives: Vec::new(),
                text_runs: Vec::new(),
            },
            scene: Scene::new(),
            static_scene: Scene::new(),
            state_overlay_scene: Scene::new(),
            waveform_motion_overlay_scene: Scene::new(),
            chrome_motion_overlay_scene: Scene::new(),
            image_upload_blob_cache: HashMap::new(),
            image_upload_blob_cache_order: VecDeque::new(),
            state_overlay_fingerprint: None,
            waveform_motion_overlay_fingerprint: None,
            chrome_motion_overlay_fingerprint: None,
            motion_model: None,
            motion_model_supported: true,
            segment_revisions: SegmentRevisions::default(),
            segment_revisions_supported: false,
            missing_segment_revision_fallback_applied: false,
            text_renderer: NativeTextRenderer::new(),
            style_cache: None,
            frame_state: NativeVelloFrameState {
                model_dirty: true,
                ..NativeVelloFrameState::default()
            },
            layout_runtime: ShellLayoutRuntime::default(),
            shell_layout: None,
            shell_state: NativeShellState::new(),
            clear_color: startup_clear_color,
            cursor_icon: CursorIcon::Default,
            last_cursor: None,
            pending_cursor: None,
            pending_volume_milli: None,
            waveform_drag_mode: None,
            waveform_view_refresh_pending: false,
            pending_hotkey_prefix: None,
            clear_playback_selection_on_click_release: false,
            selection_drag_active: false,
            last_emitted_waveform_drag_action: None,
            map_focus_drag_active: false,
            last_emitted_map_drag_sample_id: None,
            browser_scrollbar_drag: None,
            last_emitted_browser_view_start: None,
            waveform_scrollbar_drag: None,
            waveform_pan_drag: None,
            last_emitted_waveform_view_center: None,
            volume_drag_active: false,
            last_emitted_volume_milli: None,
            modifiers: ModifiersState::default(),
            text_input_target: TextInputTarget::None,
            text_input_buffer: None,
            text_editor_state: None,
            text_input_drag_active: false,
            waveform_bpm_input_buffer: None,
            clipboard: None,
            clipboard_fallback_text: String::new(),
            last_redraw: Instant::now(),
            resumed_count: 0,
            window_event_count: 0,
            redraw_count: 0,
            first_frame_presented: false,
            startup_window_visible: false,
            startup_model_pull_pending: Self::startup_should_defer_first_model_pull(),
            startup_deferred_model_refresh_pending: false,
            startup_reveal_deadline: None,
            startup_timing: StartupTimingProfile::new(),
            target_frame_interval,
            focus_animation_interval,
            idle_status_refresh_interval,
            next_idle_status_refresh: Instant::now() + idle_status_refresh_interval,
            cursor_activity_redraw_interval,
            cursor_activity_redraw_until: None,
            model_refresh_count: 0,
            profiler: NativeVelloProfiler::new(),
        }
    }

    pub(in crate::gui_runtime::native_vello) fn initialize_runtime(
        &mut self,
        event_loop: &ActiveEventLoop,
    ) {
        info!("radiant native vello: initializing runtime window and surface");
        self.startup_timing.mark_init_started();
        let Some(window) = self.create_startup_window(event_loop) else {
            return;
        };

        let mut render_ctx = RenderContext::new();
        let size = window.inner_size();
        let width = size.width.max(1);
        let height = size.height.max(1);
        info!(
            "radiant native vello: creating render surface with {}x{}",
            width, height
        );
        let present_mode_candidates = present_mode_candidates(self.options.target_fps);
        let mut create_surface_with_mode = |present_mode| {
            std::panic::catch_unwind(AssertUnwindSafe(|| {
                pollster::block_on(render_ctx.create_surface(
                    window.clone(),
                    width,
                    height,
                    present_mode,
                ))
            }))
        };
        let mut render_surface = None;
        for (index, present_mode) in present_mode_candidates.iter().copied().enumerate() {
            let last_attempt = index + 1 == present_mode_candidates.len();
            match create_surface_with_mode(present_mode) {
                Ok(Ok(surface)) => {
                    if index == 0 {
                        info!(
                            "radiant native vello: render surface created using {:?}",
                            present_mode
                        );
                    } else {
                        info!(
                            "radiant native vello: render surface created using {:?} fallback",
                            present_mode
                        );
                    }
                    render_surface = Some(surface);
                    break;
                }
                Ok(Err(err)) => {
                    if last_attempt {
                        error!(
                            "radiant native vello: failed to create {:?} surface: {:?}",
                            present_mode, err
                        );
                        event_loop.exit();
                        return;
                    }
                    let next_mode = present_mode_candidates[index + 1];
                    warn!(
                        "radiant native vello: {:?} surface creation failed (error): {:?}; retrying {:?}",
                        present_mode, err, next_mode
                    );
                }
                Err(_) => {
                    if last_attempt {
                        error!(
                            "radiant native vello: {:?} surface creation panicked during startup",
                            present_mode
                        );
                        event_loop.exit();
                        return;
                    }
                    let next_mode = present_mode_candidates[index + 1];
                    warn!(
                        "radiant native vello: {:?} surface creation panicked; retrying {:?}",
                        present_mode, next_mode
                    );
                }
            }
        }
        let Some(render_surface) = render_surface else {
            event_loop.exit();
            return;
        };
        self.startup_timing.mark_surface_ready();
        info!("radiant native vello: render surface created");
        let Some(renderer) = self.create_renderer(event_loop, &render_ctx, &render_surface) else {
            return;
        };

        self.window_id = Some(window.id());
        self.window = Some(window);
        self.render_ctx = Some(render_ctx);
        self.render_surface = Some(render_surface);
        self.renderer = Some(renderer);
        self.frame_state.mark_layout_dirty();
        if self.startup_model_pull_pending {
            self.prepare_startup_first_frame_scene();
        } else {
            self.frame_state.mark_model_dirty();
        }
        self.rebuild_scene_if_needed();
        self.startup_timing.mark_first_scene_ready();
        self.maybe_reveal_startup_window_after_first_scene_ready();
        self.last_redraw = Instant::now();
    }

    fn create_startup_window(&mut self, event_loop: &ActiveEventLoop) -> Option<Arc<Window>> {
        let window = match event_loop.create_window(self.build_window_attributes()) {
            Ok(window) => Arc::new(window),
            Err(err) => {
                error!("radiant native vello: failed to create window: {:?}", err);
                event_loop.exit();
                return None;
            }
        };
        self.startup_timing.mark_window_created();
        info!("radiant native vello: window created");
        self.arm_startup_reveal_deadline(Instant::now());
        Some(window)
    }

    fn create_renderer(
        &mut self,
        event_loop: &ActiveEventLoop,
        render_ctx: &RenderContext,
        render_surface: &RenderSurface<'_>,
    ) -> Option<Renderer> {
        let dev_handle = &render_ctx.devices[render_surface.dev_id];
        info!("radiant native vello: creating renderer");
        let renderer = match Renderer::new(&dev_handle.device, RendererOptions::default()) {
            Ok(renderer) => renderer,
            Err(err) => {
                error!("radiant native vello: failed to create renderer: {:?}", err);
                event_loop.exit();
                return None;
            }
        };
        self.startup_timing.mark_renderer_ready();
        info!("radiant native vello: renderer created");
        Some(renderer)
    }
}

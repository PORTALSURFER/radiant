//! Startup and window lifecycle helpers for the native Vello runtime.

use super::*;

impl<B: NativeAppBridge> NativeVelloRunner<B> {
    /// Keep the native window hidden until startup sequencing decides reveal timing.
    pub(super) fn startup_should_launch_hidden() -> bool {
        true
    }

    /// Use eager full-scene startup by default and reserve deferred placeholder
    /// startup for explicit fallback paths only.
    pub(super) fn startup_should_defer_first_model_pull() -> bool {
        false
    }

    /// Resolve a deterministic startup clear color used before style/layout are ready.
    pub(super) fn startup_placeholder_clear_color() -> Rgba8 {
        StyleTokens::for_viewport_width(1280.0).clear_color
    }

    pub(super) fn new(options: NativeRunOptions, bridge: B) -> Self {
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

    pub(super) fn ui_scale_factor(&self) -> f32 {
        self.window
            .as_ref()
            .map(|window| {
                let scale = window.scale_factor() as f32;
                scale.clamp(1.0, 3.0)
            })
            .unwrap_or(1.0)
    }

    pub(super) fn build_window_attributes(&self) -> WindowAttributes {
        let mut attrs = Window::default_attributes()
            .with_title(self.options.title.clone())
            .with_maximized(self.options.maximized)
            .with_decorations(self.options.decorations)
            .with_visible(!Self::startup_should_launch_hidden());
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

    pub(super) fn initialize_runtime(&mut self, event_loop: &ActiveEventLoop) {
        info!("radiant native vello: initializing runtime window and surface");
        self.startup_timing.mark_init_started();
        let window = match event_loop.create_window(self.build_window_attributes()) {
            Ok(window) => Arc::new(window),
            Err(err) => {
                error!("radiant native vello: failed to create window: {:?}", err);
                event_loop.exit();
                return;
            }
        };
        self.startup_timing.mark_window_created();
        info!("radiant native vello: window created");
        self.arm_startup_reveal_deadline(Instant::now());
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
        let dev_handle = &render_ctx.devices[render_surface.dev_id];
        info!("radiant native vello: creating renderer");
        let renderer = match Renderer::new(&dev_handle.device, RendererOptions::default()) {
            Ok(renderer) => renderer,
            Err(err) => {
                error!("radiant native vello: failed to create renderer: {:?}", err);
                event_loop.exit();
                return;
            }
        };
        self.startup_timing.mark_renderer_ready();
        info!("radiant native vello: renderer created");

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

    /// Keep startup first-frame work minimal when the deferred fallback path is armed.
    ///
    /// This preserves static scene rebuild work (for deterministic first paint)
    /// while skipping model and overlay pulls until first present completes.
    pub(super) fn prepare_startup_first_frame_scene(&mut self) {
        let _ = self.frame_state.take_model();
        let _ = self.frame_state.take_state_overlay();
        let _ = self.frame_state.take_motion_overlay();
    }

    pub(super) fn rebuild_layout(&mut self) {
        let Some(surface) = self.render_surface.as_ref() else {
            return;
        };

        let viewport = Vector2::new(surface.config.width as f32, surface.config.height as f32);
        let style = StyleTokens::for_viewport_with_scale(viewport.x, self.ui_scale_factor());
        self.style_cache = Some(style);
        self.shell_layout = Some(Arc::new(ShellLayout::build_with_style_and_runtime(
            viewport,
            &style,
            &mut self.layout_runtime,
        )));
        self.static_segment_graph.clear();
        self.frame_state.clear_layout_dirty();
        if let Some(point) = self.pending_cursor.take() {
            let _ = self.process_cursor_move_immediately(point);
        }
    }

    /// Borrow the retained shell layout while mutating runtime state without
    /// cloning the full layout payload.
    pub(super) fn with_shell_layout<T>(
        &mut self,
        work: impl FnOnce(&mut Self, &ShellLayout) -> T,
    ) -> Option<T> {
        let layout = self.shell_layout.take()?;
        let result = work(self, layout.as_ref());
        self.shell_layout = Some(layout);
        Some(result)
    }

    pub(super) fn request_redraw_if_needed(&mut self) {
        if self.redraw_requested {
            return;
        }
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
            self.redraw_requested = true;
        }
    }

    pub(super) fn build_style_for_layout(layout: &ShellLayout) -> StyleTokens {
        StyleTokens::for_viewport_with_scale(layout.root.rect.width(), layout.ui_scale)
    }

    pub(super) fn cached_style_for_layout(&self, layout: &ShellLayout) -> StyleTokens {
        self.style_cache
            .unwrap_or_else(|| Self::build_style_for_layout(layout))
    }

    /// Arm the hidden-startup reveal timeout so redraw stalls cannot deadlock launch.
    pub(super) fn arm_startup_reveal_deadline(&mut self, now: Instant) {
        if Self::startup_should_launch_hidden() && !self.startup_window_visible {
            self.startup_reveal_deadline = Some(now + STARTUP_REVEAL_STALL_TIMEOUT);
        }
    }

    /// Build one minimal host-titled startup scene for deferred-startup fallback.
    pub(super) fn build_startup_placeholder_scene(
        &mut self,
        layout: &ShellLayout,
        style: &StyleTokens,
    ) {
        let root = layout.root.rect;
        let panel_width = (root.width() * 0.36).clamp(220.0, 420.0);
        let panel_height = (style.sizing.font_header * 2.8).clamp(58.0, 86.0);
        let panel_min = Point::new(
            root.min.x + (root.width() - panel_width) * 0.5,
            root.min.y + (root.height() - panel_height) * 0.5,
        );
        let panel = UiRect::from_min_size(panel_min, Vector2::new(panel_width, panel_height));
        let accent_height = (panel_height * 0.08).clamp(3.0, 6.0);
        let accent = UiRect::from_min_max(
            panel.min,
            Point::new(panel.max.x, panel.min.y + accent_height),
        );
        let title_text = if self.options.title.trim().is_empty() {
            String::from(crate::app::DEFAULT_APP_TITLE)
        } else {
            self.options.title.clone()
        };
        let title = TextRun {
            text: title_text,
            position: Point::new(panel.min.x + 12.0, panel.min.y + 10.0),
            font_size: style.sizing.font_header.max(12.0),
            color: style.text_primary,
            max_width: Some((panel.width() - 24.0).max(20.0)),
            align: TextAlign::Center,
        };
        let subtitle = TextRun {
            text: String::from("Starting interface..."),
            position: Point::new(panel.min.x + 12.0, panel.min.y + panel_height * 0.48),
            font_size: style.sizing.font_meta.max(10.0),
            color: style.text_muted,
            max_width: Some((panel.width() - 24.0).max(20.0)),
            align: TextAlign::Center,
        };

        self.frame_cache.clear_color = style.clear_color;
        self.frame_cache.primitives.clear();
        self.frame_cache.text_runs.clear();
        self.frame_cache.text_runs.push(title.clone());
        self.frame_cache.text_runs.push(subtitle.clone());
        self.state_overlay_frame_cache.clear_color = style.clear_color;
        self.state_overlay_frame_cache.primitives.clear();
        self.state_overlay_frame_cache.text_runs.clear();
        self.waveform_motion_overlay_frame_cache.clear_color = style.clear_color;
        self.waveform_motion_overlay_frame_cache.primitives.clear();
        self.waveform_motion_overlay_frame_cache.text_runs.clear();
        self.chrome_motion_overlay_frame_cache.clear_color = style.clear_color;
        self.chrome_motion_overlay_frame_cache.primitives.clear();
        self.chrome_motion_overlay_frame_cache.text_runs.clear();
        self.clear_color = style.clear_color;

        self.static_scene.reset();
        self.static_scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            color_from_rgba(style.surface_base),
            None,
            &to_kurbo_rect(root),
        );
        self.static_scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            color_from_rgba(style.surface_raised),
            None,
            &to_kurbo_rect(panel),
        );
        self.static_scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            color_from_rgba(style.accent_mint),
            None,
            &to_kurbo_rect(accent),
        );
        self.text_renderer
            .draw_text_runs(&mut self.static_scene, &[title, subtitle]);
        self.state_overlay_scene.reset();
        self.waveform_motion_overlay_scene.reset();
        self.chrome_motion_overlay_scene.reset();
        self.scene.reset();
        self.scene.append(&self.static_scene, None);
    }
}

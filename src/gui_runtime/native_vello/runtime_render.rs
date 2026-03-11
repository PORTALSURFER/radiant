//! Scene rebuild and present helpers for the native Vello runtime.

use super::*;

impl<B: NativeAppBridge> NativeVelloRunner<B> {
    pub(super) fn rebuild_scene_if_needed(&mut self) {
        if self.shell_layout.is_none() || self.frame_state.layout_dirty {
            self.rebuild_layout();
        }
        let model_refresh_requested = self.frame_state.take_model();
        let static_rebuild_requested = self.frame_state.take_scene();
        let state_overlay_requested = self.frame_state.take_state_overlay();
        let motion_overlay_requested = self.frame_state.take_motion_overlay();
        if self.startup_model_pull_pending
            && !self.first_frame_presented
            && !model_refresh_requested
            && static_rebuild_requested
        {
            let Some(layout) = self.shell_layout.as_ref().map(Arc::clone) else {
                return;
            };
            let style = self.cached_style_for_layout(layout.as_ref());
            self.build_startup_placeholder_scene(layout.as_ref(), &style);
            return;
        }
        if static_rebuild_requested {
            self.profiler.add_explicit_static_rebuild();
        }
        let rebuild_static = static_rebuild_requested || model_refresh_requested;
        let rebuild_state_overlay = state_overlay_requested || rebuild_static;
        let rebuild_motion_overlay = motion_overlay_requested || rebuild_static;
        if !rebuild_static && !rebuild_state_overlay && !rebuild_motion_overlay {
            return;
        }
        self.rebuild_scene(
            model_refresh_requested,
            static_rebuild_requested,
            rebuild_static,
            rebuild_state_overlay,
            rebuild_motion_overlay,
        );
    }

    pub(super) fn apply_invalidation_scope(&mut self, scope: RuntimeInvalidationScope) {
        match scope {
            RuntimeInvalidationScope::OverlayStateOnly => {
                self.frame_state.mark_state_overlay_dirty();
            }
            RuntimeInvalidationScope::OverlayMotionOnly => {
                self.frame_state.mark_motion_overlay_dirty();
            }
            RuntimeInvalidationScope::ModelAndOverlays => {
                self.frame_state.mark_model_overlay_dirty();
            }
            RuntimeInvalidationScope::StaticAndOverlays => {
                self.frame_state.mark_model_dirty();
            }
            RuntimeInvalidationScope::LayoutAndAll => {
                self.frame_state.mark_layout_dirty();
                self.frame_state.mark_model_dirty();
                self.layout_runtime.reset();
                self.layout_runtime
                    .mark_all_dirty(ShellLayoutDirtyKind::Measure);
            }
        }
        self.request_redraw_if_needed();
    }

    pub(super) fn rebuild_overlay_and_request_redraw(&mut self) {
        self.frame_state.mark_state_overlay_dirty();
        self.request_redraw_if_needed();
    }

    fn rebuild_scene_for_tick(&mut self) {
        self.frame_state.mark_motion_overlay_dirty();
        self.rebuild_scene_if_needed();
    }

    fn rebuild_scene_for_redraw(
        &mut self,
        needs_animation: bool,
        delta_seconds: f32,
    ) -> (bool, FrameBuildResult) {
        if !needs_animation {
            if self.frame_state.has_pending_rebuild() {
                self.rebuild_scene_if_needed();
                return (true, self.frame_result_base());
            }
            return (false, self.frame_result_base());
        }
        let Some(layout) = self.shell_layout.as_ref() else {
            return (false, self.frame_result_base());
        };
        let tick_start = self.profiler.now_if_enabled();
        let style = self.cached_style_for_layout(layout);
        self.shell_state.tick_with_style(delta_seconds, &style);
        self.rebuild_scene_for_tick();
        let tick_duration = tick_start.map_or(Duration::ZERO, |start| start.elapsed());
        self.profiler.add_tick(tick_duration);
        (true, self.frame_result_base())
    }

    fn maybe_record_redraw_profile(
        &mut self,
        rebuild: Duration,
        acquire: Duration,
        render: Duration,
        blit: Duration,
        present: Duration,
        total: Duration,
    ) {
        let text_profile = if self.profiler.is_enabled() {
            self.text_renderer.take_layout_profile_counters()
        } else {
            (0, 0, 0, 0, 0, 0)
        };
        self.profiler
            .record_redraw(rebuild, acquire, render, blit, present, total, text_profile);
    }

    /// Build per-frame renderer counts shared with bridge-side telemetry.
    fn frame_result_base(&self) -> FrameBuildResult {
        FrameBuildResult {
            primitive_count: self
                .frame_cache
                .primitives
                .len()
                .saturating_add(self.state_overlay_frame_cache.primitives.len())
                .saturating_add(self.waveform_motion_overlay_frame_cache.primitives.len())
                .saturating_add(self.chrome_motion_overlay_frame_cache.primitives.len()),
            text_run_count: self
                .frame_cache
                .text_runs
                .len()
                .saturating_add(self.state_overlay_frame_cache.text_runs.len())
                .saturating_add(self.waveform_motion_overlay_frame_cache.text_runs.len())
                .saturating_add(self.chrome_motion_overlay_frame_cache.text_runs.len()),
            needs_animation: self.shell_state.needs_animation(),
            ..FrameBuildResult::default()
        }
    }

    /// Convert one duration to microseconds while saturating at `u32::MAX`.
    fn duration_us_u32(duration: Duration) -> u32 {
        duration.as_micros().min(u128::from(u32::MAX)) as u32
    }

    /// Return the configured redraw frame budget in microseconds.
    fn frame_budget_us(&self) -> u32 {
        Self::duration_us_u32(self.target_frame_interval)
    }

    /// Finalize and emit one frame result payload to the host bridge.
    fn emit_frame_result(
        &mut self,
        frame_result: &mut FrameBuildResult,
        frame_total: Duration,
        present: Duration,
        presented: bool,
        present_expected: bool,
    ) {
        let frame_budget_us = self.frame_budget_us();
        let frame_total_us = Self::duration_us_u32(frame_total);
        frame_result.frame_total_us = frame_total_us;
        frame_result.present_us = Self::duration_us_u32(present);
        frame_result.frame_budget_us = frame_budget_us;
        frame_result.presented = presented;
        frame_result.missed_present = present_expected && !presented;
        frame_result.jank = presented && frame_total_us > frame_budget_us;
        self.bridge.observe_frame_result(*frame_result);
    }

    /// Record profiler data (if enabled) and emit one finalized frame result.
    fn finish_redraw_attempt(
        &mut self,
        frame_result: &mut FrameBuildResult,
        frame_started_at: Instant,
        frame_profile_start: Option<Instant>,
        rebuild: Duration,
        acquire: Duration,
        render: Duration,
        blit: Duration,
        present: Duration,
        presented: bool,
        present_expected: bool,
    ) {
        if let Some(start) = frame_profile_start {
            self.maybe_record_redraw_profile(
                rebuild,
                acquire,
                render,
                blit,
                present,
                start.elapsed(),
            );
        }
        self.emit_frame_result(
            frame_result,
            frame_started_at.elapsed(),
            present,
            presented,
            present_expected,
        );
    }

    /// Resolve a retained image-upload blob for one RGBA payload.
    pub(super) fn cached_image_upload_blob(
        cache: &mut HashMap<ImageUploadBlobCacheKey, Blob<u8>>,
        cache_order: &mut VecDeque<ImageUploadBlobCacheKey>,
        pixels: &Arc<[u8]>,
        width: u32,
        height: u32,
    ) -> Blob<u8> {
        let key = ImageUploadBlobCacheKey {
            pixels_ptr: pixels.as_ptr() as usize,
            width,
            height,
        };
        if let Some(blob) = cache.get(&key) {
            touch_image_upload_blob_cache_key(cache_order, key);
            return blob.clone();
        }
        while cache.len() >= IMAGE_UPLOAD_BLOB_CACHE_LIMIT {
            let Some(stale_key) = cache_order.pop_front() else {
                cache.clear();
                break;
            };
            cache.remove(&stale_key);
        }
        let blob = Blob::new(Arc::new(SharedPixelBytes(Arc::clone(pixels))));
        cache.insert(key, blob.clone());
        cache_order.push_back(key);
        blob
    }

    fn encode_frame_to_scene(
        frame: &NativeViewFrame,
        scene: &mut Scene,
        text_renderer: &mut NativeTextRenderer,
        image_upload_blob_cache: &mut HashMap<ImageUploadBlobCacheKey, Blob<u8>>,
        image_upload_blob_cache_order: &mut VecDeque<ImageUploadBlobCacheKey>,
    ) {
        scene.reset();
        for primitive in frame.primitives.iter() {
            match primitive {
                Primitive::Rect(fill) => {
                    scene.fill(
                        Fill::NonZero,
                        Affine::IDENTITY,
                        color_from_rgba(fill.color),
                        None,
                        &to_kurbo_rect(fill.rect),
                    );
                }
                Primitive::Circle(fill) => {
                    scene.fill(
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
                Primitive::Image(draw) => {
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
                    let blob = Self::cached_image_upload_blob(
                        image_upload_blob_cache,
                        image_upload_blob_cache_order,
                        &draw.image.pixels,
                        width,
                        height,
                    );
                    let image_data = ImageData {
                        data: blob,
                        format: ImageFormat::Rgba8,
                        alpha_type: ImageAlphaType::Alpha,
                        width,
                        height,
                    };
                    let transform =
                        Affine::translate((draw.rect.min.x as f64, draw.rect.min.y as f64))
                            * Affine::scale_non_uniform(
                                draw.rect.width() as f64 / f64::from(width),
                                draw.rect.height() as f64 / f64::from(height),
                            );
                    scene.draw_image(&image_data, transform);
                }
            }
        }
        text_renderer.draw_text_runs(scene, &frame.text_runs);
    }

    /// Reveal the native window after startup sequencing reaches a stable frame.
    pub(super) fn maybe_reveal_startup_window(&mut self) {
        if self.startup_window_visible || !self.first_frame_presented {
            return;
        }
        if self.startup_model_pull_pending || self.startup_deferred_model_refresh_pending {
            return;
        }
        if let Some(window) = self.window.as_ref() {
            window.set_visible(true);
        }
        self.startup_window_visible = true;
        self.startup_reveal_deadline = None;
    }

    /// Reveal the window once the first full scene is ready on eager startup paths.
    pub(super) fn maybe_reveal_startup_window_after_first_scene_ready(&mut self) {
        if self.startup_window_visible
            || self.first_frame_presented
            || self.startup_model_pull_pending
            || self.startup_deferred_model_refresh_pending
        {
            return;
        }
        if let Some(window) = self.window.as_ref() {
            window.set_visible(true);
        }
        self.startup_window_visible = true;
        self.startup_reveal_deadline = None;
    }

    /// Force startup reveal when redraw delivery stalls while hidden.
    ///
    /// Some backends can throttle redraw delivery for hidden windows. This
    /// fallback ensures the app cannot remain hidden forever waiting on a
    /// second present.
    pub(super) fn maybe_force_reveal_startup_window_on_stall(&mut self, now: Instant) {
        if self.startup_window_visible {
            return;
        }
        let Some(deadline) = self.startup_reveal_deadline else {
            return;
        };
        if now < deadline {
            return;
        }
        warn!("native vello startup reveal fallback: forcing window visible after stall");
        if let Some(window) = self.window.as_ref() {
            window.set_visible(true);
        }
        self.startup_window_visible = true;
        self.startup_reveal_deadline = None;
        self.request_redraw_if_needed();
    }

    /// Handle one successful first present and schedule deferred startup pulls.
    pub(super) fn complete_first_present(&mut self) {
        if !self.first_frame_presented {
            self.first_frame_presented = true;
            self.startup_timing.mark_first_presented();
            if self.startup_model_pull_pending {
                self.startup_model_pull_pending = false;
                self.startup_deferred_model_refresh_pending = true;
                if !self.startup_window_visible {
                    self.arm_startup_reveal_deadline(Instant::now());
                }
                self.apply_invalidation_scope(RuntimeInvalidationScope::ModelAndOverlays);
            }
        }
        self.maybe_reveal_startup_window();
    }

    /// Return bridge-provided revision for one static segment.
    fn static_segment_revision(
        &self,
        segment_revisions: SegmentRevisions,
        segment: StaticFrameSegment,
    ) -> u64 {
        match segment {
            StaticFrameSegment::StatusBar => segment_revisions.status_bar,
            StaticFrameSegment::BrowserFrame => segment_revisions.browser_frame,
            StaticFrameSegment::BrowserRowsWindow => segment_revisions.browser_rows_window,
            StaticFrameSegment::MapPanel => segment_revisions.map_panel,
            StaticFrameSegment::WaveformOverlay => segment_revisions.waveform_overlay,
            StaticFrameSegment::GlobalStatic => segment_revisions.global_static,
        }
    }

    /// Return deterministic static segment identifier from cache-array index.
    pub(super) fn static_segment_from_cache_index(index: usize) -> StaticFrameSegment {
        match index {
            0 => StaticFrameSegment::GlobalStatic,
            1 => StaticFrameSegment::WaveformOverlay,
            2 => StaticFrameSegment::BrowserFrame,
            3 => StaticFrameSegment::BrowserRowsWindow,
            4 => StaticFrameSegment::MapPanel,
            5 => StaticFrameSegment::StatusBar,
            _ => unreachable!("invalid static segment index {index}"),
        }
    }

    /// Build candidate fingerprints for every retained static segment.
    fn build_static_segment_fingerprints(
        &self,
        layout: &ShellLayout,
        style: &StyleTokens,
        segment_revisions: SegmentRevisions,
    ) -> [StaticSegmentCacheFingerprint; StaticFrameSegment::COUNT] {
        let layout_width_bits = layout.root.rect.width().to_bits();
        let layout_height_bits = layout.root.rect.height().to_bits();
        let layout_scale_bits = layout.ui_scale.to_bits();
        let style_signature = static_segment_style_signature(style);
        std::array::from_fn(|idx| {
            let segment = Self::static_segment_from_cache_index(idx);
            StaticSegmentCacheFingerprint {
                segment,
                layout_width_bits,
                layout_height_bits,
                layout_scale_bits,
                style_signature,
                segment_revision: self.static_segment_revision(segment_revisions, segment),
            }
        })
    }

    fn state_overlay_cache_fingerprint(
        &self,
        model: &AppModel,
        _style: &StyleTokens,
        layout_width_bits: u32,
        layout_height_bits: u32,
        layout_scale_bits: u32,
    ) -> StateOverlayCacheFingerprint {
        StateOverlayCacheFingerprint {
            layout_width_bits,
            layout_height_bits,
            layout_scale_bits,
            shell: self.shell_state.state_overlay_fingerprint(),
            model_signature: state_overlay_model_signature(model),
        }
    }

    fn waveform_motion_overlay_cache_fingerprint(
        &self,
        motion_model: &NativeMotionModel,
        _style: &StyleTokens,
        layout_width_bits: u32,
        layout_height_bits: u32,
        layout_scale_bits: u32,
    ) -> WaveformMotionOverlayCacheFingerprint {
        WaveformMotionOverlayCacheFingerprint {
            layout_width_bits,
            layout_height_bits,
            layout_scale_bits,
            shell: self.shell_state.waveform_motion_overlay_fingerprint(),
            motion_signature: waveform_motion_overlay_model_signature(motion_model),
        }
    }

    fn chrome_motion_overlay_cache_fingerprint(
        &self,
        motion_model: &NativeMotionModel,
        _style: &StyleTokens,
        layout_width_bits: u32,
        layout_height_bits: u32,
        layout_scale_bits: u32,
    ) -> ChromeMotionOverlayCacheFingerprint {
        ChromeMotionOverlayCacheFingerprint {
            layout_width_bits,
            layout_height_bits,
            layout_scale_bits,
            shell: self.shell_state.chrome_motion_overlay_fingerprint(),
            motion_signature: chrome_motion_overlay_model_signature(motion_model),
        }
    }

    /// Rebuild and encode retained static segment scenes.
    fn rebuild_static_segment_scenes(
        &mut self,
        layout: &ShellLayout,
        style: &StyleTokens,
        dirty_segments: DirtySegments,
        segment_revisions: SegmentRevisions,
        force_rebuild: bool,
    ) -> (Duration, Duration) {
        if force_rebuild {
            self.static_segment_graph.clear();
        }
        let fingerprints = self.build_static_segment_fingerprints(layout, style, segment_revisions);
        let diff_plan = self
            .static_segment_graph
            .diff(dirty_segments, force_rebuild, fingerprints);
        let mut build_duration = Duration::ZERO;
        let mut encode_duration = Duration::ZERO;
        for segment in StaticFrameSegment::ALL {
            if !diff_plan.should_rebuild(segment) {
                continue;
            }

            let segment_build_start = Instant::now();
            self.shell_state.build_static_segment_with_style_into(
                layout,
                style,
                &self.model,
                self.motion_model.as_ref(),
                segment,
                &mut self.static_segment_frame_cache,
            );
            build_duration += segment_build_start.elapsed();

            let segment_encode_start = Instant::now();
            let frame = self.static_segment_frame_cache.frame(segment);
            let entry = self.static_segment_scene_cache.entry_mut(segment);
            Self::encode_frame_to_scene(
                frame,
                &mut entry.scene,
                &mut self.text_renderer,
                &mut self.image_upload_blob_cache,
                &mut self.image_upload_blob_cache_order,
            );
            encode_duration += segment_encode_start.elapsed();
            self.static_segment_graph
                .commit_segment(segment, diff_plan.fingerprint(segment));
        }

        self.frame_cache.clear_color = style.clear_color;
        self.static_segment_frame_cache
            .compose_into(&mut self.frame_cache);
        self.clear_color = self.frame_cache.clear_color;
        self.static_scene.reset();
        for segment in StaticFrameSegment::ALL {
            self.static_scene
                .append(self.static_segment_scene_cache.scene(segment), None);
        }
        (build_duration, encode_duration)
    }

    /// Refresh cached motion-model projection from the latest full app model.
    fn refresh_motion_model_from_model(&mut self) {
        self.motion_model = Some(NativeMotionModel::from_app_model(&self.model));
    }
}

impl<B: NativeAppBridge> NativeVelloRunner<B> {
    fn rebuild_scene(
        &mut self,
        model_refresh_requested: bool,
        static_rebuild_requested: bool,
        mut rebuild_static: bool,
        mut rebuild_state_overlay: bool,
        mut rebuild_motion_overlay: bool,
    ) {
        let mut bridge_dirty_segments = DirtySegments::all();
        let should_refresh_model =
            model_refresh_requested || (!self.motion_model_supported && rebuild_motion_overlay);
        let should_refresh_motion = rebuild_motion_overlay && self.motion_model_supported;
        self.profiler.record_scene_rebuilds(
            rebuild_static,
            rebuild_state_overlay,
            rebuild_motion_overlay,
        );
        let previous_waveform_signature = self
            .motion_model
            .as_ref()
            .and_then(|model| model.waveform_image_signature);
        if should_refresh_model {
            self.profiler.add_bridge_model_pull_rebuild();
            let pull_start = self.profiler.now_if_enabled();
            self.profiler.add_model_refresh();
            self.model_refresh_count = self.model_refresh_count.saturating_add(1);
            if self.model_refresh_count <= 24 {
                info!(
                    "native vello refreshing model: refresh_count={} rebuild_static={} rebuild_state_overlay={} rebuild_motion_overlay={}",
                    self.model_refresh_count,
                    rebuild_static,
                    rebuild_state_overlay,
                    rebuild_motion_overlay
                );
            }
            self.model = self.bridge.project_model();
            bridge_dirty_segments = self.bridge.take_dirty_segments();
            let bridge_segment_revisions = self.bridge.take_segment_revisions();
            if bridge_segment_revisions.has_static_revisions() {
                self.segment_revisions_supported = true;
            }
            if self.segment_revisions_supported {
                self.segment_revisions = bridge_segment_revisions;
            }
            let pull_duration = pull_start.map_or(Duration::ZERO, |start| start.elapsed());
            self.profiler.add_model_pull(pull_duration);
            self.shell_state.sync_from_model(&self.model);
            self.refresh_motion_model_from_model();
            self.motion_model_supported = true;
            self.sync_text_input_target();
            if self.startup_deferred_model_refresh_pending {
                self.startup_deferred_model_refresh_pending = false;
                self.startup_reveal_deadline = None;
                self.startup_timing.mark_deferred_model_refresh_done();
                self.startup_timing.maybe_emit_summary();
            }
            rebuild_static = resolve_static_rebuild(
                model_refresh_requested,
                static_rebuild_requested,
                bridge_dirty_segments,
            );
            if static_rebuild_from_dirty_mask(
                model_refresh_requested,
                static_rebuild_requested,
                bridge_dirty_segments,
            ) {
                self.profiler.add_dirty_mask_static_rebuild();
            }
        } else if should_refresh_motion {
            self.profiler.add_bridge_motion_pull_rebuild();
            let pull_start = self.profiler.now_if_enabled();
            if let Some(motion_model) = self.bridge.project_motion_model() {
                let pull_duration = pull_start.map_or(Duration::ZERO, |start| start.elapsed());
                self.profiler.add_motion_pull(pull_duration);
                if self.motion_model.as_ref() != Some(&motion_model) {
                    if previous_waveform_signature != motion_model.waveform_image_signature {
                        rebuild_static = true;
                        rebuild_state_overlay = true;
                        rebuild_motion_overlay = true;
                    }
                    self.shell_state.sync_from_motion_model(&motion_model);
                    self.motion_model = Some(motion_model);
                }
            } else {
                let pull_duration = pull_start.map_or(Duration::ZERO, |start| start.elapsed());
                self.profiler.add_motion_pull(pull_duration);
                let model_pull_start = self.profiler.now_if_enabled();
                self.profiler.add_bridge_model_pull_rebuild();
                self.motion_model_supported = false;
                self.model = self.bridge.project_model();
                bridge_dirty_segments = self.bridge.take_dirty_segments();
                let bridge_segment_revisions = self.bridge.take_segment_revisions();
                if bridge_segment_revisions.has_static_revisions() {
                    self.segment_revisions_supported = true;
                }
                if self.segment_revisions_supported {
                    self.segment_revisions = bridge_segment_revisions;
                }
                let model_pull_duration =
                    model_pull_start.map_or(Duration::ZERO, |start| start.elapsed());
                self.profiler.add_model_pull(model_pull_duration);
                self.shell_state.sync_from_model(&self.model);
                self.refresh_motion_model_from_model();
                self.sync_text_input_target();
                if self.startup_deferred_model_refresh_pending {
                    self.startup_deferred_model_refresh_pending = false;
                    self.startup_reveal_deadline = None;
                    self.startup_timing.mark_deferred_model_refresh_done();
                    self.startup_timing.maybe_emit_summary();
                }
            }
        }
        let Some(layout) = self.shell_layout.as_ref().map(Arc::clone) else {
            return;
        };
        let layout = layout.as_ref();
        let (layout_width_bits, layout_height_bits, layout_scale_bits) = (
            layout.root.rect.width().to_bits(),
            layout.root.rect.height().to_bits(),
            layout.ui_scale.to_bits(),
        );
        let style = self.cached_style_for_layout(layout);
        if rebuild_static {
            if self.incremental_frame_pipeline {
                let mut force_rebuild = !model_refresh_requested;
                if !self.segment_revisions_supported
                    && !self.missing_segment_revision_fallback_applied
                {
                    warn!(
                        "native vello bridge reported zero segment revisions; forcing one conservative static rebuild"
                    );
                    force_rebuild = true;
                    self.missing_segment_revision_fallback_applied = true;
                }
                let (build_duration, encode_duration) = self.rebuild_static_segment_scenes(
                    layout,
                    &style,
                    bridge_dirty_segments,
                    self.segment_revisions,
                    force_rebuild,
                );
                self.profiler.add_build_static(build_duration);
                self.profiler.add_encode_static(encode_duration);
            } else {
                let build_start = self.profiler.now_if_enabled();
                self.frame_cache.clear_color = style.clear_color;
                self.shell_state.build_frame_with_style_into_static(
                    layout,
                    &style,
                    &self.model,
                    &mut self.frame_cache,
                );
                let build_duration = build_start.map_or(Duration::ZERO, |start| start.elapsed());
                self.profiler.add_build_static(build_duration);
                let encode_start = self.profiler.now_if_enabled();
                Self::encode_frame_to_scene(
                    &self.frame_cache,
                    &mut self.static_scene,
                    &mut self.text_renderer,
                    &mut self.image_upload_blob_cache,
                    &mut self.image_upload_blob_cache_order,
                );
                let encode_duration = encode_start.map_or(Duration::ZERO, |start| start.elapsed());
                self.profiler.add_encode_static(encode_duration);
                self.clear_color = self.frame_cache.clear_color;
            }
        }
        let state_overlay_fingerprint = self.state_overlay_cache_fingerprint(
            &self.model,
            &style,
            layout_width_bits,
            layout_height_bits,
            layout_scale_bits,
        );
        let rebuild_state_overlay = rebuild_state_overlay
            || self.state_overlay_fingerprint.as_ref() != Some(&state_overlay_fingerprint);
        if rebuild_state_overlay {
            self.state_overlay_fingerprint = Some(state_overlay_fingerprint);
            let build_start = self.profiler.now_if_enabled();
            self.shell_state.build_state_overlay_into(
                layout,
                &style,
                &self.model,
                &mut self.state_overlay_frame_cache,
            );
            let build_duration = build_start.map_or(Duration::ZERO, |start| start.elapsed());
            self.profiler.add_build_state_overlay(build_duration);
            let encode_start = self.profiler.now_if_enabled();
            Self::encode_frame_to_scene(
                &self.state_overlay_frame_cache,
                &mut self.state_overlay_scene,
                &mut self.text_renderer,
                &mut self.image_upload_blob_cache,
                &mut self.image_upload_blob_cache_order,
            );
            let encode_duration = encode_start.map_or(Duration::ZERO, |start| start.elapsed());
            self.profiler.add_encode_state_overlay(encode_duration);
        }
        let mut rebuild_waveform_motion_overlay = rebuild_motion_overlay;
        let mut rebuild_chrome_motion_overlay = rebuild_motion_overlay;
        if let Some(motion_model) = self.motion_model.as_ref() {
            let waveform_motion_overlay_fingerprint = self
                .waveform_motion_overlay_cache_fingerprint(
                    motion_model,
                    &style,
                    layout_width_bits,
                    layout_height_bits,
                    layout_scale_bits,
                );
            rebuild_waveform_motion_overlay |= self.waveform_motion_overlay_fingerprint.as_ref()
                != Some(&waveform_motion_overlay_fingerprint);
            if rebuild_waveform_motion_overlay {
                self.waveform_motion_overlay_fingerprint =
                    Some(waveform_motion_overlay_fingerprint);
            }
            let chrome_motion_overlay_fingerprint = self.chrome_motion_overlay_cache_fingerprint(
                motion_model,
                &style,
                layout_width_bits,
                layout_height_bits,
                layout_scale_bits,
            );
            rebuild_chrome_motion_overlay |= self.chrome_motion_overlay_fingerprint.as_ref()
                != Some(&chrome_motion_overlay_fingerprint);
            if rebuild_chrome_motion_overlay {
                self.chrome_motion_overlay_fingerprint = Some(chrome_motion_overlay_fingerprint);
            }
        }
        if rebuild_waveform_motion_overlay || rebuild_chrome_motion_overlay {
            let mut build_duration = Duration::ZERO;
            let mut encode_duration = Duration::ZERO;
            if rebuild_waveform_motion_overlay {
                let motion_model = self
                    .motion_model
                    .as_ref()
                    .expect("motion model should exist before waveform-motion build");
                let build_start = self.profiler.now_if_enabled();
                self.shell_state.build_waveform_motion_overlay_into(
                    layout,
                    &style,
                    motion_model,
                    &mut self.waveform_motion_overlay_frame_cache,
                );
                build_duration += build_start.map_or(Duration::ZERO, |start| start.elapsed());
                let encode_start = self.profiler.now_if_enabled();
                Self::encode_frame_to_scene(
                    &self.waveform_motion_overlay_frame_cache,
                    &mut self.waveform_motion_overlay_scene,
                    &mut self.text_renderer,
                    &mut self.image_upload_blob_cache,
                    &mut self.image_upload_blob_cache_order,
                );
                encode_duration += encode_start.map_or(Duration::ZERO, |start| start.elapsed());
            }
            if rebuild_chrome_motion_overlay {
                let motion_model = self
                    .motion_model
                    .as_ref()
                    .expect("motion model should exist before chrome-motion build");
                let build_start = self.profiler.now_if_enabled();
                self.shell_state.build_chrome_motion_overlay_into(
                    layout,
                    &style,
                    motion_model,
                    &mut self.chrome_motion_overlay_frame_cache,
                );
                build_duration += build_start.map_or(Duration::ZERO, |start| start.elapsed());
                let encode_start = self.profiler.now_if_enabled();
                Self::encode_frame_to_scene(
                    &self.chrome_motion_overlay_frame_cache,
                    &mut self.chrome_motion_overlay_scene,
                    &mut self.text_renderer,
                    &mut self.image_upload_blob_cache,
                    &mut self.image_upload_blob_cache_order,
                );
                encode_duration += encode_start.map_or(Duration::ZERO, |start| start.elapsed());
            }
            self.profiler.add_build_motion_overlay(build_duration);
            self.profiler.add_encode_motion_overlay(encode_duration);
        } else if rebuild_motion_overlay {
            self.profiler.add_motion_overlay_skip();
        }
        if rebuild_static
            || rebuild_state_overlay
            || rebuild_waveform_motion_overlay
            || rebuild_chrome_motion_overlay
        {
            self.scene.reset();
            self.scene.append(&self.static_scene, None);
            self.scene.append(&self.state_overlay_scene, None);
            self.scene.append(&self.waveform_motion_overlay_scene, None);
            self.scene.append(&self.chrome_motion_overlay_scene, None);
        }
    }

    pub(super) fn redraw(&mut self, event_loop: &ActiveEventLoop) {
        self.redraw_count = self.redraw_count.saturating_add(1);
        self.redraw_requested = false;
        let now = Instant::now();
        let delta = (now - self.last_redraw).as_secs_f32();
        self.last_redraw = now;
        let frame_started_at = Instant::now();
        let frame_profile_start = self.profiler.now_if_enabled();
        let rebuild_start = self.profiler.now_if_enabled();
        let needs_animation = self.shell_state.needs_animation();
        let (has_rebuild, mut frame_result) = self.rebuild_scene_for_redraw(needs_animation, delta);
        let rebuild_duration = rebuild_start.map_or(Duration::ZERO, |start| start.elapsed());
        if self.redraw_count <= 8 {
            info!(
                "native vello redraw start: redraw_count={} needs_animation={} has_rebuild={} delta_ms={}",
                self.redraw_count,
                needs_animation,
                has_rebuild,
                ((delta * 1000.0) as u32)
            );
        }
        if !needs_animation && !has_rebuild && self.first_frame_presented {
            return;
        }

        let Some(window) = self.window.as_ref() else {
            self.finish_redraw_attempt(
                &mut frame_result,
                frame_started_at,
                frame_profile_start,
                rebuild_duration,
                Duration::ZERO,
                Duration::ZERO,
                Duration::ZERO,
                Duration::ZERO,
                false,
                false,
            );
            return;
        };
        let Some(dev_id) = self.render_surface.as_ref().map(|surface| surface.dev_id) else {
            self.finish_redraw_attempt(
                &mut frame_result,
                frame_started_at,
                frame_profile_start,
                rebuild_duration,
                Duration::ZERO,
                Duration::ZERO,
                Duration::ZERO,
                Duration::ZERO,
                false,
                false,
            );
            return;
        };

        let mut surface_error = None;
        let mut needs_resize = false;
        let mut out_of_memory = false;
        let acquire_start = self.profiler.now_if_enabled();
        let surface_texture = {
            let Some(surface) = self.render_surface.as_mut() else {
                self.finish_redraw_attempt(
                    &mut frame_result,
                    frame_started_at,
                    frame_profile_start,
                    rebuild_duration,
                    Duration::ZERO,
                    Duration::ZERO,
                    Duration::ZERO,
                    Duration::ZERO,
                    false,
                    false,
                );
                return;
            };
            match surface.surface.get_current_texture() {
                Ok(frame) => Some(frame),
                Err(err) => {
                    surface_error = Some(err.clone());
                    if self.redraw_count <= 8 {
                        warn!("native vello surface acquire error: {:?}", err);
                    }
                    match err {
                        wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated => {
                            let size = window.inner_size();
                            if let Some(render_ctx) = self.render_ctx.as_mut() {
                                render_ctx.resize_surface(
                                    surface,
                                    size.width.max(1),
                                    size.height.max(1),
                                );
                                needs_resize = true;
                            }
                        }
                        wgpu::SurfaceError::OutOfMemory => out_of_memory = true,
                        wgpu::SurfaceError::Timeout | wgpu::SurfaceError::Other => {}
                    }
                    None
                }
            }
        };
        let acquire_duration = acquire_start.map_or(Duration::ZERO, |start| start.elapsed());
        if let Some(err) = surface_error {
            if out_of_memory {
                error!("native vello out-of-memory in surface acquire: {:?}", err);
            } else if self.redraw_count <= 8 {
                info!("native vello non-fatal surface error: {:?}", err);
            }
            self.finish_redraw_attempt(
                &mut frame_result,
                frame_started_at,
                frame_profile_start,
                rebuild_duration,
                acquire_duration,
                Duration::ZERO,
                Duration::ZERO,
                Duration::ZERO,
                false,
                true,
            );
            if matches!(err, wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated)
                && needs_resize
            {
                self.apply_invalidation_scope(RuntimeInvalidationScope::LayoutAndAll);
                self.rebuild_scene_if_needed();
            }
            if out_of_memory {
                event_loop.exit();
            }
            return;
        }
        let Some(surface_texture) = surface_texture else {
            self.finish_redraw_attempt(
                &mut frame_result,
                frame_started_at,
                frame_profile_start,
                rebuild_duration,
                acquire_duration,
                Duration::ZERO,
                Duration::ZERO,
                Duration::ZERO,
                false,
                true,
            );
            return;
        };

        let Some(surface) = self.render_surface.as_mut() else {
            self.finish_redraw_attempt(
                &mut frame_result,
                frame_started_at,
                frame_profile_start,
                rebuild_duration,
                acquire_duration,
                Duration::ZERO,
                Duration::ZERO,
                Duration::ZERO,
                false,
                true,
            );
            return;
        };
        let Some(render_ctx) = self.render_ctx.as_ref() else {
            self.finish_redraw_attempt(
                &mut frame_result,
                frame_started_at,
                frame_profile_start,
                rebuild_duration,
                acquire_duration,
                Duration::ZERO,
                Duration::ZERO,
                Duration::ZERO,
                false,
                true,
            );
            return;
        };
        let Some(renderer) = self.renderer.as_mut() else {
            self.finish_redraw_attempt(
                &mut frame_result,
                frame_started_at,
                frame_profile_start,
                rebuild_duration,
                acquire_duration,
                Duration::ZERO,
                Duration::ZERO,
                Duration::ZERO,
                false,
                true,
            );
            return;
        };
        let dev_handle = &render_ctx.devices[dev_id];
        let surface_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let render_start = self.profiler.now_if_enabled();
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
            error!("native vello render_to_texture failed: {:?}", err);
            event_loop.exit();
            let render = render_start.map_or(Duration::ZERO, |start| start.elapsed());
            self.finish_redraw_attempt(
                &mut frame_result,
                frame_started_at,
                frame_profile_start,
                rebuild_duration,
                acquire_duration,
                render,
                Duration::ZERO,
                Duration::ZERO,
                false,
                true,
            );
            return;
        }
        let render_duration = render_start.map_or(Duration::ZERO, |start| start.elapsed());
        let blit_start = self.profiler.now_if_enabled();
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
        let blit_duration = blit_start.map_or(Duration::ZERO, |start| start.elapsed());
        let present_started_at = Instant::now();
        surface_texture.present();
        self.complete_first_present();
        let present_duration = present_started_at.elapsed();
        self.finish_redraw_attempt(
            &mut frame_result,
            frame_started_at,
            frame_profile_start,
            rebuild_duration,
            acquire_duration,
            render_duration,
            blit_duration,
            present_duration,
            true,
            true,
        );
    }
}

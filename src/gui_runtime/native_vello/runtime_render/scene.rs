//! Scene-cache rebuild and composition helpers for the native Vello runtime.

use super::*;

impl<B: NativeAppBridge> NativeVelloRunner<B> {
    /// Resolve a retained image-upload blob for one RGBA payload.
    pub(in crate::gui_runtime::native_vello) fn cached_image_upload_blob(
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
    pub(in crate::gui_runtime::native_vello) fn static_segment_from_cache_index(
        index: usize,
    ) -> StaticFrameSegment {
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
    pub(in crate::gui_runtime::native_vello) fn rebuild_scene(
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
            self.sync_browser_viewport_from_shell(layout);
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
}

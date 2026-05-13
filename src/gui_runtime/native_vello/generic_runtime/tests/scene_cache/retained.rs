use super::*;

#[derive(Default)]
struct RetainedBridge {
    render_count: usize,
    dirty_mask: u64,
    volatile: bool,
}

#[derive(Default)]
struct MultiRetainedBridge {
    render_counts: std::collections::BTreeMap<u64, usize>,
}

impl MultiRetainedBridge {
    fn render_count_for(&self, key: u64) -> usize {
        self.render_counts.get(&key).copied().unwrap_or_default()
    }
}

impl RuntimeBridge<()> for RetainedBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<()>> {
        Arc::new(UiSurface::new(SurfaceNode::retained_canvas_mapped(
            31,
            WidgetSizing::fixed(Vector2::new(120.0, 28.0)),
            crate::widgets::RetainedSurfaceDescriptor {
                key: 7,
                revision: 1,
                dirty_mask: self.dirty_mask,
                volatile: self.volatile,
            },
            |_| (),
        )))
    }

    fn render_retained_surface(
        &mut self,
        _descriptor: crate::widgets::RetainedSurfaceDescriptor,
        rect: UiRect,
        _viewport: Vector2,
    ) -> Option<PaintFrame> {
        self.render_count += 1;
        Some(PaintFrame {
            clear_color: Rgba8 {
                r: 0,
                g: 0,
                b: 0,
                a: 0,
            },
            primitives: vec![Primitive::Rect(crate::gui::paint::FillRect {
                rect,
                color: Rgba8 {
                    r: 1,
                    g: 2,
                    b: 3,
                    a: 255,
                },
            })],
            text_runs: Vec::new(),
        })
    }
}

impl RuntimeBridge<()> for MultiRetainedBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<()>> {
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
                    SurfaceNode::retained_canvas_mapped(
                        31,
                        WidgetSizing::fixed(Vector2::new(120.0, 28.0)),
                        crate::widgets::RetainedSurfaceDescriptor {
                            key: 7,
                            revision: 1,
                            dirty_mask: 0,
                            volatile: false,
                        },
                        |_| (),
                    ),
                ),
                SurfaceChild::new(
                    SlotParams::fill(),
                    SurfaceNode::retained_canvas_mapped(
                        32,
                        WidgetSizing::fixed(Vector2::new(120.0, 28.0)),
                        crate::widgets::RetainedSurfaceDescriptor {
                            key: 8,
                            revision: 1,
                            dirty_mask: 0,
                            volatile: false,
                        },
                        |_| (),
                    ),
                ),
            ],
        )))
    }

    fn render_retained_surface(
        &mut self,
        descriptor: crate::widgets::RetainedSurfaceDescriptor,
        rect: UiRect,
        _viewport: Vector2,
    ) -> Option<PaintFrame> {
        *self.render_counts.entry(descriptor.key).or_default() += 1;
        Some(PaintFrame {
            clear_color: Rgba8 {
                r: 0,
                g: 0,
                b: 0,
                a: 0,
            },
            primitives: vec![Primitive::Rect(crate::gui::paint::FillRect {
                rect,
                color: Rgba8 {
                    r: descriptor.key as u8,
                    g: 2,
                    b: 3,
                    a: 255,
                },
            })],
            text_runs: Vec::new(),
        })
    }
}

#[test]
fn retained_custom_surface_cache_skips_unchanged_bridge_render() {
    let mut core =
        GenericNativeRuntimeCore::new(RetainedBridge::default(), Vector2::new(320.0, 40.0));
    let mut scene = Scene::new();
    let mut text_renderer = NativeTextRenderer::new();
    let mut retained_cache = RetainedSurfaceFrameCache::default();
    let mut text_runs = SceneTextRunBuffer::new();
    let viewport = core.runtime.viewport();
    let plan = core.paint_plan();

    let first = encode_plan(
        &plan,
        &mut scene,
        &mut text_renderer,
        core.runtime.bridge_mut(),
        viewport,
        &mut retained_cache,
        &mut text_runs,
    );
    let second = encode_plan(
        &plan,
        &mut scene,
        &mut text_renderer,
        core.runtime.bridge_mut(),
        viewport,
        &mut retained_cache,
        &mut text_runs,
    );

    assert_eq!(first.bridge_calls, 1);
    assert_eq!(first.cache_hits, 0);
    assert_eq!(first.retained_frame_primitive_count, 1);
    assert_eq!(first.retained_frame_text_run_count, 0);
    assert_eq!(second.bridge_calls, 0);
    assert_eq!(second.cache_hits, 1);
    assert_eq!(second.retained_frame_primitive_count, 1);
    assert_eq!(second.retained_frame_text_run_count, 0);
    assert_eq!(core.runtime.bridge().render_count, 1);
}

#[test]
fn retained_custom_surface_cache_keeps_multiple_stable_surfaces() {
    let mut core =
        GenericNativeRuntimeCore::new(MultiRetainedBridge::default(), Vector2::new(320.0, 40.0));
    let mut scene = Scene::new();
    let mut text_renderer = NativeTextRenderer::new();
    let mut retained_cache = RetainedSurfaceFrameCache::default();
    let mut text_runs = SceneTextRunBuffer::new();
    let viewport = core.runtime.viewport();
    let plan = core.paint_plan();

    let first = encode_plan(
        &plan,
        &mut scene,
        &mut text_renderer,
        core.runtime.bridge_mut(),
        viewport,
        &mut retained_cache,
        &mut text_runs,
    );
    let second = encode_plan(
        &plan,
        &mut scene,
        &mut text_renderer,
        core.runtime.bridge_mut(),
        viewport,
        &mut retained_cache,
        &mut text_runs,
    );

    assert_eq!(first.bridge_calls, 2);
    assert_eq!(first.cache_hits, 0);
    assert_eq!(second.bridge_calls, 0);
    assert_eq!(second.cache_hits, 2);
    assert_eq!(core.runtime.bridge().render_count_for(7), 1);
    assert_eq!(core.runtime.bridge().render_count_for(8), 1);
}

#[test]
fn retained_custom_surface_cache_rejects_current_dirty_descriptor() {
    let mut core =
        GenericNativeRuntimeCore::new(RetainedBridge::default(), Vector2::new(320.0, 40.0));
    let mut scene = Scene::new();
    let mut text_renderer = NativeTextRenderer::new();
    let mut retained_cache = RetainedSurfaceFrameCache::default();
    let mut text_runs = SceneTextRunBuffer::new();
    let viewport = core.runtime.viewport();
    let plan = core.paint_plan();

    let first = encode_plan(
        &plan,
        &mut scene,
        &mut text_renderer,
        core.runtime.bridge_mut(),
        viewport,
        &mut retained_cache,
        &mut text_runs,
    );
    core.runtime.bridge_mut().dirty_mask = 1;
    core.refresh_surface();
    let dirty_plan = core.paint_plan();
    let second = encode_plan(
        &dirty_plan,
        &mut scene,
        &mut text_renderer,
        core.runtime.bridge_mut(),
        viewport,
        &mut retained_cache,
        &mut text_runs,
    );

    assert_eq!(first.bridge_calls, 1);
    assert_eq!(first.cache_hits, 0);
    assert_eq!(second.bridge_calls, 1);
    assert_eq!(second.cache_hits, 0);
    assert_eq!(core.runtime.bridge().render_count, 2);
}

#[test]
fn retained_custom_surface_cache_invalidates_dirty_descriptor_key() {
    let mut core =
        GenericNativeRuntimeCore::new(RetainedBridge::default(), Vector2::new(320.0, 40.0));
    let mut scene = Scene::new();
    let mut text_renderer = NativeTextRenderer::new();
    let mut retained_cache = RetainedSurfaceFrameCache::default();
    let mut text_runs = SceneTextRunBuffer::new();
    let viewport = core.runtime.viewport();
    let plan = core.paint_plan();

    let clean = encode_plan(
        &plan,
        &mut scene,
        &mut text_renderer,
        core.runtime.bridge_mut(),
        viewport,
        &mut retained_cache,
        &mut text_runs,
    );
    core.runtime.bridge_mut().dirty_mask = 1;
    core.refresh_surface();
    let dirty_plan = core.paint_plan();
    let dirty = encode_plan(
        &dirty_plan,
        &mut scene,
        &mut text_renderer,
        core.runtime.bridge_mut(),
        viewport,
        &mut retained_cache,
        &mut text_runs,
    );
    core.runtime.bridge_mut().dirty_mask = 0;
    core.refresh_surface();
    let clean_again_plan = core.paint_plan();
    let clean_again = encode_plan(
        &clean_again_plan,
        &mut scene,
        &mut text_renderer,
        core.runtime.bridge_mut(),
        viewport,
        &mut retained_cache,
        &mut text_runs,
    );

    assert_eq!(clean.bridge_calls, 1);
    assert_eq!(dirty.bridge_calls, 1);
    assert_eq!(clean_again.bridge_calls, 1);
    assert_eq!(clean_again.cache_hits, 0);
    assert_eq!(core.runtime.bridge().render_count, 3);
}

#[test]
fn retained_custom_surface_cache_rejects_volatile_descriptor() {
    let mut core = GenericNativeRuntimeCore::new(
        RetainedBridge {
            volatile: true,
            ..RetainedBridge::default()
        },
        Vector2::new(320.0, 40.0),
    );
    let mut scene = Scene::new();
    let mut text_renderer = NativeTextRenderer::new();
    let mut retained_cache = RetainedSurfaceFrameCache::default();
    let mut text_runs = SceneTextRunBuffer::new();
    let viewport = core.runtime.viewport();
    let plan = core.paint_plan();

    let first = encode_plan(
        &plan,
        &mut scene,
        &mut text_renderer,
        core.runtime.bridge_mut(),
        viewport,
        &mut retained_cache,
        &mut text_runs,
    );
    let second = encode_plan(
        &plan,
        &mut scene,
        &mut text_renderer,
        core.runtime.bridge_mut(),
        viewport,
        &mut retained_cache,
        &mut text_runs,
    );

    assert_eq!(first.bridge_calls, 1);
    assert_eq!(first.cache_hits, 0);
    assert_eq!(second.bridge_calls, 1);
    assert_eq!(second.cache_hits, 0);
    assert_eq!(core.runtime.bridge().render_count, 2);
}

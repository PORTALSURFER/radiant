use super::{fixtures::*, shared::*};

#[test]
fn deferred_surface_resize_keeps_latest_nonzero_size() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        TestFrameMessageBridge::default(),
        Vector2::new(320.0, 40.0),
    );

    runner.defer_surface_resize(PhysicalSize::new(400, 240));
    runner.defer_surface_resize(PhysicalSize::new(0, 480));
    runner.defer_surface_resize(PhysicalSize::new(640, 360));

    assert_eq!(
        runner.timing.pending_surface_resize,
        Some(PhysicalSize::new(640, 360))
    );
}

#[test]
fn window_resize_events_coalesce_until_redraw_boundary() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        TestFrameMessageBridge::default(),
        Vector2::new(320.0, 40.0),
    );

    runner.resize_surface(PhysicalSize::new(400, 240));
    runner.resize_surface(PhysicalSize::new(640, 360));

    assert_eq!(
        runner.timing.pending_surface_resize,
        Some(PhysicalSize::new(640, 360))
    );
    assert_eq!(runner.timing.pending_viewport_resize, None);
    assert_eq!(runner.core.runtime.viewport(), Vector2::new(320.0, 40.0));
}

#[test]
fn simple_dirty_resize_frame_can_render_directly_to_surface() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        NoTransientOverlayBridge::default(),
        Vector2::new(320.0, 40.0),
    );

    runner.timing.surface_resize_applied_this_frame = true;
    runner.frame.scene_texture_dirty = true;

    assert!(runner.should_render_resize_frame_directly());

    runner
        .frame
        .transient_overlay_primitives
        .push(PaintPrimitive::FillRect(crate::runtime::PaintFillRect {
            widget_id: 1,
            rect: UiRect::from_min_size(Point::new(0.0, 0.0), Vector2::new(1.0, 1.0)),
            color: Rgba8 {
                r: 255,
                g: 255,
                b: 255,
                a: 255,
            },
        }));

    assert!(!runner.should_render_resize_frame_directly());
}

#[test]
fn deferred_interactive_scene_rebuild_is_flushed_before_paint() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        TestFrameMessageBridge::default(),
        Vector2::new(320.0, 40.0),
    );

    runner.defer_interactive_scene_rebuild();
    runner.rebuild_deferred_scene_if_needed(&mut RenderFrameProfile::default());

    assert!(!runner.timing.deferred_scene_rebuild);
    assert!(runner.frame.scene_texture_dirty);
}

#[test]
fn deferred_scene_rebuild_marks_pending_without_surface_refresh() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        TestFrameMessageBridge::default(),
        Vector2::new(320.0, 40.0),
    );

    runner.defer_scene_rebuild();

    assert!(runner.timing.deferred_scene_rebuild);
    assert!(!runner.timing.deferred_surface_refresh);
}

#[test]
fn deferred_viewport_resize_is_applied_at_scene_rebuild_boundary() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        TestFrameMessageBridge::default(),
        Vector2::new(320.0, 40.0),
    );

    runner.defer_viewport_resize(Vector2::new(640.0, 120.0));

    assert_eq!(runner.core.runtime.viewport(), Vector2::new(320.0, 40.0));
    assert_eq!(
        runner.timing.pending_viewport_resize,
        Some(Vector2::new(640.0, 120.0))
    );

    runner.rebuild_deferred_scene_if_needed(&mut RenderFrameProfile::default());

    assert_eq!(runner.core.runtime.viewport(), Vector2::new(640.0, 120.0));
    assert_eq!(runner.timing.pending_viewport_resize, None);
}

#[test]
fn subpixel_equivalent_resize_updates_viewport_without_relayout() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        TestFrameMessageBridge::default(),
        Vector2::new(320.0, 40.0),
    );

    assert!(!runner.core.set_viewport(Vector2::new(320.4, 40.0)));
    assert_eq!(runner.core.runtime.viewport(), Vector2::new(320.4, 40.0));

    assert!(runner.core.set_viewport(Vector2::new(320.6, 40.0)));
    assert_eq!(runner.core.runtime.viewport(), Vector2::new(320.6, 40.0));
}

#[test]
fn subpixel_equivalent_deferred_resize_reuses_encoded_scene() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        NoTransientOverlayBridge::default(),
        Vector2::new(320.0, 40.0),
    );
    runner.rebuild_scene();
    runner.frame.scene_texture_dirty = false;

    runner.defer_viewport_resize(Vector2::new(320.4, 40.0));
    runner.rebuild_deferred_scene_if_needed(&mut RenderFrameProfile::default());

    assert!(!runner.timing.deferred_scene_rebuild);
    assert_eq!(runner.timing.pending_viewport_resize, None);
    assert_eq!(runner.core.runtime.viewport(), Vector2::new(320.4, 40.0));
    assert!(
        runner.frame.scene_texture_dirty,
        "the resized surface still needs a fresh texture render"
    );
}

#[test]
fn deferred_auxiliary_sync_tracks_interactive_rebuild_deferral() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        TestFrameMessageBridge::default(),
        Vector2::new(320.0, 40.0),
    );

    runner.defer_auxiliary_window_sync();

    assert!(runner.timing.deferred_auxiliary_window_sync);
}

#[test]
fn deferred_interactive_scene_rebuild_refreshes_surface_once_before_paint() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        CountingProjectBridge::default(),
        Vector2::new(120.0, 40.0),
    );
    let project_count = runner.core.runtime.bridge().project_count;

    runner.defer_interactive_scene_rebuild();
    runner.rebuild_deferred_scene_if_needed(&mut RenderFrameProfile::default());

    assert!(!runner.timing.deferred_scene_rebuild);
    assert!(!runner.timing.deferred_surface_refresh);
    assert_eq!(
        runner.core.runtime.bridge().project_count,
        project_count + 1,
        "deferred interactive rebuild should refresh and encode in one frame-boundary pass"
    );
}

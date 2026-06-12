use super::*;

#[test]
fn waveform_playback_uses_paint_only_transient_playhead() {
    let mono_samples: Vec<f32> = (0..512)
        .map(|index| ((index as f32 / 16.0).sin() * 0.8).clamp(-1.0, 1.0))
        .collect();
    let source = Arc::new(synthetic_file(mono_samples, 48_000, 2));
    let mut runtime = SurfaceRuntime::new(
        radiant::app(WaveformApp {
            source,
            viewport: WaveformViewport::full(512),
            zoom_anchor_ratio: 0.5,
            playing: true,
            playhead_ratio: 0.25,
        })
        .view(view)
        .animated_transient_overlay_at(
            60,
            |state| state.playing,
            |state, context, primitives| {
                paint_playhead_overlay(state, context.plan, context.animation_time, primitives);
            },
        )
        .handle_message(|state, message, context| {
            state.apply_interaction(message);
            context.request_repaint();
        })
        .into_bridge(),
        Vector2::new(1280.0, 560.0),
    );

    let activity = runtime.bridge_mut().animation_activity();
    assert!(activity.needs_animation());
    assert!(!activity.needs_frame_message());
    assert!(
        !runtime.bridge_mut().queue_animation_frame(),
        "waveform playhead animation should not enqueue app frame messages"
    );

    let plan = runtime.paint_plan(&ThemeTokens::default());
    let mut primitives = Vec::new();
    runtime.bridge_mut().paint_transient_overlay(
        TransientOverlayContext::new(
            &plan,
            Vector2::new(1280.0, 560.0),
            Duration::from_millis(500),
        ),
        &mut primitives,
    );

    assert!(
        primitives.iter().any(|primitive| matches!(
            primitive,
            PaintPrimitive::FillRect(fill) if fill.widget_id == WAVEFORM_WIDGET_ID
                && fill.rect.height() > 0.0
        )),
        "paint-only playback should append a playhead over the cached waveform paint plan"
    );
}

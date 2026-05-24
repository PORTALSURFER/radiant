use super::*;
use radiant::runtime::{RuntimeBridge, SurfaceRuntime};

#[test]
fn spectrogram_tick_scrolls_synthetic_columns_without_dsp() {
    let mut state = SpectrogramState::default();
    let initial_frame = state.frame;
    let first_column = state.columns.front().cloned();

    state.tick();

    assert_eq!(state.frame, initial_frame + state.speed as u64);
    assert_eq!(state.columns.len(), model::COLUMNS);
    assert_ne!(state.columns.front(), first_column.as_ref());
}

#[test]
fn spectrogram_widget_paints_heatmap_grid_and_labels() {
    let state = SpectrogramState::default();
    let widget = SpectrogramWidget::new(state.columns.iter().cloned().collect(), state.frame);
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(760.0, 320.0));
    let mut primitives = Vec::new();

    widget.append_paint(
        &mut primitives,
        bounds,
        &LayoutOutput::default(),
        &ThemeTokens::default(),
    );

    let fill_count = primitives
        .iter()
        .filter(|primitive| matches!(primitive, PaintPrimitive::FillRect(_)))
        .count();
    assert!(
        fill_count >= widget::visible_bin_count(),
        "spectrogram should paint one heatmap cell per visible bin"
    );
    assert!(
        primitives
            .iter()
            .any(|primitive| matches!(primitive, PaintPrimitive::StrokeRect(_))),
        "spectrogram should paint plot chrome"
    );
    assert!(
        primitives
            .iter()
            .any(|primitive| matches!(primitive, PaintPrimitive::Text(text) if text.text.as_str().contains("synthetic realtime spectrum"))),
        "spectrogram should label its GUI-only synthetic source"
    );
}

#[test]
fn spectrogram_runtime_frame_messages_advance_visual_state() {
    let bridge = spectrogram_test_bridge(SpectrogramState::default());
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(980.0, 560.0));
    let initial_status = status_text(&runtime);

    assert!(runtime.bridge_mut().needs_animation());
    assert!(runtime.bridge_mut().queue_animation_frame());
    let outcome = runtime.drain_runtime_messages();

    assert_eq!(outcome.messages_dispatched, 1);
    assert_ne!(status_text(&runtime), initial_status);
}

#[test]
fn spectrogram_hover_uses_paint_only_widget_local_state() {
    let mut widget = SpectrogramWidget::new(
        SpectrogramState::default().columns.into_iter().collect(),
        96,
    );
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(760.0, 320.0));
    let plot = widget.plot_rect(bounds);

    let output = widget.handle_input(
        bounds,
        WidgetInput::PointerMove {
            position: Point::new(plot.min.x + plot.width() * 0.5, plot.center().y),
        },
    );

    assert!(output.is_none());
    assert_eq!(widget.hover_column, Some(model::COLUMNS / 2));
    assert!(
        widget.prefers_pointer_move_paint_only(),
        "spectrogram hover should stay on the runtime-local paint-only path"
    );
    let mut overlay = Vec::new();
    widget.append_runtime_overlay_paint(
        &mut overlay,
        bounds,
        &LayoutOutput::default(),
        &ThemeTokens::default(),
    );
    assert!(
        overlay
            .iter()
            .any(|primitive| matches!(primitive, PaintPrimitive::FillRect(_))),
        "hover cursor should paint as a lightweight runtime overlay"
    );
}

#[test]
fn spectrogram_runtime_hover_does_not_refresh_surface() {
    let bridge = spectrogram_test_bridge(SpectrogramState::default());
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(980.0, 560.0));
    let bounds = runtime.layout().rects[&SPECTROGRAM_WIDGET_ID];
    let first = runtime
        .dispatch_pointer_move_with_outcome(Point::new(bounds.min.x + 80.0, bounds.center().y));
    let second = runtime
        .dispatch_pointer_move_with_outcome(Point::new(bounds.min.x + 140.0, bounds.center().y));

    assert!(first.needs_scene_rebuild());
    assert!(second.paint_only_requested);
    assert!(
        !second.needs_scene_rebuild(),
        "stable spectrogram hover should avoid reprojection and full scene rebuilds"
    );
}

fn spectrogram_test_bridge(state: SpectrogramState) -> impl RuntimeBridge<SpectrogramMessage> {
    radiant::app(state)
        .view(project_surface)
        .animation(|state| state.running)
        .on_frame(|| SpectrogramMessage::Frame)
        .update(update)
        .into_bridge()
}

fn status_text<Bridge>(runtime: &SurfaceRuntime<Bridge, SpectrogramMessage>) -> String
where
    Bridge: RuntimeBridge<SpectrogramMessage>,
{
    runtime
        .paint_plan(&ThemeTokens::default())
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            PaintPrimitive::Text(text) if text.widget_id == STATUS_WIDGET_ID => {
                Some(text.text.as_str().to_string())
            }
            _ => None,
        })
        .expect("status text should be painted")
}

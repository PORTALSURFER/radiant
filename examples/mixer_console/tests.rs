use super::*;
use model::{CHANNEL_LABELS, MIN_GAIN_DB};
use radiant::prelude::*;
use radiant::runtime::{RuntimeBridge, SurfaceRuntime};

#[test]
fn mixer_tick_animates_synthetic_decibel_meters_without_dsp() {
    let mut state = MixerState::default();
    let initial = state.channels.map(|channel| channel.meter_db);

    for _ in 0..8 {
        state.tick();
    }

    assert_eq!(state.channels.len(), CHANNEL_COUNT);
    assert_ne!(state.channels.map(|channel| channel.meter_db), initial);
    assert!(state.channels.iter().all(|channel| channel.meter_db <= 0.0));
}

#[test]
fn mixer_fader_down_drives_meter_to_silence() {
    let mut state = MixerState::default();
    update_panel(
        &mut state,
        MixerPanelMessage::SetGain {
            channel: 3,
            ratio: 0.0,
        },
    );

    assert_eq!(state.channels[3].gain_db, MIN_GAIN_DB);
    assert_eq!(state.channels[3].meter_db, MIN_GAIN_DB);
    assert_eq!(state.channels[3].peak_db, MIN_GAIN_DB);

    state.tick();

    assert_eq!(state.channels[3].meter_db, MIN_GAIN_DB);
}

#[test]
fn mixer_solo_keeps_non_solo_meters_active_for_visual_information() {
    let mut state = MixerState::default();
    update_panel(&mut state, MixerPanelMessage::ToggleSolo(1));

    for _ in 0..24 {
        state.tick();
    }

    assert!(state.channels[1].solo);
    assert!(
        state.channels[0].meter_db > MIN_GAIN_DB,
        "non-solo channels should keep showing pre-solo visual meter information"
    );
    assert!(
        state.channels[1].meter_db > MIN_GAIN_DB,
        "soloed channel should remain visually active"
    );
}

#[test]
fn mixer_solo_grays_non_solo_meter_paint() {
    let mut state = MixerState::default();
    update_panel(&mut state, MixerPanelMessage::ToggleSolo(1));
    let widget = MixerPanelWidget::new(state.channels, state.selected_channel, state.frame);
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(960.0, 380.0));
    let mut primitives = Vec::new();

    widget.append_paint(
        &mut primitives,
        bounds,
        &LayoutOutput::default(),
        &ThemeTokens::default(),
    );

    assert!(
        primitives.iter().any(|primitive| {
            matches!(
                primitive,
                PaintPrimitive::FillRect(fill) if fill.color == paint::rgba(75, 80, 86, 180)
            )
        }),
        "solo mode should gray out non-solo meter fills"
    );
}

#[test]
fn mixer_panel_paints_eight_channel_strips_and_db_labels() {
    let state = MixerState::default();
    let widget = MixerPanelWidget::new(state.channels, state.selected_channel, state.frame);
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(960.0, 380.0));
    let mut primitives = Vec::new();

    widget.append_paint(
        &mut primitives,
        bounds,
        &LayoutOutput::default(),
        &ThemeTokens::default(),
    );

    let label_count = primitives
        .iter()
        .filter(|primitive| {
            matches!(
                primitive,
                PaintPrimitive::Text(text) if CHANNEL_LABELS.contains(&text.text.as_str())
            )
        })
        .count();
    assert_eq!(label_count, CHANNEL_COUNT);
    assert!(
        primitives
            .iter()
            .any(|primitive| matches!(primitive, PaintPrimitive::Text(text) if text.text.as_str().contains("dB"))),
        "mixer should paint decibel readouts"
    );
}

#[test]
fn mixer_panel_hover_uses_paint_only_runtime_overlay() {
    let state = MixerState::default();
    let mut widget = MixerPanelWidget::new(state.channels, state.selected_channel, state.frame);
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(960.0, 380.0));
    let strip = widget.strip_rect(bounds, 2);

    let output = widget.handle_input(
        bounds,
        WidgetInput::PointerMove {
            position: strip.center(),
        },
    );

    assert!(output.is_none());
    assert_eq!(widget.hover_channel, Some(2));
    assert!(widget.prefers_pointer_move_paint_only());
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
            .any(|primitive| matches!(primitive, PaintPrimitive::StrokeRect(_))),
        "hover strip should paint as a lightweight runtime overlay"
    );
}

#[test]
fn mixer_panel_fader_drag_routes_gain_change() {
    let state = MixerState::default();
    let mut widget = MixerPanelWidget::new(state.channels, state.selected_channel, state.frame);
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(960.0, 380.0));
    let strip = widget.strip_rect(bounds, 4);
    let fader = widget.fader_rect(strip);

    let output = widget.handle_input(
        bounds,
        WidgetInput::PointerPress {
            position: Point::new(fader.center().x, fader.min.y),
            button: PointerButton::Primary,
            modifiers: Default::default(),
        },
    );

    assert_eq!(
        output.and_then(|output| output.typed_ref::<MixerPanelMessage>().copied()),
        Some(MixerPanelMessage::SetGain {
            channel: 4,
            ratio: 1.0
        })
    );
    assert!(!widget.prefers_pointer_move_paint_only());
}

#[test]
fn mixer_runtime_hover_does_not_refresh_surface() {
    let bridge = mixer_test_bridge(MixerState::default());
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(1040.0, 620.0));
    let bounds = runtime.layout().rects[&MIXER_WIDGET_ID];
    let first = runtime
        .dispatch_pointer_move_with_outcome(Point::new(bounds.min.x + 80.0, bounds.center().y));
    let second = runtime
        .dispatch_pointer_move_with_outcome(Point::new(bounds.min.x + 180.0, bounds.center().y));

    assert!(first.needs_scene_rebuild());
    assert!(second.paint_only_requested);
    assert!(
        !second.needs_scene_rebuild(),
        "stable mixer hover should avoid reprojection and full scene rebuilds"
    );
}

#[test]
fn mixer_runtime_frame_messages_advance_status() {
    let bridge = mixer_test_bridge(MixerState::default());
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(1040.0, 620.0));
    let initial_status = status_text(&runtime);

    assert!(runtime.bridge_mut().needs_animation());
    assert!(runtime.bridge_mut().queue_animation_frame());
    let outcome = runtime.drain_runtime_messages();

    assert_eq!(outcome.messages_dispatched, 1);
    assert_ne!(status_text(&runtime), initial_status);
}

fn mixer_test_bridge(state: MixerState) -> impl RuntimeBridge<MixerMessage> {
    radiant::app(state)
        .view(project_surface)
        .animation(|state| state.running)
        .on_frame(|| MixerMessage::Frame)
        .update(update)
        .into_bridge()
}

fn status_text<Bridge>(runtime: &SurfaceRuntime<Bridge, MixerMessage>) -> String
where
    Bridge: RuntimeBridge<MixerMessage>,
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

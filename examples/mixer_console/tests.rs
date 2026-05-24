use super::model::{CHANNEL_LABELS, MIN_GAIN_DB, ratio_for_gain};
use super::panel::MixerDragTarget;
use super::*;
use radiant::prelude::*;
use radiant::runtime::{Event, RuntimeBridge, SurfaceRuntime};
use radiant::widgets::PointerModifiers;

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
            selection: Some(ListSelectionModifiers::new()),
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
    assert!(state.channels[0].meter_db > MIN_GAIN_DB);
    assert!(state.channels[1].meter_db > MIN_GAIN_DB);
}

#[test]
fn mixer_solo_grays_non_solo_meter_paint() {
    let mut state = MixerState::default();
    update_panel(&mut state, MixerPanelMessage::ToggleSolo(1));
    let widget = mixer_widget(&state);
    let bounds = mixer_bounds();
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
fn mixer_panel_paints_dense_channel_strips_sends_and_db_labels() {
    let state = MixerState::default();
    let widget = mixer_widget(&state);
    let bounds = mixer_bounds();
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
    assert!(primitives.len() > CHANNEL_COUNT * 24);
    assert!(
        primitives
            .iter()
            .any(|primitive| matches!(primitive, PaintPrimitive::Text(text) if text.text.as_str().contains("dB")))
    );
}

#[test]
fn mixer_panel_hover_uses_paint_only_runtime_overlay() {
    let state = MixerState::default();
    let mut widget = mixer_widget(&state);
    let bounds = mixer_bounds();
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
            .any(|primitive| matches!(primitive, PaintPrimitive::StrokeRect(_)))
    );
}

#[test]
fn mixer_panel_fader_drag_routes_gain_change() {
    let state = MixerState::default();
    let mut widget = mixer_widget(&state);
    let bounds = mixer_bounds();
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
            ratio: 1.0,
            selection: Some(ListSelectionModifiers::new()),
        })
    );
    assert!(widget.prefers_pointer_move_paint_only());
}

#[test]
fn mixer_panel_supports_shift_and_control_multi_channel_selection() {
    let state = MixerState::default();
    let mut widget = mixer_widget(&state);
    let bounds = mixer_bounds();

    press_strip_label(&mut widget, bounds, 2, PointerModifiers::default());
    press_strip_label(
        &mut widget,
        bounds,
        5,
        PointerModifiers {
            shift: true,
            ..Default::default()
        },
    );

    assert_eq!(widget.selection.selected_indices(), &[2, 3, 4, 5]);

    press_strip_label(
        &mut widget,
        bounds,
        7,
        PointerModifiers {
            command: true,
            ..Default::default()
        },
    );

    assert_eq!(widget.selection.selected_indices(), &[2, 3, 4, 5, 7]);
}

#[test]
fn mixer_reorder_moves_channel_identity_and_preserves_selection() {
    let mut state = MixerState::default();
    update_panel(
        &mut state,
        MixerPanelMessage::Select {
            channel: 2,
            modifiers: ListSelectionModifiers::new(),
        },
    );
    update_panel(
        &mut state,
        MixerPanelMessage::Select {
            channel: 4,
            modifiers: ListSelectionModifiers::toggle(),
        },
    );
    let moved = state.channels[2];
    let also_selected = state.channels[4];

    update_panel(
        &mut state,
        MixerPanelMessage::Reorder { from: 2, insert: 7 },
    );

    assert_eq!(state.channels[6], moved);
    assert_eq!(state.selected().id, also_selected.id);
    assert_eq!(state.selection.selected_indices(), &[3, 6]);
}

#[test]
fn mixer_strip_drag_paints_insertion_line_without_spreading_strips() {
    let state = MixerState::default();
    let mut widget = mixer_widget(&state);
    let bounds = mixer_bounds();
    let source = widget.strip_rect(bounds, 2);
    let target_line = widget.insertion_line_rect(bounds, 7);

    widget.handle_input(
        bounds,
        WidgetInput::PointerPress {
            position: Point::new(source.center().x, source.min.y + 22.0),
            button: PointerButton::Primary,
            modifiers: Default::default(),
        },
    );
    let move_output = widget.handle_input(
        bounds,
        WidgetInput::PointerMove {
            position: target_line.center(),
        },
    );

    assert!(move_output.is_none());
    assert_eq!(widget.drag_target, Some(MixerDragTarget::Strip(2)));
    assert_eq!(widget.reorder_insert, Some(7));
    let mut overlay = Vec::new();
    widget.append_runtime_overlay_paint(
        &mut overlay,
        bounds,
        &LayoutOutput::default(),
        &ThemeTokens::default(),
    );
    assert!(overlay.iter().any(|primitive| {
        matches!(
            primitive,
            PaintPrimitive::FillRect(fill)
                if fill.rect == target_line
                    && fill.color == paint::translucent(ThemeTokens::default().highlight_cyan, 235)
        )
    }));
}

#[test]
fn mixer_strip_drag_drop_emits_reorder_message() {
    let state = MixerState::default();
    let mut widget = mixer_widget(&state);
    let bounds = mixer_bounds();
    let source = widget.strip_rect(bounds, 2);
    let target_line = widget.insertion_line_rect(bounds, 7);

    widget.handle_input(
        bounds,
        WidgetInput::PointerPress {
            position: Point::new(source.center().x, source.min.y + 22.0),
            button: PointerButton::Primary,
            modifiers: Default::default(),
        },
    );
    widget.handle_input(
        bounds,
        WidgetInput::PointerMove {
            position: target_line.center(),
        },
    );
    let output = widget.handle_input(
        bounds,
        WidgetInput::PointerRelease {
            position: target_line.center(),
            button: PointerButton::Primary,
            modifiers: Default::default(),
        },
    );

    assert_eq!(
        output.and_then(|output| output.typed_ref::<MixerPanelMessage>().copied()),
        Some(MixerPanelMessage::Reorder { from: 2, insert: 7 })
    );
}

#[test]
fn mixer_group_fader_drag_applies_relative_gain_delta_to_selected_channels() {
    let mut state = MixerState::default();
    update_panel(
        &mut state,
        MixerPanelMessage::Select {
            channel: 4,
            modifiers: ListSelectionModifiers::new(),
        },
    );
    update_panel(
        &mut state,
        MixerPanelMessage::Select {
            channel: 5,
            modifiers: ListSelectionModifiers::toggle(),
        },
    );
    let initial_4 = state.channels[4].gain_db;
    let initial_5 = state.channels[5].gain_db;

    update_panel(
        &mut state,
        MixerPanelMessage::SetGain {
            channel: 4,
            ratio: 0.80,
            selection: None,
        },
    );

    let delta_4 = state.channels[4].gain_db - initial_4;
    let delta_5 = state.channels[5].gain_db - initial_5;
    assert_eq!(state.selection.selected_indices(), &[4, 5]);
    assert!((delta_4 - delta_5).abs() < 0.001);
}

#[test]
fn mixer_panel_fader_drag_preview_survives_rebuild_without_jittering_to_stale_gain() {
    let state = MixerState::default();
    let mut widget = mixer_widget(&state);
    let bounds = mixer_bounds();
    let strip = widget.strip_rect(bounds, 4);
    let fader = widget.fader_rect(strip);

    widget.handle_input(
        bounds,
        WidgetInput::PointerPress {
            position: Point::new(fader.center().x, fader.min.y),
            button: PointerButton::Primary,
            modifiers: Default::default(),
        },
    );
    assert!(
        widget
            .handle_input(
                bounds,
                WidgetInput::PointerMove {
                    position: Point::new(fader.center().x, fader.max.y),
                },
            )
            .is_none()
    );

    let mut rebuilt = mixer_widget(&state);
    rebuilt.synchronize_from_previous(&widget);

    assert_eq!(rebuilt.drag_target, Some(MixerDragTarget::Fader(4)));
    assert_eq!(rebuilt.drag_preview_ratio, Some(0.0));
    assert_eq!(rebuilt.fader_display_ratio(4), 0.0);
    assert_ne!(
        rebuilt.fader_display_ratio(4),
        ratio_for_gain(state.channels[4].gain_db)
    );
}

#[test]
fn mixer_group_fader_drag_preview_moves_selected_channels_together() {
    let mut state = MixerState::default();
    state
        .selection
        .select(4, CHANNEL_COUNT, ListSelectionModifiers::new());
    state
        .selection
        .select(5, CHANNEL_COUNT, ListSelectionModifiers::toggle());
    state.selected_channel = 4;
    let mut widget = mixer_widget(&state);
    let bounds = mixer_bounds();
    let strip = widget.strip_rect(bounds, 4);
    let fader = widget.fader_rect(strip);

    widget.handle_input(
        bounds,
        WidgetInput::PointerPress {
            position: Point::new(fader.center().x, fader.min.y),
            button: PointerButton::Primary,
            modifiers: Default::default(),
        },
    );
    assert!(
        widget
            .handle_input(
                bounds,
                WidgetInput::PointerMove {
                    position: Point::new(fader.center().x, fader.min.y + fader.height() * 0.20),
                },
            )
            .is_none()
    );

    let mut rebuilt = mixer_widget(&state);
    rebuilt.synchronize_from_previous(&widget);

    let delta_4 = rebuilt.fader_display_db(4) - state.channels[4].gain_db;
    let delta_5 = rebuilt.fader_display_db(5) - state.channels[5].gain_db;
    assert_eq!(rebuilt.drag_target, Some(MixerDragTarget::Fader(4)));
    assert!((delta_4 - delta_5).abs() < 0.001);
}

#[test]
fn mixer_panel_send_drag_routes_dense_aux_control_change() {
    let state = MixerState::default();
    let mut widget = mixer_widget(&state);
    let bounds = mixer_bounds();
    let strip = widget.strip_rect(bounds, 17);
    let send = widget.send_rect(strip, 2);

    let output = widget.handle_input(
        bounds,
        WidgetInput::PointerPress {
            position: Point::new(send.min.x + send.width() * 0.75, send.center().y),
            button: PointerButton::Primary,
            modifiers: Default::default(),
        },
    );

    assert_eq!(
        output.and_then(|output| output.typed_ref::<MixerPanelMessage>().copied()),
        Some(MixerPanelMessage::SetSend {
            channel: 17,
            send: 2,
            ratio: 0.75
        })
    );
    assert_eq!(
        widget.drag_target,
        Some(MixerDragTarget::Send {
            channel: 17,
            send: 2
        })
    );
}

#[test]
fn mixer_runtime_hover_does_not_refresh_surface() {
    let bridge = mixer_test_bridge(MixerState::default());
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(1440.0, 760.0));
    let bounds = runtime.layout().rects[&MIXER_WIDGET_ID];
    let first = runtime
        .dispatch_pointer_move_with_outcome(Point::new(bounds.min.x + 80.0, bounds.center().y));
    let second = runtime
        .dispatch_pointer_move_with_outcome(Point::new(bounds.min.x + 180.0, bounds.center().y));

    assert!(first.needs_scene_rebuild());
    assert!(second.paint_only_requested);
    assert!(!second.needs_scene_rebuild());
}

#[test]
fn mixer_runtime_fader_drag_motion_uses_paint_only_preview_until_release() {
    let state = MixerState::default();
    let bridge = mixer_test_bridge(state.clone());
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(1440.0, 760.0));
    let bounds = runtime.layout().rects[&MIXER_WIDGET_ID];
    let widget = mixer_widget(&state);
    let strip = widget.strip_rect(bounds, 4);
    let fader = widget.fader_rect(strip);
    let press = Point::new(fader.center().x, fader.min.y);
    let drag = Point::new(fader.center().x, fader.min.y + fader.height() * 0.35);

    runtime.dispatch_event(Event::PointerPress {
        position: press,
        button: PointerButton::Primary,
        modifiers: PointerModifiers::default(),
    });
    let _ = runtime.take_repaint_requested();
    let first_drag = runtime.dispatch_pointer_move_with_outcome(drag);
    assert!(first_drag.needs_scene_rebuild());
    let move_outcome =
        runtime.dispatch_pointer_move_with_outcome(Point::new(drag.x, drag.y + 12.0));

    assert!(move_outcome.paint_only_requested);
    assert!(!move_outcome.needs_scene_rebuild());

    runtime.dispatch_event(Event::PointerRelease {
        position: drag,
        button: PointerButton::Primary,
        modifiers: PointerModifiers::default(),
    });
    assert!(runtime.take_repaint_requested());
}

#[test]
fn mixer_runtime_frame_messages_advance_status() {
    let bridge = mixer_test_bridge(MixerState::default());
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(1440.0, 760.0));
    let initial_status = status_text(&runtime);

    assert!(runtime.bridge_mut().needs_animation());
    assert!(runtime.bridge_mut().queue_animation_frame());
    let outcome = runtime.drain_runtime_messages();

    assert_eq!(outcome.messages_dispatched, 1);
    assert_ne!(status_text(&runtime), initial_status);
}

fn mixer_widget(state: &MixerState) -> MixerPanelWidget {
    MixerPanelWidget::new(
        state.channels,
        state.selection.clone(),
        state.selected_channel,
        state.frame,
    )
}

fn mixer_test_bridge(state: MixerState) -> impl RuntimeBridge<MixerMessage> {
    radiant::app(state)
        .view(project_surface)
        .animation(|state| state.running)
        .on_frame(|| MixerMessage::Frame)
        .update(update)
        .into_bridge()
}

fn mixer_bounds() -> Rect {
    Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(1400.0, 500.0))
}

fn press_strip_label(
    widget: &mut MixerPanelWidget,
    bounds: Rect,
    channel: usize,
    modifiers: PointerModifiers,
) -> Option<MixerPanelMessage> {
    let strip = widget.strip_rect(bounds, channel);
    let position = Point::new(strip.center().x, strip.min.y + 22.0);
    let output = widget
        .handle_input(
            bounds,
            WidgetInput::PointerPress {
                position,
                button: PointerButton::Primary,
                modifiers,
            },
        )
        .and_then(|output| output.typed_ref::<MixerPanelMessage>().copied());
    let _ = widget.handle_input(
        bounds,
        WidgetInput::PointerRelease {
            position,
            button: PointerButton::Primary,
            modifiers,
        },
    );
    output
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

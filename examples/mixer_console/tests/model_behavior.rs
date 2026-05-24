use super::super::model::{MIN_GAIN_DB, ratio_for_gain};
use super::super::panel::MixerDragTarget;
use super::*;

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

    assert_eq!(
        rebuilt.interaction.drag_target,
        Some(MixerDragTarget::Fader(4))
    );
    assert_eq!(rebuilt.interaction.drag_preview_ratio, Some(0.0));
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
    assert_eq!(
        rebuilt.interaction.drag_target,
        Some(MixerDragTarget::Fader(4))
    );
    assert!((delta_4 - delta_5).abs() < 0.001);
}

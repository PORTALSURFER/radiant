use super::super::model::CHANNEL_LABELS;
use super::*;

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
                PaintPrimitive::FillRect(fill) if fill.color == Rgba8::new(75, 80, 86, 180)
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

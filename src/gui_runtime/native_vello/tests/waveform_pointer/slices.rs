use super::*;
use crate::gui::native_shell::layout_adapter::compute_waveform_slice_preview_rects;

#[test]
fn waveform_slice_preview_click_toggles_slice_selection() {
    let layout = ShellLayout::build(Vector2::new(1200.0, 800.0));
    let mut model = AppModel::default();
    model
        .waveform
        .slices
        .push(crate::app::WaveformSlicePreviewModel {
            range: crate::app::NormalizedRangeModel::new(180, 420),
            selected: false,
        });
    let mut shell_state = NativeShellState::new();
    let rects = compute_waveform_slice_preview_rects(
        layout.waveform_plot,
        &model.waveform.slices,
        model.waveform.view_start_micros,
        model.waveform.view_end_micros,
    );
    let point = Point::new(
        (rects[0].rect.min.x + rects[0].rect.max.x) * 0.5,
        (rects[0].rect.min.y + rects[0].rect.max.y) * 0.5,
    );

    assert_eq!(
        action_from_pointer(
            &layout,
            &model,
            &mut shell_state,
            point,
            ModifiersState::default(),
        ),
        Some(UiAction::ToggleWaveformSliceSelection { index: 0 })
    );
}

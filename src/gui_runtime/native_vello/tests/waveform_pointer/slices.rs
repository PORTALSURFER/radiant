use super::*;
use crate::gui::native_shell::compute_waveform_slice_preview_rects;

#[test]
fn waveform_slice_preview_click_toggles_slice_selection() {
    let layout = ShellLayout::build(Vector2::new(1200.0, 800.0));
    let mut model = AppModel::default();
    model
        .waveform
        .slices
        .push(crate::compat_app_contract::WaveformSlicePreviewModel {
            range: crate::compat_app_contract::NormalizedRangeModel::new(180, 420),
            selected: false,
            focused: false,
            marked_for_export: false,
            duplicate_cleanup_candidate: false,
            duplicate_cleanup_exempted: false,
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

#[test]
fn duplicate_cleanup_slice_click_and_right_click_use_duplicate_actions() {
    let layout = ShellLayout::build(Vector2::new(1200.0, 800.0));
    let mut model = AppModel::default();
    model.waveform_chrome.exact_duplicate_cleanup_available = true;
    model
        .waveform
        .slices
        .push(crate::compat_app_contract::WaveformSlicePreviewModel {
            range: crate::compat_app_contract::NormalizedRangeModel::new(180, 420),
            selected: false,
            focused: false,
            marked_for_export: false,
            duplicate_cleanup_candidate: true,
            duplicate_cleanup_exempted: false,
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
        Some(UiAction::AuditionWaveformDuplicateSlice { index: 0 })
    );
    assert_eq!(
        duplicate_cleanup_exemption_action_from_pointer(&layout, &model, point),
        Some(UiAction::ToggleWaveformDuplicateSliceExemption { index: 0 })
    );
}

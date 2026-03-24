use super::*;
use crate::app::AutomationNodeSnapshot;

fn child<'a>(parent: &'a AutomationNodeSnapshot, id: &str) -> &'a AutomationNodeSnapshot {
    parent
        .children
        .iter()
        .find(|node| node.id.0 == id)
        .unwrap_or_else(|| panic!("missing automation child {id}"))
}

#[test]
fn waveform_motion_overlay_draws_slice_preview_overlays() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    let slice = crate::app::WaveformSlicePreviewModel {
        range: crate::app::NormalizedRangeModel::new(180, 420),
        selected: false,
    };
    model.waveform.slices.push(slice.clone());
    let motion = NativeMotionModel::from_app_model(&model);

    let expected_rect = compute_waveform_slice_preview_rects(
        layout.waveform_plot,
        &model.waveform.slices,
        model.waveform.view_start_micros,
        model.waveform.view_end_micros,
    )[0]
    .rect;

    let mut frame = NativeViewFrame::default();
    state.build_motion_overlay_into(&layout, &style, &motion, &mut frame);

    let fill = frame
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Rect(rect) if rect.rect == expected_rect => Some(rect.color),
            _ => None,
        })
        .expect("slice preview fill");

    assert_eq!(
        fill,
        translucent_overlay_color(style.bg_secondary, style.highlight_blue_soft, 0.26)
    );
}

#[test]
fn waveform_automation_exposes_slice_toggle_and_detect_actions() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut model = AppModel::default();
    model.waveform.loaded_label = Some(String::from("kick.wav"));
    model
        .waveform
        .slices
        .push(crate::app::WaveformSlicePreviewModel {
            range: crate::app::NormalizedRangeModel::new(180, 420),
            selected: true,
        });
    let mut state = NativeShellState::new();
    let node = state.automation_snapshot(&layout, &model);
    let waveform = child(&node.root, "waveform.panel");
    let region = child(waveform, "waveform.region");

    assert!(
        region
            .available_actions
            .contains(&String::from("detect_waveform_silence_slices"))
    );
    let slice = child(waveform, "waveform.slice.000");
    assert_eq!(slice.selected, true);
    assert_eq!(slice.value.as_deref(), Some("180000-420000"));
    assert!(
        slice
            .available_actions
            .contains(&String::from("toggle_waveform_slice_selection"))
    );

    let buttons = waveform_toolbar_buttons(
        &layout,
        &style,
        &NativeMotionModel::from_app_model(&model),
        false,
        None,
    );
    assert!(
        buttons.iter().any(|button| {
            button.label == "Silence Split"
                && button.action == Some(UiAction::DetectWaveformSilenceSlices)
        }),
        "silence split toolbar button should be present"
    );
}

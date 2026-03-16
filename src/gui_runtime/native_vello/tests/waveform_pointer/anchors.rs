use super::*;

#[test]
fn waveform_anchor_prefers_selection_then_cursor_then_playhead() {
    let mut model = AppModel::default();
    assert_eq!(waveform_anchor_micros(&model), 0);

    model.waveform.playhead_milli = Some(333);
    assert_eq!(waveform_anchor_micros(&model), milli(333));

    model.waveform.cursor_milli = Some(222);
    assert_eq!(waveform_anchor_micros(&model), milli(222));

    model.waveform.selection_milli = Some(crate::app::NormalizedRangeModel::new(111, 444));
    assert_eq!(waveform_anchor_micros(&model), milli(111));
}

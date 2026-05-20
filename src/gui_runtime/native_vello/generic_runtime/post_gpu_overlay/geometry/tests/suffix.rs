use super::{super::*, fixtures::*};

#[test]
fn replayable_suffix_starts_after_last_gpu_surface() {
    let primitives = [fill(1), gpu(2), fill(3), gpu(4), stroke(5), fill(6)];
    let suffix = replayable_suffix(&primitives).expect("suffix");

    assert_eq!(suffix.len(), 2);
    assert!(matches!(suffix[0], PaintPrimitive::StrokeRect(_)));
    assert!(matches!(suffix[1], PaintPrimitive::FillRect(_)));
}

#[test]
fn replayable_suffix_is_absent_when_no_gpu_surface_exists() {
    let primitives = [fill(1), stroke(2)];

    assert!(replayable_suffix(&primitives).is_none());
}

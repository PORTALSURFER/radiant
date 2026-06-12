pub(super) const RATIO_EPSILON: f64 = 0.000_001;

pub(super) fn finite_unit_or(value: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        fallback.clamp(0.0, 1.0)
    }
}

pub(super) fn finite_unit(value: f32) -> Option<f32> {
    value.is_finite().then(|| value.clamp(0.0, 1.0))
}

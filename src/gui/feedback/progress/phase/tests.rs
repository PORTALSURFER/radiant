use super::ProgressPhase;

#[test]
fn progress_phase_maps_completed_work_into_subrange() {
    let phase = ProgressPhase::new(0.25, 0.75);

    assert_eq!(phase.fraction(0, 4), Some(0.25));
    assert_eq!(phase.fraction(2, 4), Some(0.5));
    assert_eq!(phase.fraction(4, 4), Some(0.75));
}

#[test]
fn progress_phase_clamps_completed_ratio() {
    let phase = ProgressPhase::new(0.1, 0.9);

    assert_eq!(phase.fraction(12, 10), Some(0.9));
}

#[test]
fn progress_phase_ignores_unknown_totals() {
    let phase = ProgressPhase::new(0.0, 1.0);

    assert_eq!(phase.fraction(4, 0), None);
}

#[test]
fn progress_phase_clamps_non_finite_bounds() {
    let phase = ProgressPhase::new(f32::NAN, f32::INFINITY);

    assert_eq!(phase.start(), 0.0);
    assert_eq!(phase.end(), 0.0);
}

#[test]
fn progress_phase_reports_when_fraction_is_known() {
    let mut reports = Vec::new();

    assert!(ProgressPhase::new(0.2, 0.4).report(1, 2, |fraction| reports.push(fraction)));
    assert_eq!(reports, vec![0.3]);
}

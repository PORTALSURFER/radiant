use super::*;
fn at(base: Instant, ms: u64) -> Instant {
    base + Duration::from_millis(ms)
}
fn target(owner: u64, value: f64) -> Target {
    Target {
        key: TargetKey { owner, property: 1 },
        initial: Some(0.0),
        value,
        duration: Duration::from_millis(100),
        easing: Easing::Linear,
    }
}
#[test]
fn retarget_removal_pause_and_reduced_motion() {
    let base = Instant::now();
    let mut e = Engine::default();
    e.reconcile(&[target(1, 10.0)], base, false, false);
    e.frame(at(base, 50), false, false);
    assert_eq!(e.value(target(1, 0.0).key, at(base, 50)), Some(5.0));
    e.reconcile(&[target(1, 20.0)], at(base, 50), false, false);
    e.frame(at(base, 100), false, false);
    assert_eq!(e.value(target(1, 0.0).key, at(base, 100)), Some(12.5));
    e.frame(at(base, 110), false, true);
    assert!(e.frame(at(base, 210), false, true).next_deadline.is_none());
    e.frame(at(base, 260), false, false);
    assert!(e.value(target(1, 0.0).key, at(base, 260)).unwrap() > 12.5);
    e.reconcile(&[target(1, 30.0)], at(base, 260), true, false);
    assert_eq!(e.frame(at(base, 260), true, false).diagnostics.active, 0);
    e.reconcile(&[], at(base, 260), false, false);
    assert_eq!(e.diagnostics().active, 0);
}
#[test]
fn invalid_and_bounds_are_deterministic() {
    let now = Instant::now();
    let mut e = Engine::default();
    let all: Vec<_> = (0..257).map(|i| target(i, 1.0)).collect();
    e.reconcile(&all, now, false, false);
    assert_eq!(e.diagnostics().active, MAX_ACTIVE);
    e.reconcile(
        &[Target {
            value: f64::NAN,
            ..target(9, 1.0)
        }],
        now,
        false,
        false,
    );
    assert!(e.diagnostics().rejected > 0);
}
#[test]
fn feedback_groups_are_bounded() {
    let now = Instant::now();
    let mut e = Engine::default();
    let groups: Vec<_> = (0..65)
        .map(|g| FeedbackTarget {
            instance: FeedbackInstance { group: g, owner: g },
            period: Duration::from_millis(10),
        })
        .collect();
    e.reconcile_feedback(&groups, now, false, false);
    assert_eq!(e.diagnostics().feedback_groups, MAX_FEEDBACK_GROUPS);
    assert!(e.diagnostics().rejected > 0);
}

#[test]
fn unchanged_reconcile_does_not_restart_and_removal_reinserts_fresh() {
    let now = Instant::now();
    let mut e = Engine::default();
    let t = target(1, 10.0);
    e.reconcile(&[t], now, false, false);
    e.reconcile(&[t], at(now, 50), false, false);
    e.frame(at(now, 100), false, false);
    assert_eq!(e.value(t.key, at(now, 100)), Some(10.0));
    assert_eq!(e.diagnostics().active, 0);
    e.reconcile(&[], at(now, 100), false, false);
    e.reconcile(&[t], at(now, 100), false, false);
    assert_eq!(e.value(t.key, at(now, 100)), Some(0.0));
}
#[test]
fn extreme_values_zero_duration_and_backwards_clock_are_safe() {
    let now = Instant::now();
    let mut e = Engine::default();
    let mut t = target(1, f64::MAX);
    t.initial = Some(-f64::MAX);
    e.reconcile(&[t], now, false, false);
    e.frame(at(now, 50), false, false);
    assert_eq!(e.value(t.key, at(now, 50)), Some(0.0));
    assert_eq!(e.value(t.key, now), Some(0.0));
    t.duration = Duration::ZERO;
    t.value = 42.0;
    e.reconcile(&[t], at(now, 50), false, false);
    assert_eq!(e.value(t.key, at(now, 50)), Some(42.0));
    assert!(e.frame(at(now, 50), false, false).next_deadline.is_none());
}
#[test]
fn shared_feedback_freezes_and_rejects_incompatible_period() {
    let now = Instant::now();
    let mut e = Engine::default();
    let a = FeedbackTarget {
        instance: FeedbackInstance { group: 7, owner: 1 },
        period: Duration::from_secs(1),
    };
    let b = FeedbackTarget {
        instance: FeedbackInstance { group: 7, owner: 2 },
        ..a
    };
    e.reconcile_feedback(&[a, b], now, false, false);
    assert_eq!(e.feedback_phase(a.instance, at(now, 250)), Some(0.25));
    e.frame(at(now, 250), false, true);
    assert_eq!(e.feedback_phase(b.instance, at(now, 750)), Some(0.25));
    e.frame(at(now, 750), false, false);
    assert_eq!(e.feedback_phase(a.instance, at(now, 1000)), Some(0.5));
    e.reconcile_feedback(
        &[
            a,
            FeedbackTarget {
                period: Duration::from_secs(2),
                ..b
            },
        ],
        at(now, 1000),
        false,
        false,
    );
    assert!(e.feedback_phase(b.instance, at(now, 1000)).is_none());
    assert_eq!(e.diagnostics().feedback_instances, 1);
    e.frame(at(now, 1000), true, false);
    assert_eq!(e.diagnostics().feedback_groups, 0);
}
#[test]
fn duplicate_invalid_and_overflow_inputs_retire_old_work() {
    let now = Instant::now();
    let mut e = Engine::default();
    let t = target(1, 10.0);
    e.reconcile(&[t], now, false, false);
    e.reconcile(&[t, t], at(now, 10), false, false);
    assert!(e.value(t.key, at(now, 10)).is_none());
    e.reconcile(
        &[Target {
            duration: Duration::MAX,
            ..t
        }],
        at(now, 10),
        false,
        false,
    );
    assert_eq!(e.diagnostics().active, 0);
    e.reconcile(
        &[Target {
            initial: Some(f64::NAN),
            ..t
        }],
        at(now, 10),
        false,
        false,
    );
    assert!(e.value(t.key, at(now, 10)).is_none());
}

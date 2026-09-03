#![allow(clippy::arc_with_non_send_sync, missing_docs)]

use radiant::gui::types::Vector2;
use radiant::runtime::testing::{
    DeterministicHost, DeterministicHostConfig, DeterministicTrace, DeterministicTraceCapture,
    DeterministicTraceError, DeterministicTraceIdentity, DeterministicTraceLimits,
    first_divergence,
};
use radiant::runtime::{PlatformResponse, RuntimeBridge, SurfaceNode, UiSurface};
use std::sync::Arc;

#[derive(Default)]
struct MinimalBridge;

impl RuntimeBridge<()> for MinimalBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<()>> {
        Arc::new(UiSurface::new(SurfaceNode::column(1, 0.0, Vec::new())))
    }
}

fn factory(
    count: &mut usize,
) -> impl FnOnce(
    DeterministicHostConfig,
    &serde_json::Value,
    &serde_json::Value,
) -> Result<DeterministicHost<MinimalBridge, ()>, &'static str>
+ '_ {
    move |config, _, _| {
        *count += 1;
        DeterministicHost::new(MinimalBridge, config).map_err(|_| "host")
    }
}

fn initial_snapshot() -> radiant::runtime::testing::NormalizedSnapshot {
    DeterministicHost::new(
        MinimalBridge,
        DeterministicHostConfig::new(Vector2::new(100.0, 80.0)),
    )
    .expect("host")
    .published_snapshot()
    .clone()
}

#[test]
fn public_fixtures_are_canonical_and_replay_through_a_real_host() {
    for fixture in [
        "focus",
        "overlay",
        "async",
        "input-capture",
        "reconciliation",
    ] {
        let bytes = match fixture {
            "focus" => &include_bytes!("fixtures/deterministic_traces/focus.json")[..],
            "overlay" => &include_bytes!("fixtures/deterministic_traces/overlay.json")[..],
            "async" => &include_bytes!("fixtures/deterministic_traces/async.json")[..],
            "input-capture" => {
                &include_bytes!("fixtures/deterministic_traces/input-capture.json")[..]
            }
            _ => &include_bytes!("fixtures/deterministic_traces/reconciliation.json")[..],
        };
        let bytes = bytes.strip_suffix(b"\n").unwrap_or(bytes);
        let trace = DeterministicTrace::from_json_bytes(bytes, DeterministicTraceLimits::default())
            .expect(fixture);
        assert_eq!(trace.to_json_bytes().expect("encode"), bytes);
        let mut invocations = 0;
        let report = trace
            .replay(
                factory(&mut invocations),
                |host, value| {
                    host.set_application_observation(value.clone());
                    Ok::<(), &'static str>(())
                },
                |_| Ok::<_, &'static str>(Ok(PlatformResponse::Completed)),
            )
            .expect("replay");
        assert!(trace.operation_count() > 0);
        assert_eq!(trace.identity().scenario, fixture);
        assert_eq!(report.operations, trace.operation_count());
        assert_eq!(invocations, 1);
    }
}

fn capture_with_limits(limits: DeterministicTraceLimits) -> DeterministicTrace {
    let mut capture = DeterministicTraceCapture::new(
        DeterministicTraceIdentity {
            application: "test-app".into(),
            scenario: "preflight".into(),
        },
        DeterministicHostConfig::new(Vector2::new(100.0, 80.0)),
        serde_json::json!({"state": 1}),
        serde_json::json!({"view": "root"}),
        limits,
    )
    .expect("capture");
    capture
        .publication(initial_snapshot())
        .expect("initial publication");
    capture
        .advance_virtual_time(std::time::Duration::from_nanos(1))
        .expect("operation");
    capture.finish().expect("finish")
}

fn canonical_bytes() -> Vec<u8> {
    capture_with_limits(DeterministicTraceLimits::default())
        .to_json_bytes()
        .unwrap()
}

#[test]
fn replay_rejects_stale_worker_completion_after_factory_creation() {
    let bytes = replace_once(
        &canonical_bytes(),
        "{\"AdvanceVirtualTime\":{\"nanos\":1}}",
        "{\"CompleteWorker\":{\"id\":1}}",
    );
    let trace = DeterministicTrace::from_json_bytes(&bytes, Default::default()).unwrap();
    let mut invocations = 0;

    let result = trace.replay(
        factory(&mut invocations),
        |_, _| Ok::<(), &'static str>(()),
        |_| Ok::<_, &'static str>(Ok(PlatformResponse::Completed)),
    );

    assert!(matches!(result, Err(DeterministicTraceError::Replay(_))));
    assert_eq!(invocations, 1);
}

fn replace_once(base: &[u8], from: &str, to: &str) -> Vec<u8> {
    String::from_utf8(base.to_vec())
        .unwrap()
        .replacen(from, to, 1)
        .into_bytes()
}

fn assert_decode_rejected(bytes: &[u8], limits: DeterministicTraceLimits, expected: &str) {
    let result = DeterministicTrace::from_json_bytes(bytes, limits);
    assert!(format!("{result:?}").contains(expected), "{result:?}");
}

#[test]
fn preflight_rejects_malformed_traces_without_factory_invocation() {
    let valid = canonical_bytes();
    let invocations = 0;
    let assert_no_factory = |bytes: &[u8]| {
        let result =
            DeterministicTrace::from_json_bytes(bytes, DeterministicTraceLimits::default());
        assert!(result.is_err());
        assert_eq!(invocations, 0);
    };

    assert_no_factory(&valid[..valid.len() - 1]);
    let mut unknown = valid[..valid.len() - 1].to_vec();
    unknown.extend_from_slice(b",\"unknown\":true}");
    assert_decode_rejected(&unknown, Default::default(), "Malformed");
    assert_decode_rejected(
        &replace_once(&valid, "radiant.deterministic-trace", "other"),
        Default::default(),
        "WrongFormat",
    );
    assert_decode_rejected(
        &replace_once(&valid, "\"version\":1", "\"version\":99"),
        Default::default(),
        "WrongVersion",
    );
    assert_decode_rejected(
        &valid,
        DeterministicTraceLimits {
            max_operations: 1,
            ..Default::default()
        },
        "operations",
    );

    assert_decode_rejected(
        &replace_once(&valid, "\"viewport\":[100.0", "\"viewport\":[0.0"),
        Default::default(),
        "geometry",
    );
    let zero_completion = replace_once(
        &valid,
        "{\"AdvanceVirtualTime\":{\"nanos\":1}}",
        "{\"CompleteWorker\":{\"id\":0}}",
    );
    assert_decode_rejected(&zero_completion, Default::default(), "zero completion");
    let overflow = replace_once(&valid, "\"nanos\":1", &format!("\"nanos\":{}", u128::MAX));
    assert_decode_rejected(&overflow, Default::default(), "overflow");

    assert_decode_rejected(
        &replace_once(
            &valid,
            "\"initial_state\":{\"state\":1}",
            "\"initial_state\":{\"nested\":{\"value\":1}}",
        ),
        DeterministicTraceLimits {
            max_json_depth: 0,
            ..Default::default()
        },
        "depth",
    );
    assert_decode_rejected(
        &valid,
        DeterministicTraceLimits {
            max_bytes: 1,
            ..Default::default()
        },
        "bytes",
    );
}

#[test]
fn capture_enforces_value_identity_operation_and_snapshot_budgets() {
    let mut capture = DeterministicTraceCapture::new(
        DeterministicTraceIdentity {
            application: "a".into(),
            scenario: "s".into(),
        },
        DeterministicHostConfig::new(Vector2::new(10.0, 10.0)),
        serde_json::Value::Null,
        serde_json::Value::Null,
        DeterministicTraceLimits {
            max_operations: 1,
            ..Default::default()
        },
    )
    .unwrap();
    capture
        .advance_virtual_time(std::time::Duration::ZERO)
        .unwrap();
    assert!(matches!(
        capture.advance_virtual_time(std::time::Duration::ZERO),
        Err(DeterministicTraceError::BudgetExceeded("operations"))
    ));
    let host = DeterministicHost::new(
        MinimalBridge,
        DeterministicHostConfig::new(Vector2::new(10.0, 10.0)),
    )
    .unwrap();
    let mut snapshots = host
        .begin_trace_capture(
            DeterministicTraceIdentity {
                application: "a".into(),
                scenario: "s".into(),
            },
            DeterministicTraceLimits {
                max_snapshots: 1,
                ..Default::default()
            },
        )
        .unwrap();
    assert!(matches!(
        host.capture_publication(&mut snapshots),
        Err(DeterministicTraceError::BudgetExceeded("snapshots"))
    ));
    let identity = DeterministicTraceCapture::new(
        DeterministicTraceIdentity {
            application: "a".into(),
            scenario: "x".repeat(100),
        },
        DeterministicHostConfig::new(Vector2::new(10.0, 10.0)),
        serde_json::Value::Null,
        serde_json::Value::Null,
        DeterministicTraceLimits {
            max_bytes: 100,
            ..Default::default()
        },
    );
    assert!(matches!(
        identity,
        Err(DeterministicTraceError::BudgetExceeded("identity"))
    ));
    let value = DeterministicTraceCapture::new(
        DeterministicTraceIdentity {
            application: "a".into(),
            scenario: "s".into(),
        },
        DeterministicHostConfig::new(Vector2::new(10.0, 10.0)),
        serde_json::json!({"value": "x".repeat(100)}),
        serde_json::Value::Null,
        DeterministicTraceLimits {
            max_bytes: 100,
            ..Default::default()
        },
    );
    assert!(matches!(
        value,
        Err(DeterministicTraceError::BudgetExceeded("value"))
    ));
}

#[test]
fn capture_rejects_incremental_encoded_size_before_retaining_operation() {
    let mut capture = DeterministicTraceCapture::new(
        DeterministicTraceIdentity {
            application: "test-app".into(),
            scenario: "incremental-bytes".into(),
        },
        DeterministicHostConfig::new(Vector2::new(100.0, 80.0)),
        serde_json::Value::Null,
        serde_json::Value::Null,
        DeterministicTraceLimits {
            max_bytes: 4_200,
            ..Default::default()
        },
    )
    .unwrap();
    capture.publication(initial_snapshot()).unwrap();
    assert!(matches!(
        capture.capture_input(serde_json::json!({"payload": "x".repeat(100)})),
        Err(DeterministicTraceError::BudgetExceeded("bytes"))
    ));
    assert_eq!(capture.finish().unwrap().operation_count(), 1);
}

#[test]
fn preflight_rejects_invalid_snapshot_order_and_configuration() {
    let config = DeterministicHostConfig::new(Vector2::new(10.0, 10.0));
    let mut host = DeterministicHost::new(MinimalBridge, config).unwrap();
    host.advance_time(std::time::Duration::from_nanos(1))
        .unwrap();
    let snapshot = host.turn().unwrap();
    let mut capture = DeterministicTraceCapture::new(
        DeterministicTraceIdentity {
            application: "a".into(),
            scenario: "order".into(),
        },
        config,
        serde_json::Value::Null,
        serde_json::Value::Null,
        Default::default(),
    )
    .unwrap();
    capture.publication(initial_snapshot()).unwrap();
    capture
        .advance_virtual_time(std::time::Duration::from_nanos(1))
        .unwrap();
    capture.publication(snapshot).unwrap();
    let bytes = capture.finish().unwrap().to_json_bytes().unwrap();
    let invalid_order = replace_once(
        &bytes,
        "\"virtual_time_nanos\":1",
        "\"virtual_time_nanos\":0",
    );
    assert_decode_rejected(&invalid_order, Default::default(), "clock");
    let invalid_config = replace_once(
        &canonical_bytes(),
        "\"max_pending_workers\":64",
        "\"max_pending_workers\":0",
    );
    let mut invocations = 0;
    let decoded = DeterministicTrace::from_json_bytes(&invalid_config, Default::default());
    assert!(decoded.is_err());
    if let Ok(trace) = decoded {
        let _ = trace.replay(
            factory(&mut invocations),
            |_, _| Ok::<(), &'static str>(()),
            |_| Ok::<_, &'static str>(Ok(PlatformResponse::Completed)),
        );
    }
    assert_eq!(invocations, 0);

    let invalid_time = replace_once(
        &canonical_bytes(),
        "\"nanos\":1",
        &format!("\"nanos\":{}", u128::MAX),
    );
    let decoded = DeterministicTrace::from_json_bytes(&invalid_time, Default::default());
    assert!(decoded.is_err());
    if let Ok(trace) = decoded {
        let _ = trace.replay(
            factory(&mut invocations),
            |_, _| Ok::<(), &'static str>(()),
            |_| Ok::<_, &'static str>(Ok(PlatformResponse::Completed)),
        );
    }
    assert_eq!(invocations, 0);
}

#[test]
fn generated_publication_replays_and_divergence_has_json_path() {
    let config = DeterministicHostConfig::new(Vector2::new(10.0, 10.0));
    let mut host = DeterministicHost::new(MinimalBridge, config).unwrap();
    let initial = host.published_snapshot().clone();
    let mut capture = DeterministicTraceCapture::new(
        DeterministicTraceIdentity {
            application: "a".into(),
            scenario: "publication".into(),
        },
        config,
        serde_json::Value::Null,
        serde_json::Value::Null,
        Default::default(),
    )
    .unwrap();
    capture.publication(initial.clone()).unwrap();
    host.advance_time(std::time::Duration::from_nanos(1))
        .unwrap();
    let published = host.turn().unwrap();
    capture
        .advance_virtual_time(std::time::Duration::from_nanos(1))
        .unwrap();
    capture.publication(published.clone()).unwrap();
    let trace = capture.finish().unwrap();
    let mut invocations = 0;
    let report = trace
        .replay(
            factory(&mut invocations),
            |_, _| Ok::<(), &'static str>(()),
            |_| Ok::<_, &'static str>(Ok(PlatformResponse::Completed)),
        )
        .unwrap();
    assert_eq!((report.operations, report.publications), (3, 2));
    assert_eq!(invocations, 1);
    let mut altered = published.clone();
    altered.application_observation = Some(serde_json::json!({"changed": true}));
    let divergence = first_divergence(&published, &altered, 2, 1).unwrap();
    assert!(divergence.json_path.contains("application_observation"));
}

#[test]
fn preflight_rejects_absent_or_misordered_first_publication_and_duplicate_ids() {
    let valid = canonical_bytes();
    let absent = replace_once(
        &valid,
        "{\"Publication\":{\"snapshot\":",
        "{\"Input\":{\"value\":null}},{\"Publication\":{\"snapshot\":",
    );
    assert_decode_rejected(&absent, Default::default(), "initial publication");
    let misordered = replace_once(
        &valid,
        "\"turn\":0,\"virtual_time_nanos\":0",
        "\"turn\":1,\"virtual_time_nanos\":0",
    );
    assert_decode_rejected(&misordered, Default::default(), "initial publication");

    let duplicate_worker = replace_once(
        &valid,
        "{\"AdvanceVirtualTime\":{\"nanos\":1}}",
        "{\"CompleteWorker\":{\"id\":1}},{\"CompleteWorker\":{\"id\":1}}",
    );
    assert_decode_rejected(&duplicate_worker, Default::default(), "duplicate worker");
    let duplicate_platform = replace_once(
        &valid,
        "{\"AdvanceVirtualTime\":{\"nanos\":1}}",
        "{\"CompletePlatform\":{\"id\":1,\"result\":null}},{\"CompletePlatform\":{\"id\":1,\"result\":null}}",
    );
    assert_decode_rejected(
        &duplicate_platform,
        Default::default(),
        "duplicate platform",
    );
}

#[test]
fn replay_uses_non_default_limits_and_labels_initial_divergence() {
    let limits = DeterministicTraceLimits {
        max_operations: 2,
        max_virtual_time_nanos: 1,
        ..Default::default()
    };
    let trace = capture_with_limits(limits);
    let bytes = trace.to_json_bytes().unwrap();
    let decoded = DeterministicTrace::from_json_bytes(&bytes, limits).unwrap();
    let mut invocations = 0;
    decoded
        .replay(
            factory(&mut invocations),
            |_, _| Ok::<(), &'static str>(()),
            |_| Ok::<_, &'static str>(Ok(PlatformResponse::Completed)),
        )
        .unwrap();
    assert_eq!(invocations, 1);

    let mut expected = initial_snapshot();
    expected.viewport.width += 1.0;
    let mut capture = DeterministicTraceCapture::new(
        DeterministicTraceIdentity {
            application: "a".into(),
            scenario: "initial".into(),
        },
        DeterministicHostConfig::new(Vector2::new(100.0, 80.0)),
        serde_json::Value::Null,
        serde_json::Value::Null,
        Default::default(),
    )
    .unwrap();
    capture.publication(expected).unwrap();
    let trace = capture.finish().unwrap();
    let error = trace
        .replay(
            factory(&mut invocations),
            |_, _| Ok::<(), &'static str>(()),
            |_| Ok::<_, &'static str>(Ok(PlatformResponse::Completed)),
        )
        .unwrap_err();
    match error {
        DeterministicTraceError::Diverged(divergence) => {
            assert_eq!(divergence.boundary, "initial");
            assert_eq!(divergence.publication, None);
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn host_config_validation_remains_public_and_bounded() {
    let base = DeterministicHostConfig::new(Vector2::new(10.0, 10.0));
    assert!(base.with_max_pending_workers(0).validate().is_err());
    assert!(
        base.with_max_pending_timers(2)
            .with_max_pending_queue_items(1)
            .validate()
            .is_err()
    );
}

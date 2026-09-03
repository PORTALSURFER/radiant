#![allow(clippy::arc_with_non_send_sync, missing_docs)]

use radiant::gui::types::{Point, Vector2};
use radiant::prelude::UiUpdateContext;
use radiant::runtime::testing::{
    DeterministicHost, DeterministicHostConfig, DeterministicTrace, DeterministicTraceCapture,
    DeterministicTraceError, DeterministicTraceIdentity, DeterministicTraceLimits,
    NormalizedSnapshot, first_divergence,
};
use radiant::runtime::{
    Command, Event, FocusTraversal, LayerKind, PlatformResponse, RuntimeBridge, SurfaceChild,
    SurfaceLayer, SurfaceNode, UiSurface,
};
use radiant::widgets::{WidgetSizing, WidgetStyle};
use serde::Deserialize;
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FixtureScenario {
    Focus,
    Overlay,
    Async,
    InputCapture,
    Reconciliation,
}

impl FixtureScenario {
    fn from_name(name: &str) -> Result<Self, String> {
        match name {
            "focus" => Ok(Self::Focus),
            "overlay" => Ok(Self::Overlay),
            "async" => Ok(Self::Async),
            "input-capture" => Ok(Self::InputCapture),
            "reconciliation" => Ok(Self::Reconciliation),
            other => Err(format!("unknown fixture scenario: {other}")),
        }
    }

    const fn name(self) -> &'static str {
        match self {
            Self::Focus => "focus",
            Self::Overlay => "overlay",
            Self::Async => "async",
            Self::InputCapture => "input-capture",
            Self::Reconciliation => "reconciliation",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum FixtureMessage {
    FocusValue(String),
    ShowOverlay,
    CaptureActivated,
    WorkerCompleted(String),
    Reconcile,
}

struct FixtureBridge {
    scenario: FixtureScenario,
    focus_value: String,
    overlay_visible: bool,
    capture_activations: usize,
    worker_result: Option<String>,
    reconciled: bool,
    events: Option<Arc<Mutex<Vec<FixtureMessage>>>>,
}

impl FixtureBridge {
    fn new(scenario: FixtureScenario) -> Self {
        Self {
            scenario,
            focus_value: String::from("seed"),
            overlay_visible: false,
            capture_activations: 0,
            worker_result: None,
            reconciled: false,
            events: None,
        }
    }

    fn with_events(mut self, events: Arc<Mutex<Vec<FixtureMessage>>>) -> Self {
        self.events = Some(events);
        self
    }
}

impl RuntimeBridge<FixtureMessage> for FixtureBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<FixtureMessage>> {
        let root = match self.scenario {
            FixtureScenario::Focus => SurfaceNode::column(
                1,
                8.0,
                vec![
                    SurfaceChild::fill(SurfaceNode::text_input(
                        10,
                        self.focus_value.clone(),
                        WidgetSizing::fixed(Vector2::new(180.0, 32.0)),
                        FixtureMessage::FocusValue,
                    )),
                    SurfaceChild::fill(SurfaceNode::text(
                        11,
                        format!("Focused value: {}", self.focus_value),
                        WidgetSizing::fixed(Vector2::new(220.0, 24.0)),
                    )),
                ],
            ),
            FixtureScenario::Overlay => {
                let base = SurfaceNode::column(
                    1,
                    8.0,
                    vec![
                        SurfaceChild::fill(SurfaceNode::button(
                            20,
                            "Show overlay",
                            WidgetSizing::fixed(Vector2::new(180.0, 36.0)),
                            FixtureMessage::ShowOverlay,
                        )),
                        SurfaceChild::fill(SurfaceNode::text(
                            21,
                            format!("Overlay visible: {}", self.overlay_visible),
                            WidgetSizing::fixed(Vector2::new(220.0, 24.0)),
                        )),
                    ],
                );
                let layers = if self.overlay_visible {
                    vec![SurfaceLayer::new(
                        LayerKind::Popover,
                        SurfaceNode::overlay_panel(
                            90,
                            radiant::gui::types::Rect::from_min_size(
                                Point::new(24.0, 64.0),
                                Vector2::new(180.0, 44.0),
                            ),
                            "Overlay active",
                            WidgetStyle::default(),
                        ),
                    )]
                } else {
                    Vec::new()
                };
                SurfaceNode::scene(100, base, layers)
            }
            FixtureScenario::Async => SurfaceNode::column(
                1,
                8.0,
                vec![
                    SurfaceChild::fill(SurfaceNode::text(
                        31,
                        format!(
                            "Worker result: {}",
                            self.worker_result.as_deref().unwrap_or("pending")
                        ),
                        WidgetSizing::fixed(Vector2::new(240.0, 28.0)),
                    )),
                    SurfaceChild::fill(SurfaceNode::text(
                        32,
                        "Completion is published on a later turn",
                        WidgetSizing::fixed(Vector2::new(260.0, 24.0)),
                    )),
                ],
            ),
            FixtureScenario::InputCapture => SurfaceNode::column(
                1,
                8.0,
                vec![
                    SurfaceChild::fill(SurfaceNode::button(
                        30,
                        format!("Captured presses: {}", self.capture_activations),
                        WidgetSizing::fixed(Vector2::new(210.0, 36.0)),
                        FixtureMessage::CaptureActivated,
                    )),
                    SurfaceChild::fill(SurfaceNode::text(
                        31,
                        "Pointer ownership is retained during the press",
                        WidgetSizing::fixed(Vector2::new(280.0, 24.0)),
                    )),
                ],
            ),
            FixtureScenario::Reconciliation => {
                let child = if self.reconciled {
                    SurfaceNode::text(
                        40,
                        "Reconciled surface",
                        WidgetSizing::fixed(Vector2::new(220.0, 36.0)),
                    )
                } else {
                    SurfaceNode::button(
                        40,
                        "Replace surface",
                        WidgetSizing::fixed(Vector2::new(220.0, 36.0)),
                        FixtureMessage::Reconcile,
                    )
                };
                SurfaceNode::column(1, 0.0, vec![SurfaceChild::fill(child)])
            }
        };
        Arc::new(UiSurface::new(root))
    }

    fn update(&mut self, message: FixtureMessage) -> Command<FixtureMessage> {
        if let Some(events) = &self.events {
            events
                .lock()
                .expect("fixture event log")
                .push(message.clone());
        }
        match message {
            FixtureMessage::FocusValue(value) => self.focus_value = value,
            FixtureMessage::ShowOverlay => self.overlay_visible = true,
            FixtureMessage::CaptureActivated => self.capture_activations += 1,
            FixtureMessage::WorkerCompleted(result) => self.worker_result = Some(result),
            FixtureMessage::Reconcile => self.reconciled = true,
        }
        Command::none()
    }
}

#[derive(Debug, Deserialize)]
struct FixtureSeed {
    scenario: String,
}

#[derive(Debug, Deserialize)]
struct FixtureViewIdentity {
    root: String,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
enum FixtureAction {
    FocusAndType { character: char },
    ShowOverlay,
    PointerPress { x: f32, y: f32 },
    StartWorker,
    Reconcile,
}

fn fixture_config() -> DeterministicHostConfig {
    DeterministicHostConfig::new(Vector2::new(320.0, 160.0))
}

fn fixture_seed(scenario: FixtureScenario) -> Value {
    json!({"scenario": scenario.name()})
}

fn fixture_view_identity() -> Value {
    json!({"root": "deterministic-host-surface"})
}

fn fixture_action_value(scenario: FixtureScenario) -> Value {
    match scenario {
        FixtureScenario::Focus => json!({"action":"focus_and_type","character":"R"}),
        FixtureScenario::Overlay => json!({"action":"show_overlay"}),
        FixtureScenario::Async => json!({"action":"start_worker"}),
        FixtureScenario::InputCapture => {
            json!({"action":"pointer_press","x":80.0,"y":18.0})
        }
        FixtureScenario::Reconciliation => json!({"action":"reconcile"}),
    }
}

fn worker_command() -> Command<FixtureMessage> {
    let mut context: UiUpdateContext<FixtureMessage> = UiUpdateContext::default();
    context
        .business()
        .background("deterministic-fixture-worker")
        .run(|_| String::from("ready"), FixtureMessage::WorkerCompleted);
    context.into_command()
}

fn apply_fixture_action(
    host: &mut DeterministicHost<FixtureBridge, FixtureMessage>,
    value: &Value,
) -> Result<FixtureAction, String> {
    let action: FixtureAction =
        serde_json::from_value(value.clone()).map_err(|error| error.to_string())?;
    match &action {
        FixtureAction::FocusAndType { character } => {
            host.dispatch_event(Event::traverse_focus(FocusTraversal::Forward))
                .map_err(|error| error.to_string())?;
            host.dispatch_event(Event::character(*character))
                .map_err(|error| error.to_string())?;
            host.set_application_observation(json!({
                "action": "focus_and_type",
                "focused": "name",
                "character": character.to_string(),
            }));
        }
        FixtureAction::ShowOverlay => {
            let point = Point::new(80.0, 18.0);
            let press = host
                .dispatch_event(Event::primary_press(point))
                .map_err(|error| error.to_string())?;
            if press != Some(20) {
                return Err(format!("overlay press routed to {press:?}"));
            }
            let release = host
                .dispatch_event(Event::primary_release(point))
                .map_err(|error| error.to_string())?;
            if release != Some(20) {
                return Err(format!("overlay release routed to {release:?}"));
            }
            if !host.bridge().overlay_visible {
                return Err("overlay button did not update bridge state".into());
            }
            host.set_application_observation(json!({
                "action": "show_overlay",
                "trigger": "button_activation",
            }));
        }
        FixtureAction::PointerPress { x, y } => {
            let point = Point::new(*x, *y);
            let target = host
                .dispatch_event(Event::primary_press(point))
                .map_err(|error| error.to_string())?;
            if target != Some(30) {
                return Err(format!("capture press routed to {target:?}"));
            }
            host.set_application_observation(json!({
                "action": "pointer_press",
                "position": [*x, *y],
                "capture": "button",
            }));
        }
        FixtureAction::StartWorker => {
            host.execute_command(worker_command())
                .map_err(|error| error.to_string())?;
            host.set_application_observation(json!({
                "action": "start_worker",
                "completion": "explicit",
            }));
        }
        FixtureAction::Reconcile => {
            let point = Point::new(80.0, 18.0);
            let press = host
                .dispatch_event(Event::primary_press(point))
                .map_err(|error| error.to_string())?;
            if press != Some(40) {
                return Err(format!("reconcile press routed to {press:?}"));
            }
            let release = host
                .dispatch_event(Event::primary_release(point))
                .map_err(|error| error.to_string())?;
            if release != Some(40) {
                return Err(format!("reconcile release routed to {release:?}"));
            }
            host.set_application_observation(json!({
                "action": "reconcile",
                "transition": "button_to_text",
            }));
        }
    }
    Ok(action)
}

fn assert_replay_candidate(
    action: &FixtureAction,
    snapshot: &NormalizedSnapshot,
) -> Result<(), String> {
    if snapshot.layout.rects.len() < 2 {
        return Err("fixture action produced an empty surface".into());
    }
    if snapshot.application_observation.is_none() {
        return Err("fixture action did not retain an application observation".into());
    }
    match action {
        FixtureAction::FocusAndType { .. } => {
            if snapshot.focus.focused_widget != Some(10) || snapshot.paint.total == 0 {
                return Err("focus action did not produce focus and paint state".into());
            }
        }
        FixtureAction::ShowOverlay => {
            if !snapshot.layout.rects.iter().any(|rect| rect.node_id == 90)
                || snapshot.paint.text < 3
            {
                return Err(format!(
                    "overlay action did not produce a painted overlay layer: paint={:?}, layout={:?}",
                    snapshot.paint, snapshot.layout.rects
                ));
            }
        }
        FixtureAction::PointerPress { .. } => {
            if snapshot.focus.pointer_capture != Some(30) {
                return Err("pointer press did not retain pointer capture".into());
            }
        }
        FixtureAction::StartWorker => {
            if snapshot.pending.workers.len() != 1 {
                return Err("worker action did not leave one pending worker".into());
            }
        }
        FixtureAction::Reconcile => {
            if snapshot.refresh.identity.replacement_count == 0 {
                return Err("reconcile action did not report identity replacement".into());
            }
        }
    }
    Ok(())
}

struct GeneratedFixture {
    trace: DeterministicTrace,
    snapshots: Vec<NormalizedSnapshot>,
}

fn generate_fixture(scenario: FixtureScenario) -> GeneratedFixture {
    let config = fixture_config();
    let mut host = DeterministicHost::new(FixtureBridge::new(scenario), config).expect("host");
    let mut capture = DeterministicTraceCapture::new(
        DeterministicTraceIdentity {
            application: "radiant-fixture".into(),
            scenario: scenario.name().into(),
        },
        config,
        fixture_seed(scenario),
        fixture_view_identity(),
        DeterministicTraceLimits::default(),
    )
    .expect("capture");
    let mut snapshots = vec![host.published_snapshot().clone()];
    capture
        .publication(host.published_snapshot().clone())
        .expect("initial publication");

    let action = fixture_action_value(scenario);
    capture.capture_input(action.clone()).expect("input");
    let action = apply_fixture_action(&mut host, &action).expect("action");
    let candidate = host.snapshot().expect("candidate snapshot");
    assert_replay_candidate(&action, &candidate).expect("candidate behavior");
    let published = host.turn().expect("turn");
    capture.publication(published.clone()).expect("publication");
    snapshots.push(published);

    if scenario == FixtureScenario::Async {
        let pending = host.pending_worker_tasks();
        assert_eq!(pending.len(), 1, "async fixture should queue a worker");
        let worker_id = pending[0].id;
        capture
            .complete_worker(worker_id)
            .expect("completion operation");
        host.complete_worker(worker_id).expect("worker completion");
        let completed = host.turn().expect("completion turn");
        capture
            .publication(completed.clone())
            .expect("completion publication");
        snapshots.push(completed);
    }

    GeneratedFixture {
        trace: capture.finish().expect("finish"),
        snapshots,
    }
}

fn fixture_factory(
    count: &mut usize,
    events: Arc<Mutex<Vec<FixtureMessage>>>,
) -> impl FnOnce(
    DeterministicHostConfig,
    &Value,
    &Value,
) -> Result<DeterministicHost<FixtureBridge, FixtureMessage>, String>
+ '_ {
    move |config, initial_state, initial_view_identity| {
        *count += 1;
        let seed: FixtureSeed =
            serde_json::from_value(initial_state.clone()).map_err(|error| error.to_string())?;
        let view: FixtureViewIdentity = serde_json::from_value(initial_view_identity.clone())
            .map_err(|error| error.to_string())?;
        if view.root != "deterministic-host-surface" {
            return Err("unexpected fixture view identity".into());
        }
        let scenario = FixtureScenario::from_name(&seed.scenario)?;
        DeterministicHost::new(
            FixtureBridge::new(scenario).with_events(Arc::clone(&events)),
            config,
        )
        .map_err(|error| error.to_string())
    }
}

fn fixture_bytes(scenario: FixtureScenario) -> &'static [u8] {
    match scenario {
        FixtureScenario::Focus => include_bytes!("fixtures/deterministic_traces/focus.json"),
        FixtureScenario::Overlay => include_bytes!("fixtures/deterministic_traces/overlay.json"),
        FixtureScenario::Async => include_bytes!("fixtures/deterministic_traces/async.json"),
        FixtureScenario::InputCapture => {
            include_bytes!("fixtures/deterministic_traces/input-capture.json")
        }
        FixtureScenario::Reconciliation => {
            include_bytes!("fixtures/deterministic_traces/reconciliation.json")
        }
    }
}

fn assert_generated_behavior(scenario: FixtureScenario, snapshots: &[NormalizedSnapshot]) {
    assert!(snapshots[0].layout.rects.len() >= 2);
    assert!(snapshots[1].application_observation.is_some());
    match scenario {
        FixtureScenario::Focus => {
            assert_eq!(snapshots[1].focus.focused_widget, Some(10));
            assert!(snapshots[1].paint.total > 0);
        }
        FixtureScenario::Overlay => {
            assert!(
                snapshots[1]
                    .layout
                    .rects
                    .iter()
                    .any(|rect| rect.node_id == 90)
            );
            assert!(snapshots[1].paint.text >= 3);
            assert!(snapshots[1].automation_targets.targets.len() >= 3);
        }
        FixtureScenario::Async => {
            assert_eq!(snapshots[1].pending.workers.len(), 1);
            assert!(snapshots[2].pending.workers.is_empty());
            assert!(
                snapshots[2]
                    .automation_targets
                    .targets
                    .iter()
                    .any(|target| { target.label.as_deref() == Some("Worker result: ready") })
            );
        }
        FixtureScenario::InputCapture => {
            assert_eq!(snapshots[1].focus.focused_widget, Some(30));
            assert_eq!(snapshots[1].focus.pointer_capture, Some(30));
            assert!(snapshots[1].focus.current_pointer_position.is_some());
        }
        FixtureScenario::Reconciliation => {
            assert!(snapshots[1].refresh.identity.replacement_count > 0);
            assert!(snapshots[1].paint.total > 0);
            assert_eq!(snapshots[1].focus.focused_widget, None);
        }
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
    for scenario in [
        FixtureScenario::Focus,
        FixtureScenario::Overlay,
        FixtureScenario::Async,
        FixtureScenario::InputCapture,
        FixtureScenario::Reconciliation,
    ] {
        let fixture = scenario.name();
        let bytes = fixture_bytes(scenario);
        assert!(
            !bytes.ends_with(b"\n"),
            "{fixture} fixture must be terminal-newline free"
        );
        let trace = DeterministicTrace::from_json_bytes(bytes, DeterministicTraceLimits::default())
            .expect(fixture);
        assert_eq!(trace.to_json_bytes().expect("encode"), bytes);
        let generated = generate_fixture(scenario);
        assert_eq!(
            generated.trace.to_json_bytes().expect("generated encode"),
            bytes,
            "{fixture} fixture must be generated from the production host"
        );
        assert_generated_behavior(scenario, &generated.snapshots);

        let replay_events = Arc::new(Mutex::new(Vec::new()));
        let mut invocations = 0;
        let report = trace
            .replay(
                fixture_factory(&mut invocations, Arc::clone(&replay_events)),
                |host, value| {
                    let action = apply_fixture_action(host, value)?;
                    let snapshot = host.snapshot().map_err(|error| error.to_string())?;
                    assert_replay_candidate(&action, &snapshot)
                },
                |_| Ok::<_, String>(Ok(PlatformResponse::Completed)),
            )
            .expect("replay");
        assert!(trace.operation_count() > 0);
        assert_eq!(trace.identity().scenario, fixture);
        assert_eq!(report.operations, trace.operation_count());
        assert_eq!(invocations, 1);
        let events = replay_events.lock().expect("fixture event log");
        match scenario {
            FixtureScenario::Focus => {
                assert_eq!(events.len(), 1);
                assert!(
                    matches!(&events[0], FixtureMessage::FocusValue(value) if value == "seedR")
                );
            }
            FixtureScenario::Overlay => {
                assert!(
                    events
                        .iter()
                        .any(|event| matches!(event, FixtureMessage::ShowOverlay))
                )
            }
            FixtureScenario::Async => assert!(events.iter().any(|event| matches!(
                event,
                FixtureMessage::WorkerCompleted(value) if value == "ready"
            ))),
            FixtureScenario::InputCapture => assert!(events.is_empty()),
            FixtureScenario::Reconciliation => {
                assert!(
                    events
                        .iter()
                        .any(|event| matches!(event, FixtureMessage::Reconcile))
                )
            }
        }
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
    let application_observation_key = "key/with~tokens";
    let mut expected = published.clone();
    expected.application_observation = Some(json!({application_observation_key: "expected"}));
    altered.application_observation = Some(json!({application_observation_key: "actual"}));
    let divergence = first_divergence(&expected, &altered, 2, 1).unwrap();
    assert_eq!(
        divergence.json_path,
        "/application_observation/key~1with~0tokens"
    );
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

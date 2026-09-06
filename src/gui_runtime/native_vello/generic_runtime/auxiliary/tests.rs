use super::super::frame_scheduler_policy::NativeInputStageDisposition;
use super::super::frame_stage_admission::FrameStageBudgetBinding;
use super::super::native_discrete_input_stage::{
    NativeDiscreteInputKind, NativeDiscreteInputStageEvidence,
    admit_native_discrete_input_with_budget,
};
use super::super::native_immediate_transient_stage::{
    NativeImmediateTransientKind, NativeImmediateTransientStageEvidence,
    admit_native_immediate_transient_with_budget,
};
use super::super::runner_state::NativeTargetGeneration;
use super::super::{NativeLifecycle, native_lifecycle_stage};
use super::{
    AuxiliaryNativeDiscreteInputRoute, AuxiliaryNativeWindow, AuxiliaryRecoveryOpportunity,
    AuxiliarySurfaceBridge, AuxiliaryWindowEventResult, FrameScheduleKey, FrameWork,
    FrameWorkReason, GenericNativeVelloRunner, GenericRouteOutcome, NativeAdapterGeneration,
    NativeFrameRenderFailure, NativeGenericRunError, NativeResourceMaintenanceTurn,
    SceneRebuildMode, append_initialized_auxiliary_window, auxiliary_key_is_retiring,
    auxiliary_key_is_suppressed_for_sync, auxiliary_keys_removed_during_sync,
    auxiliary_projection_contains_key, auxiliary_redraw_terminal_cause,
    take_deferred_auxiliary_recovery_failure_cause,
};
use crate::gui::{input::InputTimestamp, types::Vector2};
use crate::{
    application::empty,
    gui_runtime::NativeRunOptions,
    prelude::IntoView,
    runtime::{
        AuxiliaryFocusCommand, AuxiliaryFocusRequest, AuxiliaryWindow, AuxiliaryWindowOwner,
        Command, NativeFrameDiagnostics, NativeImeAdapterObservation,
        NativeWindowDiagnosticIdentity, RuntimeBridge, RuntimeFrameDiagnosticsHost,
        RuntimeHostCapabilities, RuntimeNativeImeAdapterObserver, SurfaceNode, UiSurface,
        WidgetMessageMapper,
    },
    widgets::{InteractiveRowWidget, WidgetSizing},
};
use native_lifecycle_stage::{NativeLifecycleStageEvidence, NativeLifecycleTransitionKind};
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use winit::window::WindowId;

fn auxiliary_input_route(
    window: &mut AuxiliaryNativeWindow<i32>,
    kind: NativeDiscreteInputKind,
    outcome: Option<GenericRouteOutcome>,
) -> AuxiliaryNativeDiscreteInputRoute {
    auxiliary_input_route_with_budget(
        window,
        kind,
        outcome,
        FrameStageBudgetBinding::not_budgeted(),
    )
}

fn auxiliary_input_route_with_budget(
    window: &mut AuxiliaryNativeWindow<i32>,
    kind: NativeDiscreteInputKind,
    outcome: Option<GenericRouteOutcome>,
    budget: FrameStageBudgetBinding,
) -> AuxiliaryNativeDiscreteInputRoute {
    let generation = NativeAdapterGeneration::from_test_serial(1);
    let key = FrameScheduleKey::Auxiliary(window.key.clone());
    let ticket = admit_native_discrete_input_with_budget(
        &mut window.runner.frame_stage_owner,
        NativeDiscreteInputStageEvidence {
            key,
            kind,
            timestamp: InputTimestamp::capture(),
            window_id: Some(WindowId::dummy()),
            adapter_generation: generation,
            active_resource_generation: Some(generation),
            target_generation: NativeTargetGeneration::from_test_serial(1),
            native_surface_target_fenced: false,
            lifecycle: NativeLifecycle::default(),
            native_window_eligible: true,
            wrapper_eligible: true,
        },
        budget,
    )
    .expect("auxiliary input ticket");
    AuxiliaryNativeDiscreteInputRoute { ticket, outcome }
}

fn auxiliary_transient_route_with_budget(
    window: &mut AuxiliaryNativeWindow<i32>,
    kind: NativeImmediateTransientKind,
    outcome: GenericRouteOutcome,
    budget: FrameStageBudgetBinding,
) -> super::AuxiliaryNativeImmediateTransientRoute {
    let generation = NativeAdapterGeneration::from_test_serial(1);
    let key = FrameScheduleKey::Auxiliary(window.key.clone());
    let ticket = admit_native_immediate_transient_with_budget(
        &mut window.runner.frame_stage_owner,
        NativeImmediateTransientStageEvidence {
            key,
            kind,
            timestamp: InputTimestamp::capture(),
            window_id: Some(WindowId::dummy()),
            adapter_generation: generation,
            active_resource_generation: Some(generation),
            target_generation: NativeTargetGeneration::from_test_serial(1),
            native_surface_target_fenced: false,
            lifecycle: NativeLifecycle::default(),
            native_window_eligible: true,
            wrapper_eligible: true,
        },
        budget,
    )
    .expect("auxiliary transient ticket");
    super::AuxiliaryNativeImmediateTransientRoute {
        ticket,
        kind: super::AuxiliaryNativeImmediateTransientRouteKind::Focused {
            outcome,
            launch_external_drag: false,
        },
    }
}

fn auxiliary_window_with_diagnostics(
    cache_on_close: bool,
    frame_diagnostics_enabled: bool,
) -> AuxiliaryNativeWindow<i32> {
    let surface = crate::runtime::test_arc_surface(empty::<i32>().into_surface());
    let projection =
        AuxiliaryWindow::new("settings", NativeRunOptions::default(), surface).on_close(7);
    let projection = if cache_on_close {
        projection.cache_on_close()
    } else {
        projection
    };
    AuxiliaryNativeWindow::new(
        projection,
        &NativeRunOptions::default(),
        Some(NativeWindowDiagnosticIdentity::from_runtime_value(2)),
        frame_diagnostics_enabled,
        false,
    )
}

fn auxiliary_window(cache_on_close: bool) -> AuxiliaryNativeWindow<i32> {
    auxiliary_window_with_diagnostics(cache_on_close, false)
}

type PublishedNativeImeAdapterObservations = Arc<Mutex<Vec<NativeImeAdapterObservation>>>;

struct RecordingNativeImeAdapterParentBridge {
    published: PublishedNativeImeAdapterObservations,
}

impl RuntimeBridge<i32> for RecordingNativeImeAdapterParentBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<i32>> {
        crate::runtime::test_arc_surface(empty::<i32>().into_surface())
    }

    fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, i32> {
        RuntimeHostCapabilities::new().with_native_ime_adapter_observer()
    }
}

impl RuntimeNativeImeAdapterObserver for RecordingNativeImeAdapterParentBridge {
    fn observe_native_ime_adapter(&mut self, observation: NativeImeAdapterObservation) {
        self.published
            .lock()
            .expect("IME adapter parent publication events should not be poisoned")
            .push(observation);
    }
}

fn ime_observer_parent(
    published: PublishedNativeImeAdapterObservations,
) -> GenericNativeVelloRunner<RecordingNativeImeAdapterParentBridge, i32> {
    GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        RecordingNativeImeAdapterParentBridge { published },
        Vector2::new(1280.0, 720.0),
    )
}

fn initialized_auxiliary_with_ime_observation(identity: u64) -> AuxiliaryNativeWindow<i32> {
    let mut window = auxiliary_window(false);
    window.runner.native_ime_adapter_observation = Some(NativeImeAdapterObservation {
        window_identity: Some(NativeWindowDiagnosticIdentity::from_runtime_value(identity)),
        ..NativeImeAdapterObservation::default()
    });
    window
}

#[test]
fn auxiliary_ime_observation_transfers_once_only_after_parent_append() {
    // Native runner fixtures retain recursive surface/runtime state. Keep
    // this lifecycle boundary test on the established large test stack.
    std::thread::Builder::new()
        .name("auxiliary-ime-observation-transfer".to_owned())
        .stack_size(16 * 1024 * 1024)
        .spawn(|| {
            let published = Arc::new(Mutex::new(Vec::new()));
            let mut parent = ime_observer_parent(Arc::clone(&published));

            parent
                .append_initialized_auxiliary_window_with_ime_observation(Ok(
                    initialized_auxiliary_with_ime_observation(2),
                ))
                .expect("initialized child should be admitted");
            assert_eq!(parent.auxiliary_windows.len(), 1);
            assert_eq!(
                *published
                    .lock()
                    .expect("IME adapter parent publication events should not be poisoned"),
                vec![NativeImeAdapterObservation {
                    window_identity: Some(NativeWindowDiagnosticIdentity::from_runtime_value(2)),
                    ..NativeImeAdapterObservation::default()
                }]
            );

            // A second parent-boundary pass does not replay the admitted child's
            // observation, because transfer consumed the child's pending value.
            let admitted = parent
                .auxiliary_windows
                .last_mut()
                .expect("admitted child should remain resident");
            assert_eq!(admitted.take_native_ime_adapter_observation(), None);
        })
        .expect("IME observation transfer thread should spawn")
        .join()
        .expect("IME observation transfer lifecycle should complete");
}

#[test]
fn auxiliary_ime_observation_discards_failed_admission_and_replacement_gets_new_identity() {
    // Native runner fixtures retain recursive surface/runtime state. Keep
    // this lifecycle boundary test on the established large test stack.
    std::thread::Builder::new()
        .name("auxiliary-ime-observation".to_owned())
        .stack_size(16 * 1024 * 1024)
        .spawn(|| {
            let published = Arc::new(Mutex::new(Vec::new()));
            let mut parent = ime_observer_parent(Arc::clone(&published));
            let failure = NativeGenericRunError::RenderDeviceLost(String::from("fixture failure"));

            assert_eq!(
                parent
                    .append_initialized_auxiliary_window_with_ime_observation(Err(failure.clone()))
                    .err(),
                Some(failure)
            );
            assert!(parent.auxiliary_windows.is_empty());
            assert!(
                published
                    .lock()
                    .expect("IME adapter parent publication events should not be poisoned")
                    .is_empty()
            );

            parent
                .append_initialized_auxiliary_window_with_ime_observation(Ok(
                    initialized_auxiliary_with_ime_observation(2),
                ))
                .expect("first child should be admitted");
            parent.auxiliary_windows.clear();
            parent
                .append_initialized_auxiliary_window_with_ime_observation(Ok(
                    initialized_auxiliary_with_ime_observation(3),
                ))
                .expect("replacement child should be admitted");

            assert_eq!(
                published
                    .lock()
                    .expect("IME adapter parent publication events should not be poisoned")
                    .iter()
                    .map(|observation| observation.window_identity)
                    .collect::<Vec<_>>(),
                vec![
                    Some(NativeWindowDiagnosticIdentity::from_runtime_value(2)),
                    Some(NativeWindowDiagnosticIdentity::from_runtime_value(3)),
                ]
            );
        })
        .expect("IME observation thread should spawn")
        .join()
        .expect("IME observation lifecycle should complete");
}

fn focusable_auxiliary_projection(key: &str, widget_id: u64) -> AuxiliaryWindow<i32> {
    let surface = crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::widget(
        InteractiveRowWidget::new(widget_id, WidgetSizing::fixed(Vector2::new(160.0, 28.0))),
        WidgetMessageMapper::none(),
    )));
    AuxiliaryWindow::new(key, NativeRunOptions::default(), surface)
}

fn focusable_auxiliary_window(
    key: &str,
    owner: AuxiliaryWindowOwner,
    widget_id: u64,
) -> AuxiliaryNativeWindow<i32> {
    AuxiliaryNativeWindow::new_with_owner(
        focusable_auxiliary_projection(key, widget_id),
        &NativeRunOptions::default(),
        None,
        false,
        false,
        false,
        owner,
    )
}

struct FocusParentBridge;

impl RuntimeBridge<i32> for FocusParentBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<i32>> {
        crate::runtime::test_arc_surface(empty::<i32>().into_surface())
    }

    fn update(&mut self, message: i32) -> Command<i32> {
        match message {
            1 => Command::focus(43),
            2 => Command::clear_focus(),
            3 => Command::batch([
                Command::focus(43),
                Command::clear_focus(),
                Command::focus(43),
            ]),
            _ => Command::none(),
        }
    }

    fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, i32> {
        RuntimeHostCapabilities::new()
    }
}

fn auxiliary_focus_parent() -> GenericNativeVelloRunner<FocusParentBridge, i32> {
    GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        FocusParentBridge,
        Vector2::new(1280.0, 720.0),
    )
}

fn execute_pending_auxiliary_focus_for_test<Bridge>(
    parent: &mut GenericNativeVelloRunner<Bridge, i32>,
) where
    Bridge: RuntimeBridge<i32>,
{
    parent.for_each_pending_auxiliary_focus_request(|window, request| {
        let _ = window.execute_auxiliary_focus_request(request);
    });
}

#[test]
fn auxiliary_focus_targets_exact_child_after_projection_sync() {
    let mut parent = auxiliary_focus_parent();
    let settings_owner = parent
        .core
        .runtime
        .acquire_auxiliary_effect_owner("settings");
    let inspector_owner = parent
        .core
        .runtime
        .acquire_auxiliary_effect_owner("inspector");
    let mut settings = focusable_auxiliary_window("settings", settings_owner.clone(), 43);
    settings.update_projection(focusable_auxiliary_projection("settings", 43));
    parent.auxiliary_windows.push(settings);
    parent
        .auxiliary_windows
        .push(focusable_auxiliary_window("inspector", inspector_owner, 43));

    let reduced = parent.reduce_auxiliary_messages(Some(settings_owner.clone()), vec![1]);
    assert!(reduced.routed);
    execute_pending_auxiliary_focus_for_test(&mut parent);

    assert_eq!(
        parent.auxiliary_focus_request_target_index(&AuxiliaryFocusRequest::new(
            settings_owner,
            AuxiliaryFocusCommand::Focus(43),
        )),
        Some(0),
        "the generation fence must select the exact projected child"
    );
    assert_eq!(
        parent.auxiliary_windows[0]
            .runner
            .core
            .runtime
            .focused_widget(),
        Some(43)
    );
    assert_eq!(
        parent.auxiliary_windows[1]
            .runner
            .core
            .runtime
            .focused_widget(),
        None
    );
    assert_eq!(parent.core.runtime.focused_widget(), None);
}

#[test]
fn auxiliary_clear_focus_isolated_by_generation_with_duplicate_widget_ids() {
    let mut parent = auxiliary_focus_parent();
    let settings_owner = parent
        .core
        .runtime
        .acquire_auxiliary_effect_owner("settings");
    let inspector_owner = parent
        .core
        .runtime
        .acquire_auxiliary_effect_owner("inspector");
    parent.auxiliary_windows.push(focusable_auxiliary_window(
        "settings",
        settings_owner.clone(),
        43,
    ));
    parent.auxiliary_windows.push(focusable_auxiliary_window(
        "inspector",
        inspector_owner.clone(),
        43,
    ));

    parent.auxiliary_windows[0]
        .runner
        .core
        .runtime
        .focus_widget(43);
    parent.auxiliary_windows[1]
        .runner
        .core
        .runtime
        .focus_widget(43);
    parent
        .core
        .runtime
        .enqueue_auxiliary_focus_request(settings_owner, AuxiliaryFocusCommand::Clear);
    execute_pending_auxiliary_focus_for_test(&mut parent);

    assert_eq!(
        parent.auxiliary_windows[0]
            .runner
            .core
            .runtime
            .focused_widget(),
        None
    );
    assert_eq!(
        parent.auxiliary_windows[1]
            .runner
            .core
            .runtime
            .focused_widget(),
        Some(43)
    );
}

#[test]
fn auxiliary_focus_requests_preserve_order_and_fail_closed_for_invalid_lifecycle() {
    let mut parent = auxiliary_focus_parent();
    let owner = parent
        .core
        .runtime
        .acquire_auxiliary_effect_owner("settings");
    parent
        .auxiliary_windows
        .push(focusable_auxiliary_window("settings", owner.clone(), 43));

    let reduced = parent.reduce_auxiliary_messages(Some(owner.clone()), vec![3]);
    assert!(reduced.routed);
    execute_pending_auxiliary_focus_for_test(&mut parent);
    assert_eq!(
        parent.auxiliary_windows[0]
            .runner
            .core
            .runtime
            .focused_widget(),
        Some(43),
        "Focus/Clear/Focus must execute in queue order"
    );

    let missing_owner = parent
        .core
        .runtime
        .acquire_auxiliary_effect_owner("missing");
    parent
        .core
        .runtime
        .enqueue_auxiliary_focus_request(missing_owner, AuxiliaryFocusCommand::Focus(43));
    parent.auxiliary_windows[0]
        .runner
        .core
        .runtime
        .clear_focus();
    parent
        .core
        .runtime
        .enqueue_auxiliary_focus_request(owner.clone(), AuxiliaryFocusCommand::Focus(999));
    execute_pending_auxiliary_focus_for_test(&mut parent);
    assert_eq!(
        parent.auxiliary_windows[0]
            .runner
            .core
            .runtime
            .focused_widget(),
        None,
        "missing and invalid requests must fail closed"
    );

    parent.auxiliary_windows[0]
        .runner
        .core
        .runtime
        .focus_widget(43);
    parent.auxiliary_windows[0].hide();
    parent
        .core
        .runtime
        .enqueue_auxiliary_focus_request(owner, AuxiliaryFocusCommand::Clear);
    execute_pending_auxiliary_focus_for_test(&mut parent);
    assert_eq!(
        parent.auxiliary_windows[0]
            .runner
            .core
            .runtime
            .focused_widget(),
        Some(43),
        "hidden requests must not reach a child"
    );
    assert!(
        parent
            .core
            .runtime
            .take_pending_auxiliary_focus_requests()
            .is_empty()
    );
}

#[test]
fn stale_generation_and_retiring_or_sibling_children_are_not_fallback_targets() {
    let mut parent = auxiliary_focus_parent();
    let old_owner = parent
        .core
        .runtime
        .acquire_auxiliary_effect_owner("settings");
    parent.auxiliary_windows.push(focusable_auxiliary_window(
        "settings",
        old_owner.clone(),
        43,
    ));
    assert!(
        parent
            .core
            .runtime
            .retire_auxiliary_effect_owner(&old_owner)
    );
    let replacement_owner = parent
        .core
        .runtime
        .acquire_auxiliary_effect_owner("settings");
    parent.auxiliary_windows.push(focusable_auxiliary_window(
        "settings",
        replacement_owner.clone(),
        43,
    ));
    let sibling_owner = parent
        .core
        .runtime
        .acquire_auxiliary_effect_owner("inspector");

    let stale = AuxiliaryFocusRequest::new(old_owner, AuxiliaryFocusCommand::Focus(43));
    let sibling = AuxiliaryFocusRequest::new(sibling_owner, AuxiliaryFocusCommand::Focus(43));
    assert_eq!(parent.auxiliary_focus_request_target_index(&stale), None);
    assert_eq!(parent.auxiliary_focus_request_target_index(&sibling), None);

    parent.auxiliary_windows[1].begin_retiring();
    let retiring = AuxiliaryFocusRequest::new(replacement_owner, AuxiliaryFocusCommand::Focus(43));
    assert_eq!(parent.auxiliary_focus_request_target_index(&retiring), None);
}

#[test]
fn auxiliary_input_ticket_stays_live_through_parent_reduction() {
    let surface = crate::runtime::test_arc_surface(empty::<i32>().into_surface());
    let mut parent = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        AuxiliarySurfaceBridge::new(surface, false, false),
        Vector2::new(1280.0, 720.0),
    );
    parent.auxiliary_windows.push(auxiliary_window(false));

    let pending = auxiliary_input_route(
        &mut parent.auxiliary_windows[0],
        NativeDiscreteInputKind::KeyboardInput,
        None,
    );
    assert!(parent.auxiliary_windows[0].frame_stage_owner_has_in_flight());

    let reduced = parent.reduce_auxiliary_messages(None, Vec::new());
    assert_eq!(reduced, GenericRouteOutcome::default());
    assert!(parent.auxiliary_windows[0].frame_stage_owner_has_in_flight());

    let resolution = parent.auxiliary_windows[0]
        .resolve_native_discrete_input_route(pending)
        .expect("exact auxiliary input completion");
    assert_eq!(
        resolution.disposition,
        NativeInputStageDisposition::ContinueNow
    );
    assert!(resolution.child_outcome.is_none());
    assert!(!parent.auxiliary_windows[0].frame_stage_owner_has_in_flight());
}

#[test]
fn all_auxiliary_input_kinds_and_keyboard_none_outcome_settle_exactly() {
    for kind in [
        NativeDiscreteInputKind::MouseInput,
        NativeDiscreteInputKind::KeyboardInput,
        NativeDiscreteInputKind::ModifiersChanged,
        NativeDiscreteInputKind::Ime,
    ] {
        let mut window = auxiliary_window(false);
        let pending = auxiliary_input_route(&mut window, kind, None);
        assert!(window.frame_stage_owner_has_in_flight());

        let resolution = window
            .resolve_native_discrete_input_route(pending)
            .expect("covered auxiliary input kind should settle");
        assert_eq!(
            resolution.disposition,
            NativeInputStageDisposition::ContinueNow
        );
        assert!(resolution.child_outcome.is_none());
        assert!(!window.frame_stage_owner_has_in_flight());
    }
}

#[test]
fn auxiliary_completion_maps_identical_child_and_parent_dispositions() {
    let now = Instant::now();
    let cases = [
        (
            FrameStageBudgetBinding::not_budgeted(),
            None,
            NativeInputStageDisposition::ContinueNow,
        ),
        (
            FrameStageBudgetBinding::input_transient_at(Duration::from_millis(1), now),
            Some(now + Duration::from_micros(500)),
            NativeInputStageDisposition::ContinueNow,
        ),
        (
            FrameStageBudgetBinding::input_transient_at(Duration::from_millis(1), now),
            Some(now + Duration::from_millis(2)),
            NativeInputStageDisposition::DeferLowerPriority,
        ),
    ];

    for (budget, completed_at, expected) in cases {
        let mut child = auxiliary_window(false);
        let mut child_work = GenericRouteOutcome::default();
        child_work.request_scene_rebuild(FrameWorkReason::RoutedInput);
        let pending = auxiliary_input_route_with_budget(
            &mut child,
            NativeDiscreteInputKind::MouseInput,
            Some(child_work),
            budget,
        );
        let resolution = child
            .resolve_native_discrete_input_route_at(pending, completed_at)
            .expect("exact auxiliary input completion");
        assert_eq!(resolution.disposition, expected);
        let child_outcome = resolution
            .child_outcome
            .expect("routed child outcome should be retained");

        let surface = crate::runtime::test_arc_surface(empty::<i32>().into_surface());
        let mut parent = GenericNativeVelloRunner::new(
            NativeRunOptions::default(),
            AuxiliarySurfaceBridge::new(surface, false, false),
            Vector2::new(1280.0, 720.0),
        );
        let parent_outcome = parent.apply_auxiliary_native_discrete_input_resolution(
            GenericRouteOutcome::default(),
            resolution.disposition,
            Some(child_outcome),
        );

        assert_eq!(
            child_outcome.native_input_stage_disposition(),
            Some(expected)
        );
        assert_eq!(
            parent_outcome.native_input_stage_disposition(),
            Some(expected)
        );
        assert_eq!(
            child_outcome.native_input_stage_disposition(),
            parent_outcome.native_input_stage_disposition()
        );
    }
}

#[test]
fn auxiliary_completion_mismatch_suppresses_child_and_parent_lower_stage_work() {
    let surface = crate::runtime::test_arc_surface(empty::<i32>().into_surface());
    let mut parent = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        AuxiliarySurfaceBridge::new(surface, false, false),
        Vector2::new(1280.0, 720.0),
    );
    parent.auxiliary_windows.push(auxiliary_window(false));
    let mut child_work = GenericRouteOutcome::default();
    child_work.request_scene_rebuild(FrameWorkReason::RoutedInput);
    let pending = auxiliary_input_route(
        &mut parent.auxiliary_windows[0],
        NativeDiscreteInputKind::MouseInput,
        Some(child_work),
    );
    parent.auxiliary_windows[0].invalidate_terminal_convergence_stage_owner();

    let reduced_parent = parent.reduce_auxiliary_messages(None, Vec::new());
    assert!(
        parent.auxiliary_windows[0]
            .resolve_native_discrete_input_route(pending)
            .is_none()
    );
    assert_eq!(
        reduced_parent.native_input_stage_disposition(),
        None,
        "a mismatch must not authorize parent lower-stage work"
    );
    assert!(!parent.timing.deferred_auxiliary_window_sync);
}

#[test]
fn auxiliary_immediate_transient_settles_after_parent_reduction_once() {
    let now = Instant::now();
    let budget = Duration::from_millis(1);
    let mut parent = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        AuxiliarySurfaceBridge::new(
            crate::runtime::test_arc_surface(empty::<i32>().into_surface()),
            false,
            false,
        ),
        Vector2::new(1280.0, 720.0),
    );
    parent.auxiliary_windows.push(auxiliary_window(false));
    let mut child_work = GenericRouteOutcome::default();
    child_work.request_scene_rebuild(FrameWorkReason::RoutedInput);
    let pending = auxiliary_transient_route_with_budget(
        &mut parent.auxiliary_windows[0],
        NativeImmediateTransientKind::Focused(false),
        child_work,
        FrameStageBudgetBinding::input_transient_at(budget, now),
    );

    let reduced_parent = parent.reduce_auxiliary_messages(None, Vec::new());
    assert!(parent.auxiliary_windows[0].frame_stage_owner_has_in_flight());
    let resolution = parent.auxiliary_windows[0]
        .resolve_native_immediate_transient_route_at(pending, Some(now + budget))
        .expect("exact transient completion");
    let child_outcome = match resolution.child_route {
        super::AuxiliaryNativeImmediateTransientResolvedRoute::Outcome(outcome) => outcome,
        _ => panic!("focused transient should retain its child outcome"),
    };
    let parent_outcome = parent.apply_auxiliary_native_discrete_input_resolution(
        reduced_parent,
        resolution.disposition,
        Some(child_outcome),
    );

    assert_eq!(
        resolution.disposition,
        NativeInputStageDisposition::ContinueNow
    );
    assert_eq!(
        child_outcome.native_input_stage_disposition(),
        Some(NativeInputStageDisposition::ContinueNow)
    );
    assert_eq!(
        parent_outcome.native_input_stage_disposition(),
        child_outcome.native_input_stage_disposition()
    );
    assert!(!parent.auxiliary_windows[0].frame_stage_owner_has_in_flight());
    assert_eq!(
        parent.auxiliary_windows[0]
            .runner
            .frame_stage_owner
            .immediate_transient_budget_breach_count(),
        0
    );
}

#[test]
fn auxiliary_immediate_transient_exceeded_matches_parent_and_defers_sibling_sync() {
    let now = Instant::now();
    let budget = Duration::from_millis(1);
    let mut parent = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        AuxiliarySurfaceBridge::new(
            crate::runtime::test_arc_surface(empty::<i32>().into_surface()),
            false,
            false,
        ),
        Vector2::new(1280.0, 720.0),
    );
    parent.auxiliary_windows.push(auxiliary_window(false));
    let mut child_work = GenericRouteOutcome::default();
    child_work.request_scene_rebuild(FrameWorkReason::RoutedInput);
    let pending = auxiliary_transient_route_with_budget(
        &mut parent.auxiliary_windows[0],
        NativeImmediateTransientKind::CursorMoved,
        child_work,
        FrameStageBudgetBinding::input_transient_at(budget, now),
    );
    let resolution = parent.auxiliary_windows[0]
        .resolve_native_immediate_transient_route_at(
            pending,
            Some(now + budget + Duration::from_micros(1)),
        )
        .expect("exceeded transient completion");
    let child_outcome = match resolution.child_route {
        super::AuxiliaryNativeImmediateTransientResolvedRoute::Outcome(outcome) => outcome,
        _ => panic!("cursor move fixture should retain its child outcome"),
    };
    let parent_outcome = parent.apply_auxiliary_native_discrete_input_resolution(
        GenericRouteOutcome::default(),
        resolution.disposition,
        Some(child_outcome),
    );

    assert_eq!(
        resolution.disposition,
        NativeInputStageDisposition::DeferLowerPriority
    );
    assert_eq!(
        child_outcome.native_input_stage_disposition(),
        Some(NativeInputStageDisposition::DeferLowerPriority)
    );
    assert_eq!(
        parent_outcome.native_input_stage_disposition(),
        child_outcome.native_input_stage_disposition()
    );
    assert!(parent.timing.deferred_auxiliary_window_sync);
    assert!(!parent.timing.redraw_requested);
    assert!(!parent.runtime_wakeup.is_pending());
    assert_eq!(
        parent.auxiliary_windows[0]
            .runner
            .frame_stage_owner
            .immediate_transient_budget_breach_count(),
        1
    );
}

#[test]
fn auxiliary_immediate_transient_mismatch_is_stale_without_policy_or_replay() {
    let now = Instant::now();
    let mut parent = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        AuxiliarySurfaceBridge::new(
            crate::runtime::test_arc_surface(empty::<i32>().into_surface()),
            false,
            false,
        ),
        Vector2::new(1280.0, 720.0),
    );
    parent.auxiliary_windows.push(auxiliary_window(false));
    let mut child_work = GenericRouteOutcome::default();
    child_work.request_scene_rebuild(FrameWorkReason::RoutedInput);
    let pending = auxiliary_transient_route_with_budget(
        &mut parent.auxiliary_windows[0],
        NativeImmediateTransientKind::CursorLeft,
        child_work,
        FrameStageBudgetBinding::input_transient_at(Duration::from_millis(1), now),
    );
    parent.auxiliary_windows[0].invalidate_terminal_convergence_stage_owner();

    let reduced_parent = parent.reduce_auxiliary_messages(None, Vec::new());
    assert!(
        parent.auxiliary_windows[0]
            .resolve_native_immediate_transient_route(pending)
            .is_none()
    );
    assert_eq!(reduced_parent.native_input_stage_disposition(), None);
    assert!(!parent.timing.deferred_auxiliary_window_sync);
    assert!(!parent.auxiliary_windows[0].frame_stage_owner_has_in_flight());
}

#[test]
fn abandoned_auxiliary_input_route_is_vetoed_without_replay() {
    let surface = crate::runtime::test_arc_surface(empty::<i32>().into_surface());
    let mut parent = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        AuxiliarySurfaceBridge::new(surface, false, false),
        Vector2::new(1280.0, 720.0),
    );
    parent.auxiliary_windows.push(auxiliary_window(false));
    let pending = auxiliary_input_route(
        &mut parent.auxiliary_windows[0],
        NativeDiscreteInputKind::MouseInput,
        Some(GenericRouteOutcome::default()),
    );

    parent.cancel_auxiliary_native_discrete_input_route(0, Some(pending));

    assert!(!parent.auxiliary_windows[0].frame_stage_owner_has_in_flight());
}

#[test]
fn exceeded_auxiliary_resolution_arms_parent_without_parent_redraw_or_wakeup() {
    let surface = crate::runtime::test_arc_surface(empty::<i32>().into_surface());
    let mut parent = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        AuxiliarySurfaceBridge::new(surface, false, false),
        Vector2::new(1280.0, 720.0),
    );

    let no_work = parent.apply_auxiliary_native_discrete_input_resolution(
        GenericRouteOutcome::default(),
        NativeInputStageDisposition::DeferLowerPriority,
        None,
    );
    assert_eq!(
        no_work.native_input_stage_disposition(),
        Some(NativeInputStageDisposition::DeferLowerPriority)
    );
    assert!(parent.timing.deferred_auxiliary_window_sync);
    assert!(!parent.timing.redraw_requested);
    assert!(!parent.runtime_wakeup.is_pending());

    parent.timing.deferred_auxiliary_window_sync = false;
    let mut child_work = GenericRouteOutcome::default();
    child_work.request_scene_rebuild(FrameWorkReason::RoutedInput);
    let child_only = parent.apply_auxiliary_native_discrete_input_resolution(
        GenericRouteOutcome::default(),
        NativeInputStageDisposition::DeferLowerPriority,
        Some(child_work),
    );
    assert_eq!(
        child_only.native_input_stage_disposition(),
        Some(NativeInputStageDisposition::DeferLowerPriority)
    );
    assert!(parent.timing.deferred_auxiliary_window_sync);
    assert!(!parent.timing.redraw_requested);
    assert!(!parent.runtime_wakeup.is_pending());

    parent
        .auxiliary_windows
        .push(auxiliary_window_with_diagnostics(false, true));
    child_work.runtime_work_remaining = true;
    let frame_work = child_work.frame_work();
    parent.auxiliary_windows[0]
        .runner
        .apply_route_outcome_with_timed_frame(
            child_work.with_native_input_stage_disposition(
                NativeInputStageDisposition::DeferLowerPriority,
            ),
            false,
        );

    assert!(!parent.auxiliary_windows[0].runner.timing.redraw_requested);
    assert!(
        !parent.auxiliary_windows[0]
            .runner
            .runtime_wakeup
            .is_pending()
    );
    assert_eq!(
        parent.auxiliary_windows[0].runner.timing.pending_frame_work,
        frame_work
    );
    assert!(
        parent.auxiliary_windows[0]
            .runner
            .timing
            .deferred_scene_rebuild
    );
    assert!(parent.timing.deferred_auxiliary_window_sync);

    let mut profile = super::super::RenderFrameProfile::default();
    assert!(
        parent.auxiliary_windows[0]
            .runner
            .rebuild_deferred_scene_if_needed(&mut profile)
    );
    assert_eq!(
        parent.auxiliary_windows[0].runner.take_pending_frame_work(),
        frame_work
    );
    assert_eq!(
        parent.auxiliary_windows[0].runner.take_pending_frame_work(),
        FrameWork::None
    );
}

#[test]
fn continue_now_does_not_arm_parent_deferred_auxiliary_sync() {
    let surface = crate::runtime::test_arc_surface(empty::<i32>().into_surface());
    let mut parent = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        AuxiliarySurfaceBridge::new(surface, false, false),
        Vector2::new(1280.0, 720.0),
    );

    let outcome = parent.apply_auxiliary_native_discrete_input_resolution(
        GenericRouteOutcome::default(),
        NativeInputStageDisposition::ContinueNow,
        Some(GenericRouteOutcome::default()),
    );

    assert_eq!(
        outcome.native_input_stage_disposition(),
        Some(NativeInputStageDisposition::ContinueNow)
    );
    assert!(!parent.timing.deferred_auxiliary_window_sync);
}

#[test]
fn constructed_auxiliary_runner_owns_its_exact_schedule_key() {
    let window = auxiliary_window(false);

    assert_eq!(
        window.runner.frame_stage_owner.key(),
        &FrameScheduleKey::Auxiliary(String::from("settings"))
    );
}

#[test]
fn auxiliary_projection_key_lookup_uses_projected_windows_without_key_clones() {
    let surface = crate::runtime::test_arc_surface(empty::<()>().into_surface());
    let projections = vec![
        AuxiliaryWindow::new(
            "settings",
            crate::gui_runtime::NativeRunOptions::default(),
            Arc::clone(&surface),
        ),
        AuxiliaryWindow::new(
            "inspector",
            crate::gui_runtime::NativeRunOptions::default(),
            surface,
        ),
    ];

    assert!(auxiliary_projection_contains_key(&projections, "settings"));
    assert!(auxiliary_projection_contains_key(&projections, "inspector"));
    assert!(!auxiliary_projection_contains_key(&projections, "mixer"));
}

#[test]
fn same_key_recreation_is_suppressed_through_removal_sync_only() {
    let mut retiring = auxiliary_window(false);
    retiring.begin_retiring();
    let retiring_keys_before_maintenance = vec![retiring.key().to_owned()];

    let still_retiring = vec![retiring];
    let removed_keys =
        auxiliary_keys_removed_during_sync(&retiring_keys_before_maintenance, &still_retiring);
    assert!(removed_keys.is_empty());
    assert!(auxiliary_key_is_suppressed_for_sync(
        &still_retiring,
        &removed_keys,
        "settings"
    ));

    let removed_keys =
        auxiliary_keys_removed_during_sync::<i32>(&retiring_keys_before_maintenance, &[]);
    assert_eq!(removed_keys, [String::from("settings")]);
    assert!(auxiliary_key_is_suppressed_for_sync::<i32>(
        &[],
        &removed_keys,
        "settings"
    ));

    // A later independent sync has no removal witness and may recreate
    // the same projected key.
    assert!(!auxiliary_key_is_suppressed_for_sync::<i32>(
        &[],
        &[],
        "settings"
    ));
    assert!(!auxiliary_key_is_retiring::<i32>(&[], "settings"));
}

#[test]
fn failed_auxiliary_initialization_propagates_without_appending_child() {
    let failure = crate::gui_runtime::NativeGenericRunError::NativeInitialization {
        stage: crate::gui_runtime::NativeInitializationStage::RendererCreation,
        message: String::from("renderer rejected device"),
    };
    let mut windows = vec![String::from("existing")];

    assert_eq!(
        append_initialized_auxiliary_window(&mut windows, Err(failure.clone())),
        Err(failure)
    );
    assert_eq!(windows, [String::from("existing")]);

    assert_eq!(
        append_initialized_auxiliary_window(&mut windows, Ok(String::from("ready"))),
        Ok(())
    );
    assert_eq!(windows, [String::from("existing"), String::from("ready")]);
}

#[test]
fn auxiliary_redraw_failure_crosses_the_child_event_boundary() {
    let failure = NativeFrameRenderFailure::from_message("backend rejected scene");

    assert_eq!(
        auxiliary_redraw_terminal_cause(Err(failure)),
        Some(crate::gui_runtime::NativeGenericRunError::FrameRender(
            String::from("backend rejected scene"),
        ))
    );
    assert_eq!(auxiliary_redraw_terminal_cause(Ok(())), None);
}

#[test]
fn destructive_close_stages_a_ticket_and_retains_its_message_until_accepted() {
    let mut window = auxiliary_window(false);

    let first = window.handle_close_requested(None);
    assert!(first.messages.is_empty());
    assert!(first.message_origin.is_none());
    let admission = first.close_admission.expect("close ticket");
    assert!(window.is_admitted());
    assert!(window.active);
    assert!(window.window_id().is_none());
    assert!(window.close_message.is_some());
    assert!(window.prepare_destructive_close(&admission.ticket));
    assert!(window.complete_native_lifecycle(admission.ticket));
    assert!(window.is_retiring());
    assert!(!window.active);
    assert_eq!(window.take_close_message(), Some(7));
    assert!(!window.runner.core.runtime.begin_closing());

    let unrelated = auxiliary_window(true);
    assert!(auxiliary_key_is_retiring(
        std::slice::from_ref(&window),
        "settings"
    ));
    assert!(!auxiliary_key_is_retiring(
        std::slice::from_ref(&unrelated),
        "mixer"
    ));

    let duplicate = window.handle_close_requested(None);
    assert_eq!(duplicate.messages, Vec::<i32>::new());
    assert!(duplicate.terminal_cause.is_none());

    let late = AuxiliaryWindowEventResult::<i32>::ignored();
    assert!(late.messages.is_empty());
    assert!(late.terminal_cause.is_none());
    assert!(!late.shutdown_requested);
}

#[test]
fn destructive_close_duplicate_is_suppressed_until_the_first_ticket_is_vetoed() {
    let mut window = auxiliary_window(false);
    let first = window.stage_destructive_close_for_test();
    let admission = first.close_admission.expect("first close ticket");

    let duplicate = window.stage_destructive_close_for_test();
    assert!(duplicate.close_admission.is_none());
    assert!(duplicate.messages.is_empty());
    assert!(window.native_lifecycle_stage_ticket_is_current(&admission.ticket));
    assert!(window.has_close_message_for_test());

    assert!(window.veto_native_lifecycle(admission.ticket));
    let retry = window.stage_destructive_close_for_test();
    assert!(retry.close_admission.is_some());
    assert!(
        window.veto_native_lifecycle(retry.close_admission.expect("retry close ticket").ticket)
    );
}

#[test]
fn destructive_close_while_recovering_preserves_child_cause_and_cancels_only_child_recovery() {
    let mut window = auxiliary_window(false);
    assert!(window.admit_device_recovery());
    let original =
        crate::gui_runtime::NativeGenericRunError::RenderDeviceLost(String::from("child loss"));
    window.runner.recovery_cause = Some(original.clone());

    let close = window.stage_destructive_close_for_test();
    let admission = close.close_admission.expect("recovering close ticket");
    assert!(admission.ticket.evidence().source_phase.is_recovering());
    assert!(window.prepare_destructive_close(&admission.ticket));
    assert!(window.complete_native_lifecycle(admission.ticket));
    assert!(window.runner.is_closing());
    assert_eq!(window.runner.take_terminal_cause(), Some(original));
    assert!(window.is_retiring());
    assert!(!window.runner.recovery.has_in_flight_candidate());
}

#[test]
fn child_outbox_messages_carry_the_auxiliary_generation_owner() {
    let mut window = auxiliary_window(false);
    let _ = window.runner.core.runtime.dispatch_message(17);
    let result = window.event_result(None, false);
    assert_eq!(result.messages, [17]);
    let owner = window.effect_owner();
    assert!(
        result
            .message_origin
            .is_some_and(|origin| origin.is_same_generation(&owner))
    );
}

#[test]
fn whole_run_retirement_reuses_retiring_transition_without_dispatching_close_message() {
    let mut window = auxiliary_window_with_diagnostics(false, true);
    window.stage_frame_diagnostics_for_test(NativeFrameDiagnostics {
        window_identity: Some(NativeWindowDiagnosticIdentity::from_runtime_value(2)),
        frame_sequence: Some(13),
        ..NativeFrameDiagnostics::default()
    });

    window.begin_retiring();

    assert!(window.is_retiring());
    window.mark_parent_observation_finalized();
    assert_eq!(window.take_ready_frame_diagnostics(), None);
    let late_close = window.handle_close_requested(None);
    assert!(late_close.messages.is_empty());
    assert!(!late_close.shutdown_requested);
}

#[test]
fn cached_close_hides_reuses_and_does_not_begin_closing() {
    let mut window = auxiliary_window(true);
    let owner = window.effect_owner();

    let close = window.handle_close_requested(None);
    assert_eq!(close.messages, [7]);
    assert!(!window.is_retiring());
    assert!(!window.active);

    let surface = crate::runtime::test_arc_surface(empty::<i32>().into_surface());
    window.update_projection(
        AuxiliaryWindow::new("settings", NativeRunOptions::default(), surface).cache_on_close(),
    );
    assert!(window.active);
    assert!(!window.is_retiring());
    assert!(window.effect_owner().is_same_generation(&owner));
    assert!(window.runner.core.runtime.begin_closing());

    let duplicate = window.handle_close_requested(None);
    assert!(duplicate.messages.is_empty());
}

#[test]
fn cached_auxiliary_hide_recovery_and_show_rearm_mailbox_dormancy() {
    let mut window = auxiliary_window(true);

    window.hide();
    assert!(!window.active);
    assert!(window.runner.window.native_visual_requests.is_suspended());
    assert!(!window.runner.window.native_visual_requests.has_work());

    // A device/resource invalidation must not turn a cached inactive child
    // back into an unsolicited redraw source.
    assert!(window.runner.invalidate_native_visual_requests());
    window
        .runner
        .request_redraw_for_frame_work(FrameWork::RebuildScene {
            reason: FrameWorkReason::RuntimeSurfaceRepaint,
            mode: SceneRebuildMode::Immediate,
        });
    assert!(window.runner.window.native_visual_requests.is_suspended());
    assert!(!window.runner.window.native_visual_requests.has_work());

    // Repeated inactive callbacks reassert dormancy; explicit show is the
    // only rearm and leaves no stale packet to replay.
    window.hide();
    window.show();
    assert!(window.active);
    assert!(!window.runner.window.native_visual_requests.is_suspended());
    assert!(!window.runner.window.native_visual_requests.has_work());
}

#[test]
fn auxiliary_active_and_inactive_recovery_preserve_dormancy_boundaries() {
    let mut active = auxiliary_window(true);
    active.runner.set_native_window_visibility(true);
    assert!(active.runner.window.logical_window_visible);
    assert!(active.admit_device_recovery());
    assert!(active.runner.window.logical_window_visible);
    assert!(active.finish_device_recovery_if_no_rebuild());
    active
        .runner
        .apply_native_window_visibility(active.runner.window.logical_window_visible);
    assert!(active.runner.window.logical_window_visible);

    let mut inactive = auxiliary_window(true);
    inactive.hide();
    assert!(inactive.runner.window.native_visual_requests.is_suspended());
    assert!(inactive.admit_device_recovery());
    assert!(inactive.finish_device_recovery_if_no_rebuild());
    assert!(!inactive.active);
    assert!(inactive.runner.window.native_visual_requests.is_suspended());
    assert!(!inactive.runner.window.native_visual_requests.has_work());
    inactive.show();
    assert!(inactive.active);
    assert!(!inactive.runner.window.native_visual_requests.is_suspended());
    assert!(!inactive.runner.window.native_visual_requests.has_work());
}

#[test]
fn auxiliary_discrete_input_requires_active_admitted_materialized_wrapper() {
    let generation = NativeAdapterGeneration::from_test_serial(1);
    let mut window = auxiliary_window(false);

    assert!(!window.native_discrete_input_wrapper_is_eligible(generation));
    window.hide();
    assert!(!window.native_discrete_input_wrapper_is_eligible(generation));
    window.show();
    assert!(!window.native_discrete_input_wrapper_is_eligible(generation));

    window.begin_retiring();
    assert!(!window.native_discrete_input_wrapper_is_eligible(generation));
    assert!(!window.runner.frame_stage_owner.has_in_flight());
}

#[test]
fn lazy_finish_veto_retains_recovering_and_rebuild_pending() {
    let mut window = auxiliary_window(true);
    let generation = NativeAdapterGeneration::from_test_serial(1);

    assert!(window.admit_device_recovery());
    window.recovery_rebuild_pending = true;
    let mut source_phase = NativeLifecycle::default();
    assert!(source_phase.admit_recovery());
    let evidence = NativeLifecycleStageEvidence {
        key: FrameScheduleKey::Auxiliary(String::from("settings")),
        transition: NativeLifecycleTransitionKind::FinishDeviceRecovery,
        source_phase,
        window_id: Some(WindowId::dummy()),
        adapter_generation: Some(generation),
        active_resource_generation: Some(generation),
        target_generation: NativeTargetGeneration::from_test_serial(2),
        target_fenced: false,
    };
    let ticket = window
        .admit_native_lifecycle_finish_with_evidence(evidence.clone())
        .expect("finish lifecycle ticket");

    assert!(window.native_lifecycle_ticket_is_current_with_evidence(&ticket, &evidence));
    assert!(window.veto_native_lifecycle(ticket));
    assert!(window.runner.is_recovering());
    assert!(window.recovery_rebuild_pending());
}

#[test]
fn retiring_auxiliary_is_excluded_from_finish_lifecycle_admission() {
    let mut window = auxiliary_window(false);
    window.begin_retiring();

    assert!(
        window
            .admit_native_lifecycle_finish(Some(NativeAdapterGeneration::from_test_serial(1)))
            .is_none()
    );
}

#[test]
fn terminal_closing_includes_retiring_child_and_skips_closing_child() {
    let mut retiring = auxiliary_window(false);
    retiring.begin_retiring();
    assert!(retiring.is_retiring());
    assert!(retiring.should_stage_native_closing());
    let ticket = retiring
        .admit_native_closing(None)
        .expect("retiring child closing ticket");
    assert!(retiring.veto_native_lifecycle(ticket));

    let mut closing = auxiliary_window(false);
    assert!(closing.runner.prepare_native_shutdown(None).is_some());
    assert!(!closing.should_stage_native_closing());
    assert!(closing.admit_native_closing(None).is_none());
}

#[test]
fn terminal_convergence_invalidates_auxiliary_lifecycle_owner() {
    let mut auxiliary = auxiliary_window(false);
    let ticket = auxiliary
        .admit_native_closing(None)
        .expect("auxiliary terminal lifecycle ticket");
    let identity = ticket.stage_ticket().identity().clone();
    let owner_generation = auxiliary.runner.frame_stage_owner.owner_generation();
    assert!(auxiliary.runner.frame_stage_owner.has_in_flight());
    assert!(auxiliary.prepare_whole_run_closing());
    assert!(auxiliary.runner.is_closing());

    auxiliary.invalidate_terminal_convergence_stage_owner();

    assert!(!auxiliary.runner.frame_stage_owner.has_in_flight());
    assert!(auxiliary.runner.frame_stage_owner.owner_generation() > owner_generation);
    assert!(auxiliary.runner.frame_stage_owner.stale(&identity));
    assert!(
        !auxiliary
            .runner
            .native_lifecycle_stage_ticket_is_current(&ticket)
    );
    assert!(!auxiliary.veto_native_lifecycle(ticket));
}

#[test]
fn auxiliary_parent_handoff_requires_finalization_and_admission() {
    let diagnostics = NativeFrameDiagnostics {
        window_identity: Some(NativeWindowDiagnosticIdentity::from_runtime_value(2)),
        frame_sequence: Some(13),
        ..NativeFrameDiagnostics::default()
    };
    let mut window = auxiliary_window_with_diagnostics(true, true);

    window
        .runner
        .core
        .runtime
        .bridge_mut()
        .observe_frame_diagnostics(diagnostics);
    assert_eq!(window.take_ready_frame_diagnostics(), None);

    window.mark_parent_observation_finalized();
    assert_eq!(window.take_ready_frame_diagnostics(), Some(diagnostics));

    window
        .runner
        .core
        .runtime
        .bridge_mut()
        .observe_frame_diagnostics(diagnostics);
    window.require_scheduled_frame_admission();
    window.mark_parent_observation_finalized();
    assert_eq!(window.take_ready_frame_diagnostics(), None);

    window.mark_scheduled_frame_admission_recorded();
    assert_eq!(window.take_ready_frame_diagnostics(), Some(diagnostics));

    window
        .runner
        .core
        .runtime
        .bridge_mut()
        .observe_frame_diagnostics(diagnostics);
    let close = window.handle_close_requested(None);
    assert_eq!(close.messages, [7]);
    assert_eq!(window.take_ready_frame_diagnostics(), None);
}

#[test]
fn maintenance_removes_retiring_child_only_after_gpu_state_is_empty() {
    let surface = crate::runtime::test_arc_surface(empty::<i32>().into_surface());
    let mut parent = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        AuxiliarySurfaceBridge::new(surface, false, false),
        Vector2::new(1280.0, 720.0),
    );
    parent.auxiliary_windows.push(auxiliary_window(false));
    let child = parent
        .auxiliary_windows
        .last_mut()
        .expect("test parent should retain the auxiliary child");
    let close = child.handle_close_requested(None);
    let admission = close.close_admission.expect("close ticket");
    assert!(child.prepare_destructive_close(&admission.ticket));
    assert!(child.complete_native_lifecycle(admission.ticket));
    assert_eq!(child.take_close_message(), Some(7));

    let mut turn = NativeResourceMaintenanceTurn::new();
    assert!(parent.maintain_native_resources_with_turn(&mut turn));

    assert!(parent.auxiliary_windows.is_empty());
    assert!(parent.timing.deferred_auxiliary_window_sync);
    assert!(!turn.has_pending());
}

#[test]
fn destructive_close_without_message_still_enters_bounded_retirement() {
    let surface = crate::runtime::test_arc_surface(empty::<i32>().into_surface());
    let mut parent = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        AuxiliarySurfaceBridge::new(surface, false, false),
        Vector2::new(1280.0, 720.0),
    );
    let owner = parent
        .core
        .runtime
        .acquire_auxiliary_effect_owner("settings");
    let child_surface = crate::runtime::test_arc_surface(empty::<i32>().into_surface());
    parent
        .auxiliary_windows
        .push(AuxiliaryNativeWindow::new_with_owner(
            AuxiliaryWindow::new("settings", NativeRunOptions::default(), child_surface),
            &NativeRunOptions::default(),
            None,
            false,
            false,
            false,
            owner.clone(),
        ));
    let child = parent
        .auxiliary_windows
        .last_mut()
        .expect("test parent should retain the auxiliary child");
    let close = child.stage_destructive_close_for_test();
    let admission = close.close_admission.expect("close ticket");
    assert!(child.prepare_destructive_close(&admission.ticket));
    assert!(child.complete_native_lifecycle(admission.ticket));
    assert_eq!(child.take_close_message(), None);
    assert!(child.is_retiring());

    let mut turn = NativeResourceMaintenanceTurn::new();
    assert!(parent.maintain_native_resources_with_turn(&mut turn));
    assert!(parent.auxiliary_windows.is_empty());
    assert!(parent.timing.deferred_auxiliary_window_sync);
}

#[test]
fn recovery_opportunity_admits_at_most_one_auxiliary_rebuild() {
    let mut opportunity = AuxiliaryRecoveryOpportunity::default();

    assert!(opportunity.admit_rebuild());
    assert!(!opportunity.admit_rebuild());
    assert_eq!(opportunity.rebuilds(), 1);
}

#[test]
fn deferred_recovery_rebuilds_two_children_across_opportunities_and_clears_followup() {
    let mut pending_children = 2;
    let mut recovery_followup_pending = true;
    let mut total_rebuilds = 0;

    while pending_children != 0 {
        let mut opportunity = AuxiliaryRecoveryOpportunity::default();

        assert!(opportunity.admit_rebuild());
        assert!(!opportunity.admit_rebuild());
        pending_children -= usize::from(opportunity.rebuilds());
        total_rebuilds += usize::from(opportunity.rebuilds());

        if pending_children == 0 {
            recovery_followup_pending = false;
        } else {
            assert!(recovery_followup_pending);
        }
    }

    assert_eq!(total_rebuilds, 2);
    assert_eq!(pending_children, 0);
    assert!(!recovery_followup_pending);
}

#[test]
fn deferred_auxiliary_rebuild_failure_preserves_render_device_loss_and_fences_followup() {
    let recovery_cause =
        crate::gui_runtime::NativeGenericRunError::RenderDeviceLost(String::from("driver reset"));
    let auxiliary_error = crate::gui_runtime::NativeGenericRunError::NativeInitialization {
        stage: crate::gui_runtime::NativeInitializationStage::RendererCreation,
        message: String::from("auxiliary renderer rejected device"),
    };
    let mut retained_cause = Some(recovery_cause.clone());
    let mut followup_pending = true;

    assert_eq!(
        take_deferred_auxiliary_recovery_failure_cause(
            &mut retained_cause,
            &mut followup_pending,
            auxiliary_error,
        ),
        recovery_cause
    );
    assert!(retained_cause.is_none());
    assert!(!followup_pending);
}

#[test]
fn deferred_auxiliary_rebuild_failure_falls_back_to_auxiliary_error_without_recovery_cause() {
    let auxiliary_error = crate::gui_runtime::NativeGenericRunError::NativeInitialization {
        stage: crate::gui_runtime::NativeInitializationStage::RenderSurfaceCreation,
        message: String::from("auxiliary surface rejected device"),
    };
    let mut retained_cause = None;
    let mut followup_pending = true;

    assert_eq!(
        take_deferred_auxiliary_recovery_failure_cause(
            &mut retained_cause,
            &mut followup_pending,
            auxiliary_error.clone(),
        ),
        auxiliary_error
    );
    assert!(retained_cause.is_none());
    assert!(!followup_pending);
}

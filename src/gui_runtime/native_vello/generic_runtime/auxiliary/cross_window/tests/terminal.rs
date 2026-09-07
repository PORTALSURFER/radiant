use super::*;
use crate::{
    application::empty,
    gui::{
        drag_drop::DragOperation,
        pointer_ingress::{
            DeviceKind, PointerButtons, PointerContactId, PointerIngress, PointerPhase,
        },
    },
    gui_runtime::native_vello::generic_runtime::NativeLifecycle,
    runtime::Command,
    widgets::PointerButton,
};

#[test]
fn retired_receiver_cleanup_cannot_replay_into_same_key_replacement() {
    on_large_stack(retired_receiver_cleanup_body);
}

fn retired_receiver_cleanup_body() {
    let (mut parent, events, ids) = parent_with_receivers(&["receiver"]);
    let export = activate_source(&mut parent.core.runtime);
    let source = parent
        .drag_endpoint(WindowId::from(101_u64))
        .expect("source");
    let receiver = parent.drag_endpoint(ids[0]).expect("receiver");
    let first = sample(source, parent.drag_parent_projection(), export.key());
    parent.route_drag_sample_with_test_receiver(first, |_, _| {
        Some((receiver.clone(), Point::new(20.0, 20.0)))
    });
    assert!(
        events
            .borrow()
            .contains(&Message::Target("receiver", DropPhase::Entered))
    );
    let old_owner = parent.auxiliary_windows[0].effect_owner();
    assert!(
        parent
            .core
            .runtime
            .retire_auxiliary_effect_owner(&old_owner)
    );
    let replacement_owner = parent
        .core
        .runtime
        .acquire_auxiliary_effect_owner("receiver");
    assert!(!old_owner.is_same_generation(&replacement_owner));
    let mut replacement = AuxiliaryNativeWindow::new_with_owner(
        crate::runtime::AuxiliaryWindow::new(
            "receiver",
            crate::gui_runtime::NativeRunOptions::default(),
            receiver_surface("receiver"),
        ),
        &crate::gui_runtime::NativeRunOptions::default(),
        None,
        false,
        false,
        false,
        replacement_owner,
    );
    // Reusing even the native window ID cannot recreate the retired owner.
    replacement.runner.window.id = Some(ids[0]);
    parent.auxiliary_windows[0] = replacement;
    events.borrow_mut().clear();
    let _ = parent.core.runtime.execute_command(Command::end_drag());
    parent.route_drag_cancellations();
    parent.route_drag_cancellations();
    assert!(
        !events
            .borrow()
            .iter()
            .any(|event| matches!(event, Message::Target(_, _)))
    );
    assert!(parent.cross_window_transfers.is_empty());
    assert!(
        parent.auxiliary_windows[0]
            .runner
            .core
            .runtime
            .cross_window_foreign_preview()
            .is_none()
    );
}

struct TerminalBridge {
    source_retired: bool,
    events: Rc<RefCell<Vec<Message>>>,
}

impl RuntimeBridge<Message> for TerminalBridge {
    fn project_surface(&mut self) -> Arc<crate::runtime::UiSurface<Message>> {
        if self.source_retired {
            crate::runtime::test_arc_surface(empty::<Message>().into_surface())
        } else {
            source_surface()
        }
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        self.events.borrow_mut().push(message);
        if matches!(message, Message::Target(_, DropPhase::Dropped)) {
            self.source_retired = true;
            Command::RequestProjectionRefresh
        } else {
            Command::none()
        }
    }

    fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, Message> {
        RuntimeHostCapabilities::new().with_windows()
    }
}

impl RuntimeWindowHost<Message> for TerminalBridge {
    fn project_auxiliary_windows(&mut self) -> Vec<crate::runtime::AuxiliaryWindow<Message>> {
        vec![crate::runtime::AuxiliaryWindow::new(
            "receiver",
            crate::gui_runtime::NativeRunOptions::default(),
            receiver_surface("receiver"),
        )]
    }
}

#[test]
fn checked_pointer_drop_maps_source_completion_before_target_reducer_retires_source() {
    on_large_stack(checked_pointer_drop_body);
}

fn checked_pointer_drop_body() {
    let events = Rc::new(RefCell::new(Vec::new()));
    let mut parent = GenericNativeVelloRunner::new(
        crate::gui_runtime::NativeRunOptions::default(),
        TerminalBridge {
            source_retired: false,
            events: Rc::clone(&events),
        },
        Vector2::new(100.0, 100.0),
    );
    let source_id = WindowId::from(201_u64);
    let receiver_id = WindowId::from(202_u64);
    parent.window.id = Some(source_id);
    let owner = parent
        .core
        .runtime
        .acquire_auxiliary_effect_owner("receiver");
    let mut child = AuxiliaryNativeWindow::new_with_owner(
        crate::runtime::AuxiliaryWindow::new(
            "receiver",
            crate::gui_runtime::NativeRunOptions::default(),
            receiver_surface("receiver"),
        ),
        &crate::gui_runtime::NativeRunOptions::default(),
        None,
        false,
        false,
        false,
        owner,
    );
    child.runner.window.id = Some(receiver_id);
    parent.auxiliary_windows.push(child);
    let source = parent.drag_endpoint(source_id).expect("source endpoint");
    let receiver = parent
        .drag_endpoint(receiver_id)
        .expect("receiver endpoint");
    let device = InputDeviceId::from_host(93).expect("device");
    let contact = PointerContactId::from_host(94).expect("contact");
    let started = PointerIngress::new(
        DeviceKind::Mouse,
        device,
        contact,
        PointerPhase::Started {
            button: PointerButton::Primary,
        },
        Point::new(20.0, 20.0),
        PointerButtons::PRIMARY,
        Default::default(),
        None,
        None,
        None,
        None,
    )
    .expect("checked start");
    let token = parent
        .core
        .runtime
        .dispatch_pointer_ingress_with_admission(started)
        .sequence_token()
        .expect("source pointer token");
    let moved = PointerIngress::from_runtime(
        DeviceKind::Mouse,
        device,
        contact,
        PointerPhase::Moved,
        Point::new(40.0, 20.0),
        PointerButtons::PRIMARY,
        Default::default(),
        None,
        None,
        None,
        None,
        token,
    )
    .expect("checked move");
    let mut input = NativeCrossWindowInput::new(CrossWindowInputHint::foreign_or_none());
    parent.dispatch_checked_pointer_ingress(moved, Some(&mut input));
    assert!(matches!(
        input.last_disposition,
        Some(PointerIngressDisposition::RoutedGesture(_))
    ));
    let moved = NativeDragSample {
        source: source.clone(),
        location: cross_window_hit::test_drag_location(),
        parent_projection: parent.drag_parent_projection(),
        input,
    };
    parent.route_drag_sample_with_test_receiver(moved, |_, _| {
        Some((receiver.clone(), Point::new(20.0, 20.0)))
    });
    assert!(
        events
            .borrow()
            .contains(&Message::Target("receiver", DropPhase::Entered))
    );
    events.borrow_mut().clear();

    use crate::gui_runtime::native_vello::generic_runtime::{
        frame_scheduler_policy::discrete_input_completion_disposition,
        frame_stage_admission::FrameStageBudgetBinding,
        native_discrete_input_stage::{
            NativeDiscreteInputKind, NativeDiscreteInputStageEvidence,
            admit_native_discrete_input_with_budget, complete_native_discrete_input_at,
        },
        runner_state::NativeTargetGeneration,
    };
    let started_at = std::time::Instant::now();
    let budget = std::time::Duration::from_millis(1);
    let generation = NativeAdapterGeneration::from_test_serial(1);
    let ticket = admit_native_discrete_input_with_budget(
        &mut parent.frame_stage_owner,
        NativeDiscreteInputStageEvidence {
            key: FrameScheduleKey::Primary,
            kind: NativeDiscreteInputKind::MouseInput,
            timestamp: crate::gui::input::InputTimestamp::capture(),
            window_id: Some(source_id),
            adapter_generation: generation,
            active_resource_generation: Some(generation),
            target_generation: NativeTargetGeneration::from_test_serial(1),
            native_surface_target_fenced: false,
            lifecycle: NativeLifecycle::default(),
            native_window_eligible: true,
            wrapper_eligible: true,
        },
        FrameStageBudgetBinding::input_transient_at(budget, started_at),
    )
    .expect("exact release ticket with deterministic budget");
    let ended = PointerIngress::from_runtime(
        DeviceKind::Mouse,
        device,
        contact,
        PointerPhase::Ended {
            button: PointerButton::Primary,
        },
        Point::new(40.0, 20.0),
        PointerButtons::empty(),
        Default::default(),
        None,
        None,
        None,
        None,
        token,
    )
    .expect("checked release");
    let mut input = NativeCrossWindowInput::new(CrossWindowInputHint::foreign_or_none());
    parent.dispatch_checked_pointer_ingress(ended, Some(&mut input));
    assert!(
        input.terminal.is_some(),
        "release detaches terminal authority on this stack"
    );
    assert!(
        parent.core.runtime.cross_window_drag_export().is_none(),
        "capture is already detached"
    );
    let ended = NativeDragSample {
        source,
        location: cross_window_hit::test_drag_location(),
        parent_projection: parent.drag_parent_projection(),
        input,
    };
    assert!(parent.frame_stage_owner.has_in_flight());
    let drag = parent.route_drag_sample_with_test_receiver(ended, |_, _| {
        Some((receiver.clone(), Point::new(20.0, 20.0)))
    });
    assert!(
        parent.frame_stage_owner.has_in_flight(),
        "semantic drop does not settle the caller's ticket"
    );
    assert!(parent.core.runtime.bridge().source_retired);
    let completion = complete_native_discrete_input_at(
        &mut parent.frame_stage_owner,
        ticket,
        Some(started_at + budget + std::time::Duration::from_micros(1)),
    );
    let disposition = discrete_input_completion_disposition(completion);
    assert_eq!(
        disposition,
        Some(NativeInputStageDisposition::DeferLowerPriority)
    );
    parent.apply_drag_route_visuals(&drag, disposition);
    assert!(!parent.frame_stage_owner.has_in_flight());
    let events = events.borrow();
    let dropped = events
        .iter()
        .position(|event| *event == Message::Target("receiver", DropPhase::Dropped))
        .expect("target receives drop");
    assert_eq!(
        events.get(dropped + 1),
        Some(&Message::Source(DragSourcePhase::Completed(
            DragOperation::Copy
        )))
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, Message::Source(DragSourcePhase::Completed(_))))
            .count(),
        1
    );
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Message::Source(DragSourcePhase::Cancelled(_))))
    );
    assert!(parent.cross_window_transfers.is_empty());
}

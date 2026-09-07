//! Application-local native window qualification for typed drag transfers.

use super::super::cross_window_hit;
use super::super::cross_window_input::NativeCrossWindowInput;
use super::super::input::logical_point_from_winit;
use super::*;
use crate::gui::types::Point;
use crate::{
    gui::{drag_drop::DragCancelReason, pointer_ingress::PointerIngressDisposition},
    runtime::{
        CrossWindowDragExport, CrossWindowDragKey, CrossWindowForeignInput,
        CrossWindowForeignRoute, CrossWindowForeignTerminal, CrossWindowInputHint,
        CrossWindowSourceProof, CrossWindowTerminalMessages, CrossWindowTerminalRequest,
    },
};

struct ForeignDrive {
    source: NativeDragEndpoint,
    source_proof: CrossWindowSourceProof,
    key: CrossWindowDragKey,
    location: cross_window_hit::NativeDragLocation,
    position: Point,
    input: CrossWindowForeignInput,
}

// The primary and auxiliary controllers have distinct bridge types. Keep
// their native ownership checks identical without erasing the UI-local Message.
macro_rules! with_drag_runtime {
    ($host:expr, $endpoint:expr, $runtime:ident, $body:expr) => {{
        let endpoint = $endpoint;
        if !$host.is_running() {
            None
        } else if endpoint.owner.is_none() {
            if $host.window.id == Some(endpoint.window) {
                let $runtime = &mut $host.core.runtime;
                Some($body)
            } else {
                None
            }
        } else {
            $host
                .auxiliary_windows
                .iter_mut()
                .find(|window| endpoint.matches_auxiliary(window))
                .map(|window| {
                    let $runtime = &mut window.runner.core.runtime;
                    $body
                })
        }
    }};
}

mod autoscroll;

#[cfg(test)]
mod tests {
    mod autoscroll;
    mod terminal;

    use super::*;
    use crate::{
        application::{DragSource, DropTarget, button},
        gui::{
            drag_drop::{DragSourcePhase, DropPhase},
            pointer_ingress::{
                DeviceKind, GestureIngress, GestureKind, GesturePhase, GestureUnit, InputDeviceId,
                PointerButtons, PointerContactId, PointerIngress, PointerPhase,
            },
            types::Vector2,
        },
        gui_runtime::native_vello::generic_runtime::frame_scheduler_policy::NativeInputStageDisposition,
        gui_runtime::native_vello::generic_runtime::route_outcome::{
            FrameWork, FrameWorkReason, GenericRouteOutcome,
        },
        prelude::IntoView,
        runtime::{GestureRequest, RuntimeHostCapabilities, RuntimeWindowHost},
        widgets::PointerButton,
    };
    use std::{cell::RefCell, rc::Rc, sync::Arc};

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Message {
        Scrolled,
        Target(&'static str, DropPhase),
        Source(DragSourcePhase),
    }

    struct Bridge {
        source: Arc<crate::runtime::UiSurface<Message>>,
        receivers: Vec<(&'static str, Arc<crate::runtime::UiSurface<Message>>)>,
        events: Rc<RefCell<Vec<Message>>>,
        retire_auxiliary_source_on: Option<DropPhase>,
        auxiliary_source_live: bool,
    }

    impl RuntimeBridge<Message> for Bridge {
        fn project_surface(&mut self) -> Arc<crate::runtime::UiSurface<Message>> {
            Arc::clone(&self.source)
        }

        fn update(&mut self, message: Message) -> crate::runtime::Command<Message> {
            let retire_auxiliary_source = matches!(message, Message::Target("retire-source", phase) if self.retire_auxiliary_source_on == Some(phase));
            self.events.borrow_mut().push(message);
            if retire_auxiliary_source {
                self.auxiliary_source_live = false;
                crate::runtime::Command::RequestProjectionRefresh
            } else {
                crate::runtime::Command::none()
            }
        }

        fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, Message> {
            RuntimeHostCapabilities::new().with_windows()
        }
    }

    impl RuntimeWindowHost<Message> for Bridge {
        fn project_auxiliary_windows(&mut self) -> Vec<crate::runtime::AuxiliaryWindow<Message>> {
            self.receivers
                .iter()
                .filter_map(|(key, surface)| {
                    let surface = if *key == "source" {
                        self.auxiliary_source_live
                            .then(|| Arc::clone(&self.source))?
                    } else {
                        Arc::clone(surface)
                    };
                    Some(crate::runtime::AuxiliaryWindow::new(
                        *key,
                        crate::gui_runtime::NativeRunOptions::default(),
                        surface,
                    ))
                })
                .collect()
        }
    }

    fn source_surface() -> Arc<crate::runtime::UiSurface<Message>> {
        crate::runtime::test_arc_surface(
            button("source")
                .filter_mapped(|_| None::<Message>)
                .width(100.0)
                .height(100.0)
                .id(1)
                .drag_source(
                    DragSource::new(7_u8)
                        .on_event_with_revision((), |event| Some(Message::Source(event.phase()))),
                )
                .id(10)
                .into_surface(),
        )
    }

    fn source_with_local_target_surface() -> Arc<crate::runtime::UiSurface<Message>> {
        crate::runtime::test_arc_surface(
            button("source")
                .filter_mapped(|_| None::<Message>)
                .width(100.0)
                .height(100.0)
                .id(1)
                .drag_source(
                    DragSource::new(7_u8)
                        .on_event_with_revision((), |event| Some(Message::Source(event.phase()))),
                )
                .drop_target(
                    DropTarget::<u8, Message>::new().on_event_with_revision((), |event| {
                        Some(Message::Target("local", event.phase()))
                    }),
                )
                .id(10)
                .into_surface(),
        )
    }

    fn receiver_surface(label: &'static str) -> Arc<crate::runtime::UiSurface<Message>> {
        crate::runtime::test_arc_surface(
            button(label)
                .filter_mapped(|_| None::<Message>)
                .width(100.0)
                .height(100.0)
                .id(2)
                .drop_target(
                    DropTarget::<u8, Message>::new().on_event_with_revision((), move |event| {
                        Some(Message::Target(label, event.phase()))
                    }),
                )
                .id(20)
                .into_surface(),
        )
    }

    fn activate_source_with_token<BridgeType>(
        runtime: &mut crate::runtime::SurfaceRuntime<BridgeType, Message>,
    ) -> (CrossWindowDragExport, crate::runtime::GestureSequenceToken)
    where
        BridgeType: RuntimeBridge<Message>,
    {
        let ingress = |phase, amount| {
            GestureIngress::new(
                GestureKind::Pan,
                phase,
                GestureUnit::LogicalPixels,
                Vector2::new(amount, 0.0),
                InputDeviceId::from_host(77).expect("test device"),
                Some(Point::new(20.0, 20.0)),
                Default::default(),
                None,
                None,
            )
            .expect("finite test gesture")
        };
        let token = runtime
            .dispatch_gesture_request(GestureRequest::new(ingress(GesturePhase::Started, 0.0)))
            .token()
            .expect("drag source pan admission");
        let accepted = runtime.dispatch_gesture_request(
            GestureRequest::new(ingress(GesturePhase::Changed, 12.0)).with_token(token),
        );
        assert!(matches!(
            accepted.outcome(),
            crate::runtime::GestureOutcome::Accepted(_)
                | crate::runtime::GestureOutcome::AcceptedContainer(_)
        ));
        (
            runtime
                .cross_window_drag_export()
                .expect("typed drag export"),
            token,
        )
    }

    fn activate_source<BridgeType>(
        runtime: &mut crate::runtime::SurfaceRuntime<BridgeType, Message>,
    ) -> CrossWindowDragExport
    where
        BridgeType: RuntimeBridge<Message>,
    {
        activate_source_with_token(runtime).0
    }

    fn sample(
        source: NativeDragEndpoint,
        parent_projection: u64,
        key: CrossWindowDragKey,
    ) -> NativeDragSample<Message> {
        let mut input = NativeCrossWindowInput::new(CrossWindowInputHint::foreign_or_none());
        input.last_disposition = Some(PointerIngressDisposition::RoutedGesture(
            crate::layout::NodeId::from(1_u64),
        ));
        input.source_moved = Some(key);
        NativeDragSample {
            source,
            location: cross_window_hit::test_drag_location(),
            parent_projection,
            input,
        }
    }

    type ParentWithReceivers = (
        GenericNativeVelloRunner<Bridge, Message>,
        Rc<RefCell<Vec<Message>>>,
        Vec<WindowId>,
    );

    fn parent_with_receivers(names: &[&'static str]) -> ParentWithReceivers {
        parent_with_source(source_surface(), names)
    }

    fn parent_with_source(
        source: Arc<crate::runtime::UiSurface<Message>>,
        names: &[&'static str],
    ) -> ParentWithReceivers {
        let events = Rc::new(RefCell::new(Vec::new()));
        let receivers = names
            .iter()
            .map(|name| (*name, receiver_surface(name)))
            .collect::<Vec<_>>();
        let auxiliary_source = Arc::clone(&source);
        let mut parent = GenericNativeVelloRunner::new(
            crate::gui_runtime::NativeRunOptions::default(),
            Bridge {
                source,
                receivers,
                events: Rc::clone(&events),
                retire_auxiliary_source_on: None,
                auxiliary_source_live: names.contains(&"source"),
            },
            Vector2::new(100.0, 100.0),
        );
        let source_id = WindowId::from(101_u64);
        parent.window.id = Some(source_id);
        let mut ids = Vec::new();
        for (index, name) in names.iter().enumerate() {
            let owner = parent.core.runtime.acquire_auxiliary_effect_owner(name);
            let surface = if *name == "source" {
                Arc::clone(&auxiliary_source)
            } else {
                receiver_surface(name)
            };
            let child = crate::runtime::AuxiliaryWindow::new(
                *name,
                crate::gui_runtime::NativeRunOptions::default(),
                surface,
            );
            let mut child = AuxiliaryNativeWindow::new_with_owner(
                child,
                &crate::gui_runtime::NativeRunOptions::default(),
                None,
                false,
                false,
                false,
                owner,
            );
            let id = WindowId::from(102_u64 + index as u64);
            child.runner.window.id = Some(id);
            parent.auxiliary_windows.push(child);
            ids.push(id);
        }
        (parent, events, ids)
    }

    fn on_large_stack(test: fn()) {
        std::thread::Builder::new()
            .name("cross-window-coordinator".to_owned())
            .stack_size(16 * 1024 * 1024)
            .spawn(test)
            .expect("coordinator test thread")
            .join()
            .expect("coordinator test should not panic");
    }

    #[test]
    fn receiver_left_rehits_before_entering_callback_replacement() {
        on_large_stack(receiver_left_rehits_before_entering_callback_replacement_body);
    }

    fn receiver_left_rehits_before_entering_callback_replacement_body() {
        let (mut parent, events, ids) = parent_with_receivers(&["left", "stale", "fresh"]);
        let export = activate_source(&mut parent.core.runtime);
        events.borrow_mut().clear();
        let source = parent
            .drag_endpoint(WindowId::from(101_u64))
            .expect("source endpoint");
        let endpoints = ids
            .iter()
            .map(|id| parent.drag_endpoint(*id).expect("receiver endpoint"))
            .collect::<Vec<_>>();

        let first = sample(
            source.clone(),
            parent.drag_parent_projection(),
            export.key(),
        );
        let _ = parent.route_drag_sample_with_test_receiver(first, |_, _| {
            Some((endpoints[0].clone(), Point::new(20.0, 20.0)))
        });

        let second = sample(source, parent.drag_parent_projection(), export.key());
        let mut hits = vec![
            endpoints[1].clone(),
            endpoints[2].clone(),
            endpoints[2].clone(),
        ]
        .into_iter();
        let _ = parent.route_drag_sample_with_test_receiver(second, move |_, _| {
            hits.next()
                .map(|endpoint| (endpoint, Point::new(20.0, 20.0)))
        });

        assert_eq!(
            events.borrow().as_slice(),
            [
                Message::Target("left", DropPhase::Entered),
                Message::Source(DragSourcePhase::Moved),
                Message::Target("left", DropPhase::Left),
                Message::Target("fresh", DropPhase::Entered),
                Message::Source(DragSourcePhase::Moved),
            ],
            "the re-hit after Left must select fresh, never the stale initial receiver"
        );
    }

    #[test]
    fn receiver_mapper_refreshes_auxiliary_source_before_storing_or_mapping_source_move() {
        on_large_stack(
            receiver_mapper_refreshes_auxiliary_source_before_storing_or_mapping_source_move_body,
        );
    }

    fn receiver_mapper_refreshes_auxiliary_source_before_storing_or_mapping_source_move_body() {
        for phase in [DropPhase::Entered, DropPhase::Over, DropPhase::Left] {
            let (mut parent, events, ids) =
                parent_with_receivers(&["source", "retire-source", "other"]);
            let source = parent
                .drag_endpoint(ids[0])
                .expect("auxiliary source endpoint");
            let mut refresh = NativeDragRoute::default();
            assert!(
                parent.drag_refresh_endpoint_and_drain(&source, &mut refresh),
                "the fixture projects the auxiliary source through the parent bridge"
            );
            let export = activate_source(&mut parent.auxiliary_windows[0].runner.core.runtime);
            let retiring = parent.drag_endpoint(ids[1]).expect("retiring receiver");
            let other = parent.drag_endpoint(ids[2]).expect("other receiver");

            if phase != DropPhase::Entered {
                let initial = sample(
                    source.clone(),
                    parent.drag_parent_projection(),
                    export.key(),
                );
                let _ = parent.route_drag_sample_with_test_receiver(initial, |_, _| {
                    Some((retiring.clone(), Point::new(20.0, 20.0)))
                });
            }
            events.borrow_mut().clear();
            parent.core.runtime.bridge_mut().retire_auxiliary_source_on = Some(phase);
            let route = sample(
                source.clone(),
                parent.drag_parent_projection(),
                export.key(),
            );
            let next = if phase == DropPhase::Left {
                other.clone()
            } else {
                retiring.clone()
            };
            let _ = parent.route_drag_sample_with_test_receiver(route, move |_, _| {
                Some((next.clone(), Point::new(20.0, 20.0)))
            });

            let observed = events.borrow();
            assert!(
                observed.contains(&Message::Target("retire-source", phase)),
                "the fixture must exercise its requested receiver mapper"
            );
            assert_eq!(
                observed
                    .iter()
                    .filter(|message| {
                        matches!(*message, Message::Source(DragSourcePhase::Cancelled(_)))
                    })
                    .count(),
                1,
                "the refreshed auxiliary source drains one retirement callback for {phase:?}"
            );
            assert!(
                !observed.contains(&Message::Source(DragSourcePhase::Moved)),
                "{phase:?} cannot map SourceMoved from the old source projection"
            );
            assert!(
                phase != DropPhase::Left
                    || !observed.contains(&Message::Target("other", DropPhase::Entered)),
                "a Left that retires the source cannot admit the next receiver"
            );
            drop(observed);
            assert!(parent.cross_window_transfers.is_empty());

            let _ = parent.route_drag_cancellations();
            assert_eq!(
                events
                    .borrow()
                    .iter()
                    .filter(|message| {
                        matches!(*message, Message::Source(DragSourcePhase::Cancelled(_)))
                    })
                    .count(),
                1,
                "the removed source leaves no transfer that can replay its cancellation"
            );
        }
    }

    #[test]
    fn missing_auxiliary_source_projection_retires_capture_and_rejects_next_sample() {
        on_large_stack(
            missing_auxiliary_source_projection_retires_capture_and_rejects_next_sample_body,
        );
    }

    fn missing_auxiliary_source_projection_retires_capture_and_rejects_next_sample_body() {
        let (mut parent, events, ids) = parent_with_receivers(&["source", "receiver"]);
        let source = parent
            .drag_endpoint(ids[0])
            .expect("auxiliary source endpoint");
        let mut refresh = NativeDragRoute::default();
        assert!(
            parent.drag_refresh_endpoint_and_drain(&source, &mut refresh),
            "the fixture projects the auxiliary source through the parent bridge"
        );
        let export = activate_source(&mut parent.auxiliary_windows[0].runner.core.runtime);
        let receiver = parent.drag_endpoint(ids[1]).expect("receiver endpoint");
        parent
            .core
            .runtime
            .bridge_mut()
            .receivers
            .retain(|(key, _)| *key != "source");
        events.borrow_mut().clear();

        let mut outcome = NativeDragRoute::default();
        assert!(
            !parent.drag_refresh_endpoint_and_drain(&source, &mut outcome),
            "a source with no current auxiliary projection fails closed"
        );
        assert!(!parent.auxiliary_windows[0].input_projection_current());
        assert!(
            parent.drag_endpoint(ids[0]).is_none(),
            "invalid projection evidence rejects the source before native wrapper admission"
        );
        assert_eq!(
            events
                .borrow()
                .iter()
                .filter(|message| {
                    matches!(*message, Message::Source(DragSourcePhase::Cancelled(_)))
                })
                .count(),
            1,
            "the still-current owner reduces one source cancellation before it is fenced"
        );

        let route = sample(source, parent.drag_parent_projection(), export.key());
        let _ = parent.route_drag_sample_with_test_receiver(route, |_, _| {
            Some((receiver.clone(), Point::new(20.0, 20.0)))
        });
        assert!(parent.cross_window_transfers.is_empty());
        assert!(
            !events
                .borrow()
                .iter()
                .any(|message| matches!(*message, Message::Target(_, _))),
            "a rejected stale source cannot map a receiver callback"
        );
    }

    #[test]
    fn terminal_fallback_does_not_map_detached_source_after_receiver_cleanup_retires_it() {
        on_large_stack(
            terminal_fallback_does_not_map_detached_source_after_receiver_cleanup_retires_it_body,
        );
    }

    fn terminal_fallback_does_not_map_detached_source_after_receiver_cleanup_retires_it_body() {
        let (mut parent, events, ids) = parent_with_receivers(&["source", "retire-source"]);
        let source = parent
            .drag_endpoint(ids[0])
            .expect("auxiliary source endpoint");
        let receiver = parent.drag_endpoint(ids[1]).expect("receiver endpoint");
        let device = InputDeviceId::from_host(95).expect("test device");
        let contact = PointerContactId::from_host(96).expect("test contact");
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
        let token = parent.auxiliary_windows[0]
            .runner
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
        parent.auxiliary_windows[0]
            .runner
            .dispatch_checked_pointer_ingress(moved, Some(&mut input));
        let entered = NativeDragSample {
            source: source.clone(),
            location: cross_window_hit::test_drag_location(),
            parent_projection: parent.drag_parent_projection(),
            input,
        };
        parent.route_drag_sample_with_test_receiver(entered, |_, _| {
            Some((receiver.clone(), Point::new(40.0, 20.0)))
        });
        assert!(
            events
                .borrow()
                .contains(&Message::Target("retire-source", DropPhase::Entered))
        );

        events.borrow_mut().clear();
        parent.core.runtime.bridge_mut().retire_auxiliary_source_on = Some(DropPhase::Cancelled);
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
        .expect("checked end");
        let mut input = NativeCrossWindowInput::new(CrossWindowInputHint::foreign_or_none());
        parent.auxiliary_windows[0]
            .runner
            .dispatch_checked_pointer_ingress(ended, Some(&mut input));
        assert!(
            input.terminal.is_some(),
            "release detaches terminal authority"
        );
        let ended = NativeDragSample {
            source,
            location: cross_window_hit::test_drag_location(),
            parent_projection: parent.drag_parent_projection(),
            input,
        };
        let _ = parent.route_drag_sample_with_test_receiver(ended, |_, _| None);

        let observed = events.borrow();
        assert!(observed.contains(&Message::Target("retire-source", DropPhase::Cancelled)));
        assert!(
            !observed.iter().any(|message| matches!(
                *message,
                Message::Source(DragSourcePhase::Cancelled(_))
                    | Message::Source(DragSourcePhase::Completed(_))
                    | Message::Target(_, DropPhase::Dropped)
            )),
            "a detached terminal cannot map a stale source cancellation, completion, or drop"
        );
        assert!(parent.cross_window_transfers.is_empty());
    }

    #[test]
    fn foreign_left_returns_to_local_target_before_source_moved() {
        on_large_stack(foreign_left_returns_to_local_target_before_source_moved_body);
    }

    fn foreign_left_returns_to_local_target_before_source_moved_body() {
        let (mut parent, events, ids) =
            parent_with_source(source_with_local_target_surface(), &["foreign"]);
        let source = parent
            .drag_endpoint(WindowId::from(101_u64))
            .expect("source endpoint");
        let foreign = parent.drag_endpoint(ids[0]).expect("foreign endpoint");
        let device = InputDeviceId::from_host(61).expect("test device");
        let contact = PointerContactId::from_host(62).expect("test contact");
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
        let foreign_move = PointerIngress::from_runtime(
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
        .expect("checked foreign move");
        let mut foreign_input =
            NativeCrossWindowInput::new(CrossWindowInputHint::foreign_or_none());
        parent.dispatch_checked_pointer_ingress(foreign_move, Some(&mut foreign_input));
        assert!(matches!(
            foreign_input.last_disposition,
            Some(PointerIngressDisposition::RoutedGesture(_))
        ));
        events.borrow_mut().clear();

        let first = NativeDragSample {
            source: source.clone(),
            location: cross_window_hit::test_drag_location(),
            parent_projection: parent.drag_parent_projection(),
            input: foreign_input,
        };
        let _ = parent.route_drag_sample_with_test_receiver(first, |_, _| {
            Some((foreign.clone(), Point::new(20.0, 20.0)))
        });
        let local_move = PointerIngress::from_runtime(
            DeviceKind::Mouse,
            device,
            contact,
            PointerPhase::Moved,
            Point::new(45.0, 20.0),
            PointerButtons::PRIMARY,
            Default::default(),
            None,
            None,
            None,
            None,
            token,
        )
        .expect("checked local move");
        let mut local_input = NativeCrossWindowInput::new(CrossWindowInputHint::local());
        parent.dispatch_checked_pointer_ingress(local_move, Some(&mut local_input));
        assert!(matches!(
            local_input.last_disposition,
            Some(PointerIngressDisposition::RoutedGesture(_))
        ));
        let second = NativeDragSample {
            source: source.clone(),
            location: cross_window_hit::test_drag_location(),
            parent_projection: parent.drag_parent_projection(),
            input: local_input,
        };
        let _ = parent.route_drag_sample_with_test_receiver(second, |_, _| {
            Some((source.clone(), Point::new(45.0, 20.0)))
        });

        assert_eq!(
            events.borrow().as_slice(),
            [
                Message::Target("foreign", DropPhase::Entered),
                Message::Target("foreign", DropPhase::Left),
                Message::Target("local", DropPhase::Entered),
                Message::Source(DragSourcePhase::Moved),
            ],
            "returning to the source resolves foreign Left before local Entered and SourceMoved"
        );
    }

    #[test]
    fn retired_source_cancels_current_foreign_receiver_once() {
        on_large_stack(retired_source_cancels_current_foreign_receiver_once_body);
    }

    fn retired_source_cancels_current_foreign_receiver_once_body() {
        let (mut parent, events, ids) = parent_with_receivers(&["receiver"]);
        let (export, token) = activate_source_with_token(&mut parent.core.runtime);
        let source = parent
            .drag_endpoint(WindowId::from(101_u64))
            .expect("source endpoint");
        let receiver = parent.drag_endpoint(ids[0]).expect("receiver endpoint");
        let route = sample(source, parent.drag_parent_projection(), export.key());
        let _ = parent.route_drag_sample_with_test_receiver(route, |_, _| {
            Some((receiver.clone(), Point::new(20.0, 20.0)))
        });

        let ended = GestureIngress::new(
            GestureKind::Pan,
            GesturePhase::Ended,
            GestureUnit::LogicalPixels,
            Vector2::new(12.0, 0.0),
            InputDeviceId::from_host(77).expect("test device"),
            Some(Point::new(20.0, 20.0)),
            Default::default(),
            None,
            None,
        )
        .expect("finite terminal gesture");
        let _ = parent
            .core
            .runtime
            .dispatch_gesture_request(GestureRequest::new(ended).with_token(token));

        let _ = parent.route_drag_cancellations();
        let _ = parent.route_drag_cancellations();
        assert_eq!(
            events
                .borrow()
                .iter()
                .filter(|event| **event == Message::Target("receiver", DropPhase::Cancelled))
                .count(),
            1,
            "a retired source clears the current foreign receiver exactly once"
        );
    }

    #[test]
    fn timed_cancellation_cleans_receiver_once_without_replaying_for_paint_only_work() {
        on_large_stack(
            timed_cancellation_cleans_receiver_once_without_replaying_for_paint_only_work_body,
        );
    }

    fn timed_cancellation_cleans_receiver_once_without_replaying_for_paint_only_work_body() {
        let (mut parent, events, ids) = parent_with_receivers(&["receiver"]);
        let (export, token) = activate_source_with_token(&mut parent.core.runtime);
        let source = parent
            .drag_endpoint(WindowId::from(101_u64))
            .expect("source endpoint");
        let receiver = parent.drag_endpoint(ids[0]).expect("receiver endpoint");
        let route = sample(source, parent.drag_parent_projection(), export.key());
        let _ = parent.route_drag_sample_with_test_receiver(route, |_, _| {
            Some((receiver.clone(), Point::new(20.0, 20.0)))
        });
        events.borrow_mut().clear();

        let ended = GestureIngress::new(
            GestureKind::Pan,
            GesturePhase::Ended,
            GestureUnit::LogicalPixels,
            Vector2::new(12.0, 0.0),
            InputDeviceId::from_host(77).expect("test device"),
            Some(Point::new(20.0, 20.0)),
            Default::default(),
            None,
            None,
        )
        .expect("finite terminal gesture");
        let _ = parent
            .core
            .runtime
            .dispatch_gesture_request(GestureRequest::new(ended).with_token(token));

        let mut timed = GenericRouteOutcome {
            routed: true,
            ..Default::default()
        };
        parent.collect_timed_drag_cancellations(&mut timed);
        assert_eq!(
            events
                .borrow()
                .iter()
                .filter(|event| **event == Message::Target("receiver", DropPhase::Cancelled))
                .count(),
            1,
            "a timed semantic drain cleans the current foreign receiver"
        );

        let mut paint_only = GenericRouteOutcome {
            routed: false,
            frame_work: FrameWork::PaintOnly {
                reason: FrameWorkReason::TimedPaintOnlyAnimation,
            },
            ..Default::default()
        };
        parent.collect_timed_drag_cancellations(&mut paint_only);
        assert_eq!(
            events
                .borrow()
                .iter()
                .filter(|event| **event == Message::Target("receiver", DropPhase::Cancelled))
                .count(),
            1,
            "paint-only work must not replay an already delivered foreign cancellation"
        );
    }

    #[test]
    fn redraw_scan_wakes_one_semantic_cleanup_after_source_refresh_retirement() {
        on_large_stack(redraw_scan_wakes_one_semantic_cleanup_after_source_refresh_retirement_body);
    }

    fn redraw_scan_wakes_one_semantic_cleanup_after_source_refresh_retirement_body() {
        let (mut parent, events, ids) = parent_with_receivers(&["receiver"]);
        let export = activate_source(&mut parent.core.runtime);
        let source = parent
            .drag_endpoint(WindowId::from(101_u64))
            .expect("source endpoint");
        let receiver = parent.drag_endpoint(ids[0]).expect("receiver endpoint");
        let route = sample(source, parent.drag_parent_projection(), export.key());
        let _ = parent.route_drag_sample_with_test_receiver(route, |_, _| {
            Some((receiver.clone(), Point::new(20.0, 20.0)))
        });
        events.borrow_mut().clear();

        parent.core.runtime.bridge_mut().source =
            crate::runtime::test_arc_surface(crate::application::empty::<Message>().into_surface());
        parent.core.refresh_surface();
        events.borrow_mut().clear();
        assert!(
            parent.has_expired_drag_transfer(),
            "a source refresh that retires the drag wakes semantic receiver cleanup"
        );
        assert!(
            events.borrow().is_empty(),
            "the redraw scan itself does not run application mappers"
        );

        let _ = parent.route_drag_cancellations();
        assert_eq!(
            events
                .borrow()
                .iter()
                .filter(|event| **event == Message::Target("receiver", DropPhase::Cancelled))
                .count(),
            1,
            "the scheduled semantic collector cancels the still-current receiver once"
        );
        assert!(
            !parent.has_expired_drag_transfer(),
            "the completed cleanup leaves no transfer for a later paint to wake"
        );
    }

    #[test]
    fn deferred_completion_keeps_foreign_semantics_and_defers_only_visual_work() {
        on_large_stack(
            deferred_completion_keeps_foreign_semantics_and_defers_only_visual_work_body,
        );
    }

    fn deferred_completion_keeps_foreign_semantics_and_defers_only_visual_work_body() {
        let (mut parent, events, ids) = parent_with_receivers(&["receiver"]);
        let export = activate_source(&mut parent.core.runtime);
        let source = parent
            .drag_endpoint(WindowId::from(101_u64))
            .expect("source endpoint");
        let receiver = parent.drag_endpoint(ids[0]).expect("receiver endpoint");
        let route = sample(source, parent.drag_parent_projection(), export.key());
        let drag = parent.route_drag_sample_with_test_receiver(route, |_, _| {
            Some((receiver.clone(), Point::new(20.0, 20.0)))
        });
        assert!(
            events
                .borrow()
                .contains(&Message::Target("receiver", DropPhase::Entered)),
            "semantic target admission occurs inside the live input ticket"
        );

        parent
            .apply_drag_route_visuals(&drag, Some(NativeInputStageDisposition::DeferLowerPriority));
        assert!(
            parent.auxiliary_windows[0]
                .runner
                .timing
                .deferred_scene_rebuild,
            "only visual publication is retained after an over-budget completion"
        );
    }
}

pub(in crate::gui_runtime::native_vello::generic_runtime) struct NativeDragTransfer {
    source: NativeDragEndpoint,
    key: CrossWindowDragKey,
    receiver: NativeDragEndpoint,
    parent_projection: u64,
    location: cross_window_hit::NativeDragLocation,
    position: Point,
}

pub(in crate::gui_runtime::native_vello::generic_runtime) struct NativeDragSample<Message> {
    source: NativeDragEndpoint,
    location: cross_window_hit::NativeDragLocation,
    parent_projection: u64,
    pub(in crate::gui_runtime::native_vello::generic_runtime) input:
        NativeCrossWindowInput<Message>,
}

/// Semantic work admitted by one native pointer ticket. The outer native
/// lifecycle merges `outcome` before completing that ticket, then applies the
/// visual dirtiness according to the completion disposition.
#[derive(Default)]
pub(in crate::gui_runtime::native_vello::generic_runtime) struct NativeDragRoute {
    pub(in crate::gui_runtime::native_vello::generic_runtime) outcome: GenericRouteOutcome,
    pub(in crate::gui_runtime::native_vello::generic_runtime) visual_work:
        Vec<NativeDragVisualWork>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(in crate::gui_runtime::native_vello::generic_runtime) enum NativeDragVisualKind {
    PaintOnly,
    RebuildScene,
}

#[derive(Clone)]
pub(in crate::gui_runtime::native_vello::generic_runtime) struct NativeDragVisualWork {
    endpoint: NativeDragEndpoint,
    kind: NativeDragVisualKind,
}

impl NativeDragRoute {
    /// Merge only retained visual dirtiness. Semantic outcomes are deliberately
    /// kept separate so each native ticket remains the sole reducer boundary.
    pub(in crate::gui_runtime::native_vello::generic_runtime) fn merge_visuals_from(
        &mut self,
        other: &Self,
    ) {
        for visual in &other.visual_work {
            self.mark_visual(&visual.endpoint, visual.kind);
        }
    }

    fn mark_visual(&mut self, endpoint: &NativeDragEndpoint, kind: NativeDragVisualKind) {
        if let Some(existing) = self
            .visual_work
            .iter_mut()
            .find(|candidate| candidate.endpoint.same(endpoint))
        {
            if kind == NativeDragVisualKind::RebuildScene {
                existing.kind = kind;
            }
        } else {
            self.visual_work.push(NativeDragVisualWork {
                endpoint: endpoint.clone(),
                kind,
            });
        }
    }

    fn mark_paint(&mut self, endpoint: &NativeDragEndpoint) {
        self.mark_visual(endpoint, NativeDragVisualKind::PaintOnly);
    }

    fn mark_rebuild(&mut self, endpoint: &NativeDragEndpoint) {
        self.mark_visual(endpoint, NativeDragVisualKind::RebuildScene);
    }
}

/// A window id alone is insufficient when a cached auxiliary key is reopened.
/// Carry the parent-owned incarnation through every coordinator operation.
#[derive(Clone)]
pub(in crate::gui_runtime::native_vello::generic_runtime) struct NativeDragEndpoint {
    pub(in crate::gui_runtime::native_vello::generic_runtime) window: WindowId,
    pub(in crate::gui_runtime::native_vello::generic_runtime) owner: Option<AuxiliaryWindowOwner>,
}

impl NativeDragEndpoint {
    fn same(&self, other: &Self) -> bool {
        self.window == other.window
            && match (&self.owner, &other.owner) {
                (None, None) => true,
                (Some(left), Some(right)) => left.is_same_generation(right),
                _ => false,
            }
    }
    fn matches_auxiliary<Message>(&self, window: &AuxiliaryNativeWindow<Message>) -> bool {
        window.active
            && window.is_admitted()
            && window.input_projection_current()
            && !window.recovery_rebuild_pending
            && window.window_id() == Some(self.window)
            && self
                .owner
                .as_ref()
                .is_some_and(|owner| owner.is_open() && owner.is_same_generation(&window.owner))
    }
}

impl<Bridge, Message> GenericNativeVelloRunner<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    fn drag_parent_projection(&self) -> u64 {
        self.core.runtime.refresh_counters().application_projection
    }

    fn drag_endpoint_is_current(&mut self, endpoint: &NativeDragEndpoint) -> bool {
        with_drag_runtime!(self, endpoint, runtime, {
            let _ = runtime;
            true
        })
        .unwrap_or(false)
    }

    fn drag_source_proof_is_current(
        &mut self,
        endpoint: &NativeDragEndpoint,
        proof: &CrossWindowSourceProof,
    ) -> bool {
        with_drag_runtime!(self, endpoint, runtime, {
            runtime.cross_window_source_proof_is_current(proof)
        })
        .unwrap_or(false)
    }

    fn drag_reduce_messages(
        &mut self,
        endpoint: &NativeDragEndpoint,
        messages: Vec<Message>,
        outcome: &mut NativeDragRoute,
    ) -> bool {
        for message in messages {
            if !self.drag_endpoint_is_current(endpoint) {
                return false;
            }
            outcome
                .outcome
                .merge(self.reduce_owned_window_message(endpoint.owner.as_ref(), message));
        }
        true
    }

    fn drag_route_foreign(
        &mut self,
        endpoint: &NativeDragEndpoint,
        input: CrossWindowForeignInput,
    ) -> Option<CrossWindowForeignRoute<Message>> {
        with_drag_runtime!(
            self,
            endpoint,
            runtime,
            runtime.route_cross_window_foreign(input)
        )
    }

    fn drag_requalify_foreign(
        &mut self,
        endpoint: &NativeDragEndpoint,
        key: CrossWindowDragKey,
    ) -> Option<CrossWindowForeignRoute<Message>> {
        with_drag_runtime!(
            self,
            endpoint,
            runtime,
            runtime.requalify_cross_window_foreign(key)
        )
    }

    fn drag_finish_foreign_feedback(
        &mut self,
        endpoint: &NativeDragEndpoint,
        key: CrossWindowDragKey,
    ) -> bool {
        with_drag_runtime!(self, endpoint, runtime, {
            runtime.finish_cross_window_foreign_feedback(key)
        })
        .unwrap_or(false)
    }

    fn drag_foreign_target_id(
        &mut self,
        endpoint: &NativeDragEndpoint,
        key: CrossWindowDragKey,
    ) -> Option<crate::widgets::WidgetId> {
        with_drag_runtime!(self, endpoint, runtime, {
            runtime.cross_window_foreign_target_id(key)
        })
        .flatten()
    }

    fn drag_local_target_id(
        &mut self,
        endpoint: &NativeDragEndpoint,
        key: CrossWindowDragKey,
    ) -> Option<crate::widgets::WidgetId> {
        with_drag_runtime!(self, endpoint, runtime, {
            runtime.cross_window_local_target_id(key)
        })
        .flatten()
    }

    fn drag_clear_foreign(
        &mut self,
        endpoint: &NativeDragEndpoint,
        key: CrossWindowDragKey,
    ) -> Option<CrossWindowForeignRoute<Message>> {
        with_drag_runtime!(
            self,
            endpoint,
            runtime,
            runtime.clear_cross_window_foreign(key)
        )
    }

    fn drag_cancel_foreign(
        &mut self,
        endpoint: &NativeDragEndpoint,
        key: CrossWindowDragKey,
    ) -> Option<CrossWindowForeignRoute<Message>> {
        with_drag_runtime!(
            self,
            endpoint,
            runtime,
            runtime.cancel_cross_window_foreign(key)
        )
    }

    fn drag_discard_foreign(&mut self, endpoint: &NativeDragEndpoint, key: CrossWindowDragKey) {
        let _ = with_drag_runtime!(self, endpoint, runtime, {
            runtime.take_cross_window_foreign_terminal(key)
        });
    }

    fn drag_take_foreign_terminal(
        &mut self,
        endpoint: &NativeDragEndpoint,
        key: CrossWindowDragKey,
    ) -> Option<CrossWindowForeignTerminal<Message>> {
        with_drag_runtime!(self, endpoint, runtime, {
            runtime.take_cross_window_foreign_terminal(key)
        })
        .flatten()
    }

    fn drag_map_terminal(
        &mut self,
        endpoint: &NativeDragEndpoint,
        request: &CrossWindowTerminalRequest<Message>,
        terminal: Option<&CrossWindowForeignTerminal<Message>>,
        reason: DragCancelReason,
    ) -> Option<CrossWindowTerminalMessages<Message>> {
        with_drag_runtime!(self, endpoint, runtime, {
            runtime.map_cross_window_terminal(request, terminal, reason)
        })
    }

    fn drag_map_source_moved(
        &mut self,
        endpoint: &NativeDragEndpoint,
        key: CrossWindowDragKey,
        target: Option<crate::widgets::WidgetId>,
    ) -> Option<Message> {
        with_drag_runtime!(self, endpoint, runtime, {
            runtime.map_cross_window_source_moved(key, target)
        })
        .flatten()
    }

    fn drag_advance_local_target(
        &mut self,
        endpoint: &NativeDragEndpoint,
        key: CrossWindowDragKey,
    ) -> Option<CrossWindowForeignRoute<Message>> {
        with_drag_runtime!(self, endpoint, runtime, {
            runtime.advance_cross_window_local_target(key)
        })
    }

    fn invalidate_missing_drag_source_projection(
        &mut self,
        endpoint: &NativeDragEndpoint,
        outcome: &mut NativeDragRoute,
    ) {
        let Some(index) = self
            .auxiliary_windows
            .iter()
            .position(|window| endpoint.matches_auxiliary(window))
        else {
            return;
        };
        let messages = self.auxiliary_windows[index].end_drag_before_projection_invalidation();
        // The owner remains current until its terminal mapper has reached the
        // parent reducer. Only then fence further native admission.
        if !messages.is_empty() {
            let _ = self.drag_reduce_messages(endpoint, messages, outcome);
        }
        self.auxiliary_windows[index].invalidate_input_projection();
    }

    fn refresh_drag_endpoint(
        &mut self,
        endpoint: &NativeDragEndpoint,
        outcome: &mut NativeDragRoute,
    ) -> bool {
        if !self.is_running() {
            return false;
        }
        if endpoint.owner.is_none() {
            // Parent reduction already performs its normal synchronous
            // refresh. Repeating it here can retire conservative bindings
            // between two otherwise-admitted drag phases.
            return self.window.id == Some(endpoint.window);
        }
        let Some(index) = self
            .auxiliary_windows
            .iter()
            .position(|window| endpoint.matches_auxiliary(window))
        else {
            return false;
        };
        let key = self.auxiliary_windows[index].key().to_owned();
        let mut projections = self
            .core
            .runtime
            .host_project_auxiliary_windows()
            .into_iter()
            .filter(|projection| projection.key == key);
        let Some(projection) = projections.next() else {
            self.invalidate_missing_drag_source_projection(endpoint, outcome);
            return false;
        };
        if projections.next().is_some() {
            self.invalidate_missing_drag_source_projection(endpoint, outcome);
            return false;
        }
        let service = self.core.runtime.command_service();
        let window = &mut self.auxiliary_windows[index];
        if !endpoint.matches_auxiliary(window) {
            return false;
        }
        window.runner.core.runtime.bridge_mut().surface = projection.surface;
        window.runner.core.runtime.bridge_mut().command_service = service;
        window.runner.core.refresh_surface();
        true
    }

    fn take_drag_endpoint_messages(
        &mut self,
        endpoint: &NativeDragEndpoint,
    ) -> Option<Vec<Message>> {
        if endpoint.owner.is_none() {
            return (self.window.id == Some(endpoint.window)).then(Vec::new);
        }
        self.auxiliary_windows
            .iter_mut()
            .find(|window| endpoint.matches_auxiliary(window))
            .map(|window| window.runner.core.runtime.bridge_mut().take_messages())
    }

    /// An auxiliary refresh can retire a source-owned interaction and queue a
    /// terminal mapper in its bridge. Drain and reduce that mapper before the
    /// coordinator invokes another receiver callback, then reproject once so
    /// the next source-proof check observes the application result.
    fn drag_refresh_endpoint_and_drain(
        &mut self,
        endpoint: &NativeDragEndpoint,
        outcome: &mut NativeDragRoute,
    ) -> bool {
        if !self.refresh_drag_endpoint(endpoint, outcome) {
            return false;
        }
        outcome.mark_rebuild(endpoint);
        if endpoint.owner.is_none() {
            return true;
        }
        for _ in 0..2 {
            let Some(messages) = self.take_drag_endpoint_messages(endpoint) else {
                return false;
            };
            if messages.is_empty() {
                return true;
            }
            if !self.drag_reduce_messages(endpoint, messages, outcome)
                || !self.refresh_drag_endpoint(endpoint, outcome)
            {
                return false;
            }
            outcome.mark_rebuild(endpoint);
        }
        // A refresh-generated terminal mapper must not be deferred past the
        // source fence. Reduce the bounded final batch, then fail closed
        // rather than invoke another target callback without a fresh surface.
        let Some(messages) = self.take_drag_endpoint_messages(endpoint) else {
            return false;
        };
        if !messages.is_empty() {
            let _ = self.drag_reduce_messages(endpoint, messages, outcome);
            return false;
        }
        true
    }

    pub(in crate::gui_runtime::native_vello::generic_runtime) fn prepare_drag_sample(
        &self,
        window: WindowId,
        event: &WindowEvent,
    ) -> Option<NativeDragSample<Message>> {
        if !cross_window_hit::SUPPORTED {
            return None;
        }
        let source = self.drag_endpoint(window)?;
        let (candidate, cursor, scale) = if source.owner.is_none() {
            (
                self.core.runtime.has_cross_window_drag_candidate(),
                self.input.last_cursor,
                self.window.dpi_scale,
            )
        } else {
            let window = self
                .auxiliary_windows
                .iter()
                .find(|window| source.matches_auxiliary(window))?;
            (
                window.runner.core.runtime.has_cross_window_drag_candidate(),
                window.runner.input.last_cursor,
                window.runner.window.dpi_scale,
            )
        };
        if !candidate {
            return None;
        }
        let position = match event {
            WindowEvent::CursorMoved { position, .. } => logical_point_from_winit(*position, scale),
            WindowEvent::MouseInput {
                state: winit::event::ElementState::Released,
                ..
            } => cursor,
            WindowEvent::Touch(touch) => logical_point_from_winit(touch.location, scale),
            _ => return None,
        }?;
        let location = self.capture_drag_location(&source, position)?;
        let receiver = self.resolve_drag_receiver(location);
        let previous = self
            .cross_window_transfers
            .iter()
            .any(|transfer| transfer.source.same(&source));
        // Preserve the ordinary local path until a sample actually leaves the
        // source window. Returning from a foreign target needs ordered cleanup.
        if !previous
            && receiver
                .as_ref()
                .is_some_and(|(receiver, _)| receiver.same(&source))
        {
            return None;
        }
        Some(NativeDragSample {
            source,
            location,
            parent_projection: self.drag_parent_projection(),
            input: NativeCrossWindowInput::new(CrossWindowInputHint::foreign_or_none()),
        })
    }

    pub(in crate::gui_runtime::native_vello::generic_runtime) fn drag_endpoint(
        &self,
        window: WindowId,
    ) -> Option<NativeDragEndpoint> {
        if !self.is_running() {
            return None;
        }
        if self.window.id == Some(window) {
            return Some(NativeDragEndpoint {
                window,
                owner: None,
            });
        }
        self.auxiliary_windows.iter().find_map(|candidate| {
            let endpoint = NativeDragEndpoint {
                window,
                owner: Some(candidate.effect_owner()),
            };
            endpoint.matches_auxiliary(candidate).then_some(endpoint)
        })
    }

    pub(in crate::gui_runtime::native_vello::generic_runtime) fn drag_source_export(
        &self,
        endpoint: &NativeDragEndpoint,
    ) -> Option<CrossWindowDragExport> {
        if !self.is_running() {
            return None;
        }
        if endpoint.owner.is_none() {
            return (self.window.id == Some(endpoint.window))
                .then(|| self.core.runtime.cross_window_drag_export())
                .flatten();
        }
        self.auxiliary_windows
            .iter()
            .find(|window| endpoint.matches_auxiliary(window))
            .and_then(|window| window.runner.core.runtime.cross_window_drag_export())
    }

    /// Read-only redraw fence for a receiver whose source capture disappeared
    /// during prepared work. The caller only schedules the ordinary semantic
    /// cleanup route; it must not map callbacks from a redraw itself.
    pub(in crate::gui_runtime::native_vello::generic_runtime) fn has_expired_drag_transfer(
        &self,
    ) -> bool {
        self.cross_window_transfers.iter().any(|transfer| {
            match self.drag_source_export(&transfer.source) {
                Some(export) => export.key() != transfer.key || !export.is_live(),
                None => true,
            }
        })
    }

    fn capture_drag_location(
        &self,
        source: &NativeDragEndpoint,
        position: Point,
    ) -> Option<cross_window_hit::NativeDragLocation> {
        let source_window = if source.owner.is_none() {
            (self.window.id == Some(source.window))
                .then_some(self.window.window.as_deref())
                .flatten()
        } else {
            self.auxiliary_windows
                .iter()
                .find(|window| source.matches_auxiliary(window))
                .and_then(|window| window.runner.window.window.as_deref())
        }?;
        cross_window_hit::capture_drag_location(source_window, position)
    }

    fn resolve_drag_receiver(
        &self,
        location: cross_window_hit::NativeDragLocation,
    ) -> Option<(NativeDragEndpoint, Point)> {
        if !self.is_running() || !cross_window_hit::SUPPORTED {
            return None;
        }
        // Keep hit selection bounded, including the primary window. Reaching
        // capacity fails closed rather than selecting an arbitrary subset.
        let mut candidates = Vec::new();
        if let Some(window) = self.window.window.as_deref() {
            candidates.push(window);
        }
        for window in &self.auxiliary_windows {
            if !window.active || !window.is_admitted() || window.recovery_rebuild_pending {
                continue;
            }
            if let Some(window) = window.runner.window.window.as_deref() {
                if candidates.len() == 64 {
                    return None;
                }
                candidates.push(window);
            }
        }
        let hit = cross_window_hit::hit_test_drag_location(location, &candidates)?;
        Some((self.drag_endpoint(hit.window)?, hit.position))
    }

    fn drag_remove_transfer(
        &mut self,
        source: &NativeDragEndpoint,
        key: CrossWindowDragKey,
    ) -> Option<NativeDragTransfer> {
        let index = self
            .cross_window_transfers
            .iter()
            .position(|transfer| transfer.source.same(source) && transfer.key == key)?;
        Some(self.cross_window_transfers.remove(index))
    }

    fn drag_transfer(
        &self,
        source: &NativeDragEndpoint,
        key: CrossWindowDragKey,
    ) -> Option<(NativeDragEndpoint, u64)> {
        self.cross_window_transfers
            .iter()
            .find(|transfer| transfer.source.same(source) && transfer.key == key)
            .map(|transfer| (transfer.receiver.clone(), transfer.parent_projection))
    }

    fn drag_store_transfer(
        &mut self,
        source: NativeDragEndpoint,
        key: CrossWindowDragKey,
        receiver: NativeDragEndpoint,
        location: cross_window_hit::NativeDragLocation,
        position: Point,
    ) -> bool {
        let parent_projection = self.drag_parent_projection();
        if let Some(transfer) = self
            .cross_window_transfers
            .iter_mut()
            .find(|transfer| transfer.source.same(&source) && transfer.key == key)
        {
            transfer.receiver = receiver;
            transfer.parent_projection = parent_projection;
            transfer.location = location;
            transfer.position = position;
            return true;
        }
        if self.cross_window_transfers.len() == 64 {
            return false;
        }
        self.cross_window_transfers.push(NativeDragTransfer {
            source,
            key,
            receiver,
            parent_projection,
            location,
            position,
        });
        true
    }

    fn drag_prune_expired_transfers(
        &mut self,
        preserve: Option<(&NativeDragEndpoint, CrossWindowDragKey)>,
        outcome: &mut NativeDragRoute,
    ) {
        let candidates: Vec<_> = self
            .cross_window_transfers
            .iter()
            .filter(|transfer| {
                !preserve.is_some_and(|(source, key)| {
                    transfer.source.same(source) && transfer.key == key
                })
            })
            .map(|transfer| {
                (
                    transfer.source.clone(),
                    transfer.key,
                    transfer.receiver.clone(),
                    transfer.parent_projection,
                )
            })
            .collect();
        for (source, key, receiver, projection) in candidates {
            let current = self.drag_source_export(&source).is_some_and(|export| {
                export.key() == key
                    && export.is_live()
                    && self.drag_source_proof_is_current(&source, &export.source_proof())
            });
            if current {
                continue;
            }
            let _ = self.drag_remove_transfer(&source, key);
            if projection != self.drag_parent_projection()
                && !self.drag_refresh_endpoint_and_drain(&receiver, outcome)
            {
                self.drag_discard_foreign(&receiver, key);
                continue;
            }
            let Some(route) = self.drag_cancel_foreign(&receiver, key) else {
                continue;
            };
            if self
                .drag_reduce_foreign_route(&receiver, route, outcome)
                .is_none()
            {
                self.drag_discard_foreign(&receiver, key);
            }
        }
    }

    fn drag_reduce_foreign_route(
        &mut self,
        endpoint: &NativeDragEndpoint,
        route: CrossWindowForeignRoute<Message>,
        outcome: &mut NativeDragRoute,
    ) -> Option<bool> {
        let needs_transition = route.needs_transition();
        let messages = route.into_messages();
        // Feedback/preview snapshots can change without an application mapper.
        // Keep that common path paint-only; mapper-driven surface refreshes
        // upgrade it to a base-scene rebuild below.
        outcome.mark_paint(endpoint);
        if !messages.is_empty() {
            if !self.drag_reduce_messages(endpoint, messages, outcome)
                || !self.drag_refresh_endpoint_and_drain(endpoint, outcome)
            {
                return None;
            }
            outcome.mark_rebuild(endpoint);
        }
        Some(needs_transition)
    }

    /// A receiver mapper reduces through the parent bridge. If it changed the
    /// parent projection, an auxiliary source still holds the previous bridge
    /// surface until it is explicitly refreshed. Refresh it before any next
    /// receiver phase, source mapping, or transfer retention; the proof then
    /// fences a source removed by that same reducer.
    fn drag_refresh_source_after_receiver_reduction(
        &mut self,
        source: &NativeDragEndpoint,
        proof: &CrossWindowSourceProof,
        projection_before: u64,
        outcome: &mut NativeDragRoute,
    ) -> bool {
        if projection_before != self.drag_parent_projection()
            && source.owner.is_some()
            && !self.drag_refresh_endpoint_and_drain(source, outcome)
        {
            return false;
        }
        self.drag_source_proof_is_current(source, proof)
    }

    /// Map at most two ordinary receiver transitions, refreshing only after a
    /// reducer can have changed the parent projection. The final feedback
    /// validation is intentionally silent so an `Over` cannot loop forever.
    fn drag_drive_foreign<F>(
        &mut self,
        endpoint: &NativeDragEndpoint,
        drive: ForeignDrive,
        outcome: &mut NativeDragRoute,
        receiver_at: &mut F,
    ) -> bool
    where
        F: FnMut(
            &Self,
            cross_window_hit::NativeDragLocation,
        ) -> Option<(NativeDragEndpoint, Point)>,
    {
        let Some(mut route) = self.drag_route_foreign(endpoint, drive.input) else {
            return false;
        };
        for step in 0..=2 {
            let projection_before = self.drag_parent_projection();
            let Some(needs_transition) = self.drag_reduce_foreign_route(endpoint, route, outcome)
            else {
                return false;
            };
            let projection_after_receiver_reduction = self.drag_parent_projection();
            if !self.drag_refresh_source_after_receiver_reduction(
                &drive.source,
                &drive.source_proof,
                projection_before,
                outcome,
            ) {
                return false;
            }
            if self.drag_parent_projection() != projection_after_receiver_reduction {
                // Refreshing the source can itself reduce a cancellation. Requalify
                // the receiver once for that parent projection. A receiver refresh
                // that changes projection again has no bounded, current pair of
                // endpoint proofs, so fail closed rather than map another phase.
                let projection_before_receiver_refresh = self.drag_parent_projection();
                if !self.drag_refresh_endpoint_and_drain(endpoint, outcome)
                    || self.drag_parent_projection() != projection_before_receiver_refresh
                {
                    return false;
                }
            }
            if !self.drag_source_proof_is_current(&drive.source, &drive.source_proof) {
                return false;
            }
            if !receiver_at(self, drive.location).is_some_and(|(current, current_position)| {
                current.same(endpoint) && current_position == drive.position
            }) {
                return false;
            }
            if !needs_transition {
                if self.drag_finish_foreign_feedback(endpoint, drive.key) {
                    outcome.mark_paint(endpoint);
                }
                return true;
            }
            if step == 2 {
                return false;
            }
            let Some(next) = self.drag_requalify_foreign(endpoint, drive.key) else {
                return false;
            };
            route = next;
        }
        false
    }

    fn drag_drive_local_target<F>(
        &mut self,
        endpoint: &NativeDragEndpoint,
        key: CrossWindowDragKey,
        location: cross_window_hit::NativeDragLocation,
        position: Point,
        outcome: &mut NativeDragRoute,
        receiver_at: &mut F,
    ) -> bool
    where
        F: FnMut(
            &Self,
            cross_window_hit::NativeDragLocation,
        ) -> Option<(NativeDragEndpoint, Point)>,
    {
        for step in 0..=2 {
            let Some(route) = self.drag_advance_local_target(endpoint, key) else {
                return false;
            };
            let Some(needs_transition) = self.drag_reduce_foreign_route(endpoint, route, outcome)
            else {
                return false;
            };
            if !receiver_at(self, location).is_some_and(|(current, current_position)| {
                current.same(endpoint) && current_position == position
            }) {
                return false;
            }
            if !needs_transition {
                return true;
            }
            if step == 2 {
                return false;
            }
        }
        false
    }

    fn drag_clear_previous_receiver(
        &mut self,
        transfer: NativeDragTransfer,
        outcome: &mut NativeDragRoute,
    ) -> bool {
        let Some(route) = self.drag_clear_foreign(&transfer.receiver, transfer.key) else {
            return false;
        };
        self.drag_reduce_foreign_route(&transfer.receiver, route, outcome)
            .is_some()
    }

    fn drag_cancel_receiver(
        &mut self,
        receiver: &NativeDragEndpoint,
        key: CrossWindowDragKey,
        outcome: &mut NativeDragRoute,
    ) {
        if let Some(route) = self.drag_cancel_foreign(receiver, key) {
            let _ = self.drag_reduce_foreign_route(receiver, route, outcome);
        }
    }

    /// A source refresh can reduce its detached cancellation through the
    /// parent after a receiver mapped a phase. Do not send receiver cleanup to
    /// the earlier projection when that reduction changed the parent again.
    fn drag_cancel_receiver_after_source_refresh(
        &mut self,
        receiver: &NativeDragEndpoint,
        key: CrossWindowDragKey,
        projection_before: u64,
        outcome: &mut NativeDragRoute,
    ) {
        if projection_before != self.drag_parent_projection()
            && !self.drag_refresh_endpoint_and_drain(receiver, outcome)
        {
            self.drag_discard_foreign(receiver, key);
            return;
        }
        self.drag_cancel_receiver(receiver, key, outcome);
    }

    fn drag_refresh_receiver_if_needed(
        &mut self,
        receiver: &NativeDragEndpoint,
        previous: Option<(NativeDragEndpoint, u64)>,
        outcome: &mut NativeDragRoute,
    ) -> bool {
        previous.is_some_and(|(endpoint, stamp)| {
            endpoint.same(receiver) && stamp == self.drag_parent_projection()
        }) || self.drag_refresh_endpoint_and_drain(receiver, outcome)
    }

    fn drag_terminal_input(
        request: &CrossWindowTerminalRequest<Message>,
        position: Point,
    ) -> CrossWindowForeignInput {
        CrossWindowForeignInput::new(
            request.key(),
            request.offer(),
            request.source(),
            request.lease(),
            position,
            request.modifiers(),
            request.autoscroll_policy(),
            request.metadata(),
        )
    }

    fn drag_export_input(
        export: &CrossWindowDragExport,
        position: Point,
    ) -> CrossWindowForeignInput {
        CrossWindowForeignInput::new(
            export.key(),
            export.offer(),
            export.source(),
            export.lease(),
            position,
            export.modifiers(),
            export.autoscroll_policy(),
            export.metadata(),
        )
    }

    /// Sweep bounded foreign receipts after ordinary native routing. This is
    /// intentionally separate from sample preparation: focus loss, a second
    /// contact, or command-driven drag retirement may not have a cursor sample
    /// to prepare, but a live receiver still needs one deterministic cleanup.
    pub(in crate::gui_runtime::native_vello::generic_runtime) fn route_drag_cancellations(
        &mut self,
    ) -> NativeDragRoute {
        let mut outcome = NativeDragRoute::default();
        self.drag_prune_expired_transfers(None, &mut outcome);
        outcome
    }

    /// Publish only the retained visual dirtiness after the caller has closed
    /// the native input ticket. Semantic routing has already happened in
    /// `route_drag_sample`; a completion mismatch must not publish it.
    pub(in crate::gui_runtime::native_vello::generic_runtime) fn apply_drag_route_visuals(
        &mut self,
        route: &NativeDragRoute,
        completion: Option<super::super::frame_scheduler_policy::NativeInputStageDisposition>,
    ) {
        use super::super::frame_scheduler_policy::NativeInputStageDisposition;

        let Some(completion) = completion else {
            return;
        };
        for visual in &route.visual_work {
            let endpoint = &visual.endpoint;
            let frame_work = match visual.kind {
                NativeDragVisualKind::PaintOnly => FrameWork::PaintOnly {
                    reason: FrameWorkReason::ExternalDragPreview,
                },
                NativeDragVisualKind::RebuildScene => FrameWork::RebuildScene {
                    reason: FrameWorkReason::ExternalDragPreview,
                    mode: SceneRebuildMode::Immediate,
                },
            };
            if endpoint.owner.is_none() {
                if self.window.id != Some(endpoint.window) {
                    continue;
                }
                match completion {
                    NativeInputStageDisposition::ContinueNow => {
                        self.request_redraw_for_frame_work(frame_work);
                    }
                    NativeInputStageDisposition::DeferLowerPriority => {
                        if visual.kind == NativeDragVisualKind::RebuildScene {
                            self.defer_scene_rebuild();
                        }
                        self.request_redraw_for_deferred_frame_work(FrameWork::PaintOnly {
                            reason: FrameWorkReason::ExternalDragPreview,
                        });
                    }
                }
                continue;
            }
            let Some(window) = self
                .auxiliary_windows
                .iter_mut()
                .find(|window| endpoint.matches_auxiliary(window))
            else {
                continue;
            };
            match completion {
                NativeInputStageDisposition::ContinueNow => {
                    window.runner.request_redraw_for_frame_work(frame_work);
                }
                NativeInputStageDisposition::DeferLowerPriority => {
                    if visual.kind == NativeDragVisualKind::RebuildScene {
                        window.runner.defer_scene_rebuild();
                    }
                    window
                        .runner
                        .request_redraw_for_deferred_frame_work(FrameWork::PaintOnly {
                            reason: FrameWorkReason::ExternalDragPreview,
                        });
                }
            }
        }
    }

    /// Route semantic cross-window work during a live admitted native input
    /// ticket. It deliberately does not publish native scene or paint work:
    /// the caller merges the returned outcome before closing the ticket, then
    /// applies `visual_work` using the ticket's completion disposition.
    pub(in crate::gui_runtime::native_vello::generic_runtime) fn route_drag_sample(
        &mut self,
        sample: NativeDragSample<Message>,
    ) -> NativeDragRoute {
        self.route_drag_sample_with_resolver(sample, &mut |host, location| {
            host.resolve_drag_receiver(location)
        })
    }

    #[cfg(test)]
    fn route_drag_sample_with_test_receiver<F>(
        &mut self,
        sample: NativeDragSample<Message>,
        mut receiver_at: F,
    ) -> NativeDragRoute
    where
        F: FnMut(
            &Self,
            cross_window_hit::NativeDragLocation,
        ) -> Option<(NativeDragEndpoint, Point)>,
    {
        self.route_drag_sample_with_resolver(sample, &mut receiver_at)
    }

    fn route_drag_sample_with_resolver<F>(
        &mut self,
        mut sample: NativeDragSample<Message>,
        receiver_at: &mut F,
    ) -> NativeDragRoute
    where
        F: FnMut(
            &Self,
            cross_window_hit::NativeDragLocation,
        ) -> Option<(NativeDragEndpoint, Point)>,
    {
        let mut outcome = NativeDragRoute::default();
        if !matches!(
            sample.input.last_disposition,
            Some(PointerIngressDisposition::RoutedGesture(_))
        ) {
            // A focus loss, a second contact, or a command can retire a
            // source before this sample. Its foreign receiver still gets its
            // one qualified cancellation inside the current input ticket.
            self.drag_prune_expired_transfers(None, &mut outcome);
            return outcome;
        }

        if let Some(request) = sample.input.terminal.take() {
            let key = request.key();
            self.drag_prune_expired_transfers(Some((&sample.source, key)), &mut outcome);
            if !self.drag_source_proof_is_current(&sample.source, request.source_proof()) {
                self.drag_prune_expired_transfers(None, &mut outcome);
                return outcome;
            }
            if sample.source.owner.is_some()
                && sample.parent_projection != self.drag_parent_projection()
                && !self.drag_refresh_endpoint_and_drain(&sample.source, &mut outcome)
            {
                self.drag_prune_expired_transfers(None, &mut outcome);
                return outcome;
            }
            if !self.drag_source_proof_is_current(&sample.source, request.source_proof()) {
                self.drag_prune_expired_transfers(None, &mut outcome);
                return outcome;
            }
            let mut previous = self.drag_remove_transfer(&sample.source, key);

            let mut terminal = None;
            let mut terminal_receiver = None;
            let mut receiver = receiver_at(self, sample.location);
            if receiver.as_ref().is_some_and(|(endpoint, position)| {
                endpoint.same(&sample.source) && *position != request.position()
            }) {
                receiver = None;
            }
            if let Some((receiver_endpoint, _)) = receiver.as_ref() {
                let projection_before_previous = self.drag_parent_projection();
                if previous
                    .as_ref()
                    .is_some_and(|transfer| !receiver_endpoint.same(&transfer.receiver))
                    && let Some(previous) = previous.take()
                    && !self.drag_clear_previous_receiver(previous, &mut outcome)
                {
                    return outcome;
                }
                // `Left` can synchronously retire the source. No next target
                // callback is admitted from a terminal request after that
                // retirement, even though the request itself is already
                // detached from the old capture.
                if !self.drag_refresh_source_after_receiver_reduction(
                    &sample.source,
                    request.source_proof(),
                    projection_before_previous,
                    &mut outcome,
                ) {
                    return outcome;
                }
                receiver = receiver_at(self, sample.location);
            }
            if receiver.as_ref().is_some_and(|(endpoint, position)| {
                endpoint.same(&sample.source) && *position != request.position()
            }) {
                receiver = None;
            }
            if let Some((receiver, position)) = receiver.as_ref() {
                let prior = previous
                    .as_ref()
                    .map(|transfer| (transfer.receiver.clone(), transfer.parent_projection));
                let projection_before_receiver = self.drag_parent_projection();
                if self.drag_refresh_receiver_if_needed(receiver, prior, &mut outcome)
                    && self.drag_refresh_source_after_receiver_reduction(
                        &sample.source,
                        request.source_proof(),
                        projection_before_receiver,
                        &mut outcome,
                    )
                    && self.drag_drive_foreign(
                        receiver,
                        ForeignDrive {
                            source: sample.source.clone(),
                            source_proof: request.source_proof().clone(),
                            key,
                            location: sample.location,
                            position: *position,
                            input: Self::drag_terminal_input(&request, *position),
                        },
                        &mut outcome,
                        receiver_at,
                    )
                {
                    if self.drag_source_proof_is_current(&sample.source, request.source_proof()) {
                        terminal = self.drag_take_foreign_terminal(receiver, key);
                        terminal_receiver = terminal.as_ref().map(|_| receiver.clone());
                        if terminal.is_some() {
                            outcome.mark_paint(receiver);
                        }
                    } else {
                        self.drag_cancel_receiver_after_source_refresh(
                            receiver,
                            key,
                            projection_before_receiver,
                            &mut outcome,
                        );
                    }
                } else {
                    self.drag_cancel_receiver_after_source_refresh(
                        receiver,
                        key,
                        projection_before_receiver,
                        &mut outcome,
                    );
                }
            } else if let Some(previous) = previous.take() {
                self.drag_cancel_receiver(&previous.receiver, key, &mut outcome);
            }
            // A failed drive may have reduced receiver cleanup after its last
            // source-proof check. Requalify only this fallback path: a taken
            // terminal has already proved both endpoints and must retain its
            // premapped target/source completion pair.
            if terminal.is_none()
                && !self.drag_refresh_source_after_receiver_reduction(
                    &sample.source,
                    request.source_proof(),
                    sample.parent_projection,
                    &mut outcome,
                )
            {
                if let Some(receiver) = terminal_receiver.as_ref() {
                    self.drag_discard_foreign(receiver, key);
                }
                return outcome;
            }
            let messages = self.drag_map_terminal(
                &sample.source,
                &request,
                terminal.as_ref(),
                DragCancelReason::NoTarget,
            );
            let Some(messages) = messages else {
                if let Some(receiver) = terminal_receiver.as_ref() {
                    self.drag_discard_foreign(receiver, key);
                }
                return outcome;
            };
            if let (Some(receiver), Some(message)) = (terminal_receiver.as_ref(), messages.target) {
                let _ = self.drag_reduce_messages(receiver, vec![message], &mut outcome);
            }
            if let Some(message) = messages.source {
                let _ = self.drag_reduce_messages(&sample.source, vec![message], &mut outcome);
            }
            if let Some(receiver) = terminal_receiver.as_ref() {
                let _ = self.drag_refresh_endpoint_and_drain(receiver, &mut outcome);
            }
            let _ = self.drag_refresh_endpoint_and_drain(&sample.source, &mut outcome);
            outcome.mark_paint(&sample.source);
            return outcome;
        }

        let Some(export) = self.drag_source_export(&sample.source) else {
            self.drag_prune_expired_transfers(None, &mut outcome);
            return outcome;
        };
        let key = export.key();
        self.drag_prune_expired_transfers(Some((&sample.source, key)), &mut outcome);
        if !self.drag_source_proof_is_current(&sample.source, &export.source_proof()) {
            self.drag_prune_expired_transfers(None, &mut outcome);
            return outcome;
        }
        if sample.source.owner.is_some()
            && sample.parent_projection != self.drag_parent_projection()
            && !self.drag_refresh_endpoint_and_drain(&sample.source, &mut outcome)
        {
            self.drag_prune_expired_transfers(None, &mut outcome);
            return outcome;
        }
        if !self.drag_source_proof_is_current(&sample.source, &export.source_proof()) {
            self.drag_prune_expired_transfers(None, &mut outcome);
            return outcome;
        }
        let previous = self.drag_remove_transfer(&sample.source, key);
        let projection_before_previous = self.drag_parent_projection();
        let mut receiver = receiver_at(self, sample.location);
        let mut foreign = receiver
            .as_ref()
            .filter(|(receiver, _)| !receiver.same(&sample.source));
        if let Some(previous) = previous {
            if foreign.is_some_and(|(receiver, _)| receiver.same(&previous.receiver)) {
                self.cross_window_transfers.push(previous);
            } else if !self.drag_clear_previous_receiver(previous, &mut outcome) {
                return outcome;
            } else {
                if !self.drag_refresh_source_after_receiver_reduction(
                    &sample.source,
                    &export.source_proof(),
                    projection_before_previous,
                    &mut outcome,
                ) {
                    return outcome;
                }
                // `Left` is an application callback: it can move, close, or
                // cover a window. Re-hit the frozen screen location before a
                // subsequent receiver gets Entered or Over.
                receiver = receiver_at(self, sample.location);
                foreign = receiver
                    .as_ref()
                    .filter(|(receiver, _)| !receiver.same(&sample.source));
            }
        }
        if !self.drag_source_proof_is_current(&sample.source, &export.source_proof()) {
            return outcome;
        }
        if let Some((receiver, position)) = foreign {
            let prior = self.drag_transfer(&sample.source, key);
            let projection_before_receiver = self.drag_parent_projection();
            if !self.drag_refresh_receiver_if_needed(receiver, prior, &mut outcome)
                || !self.drag_refresh_source_after_receiver_reduction(
                    &sample.source,
                    &export.source_proof(),
                    projection_before_receiver,
                    &mut outcome,
                )
                || !self.drag_drive_foreign(
                    receiver,
                    ForeignDrive {
                        source: sample.source.clone(),
                        source_proof: export.source_proof(),
                        key,
                        location: sample.location,
                        position: *position,
                        input: Self::drag_export_input(&export, *position),
                    },
                    &mut outcome,
                    receiver_at,
                )
            {
                let _ = self.drag_remove_transfer(&sample.source, key);
                self.drag_cancel_receiver_after_source_refresh(
                    receiver,
                    key,
                    projection_before_receiver,
                    &mut outcome,
                );
                return outcome;
            }
            if !self.drag_source_proof_is_current(&sample.source, &export.source_proof()) {
                let _ = self.drag_remove_transfer(&sample.source, key);
                self.drag_cancel_receiver_after_source_refresh(
                    receiver,
                    key,
                    projection_before_receiver,
                    &mut outcome,
                );
                return outcome;
            }
            if !self.drag_store_transfer(
                sample.source.clone(),
                key,
                receiver.clone(),
                sample.location,
                *position,
            ) {
                self.drag_discard_foreign(receiver, key);
                return outcome;
            }
            let target = self.drag_foreign_target_id(receiver, key);
            if sample.input.source_moved == Some(key)
                && let Some(message) = self.drag_map_source_moved(&sample.source, key, target)
                && self.drag_reduce_messages(&sample.source, vec![message], &mut outcome)
            {
                let _ = self.drag_refresh_endpoint_and_drain(&sample.source, &mut outcome);
                let _ = self.drag_refresh_endpoint_and_drain(receiver, &mut outcome);
            }
        } else {
            if !self.drag_drive_local_target(
                &sample.source,
                key,
                sample.location,
                export.position(),
                &mut outcome,
                receiver_at,
            ) {
                return outcome;
            }
            let target = self.drag_local_target_id(&sample.source, key);
            if sample.input.source_moved == Some(key)
                && let Some(message) = self.drag_map_source_moved(&sample.source, key, target)
                && self.drag_reduce_messages(&sample.source, vec![message], &mut outcome)
            {
                let _ = self.drag_refresh_endpoint_and_drain(&sample.source, &mut outcome);
            }
        }
        outcome
    }
}

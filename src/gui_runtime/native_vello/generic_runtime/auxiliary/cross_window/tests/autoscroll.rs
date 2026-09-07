use super::*;
use crate::{application::scroll, gui::drag_drop::DragAutoscrollPolicy, runtime::Command};
use std::time::Instant;

struct ScrollingBridge {
    scrolled: bool,
    auxiliary_source: bool,
    events: Rc<RefCell<Vec<Message>>>,
}

fn scrolling_source() -> Arc<crate::runtime::UiSurface<Message>> {
    crate::runtime::test_arc_surface(
        button("source")
            .filter_mapped(|_| None::<Message>)
            .width(100.0)
            .height(100.0)
            .id(1)
            .drag_source(
                DragSource::new(7_u8)
                    .autoscroll(DragAutoscrollPolicy::default())
                    .on_event_with_revision((), |event| Some(Message::Source(event.phase()))),
            )
            .id(10)
            .into_surface(),
    )
}

fn scrolling_receiver(scrolled: bool) -> Arc<crate::runtime::UiSurface<Message>> {
    let label = if scrolled { "updated" } else { "original" };
    crate::runtime::test_arc_surface(
        scroll(
            button("receiver")
                .filter_mapped(|_| None::<Message>)
                .width(100.0)
                .height(240.0)
                .id(2)
                .drop_target(
                    DropTarget::<u8, Message>::new().on_event_with_revision(label, move |event| {
                        Some(Message::Target(label, event.phase()))
                    }),
                )
                .id(if scrolled { 21 } else { 20 }),
        )
        .width(100.0)
        .height(100.0)
        .id(30)
        .on_scroll_update(|_| Message::Scrolled)
        .into_surface(),
    )
}

impl RuntimeBridge<Message> for ScrollingBridge {
    fn project_surface(&mut self) -> Arc<crate::runtime::UiSurface<Message>> {
        scrolling_source()
    }
    fn update(&mut self, message: Message) -> Command<Message> {
        self.events.borrow_mut().push(message);
        if message == Message::Scrolled {
            self.scrolled = true;
            Command::RequestProjectionRefresh
        } else {
            Command::none()
        }
    }
    fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, Message> {
        RuntimeHostCapabilities::new().with_windows()
    }
}
impl RuntimeWindowHost<Message> for ScrollingBridge {
    fn project_auxiliary_windows(&mut self) -> Vec<crate::runtime::AuxiliaryWindow<Message>> {
        let mut windows = vec![crate::runtime::AuxiliaryWindow::new(
            "receiver",
            Default::default(),
            scrolling_receiver(self.scrolled),
        )];
        if self.auxiliary_source && !self.scrolled {
            windows.push(crate::runtime::AuxiliaryWindow::new(
                "source",
                Default::default(),
                scrolling_source(),
            ));
        }
        windows
    }
}

type Fixture = (
    GenericNativeVelloRunner<ScrollingBridge, Message>,
    Rc<RefCell<Vec<Message>>>,
    NativeDragEndpoint,
    Instant,
);
fn fixture() -> Fixture {
    fixture_with_auxiliary_source(false)
}

fn fixture_with_auxiliary_source(auxiliary_source: bool) -> Fixture {
    let events = Rc::new(RefCell::new(Vec::new()));
    let mut parent = GenericNativeVelloRunner::new(
        Default::default(),
        ScrollingBridge {
            scrolled: false,
            auxiliary_source,
            events: Rc::clone(&events),
        },
        Vector2::new(100.0, 100.0),
    );
    parent.window.id = Some(WindowId::from(201_u64));
    let owner = parent
        .core
        .runtime
        .acquire_auxiliary_effect_owner("receiver");
    let mut child = AuxiliaryNativeWindow::new_with_owner(
        crate::runtime::AuxiliaryWindow::new(
            "receiver",
            Default::default(),
            scrolling_receiver(false),
        ),
        &Default::default(),
        None,
        false,
        false,
        false,
        owner,
    );
    child.runner.window.id = Some(WindowId::from(202_u64));
    child
        .runner
        .core
        .runtime
        .set_viewport(Vector2::new(100.0, 100.0));
    parent.auxiliary_windows.push(child);
    let (export, source_id) = if auxiliary_source {
        let owner = parent.core.runtime.acquire_auxiliary_effect_owner("source");
        let mut source = AuxiliaryNativeWindow::new_with_owner(
            crate::runtime::AuxiliaryWindow::new("source", Default::default(), scrolling_source()),
            &Default::default(),
            None,
            false,
            false,
            false,
            owner,
        );
        let source_id = WindowId::from(203_u64);
        source.runner.window.id = Some(source_id);
        source
            .runner
            .core
            .runtime
            .set_viewport(Vector2::new(100.0, 100.0));
        let export = activate_source(&mut source.runner.core.runtime);
        parent.auxiliary_windows.push(source);
        (export, source_id)
    } else {
        (
            activate_source(&mut parent.core.runtime),
            WindowId::from(201_u64),
        )
    };
    let source = parent.drag_endpoint(source_id).unwrap();
    let receiver = parent.drag_endpoint(WindowId::from(202_u64)).unwrap();
    let route = sample(source, parent.drag_parent_projection(), export.key());
    parent.route_drag_sample_with_test_receiver(route, |_, _| {
        Some((receiver.clone(), Point::new(20.0, 98.0)))
    });
    let runtime = &mut parent.auxiliary_windows[0].runner.core.runtime;
    let deadline = runtime
        .timed_repaint_deadline()
        .expect("foreign edge deadline");
    runtime.advance_timed_repaints(deadline);
    assert!(runtime.has_pending_cross_window_autoscroll());
    events.borrow_mut().clear();
    (parent, events, receiver, deadline)
}

#[test]
fn foreign_tick_reduces_scroll_before_target_requalification() {
    on_large_stack(|| {
        let (mut parent, events, receiver, deadline) = fixture();
        // A primary timer must not consume this auxiliary owner's pending tick.
        parent.route_drag_autoscroll_for_owner(None, deadline);
        assert!(events.borrow().is_empty());
        assert!(
            parent.auxiliary_windows[0]
                .runner
                .core
                .runtime
                .has_pending_cross_window_autoscroll()
        );
        let result =
            parent.route_drag_autoscroll_with_resolver(&receiver, deadline, &mut |_, _| {
                Some((receiver.clone(), Point::new(20.0, 98.0)))
            });
        let events = events.borrow();
        assert_eq!(events.first(), Some(&Message::Scrolled));
        assert!(events.contains(&Message::Target("updated", DropPhase::Entered)));
        assert!(!events.contains(&Message::Target("original", DropPhase::Over)));
        assert!(!result.visual_work.is_empty());
        assert!(parent.core.runtime.drag_session_active());
    });
}

#[test]
fn foreign_tick_native_hit_mismatch_leaves_without_scrolling() {
    on_large_stack(|| {
        let (mut parent, events, receiver, deadline) = fixture();
        parent.route_drag_autoscroll_with_resolver(&receiver, deadline, &mut |_, _| None);
        parent.route_drag_autoscroll_with_resolver(&receiver, deadline, &mut |_, _| None);
        assert!(!events.borrow().contains(&Message::Scrolled));
        assert_eq!(
            events
                .borrow()
                .iter()
                .filter(|event| **event == Message::Target("original", DropPhase::Left))
                .count(),
            1
        );
        assert!(parent.cross_window_transfers.is_empty());
        assert!(
            !parent.auxiliary_windows[0]
                .runner
                .core
                .runtime
                .has_pending_cross_window_autoscroll()
        );
    });
}

#[test]
fn foreign_tick_cannot_scroll_after_source_retirement() {
    on_large_stack(|| {
        let (mut parent, events, receiver, deadline) = fixture();
        parent.core.runtime.execute_command(Command::end_drag());
        events.borrow_mut().clear();
        parent.route_drag_autoscroll_with_resolver(&receiver, deadline, &mut |_, _| {
            Some((receiver.clone(), Point::new(20.0, 98.0)))
        });
        parent.route_drag_cancellations();
        assert!(!events.borrow().contains(&Message::Scrolled));
        assert_eq!(
            events
                .borrow()
                .iter()
                .filter(|event| **event == Message::Target("original", DropPhase::Cancelled))
                .count(),
            1
        );
        assert!(
            !parent.auxiliary_windows[0]
                .runner
                .core
                .runtime
                .has_pending_cross_window_autoscroll()
        );
    });
}

#[test]
fn failed_child_deadline_discards_foreign_tick_without_later_replay() {
    on_large_stack(|| {
        let (mut parent, events, receiver, deadline) = fixture();
        let child = &mut parent.auxiliary_windows[0].runner;
        child.collect_timed_drag_cancellations(&mut GenericRouteOutcome::default());
        child.finish_timed_drag_visuals(false);
        assert!(!child.take_foreign_autoscroll_completed_since_event());
        assert!(!child.core.runtime.has_pending_cross_window_autoscroll());
        child.finish_timed_drag_visuals(true);
        assert!(!child.take_foreign_autoscroll_completed_since_event());
        parent.route_drag_autoscroll_with_resolver(&receiver, deadline, &mut |_, _| {
            Some((receiver.clone(), Point::new(20.0, 98.0)))
        });
        assert!(events.borrow().is_empty());
    });
}

#[test]
fn deferred_child_tick_waits_for_its_semantic_drain_and_successful_completion() {
    on_large_stack(|| {
        let (mut parent, events, receiver, deadline) = fixture();
        let child = &mut parent.auxiliary_windows[0].runner;
        // Advancing a timer without draining its deferred Deadline cannot
        // authorize the parent, but it must preserve the due receipt.
        child.finish_timed_drag_visuals(false);
        assert!(!child.take_foreign_autoscroll_completed_since_event());
        assert!(child.core.runtime.has_pending_cross_window_autoscroll());
        child.collect_timed_drag_cancellations(&mut GenericRouteOutcome::default());
        child.finish_timed_drag_visuals(true);
        assert!(child.take_foreign_autoscroll_completed_since_event());
        assert!(!child.take_foreign_autoscroll_completed_since_event());
        assert!(events.borrow().is_empty());
        parent.route_drag_autoscroll_with_resolver(&receiver, deadline, &mut |_, _| {
            Some((receiver.clone(), Point::new(20.0, 98.0)))
        });
        assert_eq!(events.borrow().first(), Some(&Message::Scrolled));
    });
}

#[test]
fn foreign_scroll_reducer_removing_auxiliary_source_stops_target_reentry_and_rearm() {
    on_large_stack(|| {
        let (mut parent, events, receiver, deadline) = fixture_with_auxiliary_source(true);
        parent.route_drag_autoscroll_with_resolver(&receiver, deadline, &mut |_, _| {
            Some((receiver.clone(), Point::new(20.0, 98.0)))
        });
        let observed = events.borrow();
        assert_eq!(observed.first(), Some(&Message::Scrolled));
        assert_eq!(
            observed
                .iter()
                .filter(|event| matches!(event, Message::Source(DragSourcePhase::Cancelled(_))))
                .count(),
            1
        );
        assert!(!observed.contains(&Message::Source(DragSourcePhase::Moved)));
        assert!(!observed.contains(&Message::Target("updated", DropPhase::Entered)));
        assert!(!observed.contains(&Message::Target("updated", DropPhase::Over)));
        assert!(parent.cross_window_transfers.is_empty());
        assert!(parent.drag_endpoint(WindowId::from(203_u64)).is_none());
        assert!(
            !parent.auxiliary_windows[0]
                .runner
                .core
                .runtime
                .has_pending_cross_window_autoscroll()
        );
        drop(observed);
        let count = events.borrow().len();
        parent.route_drag_autoscroll_with_resolver(&receiver, deadline, &mut |_, _| {
            Some((receiver.clone(), Point::new(20.0, 98.0)))
        });
        assert_eq!(
            events.borrow().len(),
            count,
            "a retired transfer cannot replay its tick"
        );
    });
}

use super::*;
use crate::application::runtime::task::resource_operations::{
    ResourceOperationReplaceMode, ResourceOperationReserve,
};
use crate::application::{Resource, ResourceRefreshPolicy, SharedResourceCompletion, text};
use std::sync::{Arc, atomic::AtomicBool};

#[test]
fn selected_ready_factory_is_immediate_and_preserves_ordinary_widget_identity() {
    let tasks = SharedResourceTasks::new();
    let _interest = tasks
        .admit_interest(
            1,
            0,
            1,
            "view".into(),
            ResourceInterestKind::Visible,
            Arc::new(AtomicBool::new(true)),
        )
        .expect("interest");
    let ResourceOperationReserve::Reserved(reservation) = tasks
        .operations
        .reserve("view".into(), ResourceOperationReplaceMode::Join)
        .expect("reservation")
    else {
        panic!("new operation");
    };
    let current = reservation.current();
    reservation.transaction().accept();
    let mut state = Resource::<u8, &'static str>::new("view");
    state
        .begin(
            tasks.operation(&"view".into()).expect("current"),
            ResourceRefreshPolicy::RetainReady,
        )
        .expect("begin");
    assert!(state.apply_completion(
        &tasks,
        SharedResourceCompletion {
            output: Ok(42),
            current
        }
    ));
    let mut calls = 0;
    let view = resource(state.snapshot())
        .ready(|value| {
            calls += 1;
            text::<()>(format!("{value}")).id(42)
        })
        .failed(|_| panic!("unselected failure factory"))
        .refreshing(|_| panic!("unselected refresh factory"));
    assert_eq!(calls, 1);
    let surface = view.into_surface();
    assert!(surface.find_widget(42).is_some());
    assert_eq!(calls, 1);
}

#[test]
fn constructing_and_lowering_demand_does_not_admit_or_start_work() {
    let tasks = SharedResourceTasks::new();
    let state = Resource::<u8, &'static str>::new("view");
    let surface = resource(state.snapshot())
        .idle(text::<()>("idle"))
        .ready(|_| panic!("unselected ready factory"))
        .interest(&tasks, 1, ResourceInterestKind::Visible)
        .into_surface();
    assert!(surface.root().has_resource_view_demand());
    assert_eq!(tasks.interest_count(), 0);
    assert!(tasks.operation(&"view".into()).is_none());
}

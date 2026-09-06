use radiant::{
    application::{button, column},
    layout::Vector2,
    runtime::{Command, FocusTransferOutcome, SurfaceRuntime},
};

#[test]
fn focus_targets_are_runtime_and_projection_qualified_and_observation_is_inert() {
    fn bridge() -> impl radiant::runtime::RuntimeBridge<bool> {
        radiant::app(false)
            .view(|show: &bool| {
                let mut children = vec![button("First").message(false).id(1)];
                if *show {
                    children.push(button("Second").message(true).id(2));
                }
                column(children).id(100)
            })
            .update(|show, next| *show = next)
            .into_bridge()
    }
    let mut runtime = SurfaceRuntime::new(bridge(), Vector2::new(320.0, 180.0));
    let mut other = SurfaceRuntime::new(bridge(), Vector2::new(320.0, 180.0));
    let target = runtime.focus_target(1).unwrap();
    assert_eq!(target.widget_id(), 1);
    assert_eq!(runtime.focused_widget(), None);
    assert!(runtime.focus_target(2).is_none());
    assert_eq!(other.transfer_focus(&target), FocusTransferOutcome::Stale);
    assert_eq!(other.focused_widget(), None);
    assert_eq!(
        runtime.transfer_focus(&target),
        FocusTransferOutcome::Admitted(1)
    );
    runtime.dispatch_message(true);
    assert_eq!(runtime.transfer_focus(&target), FocusTransferOutcome::Stale);
    let second = runtime.focus_target(2).unwrap();
    assert_eq!(
        runtime.transfer_focus(&second),
        FocusTransferOutcome::Admitted(2)
    );
    runtime.dispatch_message(false);
    assert!(runtime.focus_target(2).is_none());
    assert_eq!(runtime.transfer_focus(&second), FocusTransferOutcome::Stale);
    runtime.dispatch_message(true);
    assert_eq!(runtime.transfer_focus(&second), FocusTransferOutcome::Stale);
    runtime.execute_command(Command::exit());
    assert!(runtime.focus_target(1).is_none());
    assert_eq!(
        runtime.transfer_focus(&target),
        FocusTransferOutcome::Unavailable
    );
}

#[test]
fn explicit_sequential_and_spatial_navigation_use_current_order_and_geometry() {
    use radiant::runtime::{FocusDirection, FocusTraversal};
    let mut runtime = SurfaceRuntime::new(
        radiant::app(())
            .view(|_: &()| {
                column([
                    button("One").message(()).id(1),
                    button("Two").message(()).id(2),
                    button("Three").message(()).id(3),
                ])
                .id(100)
            })
            .update(|_, _| {})
            .into_bridge(),
        Vector2::new(320.0, 180.0),
    );
    assert_eq!(
        runtime.traverse_focus_spatial(FocusDirection::Down),
        FocusTransferOutcome::NoDestination
    );
    assert_eq!(
        runtime.traverse_focus_explicit(FocusTraversal::Forward),
        FocusTransferOutcome::Admitted(1)
    );
    assert_eq!(
        runtime.traverse_focus_spatial(FocusDirection::Down),
        FocusTransferOutcome::Admitted(2)
    );
    assert_eq!(
        runtime.traverse_focus_spatial(FocusDirection::Down),
        FocusTransferOutcome::Admitted(3)
    );
    assert_eq!(
        runtime.traverse_focus_spatial(FocusDirection::Down),
        FocusTransferOutcome::NoDestination
    );
    assert_eq!(
        runtime.traverse_focus_spatial(FocusDirection::Up),
        FocusTransferOutcome::Admitted(2)
    );
    assert_eq!(
        runtime.traverse_focus_spatial(FocusDirection::Left),
        FocusTransferOutcome::NoDestination
    );
    assert_eq!(
        runtime.traverse_focus_explicit(FocusTraversal::Backward),
        FocusTransferOutcome::Admitted(1)
    );
    runtime.execute_command(Command::exit());
    assert_eq!(
        runtime.traverse_focus_explicit(FocusTraversal::Forward),
        FocusTransferOutcome::Unavailable
    );
    assert_eq!(
        runtime.traverse_focus_spatial(FocusDirection::Down),
        FocusTransferOutcome::Unavailable
    );
}

#[test]
fn focus_bookmarks_survive_compatible_refresh_but_never_removal_and_reappearance() {
    use radiant::runtime::FocusBookmarkError;
    fn bridge() -> impl radiant::runtime::RuntimeBridge<bool> {
        radiant::app(true)
            .view(|show: &bool| {
                let mut children = vec![button("First").message(false).id(1)];
                if *show {
                    children.push(button("Second").message(true).id(2));
                }
                column(children).id(100)
            })
            .update(|show, next| *show = next)
            .into_bridge()
    }
    let mut runtime = SurfaceRuntime::new(bridge(), Vector2::new(320.0, 180.0));
    let mut other = SurfaceRuntime::new(bridge(), Vector2::new(320.0, 180.0));
    assert!(matches!(
        runtime.capture_focus(),
        Err(FocusBookmarkError::NoFocus)
    ));
    assert!(runtime.focus_widget(1));
    let first = runtime.capture_focus().unwrap();
    assert!(runtime.focus_widget(2));
    let second = runtime.capture_focus().unwrap();
    runtime.dispatch_message(false);
    assert_eq!(
        runtime.restore_focus(&first),
        FocusTransferOutcome::Admitted(1)
    );
    assert_eq!(other.restore_focus(&first), FocusTransferOutcome::Stale);
    assert_eq!(runtime.restore_focus(&second), FocusTransferOutcome::Stale);
    runtime.dispatch_message(true);
    assert_eq!(runtime.restore_focus(&second), FocusTransferOutcome::Stale);
    assert!(runtime.focus_widget(2));
    let replacement = runtime.capture_focus().unwrap();
    assert!(runtime.focus_widget(1));
    assert_eq!(
        runtime.restore_focus(&replacement),
        FocusTransferOutcome::Admitted(2)
    );
    runtime.execute_command(Command::exit());
    assert!(matches!(
        runtime.capture_focus(),
        Err(FocusBookmarkError::Unavailable)
    ));
    assert_eq!(
        runtime.restore_focus(&first),
        FocusTransferOutcome::Unavailable
    );
}

#[test]
fn bookmark_capacity_is_bounded_and_dropped_bookmarks_release_slots() {
    use radiant::runtime::FocusBookmarkError;
    let mut runtime = SurfaceRuntime::new(
        radiant::app(())
            .view(|_: &()| column((1..=65).map(|id| button("Target").message(()).id(id))).id(100))
            .update(|_, _| {})
            .into_bridge(),
        Vector2::new(320.0, 4000.0),
    );
    let mut bookmarks = Vec::new();
    for id in 1..=64 {
        assert!(runtime.focus_widget(id));
        bookmarks.push(runtime.capture_focus().unwrap());
    }
    assert!(runtime.focus_widget(65));
    assert!(matches!(
        runtime.capture_focus(),
        Err(FocusBookmarkError::Capacity)
    ));
    bookmarks.pop();
    let last = runtime.capture_focus().unwrap();
    assert_eq!(
        runtime.restore_focus(&last),
        FocusTransferOutcome::Admitted(65)
    );
}

#[test]
fn focus_bookmark_rejects_a_replaced_widget_even_after_its_original_kind_returns() {
    let mut runtime = SurfaceRuntime::new(
        radiant::app(true)
            .view(|button_visible: &bool| {
                if *button_visible {
                    button("Target").message(false).id(1)
                } else {
                    radiant::application::text("Replacement").id(1)
                }
            })
            .update(|visible, next| *visible = next)
            .into_bridge(),
        Vector2::new(320.0, 180.0),
    );
    assert!(runtime.focus_widget(1));
    let bookmark = runtime.capture_focus().unwrap();
    runtime.dispatch_message(false);
    assert_eq!(
        runtime.restore_focus(&bookmark),
        FocusTransferOutcome::Stale
    );
    runtime.dispatch_message(true);
    assert_eq!(
        runtime.restore_focus(&bookmark),
        FocusTransferOutcome::Stale
    );
}

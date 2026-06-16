use super::*;
use radiant::runtime::{DevtoolsNodeKind, DevtoolsNodeSnapshot, DevtoolsOverlayOptions};

#[test]
fn surface_runtime_devtools_snapshot_reports_tree_bounds_and_paint_summary() {
    let bridge = declarative_runtime_bridge(
        DemoState::default(),
        project_surface,
        |_state: &mut DemoState, _message| {},
    );
    let runtime = SurfaceRuntime::new(bridge, Vector2::new(420.0, 80.0));

    let snapshot = runtime.devtools_snapshot();
    let button = devtools_node(&snapshot.root, 11).expect("button node");
    let input = devtools_node(&snapshot.root, 12).expect("text input node");

    assert_eq!(snapshot.root.node_id, 1);
    assert_eq!(snapshot.root.kind, DevtoolsNodeKind::Container);
    assert_eq!(snapshot.viewport.width(), 420.0);
    assert!(button.bounds.is_some());
    assert!(input.bounds.is_some());
    assert!(
        button
            .widget
            .as_ref()
            .is_some_and(|widget| widget.focusable),
        "interactive button should surface focusability"
    );
    assert!(
        snapshot.paint.total > 0,
        "devtools snapshot should include current frame paint statistics"
    );
    assert_eq!(snapshot.diagnostics.ui.update_handlers, 0);
}

#[test]
fn surface_runtime_devtools_snapshot_reports_live_interaction_state() {
    let bridge = declarative_runtime_bridge(
        DemoState::default(),
        project_surface,
        |_state: &mut DemoState, _message| {},
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(420.0, 80.0));

    runtime.dispatch_event(Event::PointerMove {
        position: Point::new(164.0, 12.0),
    });

    let snapshot = runtime.devtools_snapshot();
    let button = devtools_node(&snapshot.root, 11).expect("button node");

    assert_eq!(snapshot.selected_node_id, Some(11));
    assert!(
        button
            .widget
            .as_ref()
            .is_some_and(|widget| widget.state.hovered),
        "devtools snapshot should reflect runtime-local hover state"
    );
}

#[test]
fn surface_runtime_devtools_overlay_appends_runtime_overlay_primitives() {
    let bridge = declarative_runtime_bridge(
        DemoState::default(),
        project_surface,
        |_state: &mut DemoState, _message| {},
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(420.0, 80.0));

    assert!(!runtime.has_runtime_overlay_paint());
    runtime.set_devtools_overlay_options(DevtoolsOverlayOptions::enabled());

    let mut primitives = Vec::new();
    runtime.runtime_overlay_paint_into(&ThemeTokens::default(), &mut primitives);

    assert!(runtime.devtools_overlay_options().is_enabled());
    assert!(runtime.has_runtime_overlay_paint());
    assert!(
        primitives.iter().any(|primitive| {
            primitive
                .widget_id()
                .is_some_and(|widget_id| widget_id == u64::MAX - 2048)
        }),
        "devtools overlay should append backend-neutral paint for the overlay panel"
    );
}

fn devtools_node(root: &DevtoolsNodeSnapshot, node_id: u64) -> Option<&DevtoolsNodeSnapshot> {
    if root.node_id == node_id {
        return Some(root);
    }
    root.children
        .iter()
        .find_map(|child| devtools_node(child, node_id))
}

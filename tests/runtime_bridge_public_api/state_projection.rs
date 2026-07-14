use super::*;

#[test]
fn generic_runtime_bridge_projects_and_reduces_host_defined_messages() {
    let mut bridge = declarative_runtime_bridge(
        DemoState::default(),
        project_surface,
        |state: &mut DemoState, message| match message {
            DemoMessage::Increment => state.count += 1,
            DemoMessage::Rename(name) => state.name = name,
        },
    );

    let surface_before = bridge.project_surface();
    let rename = surface_before
        .dispatch_widget_output(
            12,
            radiant::widgets::WidgetOutput::typed(TextInputMessage::Changed {
                value: String::from("Projects"),
            }),
        )
        .expect("text input should emit a host-defined rename message");
    bridge.reduce_message(rename);

    let increment = bridge
        .project_surface()
        .dispatch_widget_output(
            11,
            radiant::widgets::WidgetOutput::typed(ButtonMessage::Activate),
        )
        .expect("button should emit a host-defined increment message");
    bridge.reduce_message(increment);

    let surface_after = bridge.project_surface();
    assert_eq!(
        widget_ref::<TextWidget, _>(&surface_after, 10, "text").text,
        "Projects (1)"
    );
    assert_eq!(
        widget_ref::<TextInputWidget, _>(&surface_after, 12, "text input")
            .state
            .value,
        "Projects"
    );
}

#[test]
fn owned_runtime_bridge_projects_without_shared_surface_requirement() {
    let mut bridge = declarative_owned_runtime_bridge(
        DemoState::default(),
        project_owned_surface,
        |state: &mut DemoState, message| match message {
            DemoMessage::Increment => state.count += 1,
            DemoMessage::Rename(name) => state.name = name,
        },
    );

    let before = bridge.pull_surface();
    assert_eq!(
        widget_ref::<TextWidget, _>(&before, 10, "text").text,
        "Untitled (0)"
    );

    bridge.reduce_message(DemoMessage::Rename(String::from("Owned")));
    bridge.reduce_message(DemoMessage::Increment);
    let after = bridge.pull_surface();
    assert_eq!(
        widget_ref::<TextWidget, _>(&after, 10, "text").text,
        "Owned (1)"
    );
}

#[test]
fn generic_trait_reduction_updates_shared_and_owned_declarative_bridges() {
    let mut shared = declarative_runtime_bridge(
        DemoState::default(),
        project_surface,
        |state: &mut DemoState, message| match message {
            DemoMessage::Increment => state.count += 1,
            DemoMessage::Rename(name) => state.name = name,
        },
    );
    reduce_through_runtime_bridge(&mut shared, DemoMessage::Increment);
    assert_eq!(shared.state().count, 1);

    let mut owned = declarative_owned_runtime_bridge(
        DemoState::default(),
        project_owned_surface,
        |state: &mut DemoState, message| match message {
            DemoMessage::Increment => state.count += 1,
            DemoMessage::Rename(name) => state.name = name,
        },
    );
    reduce_through_runtime_bridge(&mut owned, DemoMessage::Rename(String::from("Generic")));
    assert_eq!(owned.state().name, "Generic");
}

#[test]
fn runtime_bridge_is_the_public_app_contract() {
    let mut bridge = declarative_runtime_bridge(
        DemoState::default(),
        project_surface,
        |state: &mut DemoState, message| match message {
            DemoMessage::Increment => state.count += 1,
            DemoMessage::Rename(name) => state.name = name,
        },
    );

    let surface = project_app_once(&mut bridge);
    assert!(surface.find_widget(10).is_some());
}

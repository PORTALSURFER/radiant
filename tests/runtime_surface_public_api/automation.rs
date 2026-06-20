use super::*;
use radiant::gui::automation::{
    AUTOMATION_ACTION_FOCUS, AUTOMATION_ACTION_PRESS, AUTOMATION_ACTION_SELECT,
    AUTOMATION_ACTION_SET_TEXT, AUTOMATION_ACTION_TOGGLE, AutomationRole,
};
use radiant::widgets::{ListItemWidget, SelectableWidget};

#[test]
fn surface_runtime_automation_snapshot_reports_common_widget_semantics() {
    let bridge = declarative_runtime_bridge(
        (),
        |_state: &mut ()| {
            Arc::new(
                ui::column([
                    ui::button("Save").message(DemoMessage::Increment).id(10),
                    ui::toggle("Loop", true)
                        .message(|_| DemoMessage::Increment)
                        .id(11),
                    ui::text_input("kick.wav")
                        .placeholder("Sample name")
                        .message(DemoMessage::Rename)
                        .id(12),
                ])
                .into_surface(),
            )
        },
        |_state: &mut (), _message| {},
    );
    let runtime = SurfaceRuntime::new(bridge, Vector2::new(320.0, 120.0));

    let snapshot = runtime.automation_snapshot();
    let button = automation_node(&snapshot.root, "10").expect("button automation node");
    let toggle = automation_node(&snapshot.root, "11").expect("toggle automation node");
    let input = automation_node(&snapshot.root, "12").expect("text input automation node");

    assert_eq!(snapshot.schema_version, 2);
    assert_eq!(snapshot.viewport_width, 320);
    assert_eq!(button.semantics.role, AutomationRole::Button);
    assert_eq!(button.semantics.label.as_deref(), Some("Save"));
    assert!(button.semantics.focusable);
    assert_eq!(
        button.available_actions,
        [AUTOMATION_ACTION_FOCUS, AUTOMATION_ACTION_PRESS]
    );
    assert_eq!(toggle.semantics.role, AutomationRole::Toggle);
    assert_eq!(toggle.semantics.label.as_deref(), Some("Loop"));
    assert_eq!(toggle.semantics.checked, Some(true));
    assert_eq!(
        toggle.available_actions,
        [AUTOMATION_ACTION_FOCUS, AUTOMATION_ACTION_TOGGLE]
    );
    assert_eq!(input.semantics.role, AutomationRole::TextInput);
    assert_eq!(input.semantics.label.as_deref(), Some("Sample name"));
    assert_eq!(input.semantics.value_text.as_deref(), Some("kick.wav"));
    assert_eq!(
        input.available_actions,
        [AUTOMATION_ACTION_FOCUS, AUTOMATION_ACTION_SET_TEXT]
    );
    assert!(input.enabled);
}

#[test]
fn automation_target_snapshot_flattens_semantic_targets_with_coordinates() {
    let bridge = declarative_runtime_bridge(
        (),
        |_state: &mut ()| {
            Arc::new(
                ui::column([
                    ui::button("Save").message(DemoMessage::Increment).id(10),
                    ui::toggle("Loop", false)
                        .message(|_| DemoMessage::Increment)
                        .id(11),
                ])
                .into_surface(),
            )
        },
        |_state: &mut (), _message| {},
    );
    let runtime = SurfaceRuntime::new(bridge, Vector2::new(320.0, 96.0));

    let target_snapshot = runtime.automation_target_snapshot();
    let save = automation_target(&target_snapshot.targets, "10").expect("save target");
    let loop_toggle = automation_target(&target_snapshot.targets, "11").expect("loop target");

    assert_eq!(target_snapshot.schema_version, 1);
    assert_eq!(target_snapshot.viewport_width, 320);
    assert_eq!(save.tree_index + 1, loop_toggle.tree_index);
    assert_eq!(save.depth, save.path.len() - 1);
    assert_eq!(save.path.first().map(|id| id.0.as_str()), Some("1"));
    assert_eq!(save.path.last().map(|id| id.0.as_str()), Some("10"));
    assert_eq!(save.role, AutomationRole::Button);
    assert_eq!(save.display_text(), Some("Save"));
    assert!(save.interaction_target);
    assert!(save.center.x > save.bounds.x);
    assert!(save.center.y > save.bounds.y);
    assert_eq!(
        save.available_actions,
        [AUTOMATION_ACTION_FOCUS, AUTOMATION_ACTION_PRESS]
    );
    assert_eq!(loop_toggle.checked, Some(false));
    assert_eq!(
        loop_toggle.available_actions,
        [AUTOMATION_ACTION_FOCUS, AUTOMATION_ACTION_TOGGLE]
    );
}

#[test]
fn direct_widget_automation_semantics_cover_rows_selectables_and_custom_fallback() {
    let list_item = ListItemWidget::new(
        20,
        "Kick 01",
        WidgetSizing::fixed(Vector2::new(120.0, 24.0)),
    );
    let selectable = SelectableWidget::new(
        21,
        "Candidate",
        true,
        WidgetSizing::fixed(Vector2::new(120.0, 24.0)),
    );
    let custom = ScenePointerWidget::new(22);

    assert_eq!(list_item.automation_semantics().role, AutomationRole::Row);
    assert_eq!(
        list_item.automation_semantics().label.as_deref(),
        Some("Kick 01")
    );
    assert_eq!(
        selectable.automation_semantics().role,
        AutomationRole::Selectable
    );
    assert_eq!(
        selectable.automation_semantics().label.as_deref(),
        Some("Candidate")
    );
    assert!(selectable.automation_semantics().selected);
    assert_eq!(custom.automation_semantics().role, AutomationRole::Custom);

    let row_actions = list_item.automation_semantics().default_available_actions();
    assert_eq!(
        row_actions,
        [AUTOMATION_ACTION_FOCUS, AUTOMATION_ACTION_SELECT]
    );
}

#[test]
fn devtools_snapshot_exposes_widget_automation_semantics() {
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
    let semantics = &button.widget.as_ref().expect("button widget").semantics;

    assert_eq!(semantics.role, AutomationRole::Button);
    assert_eq!(semantics.label.as_deref(), Some("Increment"));

    let projection = snapshot.inspector_projection();
    assert!(
        projection
            .selected_details
            .iter()
            .any(|line| line.contains("role:"))
    );
    assert!(
        projection
            .tree_rows
            .iter()
            .any(|row| row.label.contains("role=Button"))
    );
}

fn automation_node<'a>(
    root: &'a radiant::gui::automation::AutomationNodeSnapshot,
    id: &str,
) -> Option<&'a radiant::gui::automation::AutomationNodeSnapshot> {
    if root.id.0 == id {
        return Some(root);
    }
    root.children
        .iter()
        .find_map(|child| automation_node(child, id))
}

fn automation_target<'a>(
    targets: &'a [radiant::gui::automation::AutomationTarget],
    id: &str,
) -> Option<&'a radiant::gui::automation::AutomationTarget> {
    targets.iter().find(|target| target.id.0 == id)
}

fn devtools_node(
    root: &radiant::runtime::DevtoolsNodeSnapshot,
    node_id: u64,
) -> Option<&radiant::runtime::DevtoolsNodeSnapshot> {
    if root.node_id == node_id {
        return Some(root);
    }
    root.children
        .iter()
        .find_map(|child| devtools_node(child, node_id))
}

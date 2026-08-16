use super::*;
use radiant::gui::automation::{
    AUTOMATION_ACTION_FOCUS, AUTOMATION_ACTION_PRESS, AUTOMATION_ACTION_SELECT,
    AUTOMATION_ACTION_SET_TEXT, AUTOMATION_ACTION_TOGGLE, AutomationRole,
    AutomationTargetAuthority,
};
use radiant::widgets::{ListItemWidget, SelectableWidget, WIDGET_CAPABILITIES_CONTRACT_VERSION};

#[test]
fn surface_runtime_automation_snapshot_reports_common_widget_semantics() {
    let bridge = declarative_runtime_bridge(
        (),
        |_state: &mut ()| {
            crate::arc_surface(
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

    assert_eq!(snapshot.schema_version, 3);
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
            crate::arc_surface(
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

    let root_id = runtime.automation_snapshot().root.id;
    let target_snapshot = runtime.automation_target_snapshot();
    let save = automation_target(&target_snapshot.targets, "10").expect("save target");
    let loop_toggle = automation_target(&target_snapshot.targets, "11").expect("loop target");

    assert_eq!(target_snapshot.schema_version, 3);
    assert_eq!(target_snapshot.viewport_width, 320);
    let expected_authority = Some(AutomationTargetAuthority::materialized(
        runtime.refresh_counters().runtime_projection,
    ));
    for target in &target_snapshot.targets {
        assert_eq!(target.authority, expected_authority);
    }
    assert_eq!(save.tree_index + 1, loop_toggle.tree_index);
    assert_eq!(save.depth, save.path.len() - 1);
    assert_eq!(save.path.first(), Some(&root_id));
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
fn runtime_owned_split_publishes_a_stable_noninteractive_separator_target() {
    let bridge = declarative_owned_runtime_bridge(
        (),
        |_state: &mut ()| {
            ui::split_pane::<()>(ui::text("First pane"), ui::text("Second pane"))
                .initial_ratio(0.25)
                .divider_extent(8.0)
                .runtime_owned_ratio()
                .into_surface()
        },
        |_state: &mut (), _message| {},
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(200.0, 80.0));

    let initial = runtime.automation_snapshot();
    assert_eq!(initial.schema_version, 3);
    let (separator, container) = one_separator_with_parent(&initial.root);
    assert_eq!(container.children.len(), 3);
    assert_eq!(container.children[1].id, separator.id);
    assert_eq!(
        separator.id.0,
        format!("radiant:layout-target:{}:53504c49545f4449", container.id.0)
    );
    assert_eq!(separator.semantics.role, AutomationRole::Separator);
    assert_eq!(
        (
            separator.bounds.x,
            separator.bounds.y,
            separator.bounds.width,
            separator.bounds.height
        ),
        (48.0, 0.0, 8.0, 80.0)
    );
    assert_eq!(separator.value.as_deref(), Some("0.25"));
    assert_eq!(
        separator
            .semantics
            .metadata
            .get("orientation")
            .map(String::as_str),
        Some("horizontal")
    );
    assert!(separator.enabled);
    assert!(!separator.semantics.selected);
    assert_eq!(separator.semantics.checked, Some(false));
    assert!(!separator.semantics.read_only);
    assert!(!separator.semantics.focusable);
    assert!(!separator.semantics.focused);
    assert!(separator.available_actions.is_empty());

    let initial_id = separator.id.clone();
    let initial_target_snapshot = runtime.automation_target_snapshot();
    assert_eq!(initial_target_snapshot.schema_version, 3);
    let initial_target = automation_target(&initial_target_snapshot.targets, &initial_id.0)
        .expect("runtime-owned separator target");
    assert_eq!(initial_target.role, AutomationRole::Separator);
    assert_eq!(initial_target.checked, Some(false));
    assert!(!initial_target.focusable);
    assert!(!initial_target.focused);
    assert!(!initial_target.interaction_target);
    assert!(initial_target.available_actions.is_empty());
    assert_eq!(
        initial_target
            .authority
            .map(|authority| authority.materialized),
        Some(true)
    );

    runtime.dispatch_event(Event::primary_press(Point::new(52.0, 40.0)));
    runtime.dispatch_event(Event::pointer_move(Point::new(48.0, 100.0)));
    runtime.dispatch_event(Event::primary_release(Point::new(48.0, 100.0)));
    let no_op = runtime.automation_snapshot();
    let (no_op_separator, _) = one_separator_with_parent(&no_op.root);
    assert_eq!(no_op_separator.id, initial_id);
    assert_eq!(no_op_separator.value.as_deref(), Some("0.25"));
    assert_eq!(no_op_separator.bounds, separator.bounds);

    runtime.dispatch_event(Event::primary_press(Point::new(52.0, 40.0)));
    runtime.dispatch_event(Event::pointer_move(Point::new(130.0, 100.0)));
    let moved = runtime.automation_snapshot();
    let (moved_separator, _) = one_separator_with_parent(&moved.root);
    let moved_ratio = (130.0_f32 / 192.0_f32).to_string();
    assert_eq!(moved_separator.id, initial_id);
    assert_eq!(moved_separator.value.as_deref(), Some(moved_ratio.as_str()));
    assert_eq!(moved_separator.bounds.x, 130.0);
    assert_eq!(moved_separator.bounds.width, 8.0);
    assert_eq!(
        moved_separator
            .semantics
            .metadata
            .get("orientation")
            .map(String::as_str),
        Some("horizontal")
    );

    runtime.dispatch_event(Event::primary_release(Point::new(130.0, 100.0)));
    let committed = runtime.automation_snapshot();
    let (committed_separator, _) = one_separator_with_parent(&committed.root);
    assert_eq!(committed_separator.id, initial_id);
    assert_eq!(
        committed_separator.value.as_deref(),
        Some(moved_ratio.as_str())
    );
    assert_eq!(committed_separator.bounds.x, 130.0);

    let before_refresh = committed.clone();
    runtime.refresh_with_scope(RepaintScope::Surface);
    assert_eq!(runtime.automation_snapshot(), before_refresh);
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
    let mut custom = ScenePointerWidget::new(22);
    custom.common.focus = radiant::widgets::FocusBehavior::Keyboard;
    custom.common.state.selected = true;
    custom.common.state.disabled = true;

    assert_eq!(list_item.automation_semantics().role, AutomationRole::Row);
    assert_eq!(
        list_item.capabilities().contract_version,
        WIDGET_CAPABILITIES_CONTRACT_VERSION
    );
    assert!(list_item.capabilities().has_semantics());
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
    assert!(selectable.capabilities().has_semantics());
    assert!(!custom.capabilities().has_semantics());
    let custom_semantics = custom.automation_semantics();
    assert_eq!(custom_semantics.role, AutomationRole::Custom);
    assert!(custom_semantics.selected);
    assert!(custom_semantics.disabled);
    assert!(!custom_semantics.focusable);
    assert_eq!(custom_semantics.tab_index, None);

    let row_actions = list_item.automation_semantics().default_available_actions();
    assert_eq!(
        row_actions,
        [AUTOMATION_ACTION_FOCUS, AUTOMATION_ACTION_SELECT]
    );
}

#[test]
fn custom_widget_semantics_capability_is_discovered_without_runtime_trait_inference() {
    let custom = SemanticWidget::new(23);
    let capabilities = custom.capabilities();

    assert_eq!(
        capabilities.contract_version,
        WIDGET_CAPABILITIES_CONTRACT_VERSION
    );
    assert!(capabilities.has_semantics());
    let semantics = custom.automation_semantics();
    assert_eq!(semantics.role, AutomationRole::Readout);
    assert_eq!(semantics.label.as_deref(), Some("Custom readout"));
    assert!(semantics.focusable);
}

#[test]
fn devtools_snapshot_exposes_widget_automation_semantics() {
    let bridge = declarative_runtime_bridge(
        DemoState::default(),
        project_surface,
        |_state: &mut DemoState, _message| {},
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(420.0, 80.0));
    runtime.dispatch_event(Event::pointer_move(Point::new(164.0, 12.0)));

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

fn one_separator_with_parent(
    root: &radiant::gui::automation::AutomationNodeSnapshot,
) -> (
    &radiant::gui::automation::AutomationNodeSnapshot,
    &radiant::gui::automation::AutomationNodeSnapshot,
) {
    fn collect<'a>(
        node: &'a radiant::gui::automation::AutomationNodeSnapshot,
        matches: &mut Vec<(
            &'a radiant::gui::automation::AutomationNodeSnapshot,
            &'a radiant::gui::automation::AutomationNodeSnapshot,
        )>,
    ) {
        for child in &node.children {
            if child.semantics.role == AutomationRole::Separator {
                matches.push((child, node));
            }
            collect(child, matches);
        }
    }

    let mut matches = Vec::new();
    collect(root, &mut matches);
    assert_eq!(matches.len(), 1, "expected exactly one semantic separator");
    matches.pop().expect("separator match")
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

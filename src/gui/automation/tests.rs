use super::test_fixtures::{fixture_node, fixture_snapshot};
use super::{AutomationNodeId, AutomationRole};

#[test]
fn automation_targets_preserve_preorder_depth_and_paths() {
    let targets = fixture_snapshot().root.automation_targets();

    assert_eq!(
        targets
            .iter()
            .map(|target| target.id.0.as_str())
            .collect::<Vec<_>>(),
        vec![
            "root",
            "toolbar.press",
            "panel.nested",
            "panel.nested.toggle"
        ]
    );
    assert_eq!(
        targets
            .iter()
            .map(|target| target.tree_index)
            .collect::<Vec<_>>(),
        vec![0, 1, 2, 3]
    );
    assert_eq!(
        targets
            .iter()
            .map(|target| target.depth)
            .collect::<Vec<_>>(),
        vec![0, 1, 1, 2]
    );
    assert_eq!(
        targets[3].path,
        vec![
            AutomationNodeId::new("root"),
            AutomationNodeId::new("panel.nested"),
            AutomationNodeId::new("panel.nested.toggle"),
        ]
    );
}

#[test]
fn target_snapshot_preserves_viewport_and_interaction_state() {
    let targets = fixture_snapshot().target_snapshot();

    assert_eq!(targets.schema_version, 1);
    assert_eq!(
        (targets.viewport_width, targets.viewport_height),
        (1280, 720)
    );
    assert!(!targets.targets[0].interaction_target);
    assert!(targets.targets[1].interaction_target);
    assert_eq!(targets.targets[1].center.x, 30.0);
    assert_eq!(targets.targets[1].center.y, 30.0);
}

#[test]
fn display_text_prefers_label_then_value_then_description() {
    let mut target = fixture_node("readout", AutomationRole::Readout, "Label")
        .automation_targets()
        .remove(0);
    target.value = Some("Value".to_owned());
    target.description = Some("Description".to_owned());

    assert_eq!(target.display_text(), Some("Label"));
    target.label = None;
    assert_eq!(target.display_text(), Some("Value"));
    target.value = None;
    assert_eq!(target.display_text(), Some("Description"));
}

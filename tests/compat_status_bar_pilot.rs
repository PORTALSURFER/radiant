//! Public pilot coverage for the migrated compatibility-shell status bar slice.

use radiant::compat::sempal_shell::{self, AppModel, ProgressOverlayModel};

#[test]
fn compat_snapshot_exposes_status_bar_readout_with_projected_copy() {
    let mut model = AppModel::default();
    model.status.left = String::from("Transport: running");
    model.status.center = String::from("rows: 12 | selected: 3 | anchor: 4 | search: clap");
    model.status.right = String::from("col: 2/3");

    let snapshot = sempal_shell::capture_gui_automation_snapshot([1280.0, 720.0], &model);
    let status_bar = find_node(&snapshot.root, "shell.status_bar").expect("status bar node");

    assert_eq!(status_bar.label.as_deref(), Some("Status bar"));
    assert_eq!(
        status_bar.value.as_deref(),
        Some(model.status.center.as_str())
    );
    assert_eq!(
        status_bar.metadata.get("left").map(String::as_str),
        Some(model.status.left.as_str())
    );
    assert_eq!(
        status_bar.metadata.get("right").map(String::as_str),
        Some(model.status.right.as_str())
    );
    assert!(status_bar.bounds.width > 0.0);
    assert!(status_bar.bounds.height > 0.0);
}

#[test]
fn compat_snapshot_keeps_status_bar_visible_during_inline_progress() {
    let mut model = AppModel::default();
    model.status.center = String::from("rows: 12 | selected: 3");
    model.progress_overlay = ProgressOverlayModel {
        visible: true,
        modal: false,
        title: String::from("Scanning"),
        detail: Some(String::from("source_a")),
        completed: 4,
        total: 9,
        cancelable: false,
        cancel_requested: false,
    };

    let snapshot = sempal_shell::capture_gui_automation_snapshot([960.0, 540.0], &model);
    let status_bar = find_node(&snapshot.root, "shell.status_bar").expect("status bar node");

    assert!(status_bar.bounds.width > 0.0);
    assert!(status_bar.bounds.height > 0.0);
    assert_eq!(
        status_bar.metadata.get("center").map(String::as_str),
        Some(model.status.center.as_str())
    );
}

fn find_node<'a>(
    node: &'a sempal_shell::AutomationNodeSnapshot,
    id: &str,
) -> Option<&'a sempal_shell::AutomationNodeSnapshot> {
    if node.id.0 == id {
        return Some(node);
    }
    node.children.iter().find_map(|child| find_node(child, id))
}

use super::{
    AutomationBounds, AutomationNodeId, AutomationNodeSemantics, AutomationNodeSnapshot,
    AutomationRole, GuiAutomationSnapshot,
};

pub(super) fn fixture_node(id: &str, role: AutomationRole, label: &str) -> AutomationNodeSnapshot {
    AutomationNodeSnapshot::from_semantics(
        AutomationNodeId::new(id),
        AutomationBounds {
            x: 10.0,
            y: 20.0,
            width: 40.0,
            height: 20.0,
        },
        AutomationNodeSemantics::new(role).with_label(label),
    )
}

pub(super) fn fixture_snapshot() -> GuiAutomationSnapshot {
    let nested = fixture_node("panel.nested", AutomationRole::Group, "Nested").with_children(vec![
        fixture_node("panel.nested.toggle", AutomationRole::Toggle, "Toggle"),
    ]);
    let root = AutomationNodeSnapshot::from_semantics(
        AutomationNodeId::new("root"),
        AutomationBounds::zero(),
        AutomationNodeSemantics::new(AutomationRole::Root),
    )
    .with_children(vec![
        fixture_node("toolbar.press", AutomationRole::Button, "Press"),
        nested,
    ]);

    GuiAutomationSnapshot {
        schema_version: 7,
        viewport_width: 1280,
        viewport_height: 720,
        root,
    }
}

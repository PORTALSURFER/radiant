use super::{
    AutomationNodeId, AutomationNodeSnapshot, AutomationTarget, GuiAutomationSnapshot,
    GuiAutomationTargetSnapshot,
};

impl AutomationNodeSnapshot {
    /// Return a flattened list of automation targets rooted at this node.
    pub fn automation_targets(&self) -> Vec<AutomationTarget> {
        let mut targets = Vec::new();
        let mut path = Vec::new();
        let mut tree_index = 0;
        self.push_automation_targets(0, &mut path, &mut tree_index, &mut targets);
        targets
    }

    fn push_automation_targets(
        &self,
        depth: usize,
        path: &mut Vec<AutomationNodeId>,
        tree_index: &mut usize,
        targets: &mut Vec<AutomationTarget>,
    ) {
        path.push(self.id.clone());
        let current_index = *tree_index;
        *tree_index += 1;
        targets.push(AutomationTarget::from_node(
            self,
            depth,
            current_index,
            path.clone(),
        ));
        for child in &self.children {
            child.push_automation_targets(depth + 1, path, tree_index, targets);
        }
        path.pop();
    }
}

impl GuiAutomationSnapshot {
    /// Return a flattened, coordinate-bearing target snapshot.
    pub fn target_snapshot(&self) -> GuiAutomationTargetSnapshot {
        GuiAutomationTargetSnapshot {
            schema_version: 1,
            viewport_width: self.viewport_width,
            viewport_height: self.viewport_height,
            targets: self.root.automation_targets(),
        }
    }
}

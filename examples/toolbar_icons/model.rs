use super::*;
use crate::icons::ToolbarIcons;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ToolId {
    Select,
    Brush,
    Erase,
    Snap,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ToolMessage {
    Toggle(ToolId),
}

#[derive(Clone, Debug)]
pub(super) struct ToolbarState {
    select: bool,
    brush: bool,
    erase: bool,
    snap: bool,
    pub(super) icons: ToolbarIcons,
}

impl Default for ToolbarState {
    fn default() -> Self {
        Self {
            select: true,
            brush: false,
            erase: false,
            snap: true,
            icons: ToolbarIcons::new(&ThemeTokens::default()),
        }
    }
}

impl ToolbarState {
    pub(super) fn active(&self, tool: ToolId) -> bool {
        match tool {
            ToolId::Select => self.select,
            ToolId::Brush => self.brush,
            ToolId::Erase => self.erase,
            ToolId::Snap => self.snap,
        }
    }

    pub(super) fn toggle(&mut self, tool: ToolId) {
        match tool {
            ToolId::Select => self.select = !self.select,
            ToolId::Brush => self.brush = !self.brush,
            ToolId::Erase => self.erase = !self.erase,
            ToolId::Snap => self.snap = !self.snap,
        }
    }

    pub(super) fn summary(&self) -> String {
        let active = [
            (self.select, "Select"),
            (self.brush, "Brush"),
            (self.erase, "Erase"),
            (self.snap, "Snap"),
        ]
        .into_iter()
        .filter_map(|(active, label)| active.then_some(label))
        .collect::<Vec<_>>();
        if active.is_empty() {
            "No tools active".to_string()
        } else {
            format!("Active: {}", active.join(", "))
        }
    }
}

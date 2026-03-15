//! Deterministic automation snapshot builders for the native shell.

use super::*;
use crate::app::{AutomationNodeSnapshot, AutomationRole, GuiAutomationSnapshot};
use std::collections::BTreeMap;

#[path = "automation/browser.rs"]
mod browser;
#[path = "automation/dialogs.rs"]
mod dialogs;
#[path = "automation/helpers.rs"]
mod helpers;
#[path = "automation/sidebar.rs"]
mod sidebar;
#[path = "automation/top_bar.rs"]
mod top_bar;
#[path = "automation/waveform.rs"]
mod waveform;

use self::{
    browser::build_browser_automation,
    dialogs::{options_panel_automation, progress_automation, prompt_automation},
    helpers::{bounds, node_id},
    sidebar::build_sidebar_automation,
    top_bar::build_top_bar_automation,
    waveform::build_waveform_automation,
};

impl NativeShellState {
    /// Build a deterministic semantic automation snapshot for the current shell state.
    pub(crate) fn automation_snapshot(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
    ) -> GuiAutomationSnapshot {
        let viewport_width =
            u32::try_from(layout.root.rect.width().round().max(1.0) as i64).unwrap_or(1);
        let viewport_height =
            u32::try_from(layout.root.rect.height().round().max(1.0) as i64).unwrap_or(1);
        GuiAutomationSnapshot {
            schema_version: 1,
            viewport_width,
            viewport_height,
            root: AutomationNodeSnapshot {
                id: node_id("shell.root"),
                role: AutomationRole::Root,
                label: Some(if model.title.trim().is_empty() {
                    String::from("Radiant shell")
                } else {
                    format!("{} shell", model.title)
                }),
                bounds: bounds(layout.root.rect),
                value: None,
                enabled: true,
                selected: false,
                available_actions: Vec::new(),
                metadata: BTreeMap::new(),
                children: self.automation_children(layout, model),
            },
        }
    }

    fn automation_children(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
    ) -> Vec<AutomationNodeSnapshot> {
        let style = style_for_layout(layout);
        vec![
            build_top_bar_automation(self, layout, model),
            build_sidebar_automation(self, layout, model, &style),
            build_waveform_automation(self, layout, model, &style),
            build_browser_automation(self, layout, model, &style),
            self.status_bar_automation(layout, model),
        ]
        .into_iter()
        .chain(options_panel_automation(layout, model, &style))
        .chain(prompt_automation(layout, model, &style))
        .chain(progress_automation(layout, model, &style))
        .collect()
    }

    fn status_bar_automation(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
    ) -> AutomationNodeSnapshot {
        AutomationNodeSnapshot {
            id: node_id("shell.status_bar"),
            role: AutomationRole::Readout,
            label: Some(String::from("Status bar")),
            bounds: bounds(layout.status_bar),
            value: Some(model.status.center.clone()),
            enabled: true,
            selected: false,
            available_actions: Vec::new(),
            metadata: helpers::metadata(&[
                ("left", model.status.left.as_str()),
                ("center", model.status.center.as_str()),
                ("right", model.status.right.as_str()),
            ]),
            children: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::BrowserRowModel;
    use crate::gui::types::Vector2;

    fn child<'a>(parent: &'a AutomationNodeSnapshot, id: &str) -> &'a AutomationNodeSnapshot {
        parent
            .children
            .iter()
            .find(|node| node.id.0 == id)
            .unwrap_or_else(|| panic!("missing automation child {id}"))
    }

    #[test]
    fn metadata_omits_empty_values() {
        let metadata = helpers::metadata(&[("kept", "value"), ("empty", "")]);
        assert_eq!(metadata.len(), 1);
        assert_eq!(metadata.get("kept").map(String::as_str), Some("value"));
        assert!(!metadata.contains_key("empty"));
    }

    #[test]
    fn slug_normalizes_non_alphanumeric_labels() {
        assert_eq!(helpers::slug("Open Update!"), "open_update_");
        assert_eq!(helpers::slug("BPM Value"), "bpm_value");
    }

    #[test]
    fn top_bar_surface_smoke_includes_panel_and_update_group() {
        let layout = ShellLayout::build(Vector2::new(1440.0, 810.0));
        let model = AppModel::default();
        let mut state = NativeShellState::new();
        let node = top_bar::build_top_bar_automation(&mut state, &layout, &model);
        assert_eq!(node.id.0, "shell.top_bar");
        let update = child(&node, "shell.top_bar.update_panel");
        assert_eq!(update.role, AutomationRole::Group);
    }

    #[test]
    fn sidebar_surface_smoke_includes_sources_panel() {
        let layout = ShellLayout::build(Vector2::new(1440.0, 810.0));
        let model = AppModel::default();
        let style = style_for_layout(&layout);
        let mut state = NativeShellState::new();
        let node = sidebar::build_sidebar_automation(&mut state, &layout, &model, &style);
        assert_eq!(node.id.0, "sources.panel");
        assert_eq!(node.role, AutomationRole::Panel);
    }

    #[test]
    fn waveform_surface_smoke_includes_waveform_region() {
        let layout = ShellLayout::build(Vector2::new(1440.0, 810.0));
        let model = AppModel::default();
        let style = style_for_layout(&layout);
        let mut state = NativeShellState::new();
        let node = waveform::build_waveform_automation(&mut state, &layout, &model, &style);
        let region = child(&node, "waveform.region");
        assert_eq!(region.role, AutomationRole::WaveformRegion);
    }

    #[test]
    fn browser_surface_smoke_includes_browser_panel_and_table() {
        let layout = ShellLayout::build(Vector2::new(1440.0, 810.0));
        let model = AppModel::default();
        let style = style_for_layout(&layout);
        let mut state = NativeShellState::new();
        let node = browser::build_browser_automation(&mut state, &layout, &model, &style);
        assert_eq!(node.id.0, "browser.panel");
        let table = child(&node, "browser.table");
        assert_eq!(table.role, AutomationRole::Table);
    }

    #[test]
    fn browser_surface_includes_scrollbar_nodes_when_rows_overflow() {
        let layout = ShellLayout::build(Vector2::new(1440.0, 810.0));
        let style = style_for_layout(&layout);
        let mut model = AppModel::default();
        for visible_row in 0..96 {
            model.browser.rows.push(BrowserRowModel::new(
                visible_row,
                format!("row_{visible_row:03}"),
                1,
                false,
                visible_row == 12,
            ));
        }
        model.browser.visible_count = model.browser.rows.len();
        model.browser.selected_visible_row = Some(12);
        let mut state = NativeShellState::new();
        let node = browser::build_browser_automation(&mut state, &layout, &model, &style);
        let table = child(&node, "browser.table");
        let track = child(table, "browser.scrollbar.track");
        let thumb = child(table, "browser.scrollbar.thumb");

        assert_eq!(
            table.metadata.get("scrollbar_visible").map(String::as_str),
            Some("true")
        );
        assert_eq!(track.role, AutomationRole::Slider);
        assert_eq!(thumb.role, AutomationRole::Slider);
        assert_eq!(
            track.metadata.get("part").map(String::as_str),
            Some("track")
        );
        assert_eq!(
            thumb.metadata.get("part").map(String::as_str),
            Some("thumb")
        );
        assert!(track.bounds.height > thumb.bounds.height);
    }

    #[test]
    fn dialog_surface_smoke_includes_options_prompt_and_progress_when_visible() {
        let layout = ShellLayout::build(Vector2::new(1440.0, 810.0));
        let style = style_for_layout(&layout);
        let mut model = AppModel::default();
        model.options_panel.visible = true;
        model.confirm_prompt.visible = true;
        model.progress_overlay.visible = true;
        model.progress_overlay.modal = true;
        let options =
            dialogs::options_panel_automation(&layout, &model, &style).expect("options panel node");
        let prompt = dialogs::prompt_automation(&layout, &model, &style).expect("prompt node");
        let progress =
            dialogs::progress_automation(&layout, &model, &style).expect("progress node");
        assert_eq!(options.id.0, "overlay.options_panel");
        assert_eq!(prompt.id.0, "overlay.prompt");
        assert_eq!(progress.id.0, "overlay.progress");
    }
}

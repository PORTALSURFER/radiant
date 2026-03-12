//! Deterministic automation snapshot builders for the native shell.

use super::*;
use crate::app::{
    AutomationBounds, AutomationNodeId, AutomationNodeSnapshot, AutomationRole,
    GuiAutomationSnapshot, NormalizedRangeModel, UpdateStatusModel,
};
use std::collections::BTreeMap;

impl NativeShellState {
    /// Build a deterministic semantic automation snapshot for the current shell state.
    pub(crate) fn automation_snapshot(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
    ) -> GuiAutomationSnapshot {
        let style = style_for_layout(layout);
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
                children: vec![
                    self.top_bar_automation(layout, model),
                    self.sidebar_automation(layout, model, &style),
                    self.waveform_automation(layout, model, &style),
                    self.browser_automation(layout, model, &style),
                    self.status_bar_automation(layout, model),
                ]
                .into_iter()
                .chain(options_panel_automation(layout, model, &style))
                .chain(prompt_automation(layout, model, &style))
                .chain(progress_automation(layout, model, &style))
                .collect(),
            },
        }
    }

    fn top_bar_automation(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
    ) -> AutomationNodeSnapshot {
        let controls = top_bar_controls_layout(layout, style_for_layout(layout).sizing);
        let mut children = Vec::new();
        if controls.active {
            children.push(simple_node(
                "shell.top_bar.volume_slider",
                AutomationRole::Slider,
                Some(String::from("Volume")),
                controls.volume_meter,
                Some(format!("{:.3}", model.volume.clamp(0.0, 1.0))),
                true,
                false,
                vec![
                    String::from("set_volume"),
                    String::from("commit_volume_setting"),
                ],
            ));
        }
        if let Some(button_rect) = status_options_button_rect(
            layout.top_bar_action_cluster,
            style_for_layout(layout).sizing,
        ) {
            children.push(simple_node(
                "shell.top_bar.options_button",
                AutomationRole::Button,
                Some(String::from("Options")),
                button_rect,
                None,
                true,
                model.options_panel.visible,
                vec![String::from(if model.options_panel.visible {
                    "close_options_panel"
                } else {
                    "open_options_menu"
                })],
            ));
        }
        children.push(update_panel_automation(
            layout,
            model,
            &style_for_layout(layout),
        ));
        AutomationNodeSnapshot {
            id: node_id("shell.top_bar"),
            role: AutomationRole::Panel,
            label: Some(String::from("Top bar")),
            bounds: bounds(layout.top_bar),
            value: Some(model.title.clone()),
            enabled: true,
            selected: false,
            available_actions: Vec::new(),
            metadata: metadata(&[
                ("title", model.title.as_str()),
                ("backend", model.backend_label.as_str()),
            ]),
            children,
        }
    }

    fn sidebar_automation(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        style: &StyleTokens,
    ) -> AutomationNodeSnapshot {
        let source_rows = self.cached_source_row_rects(layout, style, model).to_vec();
        let folder_rows = self.cached_folder_row_rects(layout, style, model).to_vec();
        let mut children = Vec::new();
        if let Some(rect) = source_add_button_rect(layout.sidebar_header, style.sizing) {
            children.push(simple_node(
                "sources.add_button",
                AutomationRole::Button,
                Some(String::from("Add source")),
                rect,
                None,
                true,
                false,
                vec![String::from("open_add_source_dialog")],
            ));
        }
        for (index, rect) in source_rows.into_iter().enumerate() {
            let row = &model.sources.rows[index];
            children.push(AutomationNodeSnapshot {
                id: node_id(format!("sources.source_row.{index}")),
                role: AutomationRole::Row,
                label: Some(row.label.clone()),
                bounds: bounds(rect),
                value: (!row.detail.is_empty()).then(|| row.detail.clone()),
                enabled: true,
                selected: row.selected,
                available_actions: vec![
                    String::from("select_source_row"),
                    String::from("reload_source_row"),
                    String::from("hard_sync_source_row"),
                    String::from("open_source_folder_row"),
                    String::from("remove_source_row"),
                    String::from("remove_dead_links_for_source_row"),
                ],
                metadata: metadata(&[
                    ("detail", row.detail.as_str()),
                    ("missing", bool_text(row.missing)),
                ]),
                children: Vec::new(),
            });
        }
        for button in source_action_buttons(layout, style, model) {
            children.push(simple_node(
                format!("sources.action.{}", slug(button.label)),
                AutomationRole::Button,
                Some(String::from(button.label)),
                button.rect,
                None,
                button.enabled,
                false,
                vec![action_slug(&button.action)],
            ));
        }
        for (index, rect) in folder_rows.into_iter().enumerate() {
            let row = &model.sources.folder_rows[index];
            children.push(AutomationNodeSnapshot {
                id: node_id(format!("sources.folder_row.{index}")),
                role: AutomationRole::Row,
                label: Some(row.label.clone()),
                bounds: bounds(rect),
                value: (!row.detail.is_empty()).then(|| row.detail.clone()),
                enabled: true,
                selected: row.selected || row.focused,
                available_actions: vec![
                    String::from("focus_folder_row"),
                    String::from("move_folder_focus"),
                    String::from("start_folder_rename"),
                    String::from("delete_focused_folder"),
                ],
                metadata: metadata(&[
                    ("depth", &row.depth.to_string()),
                    ("focused", bool_text(row.focused)),
                    ("root", bool_text(row.is_root)),
                    ("expanded", bool_text(row.expanded)),
                ]),
                children: Vec::new(),
            });
        }
        AutomationNodeSnapshot {
            id: node_id("sources.panel"),
            role: AutomationRole::Panel,
            label: Some(String::from("Sources")),
            bounds: bounds(layout.sidebar),
            value: Some(model.sources.header.clone()),
            enabled: true,
            selected: matches!(
                model.focus_context,
                crate::app::FocusContextModel::SourcesList
                    | crate::app::FocusContextModel::SourceFolders
            ),
            available_actions: vec![String::from("focus_sources_panel")],
            metadata: metadata(&[
                ("source_search", model.sources.search_query.as_str()),
                ("folder_search", model.sources.folder_search_query.as_str()),
            ]),
            children,
        }
    }

    fn waveform_automation(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        style: &StyleTokens,
    ) -> AutomationNodeSnapshot {
        let motion_model = NativeMotionModel::from_app_model(model);
        let mut children = Vec::new();
        for button in waveform_toolbar_buttons(
            layout,
            style,
            &motion_model,
            self.waveform_bpm_input_active,
            self.waveform_bpm_input_display.as_deref(),
        ) {
            children.push(simple_node(
                format!("waveform.toolbar.{}", slug(button.label)),
                if button.label == "BPM Value" {
                    AutomationRole::SearchField
                } else {
                    AutomationRole::Button
                },
                Some(String::from(button.label)),
                button.rect,
                button.display_text.clone(),
                button.enabled,
                button.active,
                button
                    .action
                    .as_ref()
                    .map(|action| vec![action_slug(action)])
                    .unwrap_or_default(),
            ));
        }
        children.push(AutomationNodeSnapshot {
            id: node_id("waveform.region"),
            role: AutomationRole::WaveformRegion,
            label: Some(String::from("Waveform")),
            bounds: bounds(layout.waveform_plot),
            value: model.waveform.loaded_label.clone(),
            enabled: true,
            selected: matches!(model.focus_context, crate::app::FocusContextModel::Waveform),
            available_actions: vec![
                String::from("seek_waveform"),
                String::from("set_waveform_cursor"),
                String::from("set_waveform_selection_range"),
                String::from("zoom_waveform"),
                String::from("set_waveform_view_center"),
            ],
            metadata: metadata(&[
                ("loop_enabled", bool_text(model.waveform.loop_enabled)),
                (
                    "tempo_label",
                    model.waveform.tempo_label.as_deref().unwrap_or(""),
                ),
                (
                    "zoom_label",
                    model.waveform.zoom_label.as_deref().unwrap_or(""),
                ),
                (
                    "cursor_milli",
                    &model
                        .waveform
                        .cursor_milli
                        .map(|value| value.to_string())
                        .unwrap_or_default(),
                ),
                (
                    "selection_micros",
                    &model
                        .waveform
                        .selection_milli
                        .map(selection_micros_text)
                        .unwrap_or_default(),
                ),
                (
                    "view_micros",
                    &format!(
                        "{}-{}",
                        model.waveform.view_start_micros, model.waveform.view_end_micros
                    ),
                ),
            ]),
            children: Vec::new(),
        });
        AutomationNodeSnapshot {
            id: node_id("waveform.panel"),
            role: AutomationRole::Panel,
            label: Some(String::from("Waveform panel")),
            bounds: bounds(layout.waveform_card),
            value: model.waveform.loaded_label.clone(),
            enabled: true,
            selected: matches!(model.focus_context, crate::app::FocusContextModel::Waveform),
            available_actions: vec![String::from("focus_waveform_panel")],
            metadata: BTreeMap::new(),
            children,
        }
    }

    fn browser_automation(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        style: &StyleTokens,
    ) -> AutomationNodeSnapshot {
        let buttons = browser_action_buttons(layout, style, model);
        let toolbar = browser_toolbar_layout(layout, style, &buttons);
        let tabs = compute_browser_tabs_rects(layout.browser_tabs, style.sizing);
        let mut children = vec![
            simple_node(
                "browser.tab.samples",
                AutomationRole::Tab,
                Some(model.browser_chrome.samples_tab_label.clone()),
                tabs.samples,
                None,
                true,
                !model.map.active,
                vec![String::from("set_browser_tab")],
            ),
            simple_node(
                "browser.tab.map",
                AutomationRole::Tab,
                Some(model.browser_chrome.map_tab_label.clone()),
                tabs.map,
                None,
                true,
                model.map.active,
                vec![String::from("set_browser_tab")],
            ),
        ];
        if toolbar.search_field.width() > 1.0 {
            children.push(simple_node(
                "browser.search_field",
                AutomationRole::SearchField,
                Some(String::from("Browser search")),
                toolbar.search_field,
                Some(model.browser.search_query.clone()),
                true,
                false,
                vec![
                    String::from("focus_browser_search"),
                    String::from("set_browser_search"),
                ],
            ));
        }
        for (index, chip) in toolbar.rating_filter_chips.iter().copied().enumerate() {
            if chip.width() <= 1.0 {
                continue;
            }
            let level = BROWSER_RATING_FILTER_LEVELS[index];
            children.push(simple_node(
                format!("browser.rating_filter.{level}"),
                AutomationRole::Button,
                Some(format!("Rating filter {level}")),
                chip,
                None,
                true,
                model.browser.active_rating_filters[index],
                vec![String::from("toggle_browser_rating_filter")],
            ));
        }
        if model.map.active {
            children.push(map_canvas_automation(layout, model, style));
        } else {
            children.push(self.browser_table_automation(layout, model, style));
        }
        AutomationNodeSnapshot {
            id: node_id("browser.panel"),
            role: AutomationRole::Panel,
            label: Some(String::from("Browser panel")),
            bounds: bounds(layout.browser_panel),
            value: Some(model.browser_chrome.item_count_label.clone()),
            enabled: true,
            selected: matches!(
                model.focus_context,
                crate::app::FocusContextModel::SampleBrowser
            ),
            available_actions: vec![String::from("focus_browser_panel")],
            metadata: metadata(&[
                (
                    "active_tab",
                    model.browser.active_tab_label.as_deref().unwrap_or(""),
                ),
                ("search_query", model.browser.search_query.as_str()),
                (
                    "focused_sample_label",
                    model.browser.focused_sample_label.as_deref().unwrap_or(""),
                ),
                (
                    "selected_visible_row",
                    &model
                        .browser
                        .selected_visible_row
                        .map(|value| value.to_string())
                        .unwrap_or_default(),
                ),
            ]),
            children,
        }
    }

    fn browser_table_automation(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        style: &StyleTokens,
    ) -> AutomationNodeSnapshot {
        let rows = self.cached_browser_rows(layout, style, model).to_vec();
        let mut table_node = simple_node(
            "browser.table",
            AutomationRole::Table,
            Some(String::from("Browser rows")),
            layout.browser_rows,
            Some(model.browser_chrome.item_count_label.clone()),
            true,
            matches!(
                model.focus_context,
                crate::app::FocusContextModel::SampleBrowser
            ),
            vec![String::from("focus_browser_panel")],
        );
        table_node.children = rows
            .into_iter()
            .map(|row| AutomationNodeSnapshot {
                id: node_id(format!("browser.row.{}", row.visible_row)),
                role: AutomationRole::Row,
                label: Some(row.label.clone()),
                bounds: bounds(row.rect),
                value: (!row.bucket_label.is_empty()).then_some(row.bucket_label.clone()),
                enabled: true,
                selected: row.selected || row.focused,
                available_actions: vec![
                    String::from("focus_browser_row"),
                    String::from("toggle_browser_row_selection"),
                    String::from("commit_focused_browser_row"),
                ],
                metadata: metadata(&[
                    ("column", &row.column.to_string()),
                    ("rating_level", &row.rating_level.to_string()),
                    ("focused", bool_text(row.focused)),
                    ("missing", bool_text(row.missing)),
                    ("locked", bool_text(row.locked)),
                ]),
                children: Vec::new(),
            })
            .collect();
        table_node
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
            metadata: metadata(&[
                ("left", model.status.left.as_str()),
                ("center", model.status.center.as_str()),
                ("right", model.status.right.as_str()),
            ]),
            children: Vec::new(),
        }
    }
}

fn update_panel_automation(
    layout: &ShellLayout,
    model: &AppModel,
    style: &StyleTokens,
) -> AutomationNodeSnapshot {
    let button_specs = update_button_specs(model);
    let labels: Vec<_> = button_specs.iter().map(|spec| spec.label).collect();
    let button_rects = compute_update_action_button_rects(
        layout.top_bar,
        layout.top_bar_action_cluster,
        style.sizing,
        &labels,
    );
    let mut children = Vec::new();
    for (spec, rect) in button_specs.into_iter().zip(button_rects) {
        children.push(simple_node(
            format!("shell.top_bar.update.{}", spec.node_slug),
            AutomationRole::Button,
            Some(String::from(spec.label)),
            rect,
            None,
            spec.enabled,
            false,
            vec![String::from(spec.action_id)],
        ));
    }
    AutomationNodeSnapshot {
        id: node_id("shell.top_bar.update_panel"),
        role: AutomationRole::Group,
        label: Some(String::from("Updates")),
        bounds: bounds(layout.top_bar_action_cluster),
        value: Some(model.update.status_label.clone()),
        enabled: true,
        selected: !children.is_empty(),
        available_actions: children
            .iter()
            .flat_map(|child| child.available_actions.clone())
            .collect(),
        metadata: metadata(&[
            ("status", update_status_text(model.update.status)),
            ("status_label", model.update.status_label.as_str()),
            ("action_hint", model.update.action_hint_label.as_str()),
            ("release_notes", model.update.release_notes_label.as_str()),
            (
                "available_tag",
                model.update.available_tag.as_deref().unwrap_or(""),
            ),
            (
                "available_url",
                model.update.available_url.as_deref().unwrap_or(""),
            ),
            (
                "last_error",
                model.update.last_error.as_deref().unwrap_or(""),
            ),
        ]),
        children,
    }
}

fn options_panel_automation(
    layout: &ShellLayout,
    model: &AppModel,
    style: &StyleTokens,
) -> Option<AutomationNodeSnapshot> {
    let panel = options_panel_layout(layout, style, model)?;
    Some(AutomationNodeSnapshot {
        id: node_id("overlay.options_panel"),
        role: AutomationRole::Dialog,
        label: Some(String::from("Options")),
        bounds: bounds(panel.panel_rect),
        value: None,
        enabled: true,
        selected: true,
        available_actions: vec![String::from("close_options_panel")],
        metadata: BTreeMap::new(),
        children: panel
            .buttons
            .into_iter()
            .map(|button| {
                simple_node(
                    format!("overlay.options_panel.{}", slug(button.label)),
                    AutomationRole::Button,
                    Some(String::from(button.label)),
                    button.rect,
                    None,
                    button.enabled,
                    false,
                    vec![action_slug(&button.action)],
                )
            })
            .collect(),
    })
}

fn prompt_automation(
    layout: &ShellLayout,
    model: &AppModel,
    style: &StyleTokens,
) -> Option<AutomationNodeSnapshot> {
    if !model.confirm_prompt.visible {
        return None;
    }
    let (confirm_button, cancel_button) = prompt_buttons(layout, style);
    let dialog = compute_prompt_overlay_visual_layout(
        layout.root.rect,
        layout.content,
        style.sizing,
        model.confirm_prompt.input_value.is_some(),
        model.confirm_prompt.target_label.is_some(),
    )
    .sections
    .dialog;
    let mut children = vec![
        simple_node(
            "overlay.prompt.confirm",
            AutomationRole::Button,
            Some(model.confirm_prompt.confirm_label.clone()),
            confirm_button,
            None,
            model.confirm_prompt.input_error.is_none(),
            false,
            vec![String::from("confirm_prompt")],
        ),
        simple_node(
            "overlay.prompt.cancel",
            AutomationRole::Button,
            Some(model.confirm_prompt.cancel_label.clone()),
            cancel_button,
            None,
            true,
            false,
            vec![String::from("cancel_prompt")],
        ),
    ];
    if let Some(input_rect) = prompt_input_rect(layout, style, model) {
        children.push(simple_node(
            "overlay.prompt.input",
            AutomationRole::SearchField,
            Some(String::from("Prompt input")),
            input_rect,
            model.confirm_prompt.input_value.clone(),
            true,
            false,
            vec![String::from("set_prompt_input")],
        ));
    }
    Some(AutomationNodeSnapshot {
        id: node_id("overlay.prompt"),
        role: AutomationRole::Dialog,
        label: Some(model.confirm_prompt.title.clone()),
        bounds: bounds(dialog),
        value: Some(model.confirm_prompt.message.clone()),
        enabled: true,
        selected: true,
        available_actions: Vec::new(),
        metadata: metadata(&[
            ("kind", &format!("{:?}", model.confirm_prompt.kind)),
            (
                "input_error",
                model.confirm_prompt.input_error.as_deref().unwrap_or(""),
            ),
        ]),
        children,
    })
}

fn progress_automation(
    layout: &ShellLayout,
    model: &AppModel,
    style: &StyleTokens,
) -> Option<AutomationNodeSnapshot> {
    if !model.progress_overlay.visible || !model.progress_overlay.modal {
        return None;
    }
    Some(AutomationNodeSnapshot {
        id: node_id("overlay.progress"),
        role: AutomationRole::Dialog,
        label: Some(model.progress_overlay.title.clone()),
        bounds: bounds(
            compute_progress_overlay_visual_layout(
                layout.root.rect,
                layout.content,
                style.sizing,
                true,
                0.0,
            )
            .sections
            .dialog,
        ),
        value: model.progress_overlay.detail.clone(),
        enabled: true,
        selected: true,
        available_actions: Vec::new(),
        metadata: metadata(&[
            ("completed", &model.progress_overlay.completed.to_string()),
            ("total", &model.progress_overlay.total.to_string()),
        ]),
        children: model
            .progress_overlay
            .cancelable
            .then(|| {
                vec![simple_node(
                    "overlay.progress.cancel",
                    AutomationRole::Button,
                    Some(String::from("Cancel")),
                    progress_cancel_button(layout, style, true),
                    None,
                    !model.progress_overlay.cancel_requested,
                    false,
                    vec![String::from("cancel_progress")],
                )]
            })
            .unwrap_or_default(),
    })
}

fn map_canvas_automation(
    layout: &ShellLayout,
    model: &AppModel,
    style: &StyleTokens,
) -> AutomationNodeSnapshot {
    let canvas = compute_browser_map_canvas_rect(layout.browser_rows, style.sizing);
    let mut map_node = simple_node(
        "browser.map_canvas",
        AutomationRole::MapCanvas,
        Some(String::from("Similarity map")),
        canvas,
        Some(model.map.summary.clone()),
        true,
        true,
        vec![String::from("focus_map_sample")],
    );
    map_node.children = model
        .map
        .points
        .iter()
        .map(|point| AutomationNodeSnapshot {
            id: node_id(format!("browser.map.point.{}", point.sample_id)),
            role: AutomationRole::MapPoint,
            label: Some(String::from(point.sample_id.as_ref())),
            bounds: bounds(circle_rect(
                compute_browser_map_point_center(canvas, point.x_milli, point.y_milli),
                10.0,
            )),
            value: None,
            enabled: true,
            selected: model.map.selected_sample_id.as_deref() == Some(point.sample_id.as_ref()),
            available_actions: vec![String::from("focus_map_sample")],
            metadata: metadata(&[
                ("x_milli", &point.x_milli.to_string()),
                ("y_milli", &point.y_milli.to_string()),
            ]),
            children: Vec::new(),
        })
        .collect();
    map_node
}

fn simple_node(
    id: impl Into<String>,
    role: AutomationRole,
    label: Option<String>,
    rect: Rect,
    value: Option<String>,
    enabled: bool,
    selected: bool,
    available_actions: Vec<String>,
) -> AutomationNodeSnapshot {
    AutomationNodeSnapshot {
        id: node_id(id),
        role,
        label,
        bounds: bounds(rect),
        value,
        enabled,
        selected,
        available_actions,
        metadata: BTreeMap::new(),
        children: Vec::new(),
    }
}

fn node_id(id: impl Into<String>) -> AutomationNodeId {
    AutomationNodeId::new(id)
}

fn bounds(rect: Rect) -> AutomationBounds {
    AutomationBounds {
        x: quantize(rect.min.x),
        y: quantize(rect.min.y),
        width: quantize(rect.width()),
        height: quantize(rect.height()),
    }
}

fn quantize(value: f32) -> f32 {
    (value * 1000.0).round() / 1000.0
}

fn metadata(entries: &[(&str, &str)]) -> BTreeMap<String, String> {
    entries
        .iter()
        .filter(|(_, value)| !value.is_empty())
        .map(|(key, value)| (String::from(*key), String::from(*value)))
        .collect()
}

fn bool_text(value: bool) -> &'static str {
    if value { "true" } else { "false" }
}

fn update_status_text(status: UpdateStatusModel) -> &'static str {
    match status {
        UpdateStatusModel::Idle => "idle",
        UpdateStatusModel::Checking => "checking",
        UpdateStatusModel::Available => "available",
        UpdateStatusModel::Error => "error",
    }
}

fn selection_micros_text(range: NormalizedRangeModel) -> String {
    format!("{}-{}", range.start_micros, range.end_micros)
}

struct UpdateButtonSpec<'a> {
    label: &'a str,
    node_slug: &'a str,
    action_id: &'a str,
    enabled: bool,
}

fn update_button_specs(model: &AppModel) -> Vec<UpdateButtonSpec<'static>> {
    match model.update.status {
        UpdateStatusModel::Idle => vec![UpdateButtonSpec {
            label: "Check",
            node_slug: "check",
            action_id: "check_for_updates",
            enabled: true,
        }],
        UpdateStatusModel::Checking => Vec::new(),
        UpdateStatusModel::Available => {
            let mut buttons = Vec::new();
            if model.update.available_url.is_some() {
                buttons.push(UpdateButtonSpec {
                    label: "Open",
                    node_slug: "open",
                    action_id: "open_update_link",
                    enabled: true,
                });
                buttons.push(UpdateButtonSpec {
                    label: "Install",
                    node_slug: "install",
                    action_id: "install_update",
                    enabled: true,
                });
            }
            buttons.push(UpdateButtonSpec {
                label: "Dismiss",
                node_slug: "dismiss",
                action_id: "dismiss_update",
                enabled: true,
            });
            buttons
        }
        UpdateStatusModel::Error => vec![UpdateButtonSpec {
            label: "Retry",
            node_slug: "check",
            action_id: "check_for_updates",
            enabled: true,
        }],
    }
}

fn slug(label: &str) -> String {
    label
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect()
}

fn action_slug(action: &UiAction) -> String {
    match action {
        UiAction::SelectColumn { .. } => "select_column",
        UiAction::MoveColumn { .. } => "move_column",
        UiAction::ToggleTransport => "toggle_transport",
        UiAction::PlayFromStart => "play_from_start",
        UiAction::PlayFromCurrentPlayhead => "play_from_current_playhead",
        UiAction::HandleEscape => "handle_escape",
        UiAction::FocusBrowserPanel => "focus_browser_panel",
        UiAction::FocusSourcesPanel => "focus_sources_panel",
        UiAction::FocusWaveformPanel => "focus_waveform_panel",
        UiAction::FocusLoadedSampleInBrowser => "focus_loaded_sample_in_browser",
        UiAction::FocusBrowserSearch => "focus_browser_search",
        UiAction::BlurBrowserSearch => "blur_browser_search",
        UiAction::OpenAddSourceDialog => "open_add_source_dialog",
        UiAction::OpenOptionsMenu => "open_options_menu",
        UiAction::CloseOptionsPanel => "close_options_panel",
        UiAction::PickTrashFolder => "pick_trash_folder",
        UiAction::OpenTrashFolder => "open_trash_folder",
        UiAction::FocusFolderSearch => "focus_folder_search",
        UiAction::SetFolderSearch { .. } => "set_folder_search",
        UiAction::SelectSourceRow { .. } => "select_source_row",
        UiAction::ReloadSourceRow { .. } => "reload_source_row",
        UiAction::HardSyncSourceRow { .. } => "hard_sync_source_row",
        UiAction::OpenSourceFolderRow { .. } => "open_source_folder_row",
        UiAction::RemoveSourceRow { .. } => "remove_source_row",
        UiAction::RemoveDeadLinksForSourceRow { .. } => "remove_dead_links_for_source_row",
        UiAction::FocusFolderRow { .. } => "focus_folder_row",
        UiAction::MoveFolderFocus { .. } => "move_folder_focus",
        UiAction::StartNewFolder => "start_new_folder",
        UiAction::StartNewFolderAtRoot => "start_new_folder_at_root",
        UiAction::StartFolderRename => "start_folder_rename",
        UiAction::DeleteFocusedFolder => "delete_focused_folder",
        UiAction::ClearFolderDeleteRecoveryLog => "clear_folder_delete_recovery_log",
        UiAction::MoveBrowserFocus { .. } => "move_browser_focus",
        UiAction::SetBrowserViewStart { .. } => "set_browser_view_start",
        UiAction::FocusBrowserRow { .. } => "focus_browser_row",
        UiAction::CommitFocusedBrowserRow => "commit_focused_browser_row",
        UiAction::SaveWaveformSelectionToBrowser => "save_waveform_selection_to_browser",
        UiAction::ToggleBrowserRowSelection { .. } => "toggle_browser_row_selection",
        UiAction::ExtendBrowserSelectionToRow { .. } => "extend_browser_selection_to_row",
        UiAction::AddRangeBrowserSelection { .. } => "add_range_browser_selection",
        UiAction::ExtendBrowserSelectionFromFocus { .. } => "extend_browser_selection_from_focus",
        UiAction::AddRangeBrowserSelectionFromFocus { .. } => {
            "add_range_browser_selection_from_focus"
        }
        UiAction::ToggleFocusedBrowserRowSelection => "toggle_focused_browser_row_selection",
        UiAction::SelectAllBrowserRows => "select_all_browser_rows",
        UiAction::SetBrowserSearch { .. } => "set_browser_search",
        UiAction::ToggleBrowserRatingFilter { .. } => "toggle_browser_rating_filter",
        UiAction::SetBrowserTab { .. } => "set_browser_tab",
        UiAction::FocusMapSample { .. } => "focus_map_sample",
        UiAction::SetPromptInput { .. } => "set_prompt_input",
        UiAction::StartBrowserRename => "start_browser_rename",
        UiAction::ConfirmBrowserRename => "confirm_browser_rename",
        UiAction::CancelBrowserRename => "cancel_browser_rename",
        UiAction::TagBrowserSelection { .. } => "tag_browser_selection",
        UiAction::DeleteBrowserSelection => "delete_browser_selection",
        UiAction::NormalizeFocusedBrowserSample => "normalize_focused_browser_sample",
        UiAction::NormalizeWaveformSelectionOrSample => "normalize_waveform_selection_or_sample",
        UiAction::CropWaveformSelection => "crop_waveform_selection",
        UiAction::CropWaveformSelectionToNewSample => "crop_waveform_selection_to_new_sample",
        UiAction::TrimWaveformSelection => "trim_waveform_selection",
        UiAction::ConfirmPrompt => "confirm_prompt",
        UiAction::CancelPrompt => "cancel_prompt",
        UiAction::CancelProgress => "cancel_progress",
        UiAction::SetInputMonitoringEnabled { .. } => "set_input_monitoring_enabled",
        UiAction::SetAdvanceAfterRatingEnabled { .. } => "set_advance_after_rating_enabled",
        UiAction::SetDestructiveYoloMode { .. } => "set_destructive_yolo_mode",
        UiAction::SetInvertWaveformScroll { .. } => "set_invert_waveform_scroll",
        UiAction::ToggleLoopPlayback => "toggle_loop_playback",
        UiAction::SetWaveformChannelView { .. } => "set_waveform_channel_view",
        UiAction::SetNormalizedAuditionEnabled { .. } => "set_normalized_audition_enabled",
        UiAction::SetBpmSnapEnabled { .. } => "set_bpm_snap_enabled",
        UiAction::AdjustWaveformBpm { .. } => "adjust_waveform_bpm",
        UiAction::SetWaveformBpmValue { .. } => "set_waveform_bpm_value",
        UiAction::SetTransientSnapEnabled { .. } => "set_transient_snap_enabled",
        UiAction::SetTransientMarkersEnabled { .. } => "set_transient_markers_enabled",
        UiAction::SetSliceModeEnabled { .. } => "set_slice_mode_enabled",
        UiAction::SetVolume { .. } => "set_volume",
        UiAction::CommitVolumeSetting => "commit_volume_setting",
        UiAction::SeekWaveform { .. } => "seek_waveform",
        UiAction::SetWaveformCursor { .. } => "set_waveform_cursor",
        UiAction::SetWaveformSelectionRange { .. } => "set_waveform_selection_range",
        UiAction::SetWaveformSelectionRangeSmartScale { .. } => {
            "set_waveform_selection_range_smart_scale"
        }
        UiAction::SetWaveformEditSelectionRange { .. } => "set_waveform_edit_selection_range",
        UiAction::SetWaveformEditFadeInEnd { .. } => "set_waveform_edit_fade_in_end",
        UiAction::SetWaveformEditFadeInMuteStart { .. } => "set_waveform_edit_fade_in_mute_start",
        UiAction::SetWaveformEditFadeInCurve { .. } => "set_waveform_edit_fade_in_curve",
        UiAction::SetWaveformEditFadeOutStart { .. } => "set_waveform_edit_fade_out_start",
        UiAction::SetWaveformEditFadeOutMuteEnd { .. } => "set_waveform_edit_fade_out_mute_end",
        UiAction::SetWaveformEditFadeOutCurve { .. } => "set_waveform_edit_fade_out_curve",
        UiAction::FinishWaveformEditFadeDrag => "finish_waveform_edit_fade_drag",
        UiAction::StartWaveformSelectionDrag { .. } => "start_waveform_selection_drag",
        UiAction::UpdateWaveformSelectionDrag { .. } => "update_waveform_selection_drag",
        UiAction::FinishWaveformSelectionDrag => "finish_waveform_selection_drag",
        UiAction::BeginWaveformSelectionShift { .. } => "begin_waveform_selection_shift",
        UiAction::BeginWaveformEditSelectionShift { .. } => "begin_waveform_edit_selection_shift",
        UiAction::ClearWaveformSelection => "clear_waveform_selection",
        UiAction::ClearWaveformEditSelection => "clear_waveform_edit_selection",
        UiAction::SetWaveformViewCenter { .. } => "set_waveform_view_center",
        UiAction::ZoomWaveform { .. } => "zoom_waveform",
        UiAction::ZoomWaveformToSelection => "zoom_waveform_to_selection",
        UiAction::ZoomWaveformFull => "zoom_waveform_full",
        UiAction::Undo => "undo",
        UiAction::Redo => "redo",
        UiAction::CheckForUpdates => "check_for_updates",
        UiAction::OpenUpdateLink => "open_update_link",
        UiAction::InstallUpdate => "install_update",
        UiAction::DismissUpdate => "dismiss_update",
    }
    .to_string()
}

fn circle_rect(center: Point, diameter: f32) -> Rect {
    let radius = diameter * 0.5;
    Rect::from_min_max(
        Point::new(center.x - radius, center.y - radius),
        Point::new(center.x + radius, center.y + radius),
    )
}

//! Sidebar automation snapshot builders.

use super::helpers::{action_slug, bool_text, bounds, metadata, node_id, simple_node, slug};
use super::*;
use crate::app::AutomationRole;

/// Build semantic automation for the sources/sidebar panel.
pub(super) fn build_sidebar_automation(
    shell: &mut NativeShellState,
    layout: &ShellLayout,
    model: &AppModel,
    style: &StyleTokens,
) -> AutomationNodeSnapshot {
    let source_rows = shell.cached_source_row_rects(layout, style, model).to_vec();
    let folder_rows = shell.cached_folder_rows(layout, style, model).to_vec();
    let sections = sidebar_sections(layout, style, model);
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
    children.push(source_list_group(
        sections.source_rows,
        source_rows,
        &model.sources.rows,
        model.focus_context == crate::app::FocusContextModel::SourcesList,
    ));
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
    children.push(folder_browser_group(
        sections.folder_header,
        sections.folder_rows,
        folder_rows,
        &model.sources.folder_rows,
        model,
        style,
        model.focus_context == crate::app::FocusContextModel::SourceFolders,
    ));
    AutomationNodeSnapshot {
        id: node_id("sources.panel"),
        role: AutomationRole::Panel,
        label: Some(String::from("Sources")),
        bounds: bounds(layout.sidebar),
        value: Some(model.sources.header.clone()),
        enabled: true,
        selected: false,
        available_actions: vec![String::from("focus_sources_panel")],
        metadata: metadata(&[
            ("source_search", model.sources.search_query.as_str()),
            ("folder_search", model.sources.folder_search_query.as_str()),
        ]),
        children,
    }
}

fn source_list_group(
    rect: Rect,
    source_rows: Vec<Rect>,
    rows: &[crate::app::SourceRowModel],
    selected: bool,
) -> AutomationNodeSnapshot {
    let row_count = rows.len().to_string();
    let children = source_rows
        .into_iter()
        .enumerate()
        .filter_map(|(index, rect)| rows.get(index).map(|row| (index, rect, row)))
        .map(|(index, rect, row)| AutomationNodeSnapshot {
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
            ],
            metadata: metadata(&[
                ("detail", row.detail.as_str()),
                ("missing", bool_text(row.missing)),
            ]),
            children: Vec::new(),
        })
        .collect();
    AutomationNodeSnapshot {
        id: node_id("sources.source_list"),
        role: AutomationRole::Group,
        label: Some(String::from("Source list")),
        bounds: bounds(rect),
        value: None,
        enabled: true,
        selected,
        available_actions: vec![String::from("focus_sources_panel")],
        metadata: metadata(&[("row_count", &row_count)]),
        children,
    }
}

fn folder_browser_group(
    header_rect: Rect,
    folder_rows_band: Rect,
    folder_rows: Vec<CachedFolderRow>,
    rows: &[crate::app::FolderRowModel],
    model: &AppModel,
    style: &StyleTokens,
    selected: bool,
) -> AutomationNodeSnapshot {
    let row_count = rows.len().to_string();
    let mut children = Vec::new();
    if let Some(toggle_button) = compute_sidebar_folder_header_layout(
        header_rect,
        style.sizing,
        model.sources.folder_recovery.in_progress,
        model.sources.folder_recovery.entry_count,
        model.sources.show_all_folders,
        model.sources.can_toggle_show_all_folders,
        model.sources.flattened_view,
        model.sources.can_toggle_flattened_view,
    )
    .visibility_toggle_button
    {
        children.push(simple_node(
            "sources.folder_visibility_toggle",
            AutomationRole::Button,
            Some(String::from("Folder visibility")),
            toggle_button.rect,
            Some(if toggle_button.active {
                String::from("All folders")
            } else {
                String::from("WAV folders")
            }),
            toggle_button.enabled,
            toggle_button.active,
            vec![String::from("toggle_show_all_folders")],
        ));
    }
    if let Some(toggle_button) = compute_sidebar_folder_header_layout(
        header_rect,
        style.sizing,
        model.sources.folder_recovery.in_progress,
        model.sources.folder_recovery.entry_count,
        model.sources.show_all_folders,
        model.sources.can_toggle_show_all_folders,
        model.sources.flattened_view,
        model.sources.can_toggle_flattened_view,
    )
    .flatten_toggle_button
    {
        children.push(simple_node(
            "sources.folder_flatten_toggle",
            AutomationRole::Button,
            Some(String::from("Flattened view")),
            toggle_button.rect,
            Some(if toggle_button.active {
                String::from("All descendants")
            } else {
                String::from("Direct only")
            }),
            toggle_button.enabled,
            toggle_button.active,
            vec![String::from("toggle_folder_flattened_view")],
        ));
    }
    children.extend(
        folder_rows
            .into_iter()
            .filter_map(|rendered_row| {
                rows.get(rendered_row.row_index)
                    .map(|row| (rendered_row.row_index, rendered_row.rect, row))
            })
            .map(|(row_index, rect, row)| {
                let (role, label, value, available_actions) = if matches!(
                    row.kind,
                    crate::app::FolderRowKind::CreateDraft | crate::app::FolderRowKind::RenameDraft
                ) {
                    (
                        AutomationRole::SearchField,
                        Some(if row.kind == crate::app::FolderRowKind::RenameDraft {
                            String::from("Rename folder")
                        } else {
                            String::from("New folder")
                        }),
                        row.input_value.clone(),
                        vec![
                            String::from("focus_folder_create_input"),
                            String::from("set_folder_create_input"),
                            String::from("confirm_folder_create"),
                            String::from("cancel_folder_create"),
                        ],
                    )
                } else {
                    let mut available_actions = vec![
                        String::from("focus_folder_row"),
                        String::from("move_folder_focus"),
                        String::from("start_folder_rename"),
                        String::from("delete_focused_folder"),
                    ];
                    if row.has_children && !row.is_root {
                        available_actions.push(String::from("toggle_folder_row_expanded"));
                        available_actions.push(String::from("expand_focused_folder"));
                        available_actions.push(String::from("collapse_focused_folder"));
                    }
                    (
                        AutomationRole::Row,
                        Some(row.label.clone()),
                        (!row.detail.is_empty()).then(|| row.detail.clone()),
                        available_actions,
                    )
                };
                AutomationNodeSnapshot {
                    id: node_id(format!("sources.folder_row.{row_index}")),
                    role,
                    label,
                    bounds: bounds(rect),
                    value,
                    enabled: true,
                    selected: row.selected || row.focused || row.input_focused,
                    available_actions,
                    metadata: metadata(&[
                        ("depth", &row.depth.to_string()),
                        ("focused", bool_text(row.focused)),
                        ("root", bool_text(row.is_root)),
                        ("expanded", bool_text(row.expanded)),
                        (
                            "kind",
                            match row.kind {
                                crate::app::FolderRowKind::CreateDraft => "create_draft",
                                crate::app::FolderRowKind::RenameDraft => "rename_draft",
                                crate::app::FolderRowKind::Existing => "existing",
                            },
                        ),
                        ("input_error", row.input_error.as_deref().unwrap_or("")),
                        ("select_all_on_focus", bool_text(row.select_all_on_focus)),
                    ]),
                    children: Vec::new(),
                }
            }),
    );
    AutomationNodeSnapshot {
        id: node_id("sources.folder_browser"),
        role: AutomationRole::Group,
        label: Some(String::from("Folder browser")),
        bounds: bounds(union_rect(header_rect, folder_rows_band)),
        value: None,
        enabled: true,
        selected,
        available_actions: vec![String::from("focus_folder_panel")],
        metadata: metadata(&[
            ("row_count", &row_count),
            (
                "visibility",
                if model.sources.show_all_folders {
                    "all_folders"
                } else {
                    "wav_folders"
                },
            ),
            (
                "flattened_view",
                if model.sources.flattened_view {
                    "all_descendants"
                } else {
                    "direct_only"
                },
            ),
        ]),
        children,
    }
}

fn union_rect(first: Rect, second: Rect) -> Rect {
    Rect::from_min_max(
        Point::new(first.min.x.min(second.min.x), first.min.y.min(second.min.y)),
        Point::new(first.max.x.max(second.max.x), first.max.y.max(second.max.y)),
    )
}

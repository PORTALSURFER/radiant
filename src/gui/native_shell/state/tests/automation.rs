use super::{
    browser::build_browser_automation,
    dialogs::{options_panel_automation, progress_automation, prompt_automation},
    helpers,
    sidebar::build_sidebar_automation,
    top_bar::build_top_bar_automation,
    waveform::build_waveform_automation,
    *,
};
use crate::app::{AutomationNodeSnapshot, AutomationRole};
use crate::app::{BrowserRowModel, FocusContextModel, FolderRowModel, SourceRowModel};
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
    let node = build_top_bar_automation(&mut state, &layout, &model);
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
    let node = build_sidebar_automation(&mut state, &layout, &model, &style);
    assert_eq!(node.id.0, "sources.panel");
    assert_eq!(node.role, AutomationRole::Panel);
}

#[test]
fn sidebar_surface_exposes_distinct_source_list_and_folder_browser_focus_groups() {
    let layout = ShellLayout::build(Vector2::new(1440.0, 810.0));
    let style = style_for_layout(&layout);
    let mut model = AppModel::default();
    model
        .sources
        .rows
        .push(SourceRowModel::new("source_a", "detail_a", false, false));
    model.sources.folder_rows.push(FolderRowModel::new(
        "folder_a",
        String::new(),
        0,
        false,
        false,
        true,
        true,
        true,
    ));
    let mut state = NativeShellState::new();

    model.focus_context = FocusContextModel::SourcesList;
    let sources_node = build_sidebar_automation(&mut state, &layout, &model, &style);
    let source_list = child(&sources_node, "sources.source_list");
    let folder_browser = child(&sources_node, "sources.folder_browser");
    assert!(source_list.selected);
    assert!(!folder_browser.selected);
    assert_eq!(source_list.role, AutomationRole::Group);

    model.focus_context = FocusContextModel::SourceFolders;
    let folders_node = build_sidebar_automation(&mut state, &layout, &model, &style);
    let source_list = child(&folders_node, "sources.source_list");
    let folder_browser = child(&folders_node, "sources.folder_browser");
    assert!(!source_list.selected);
    assert!(folder_browser.selected);
    assert_eq!(folder_browser.role, AutomationRole::Group);
}

#[test]
fn sidebar_automation_exposes_inline_folder_create_row_metadata() {
    let layout = ShellLayout::build(Vector2::new(1440.0, 810.0));
    let style = style_for_layout(&layout);
    let mut model = AppModel::default();
    model.sources.folder_rows.push(FolderRowModel::new(
        "Root",
        String::new(),
        0,
        false,
        false,
        true,
        true,
        true,
    ));
    model.sources.folder_rows.push(FolderRowModel::create_draft(
        1,
        String::from("new folder"),
        String::from("New folder name"),
        Some(String::from("Folder already exists")),
        true,
    ));
    let mut state = NativeShellState::new();

    let node = build_sidebar_automation(&mut state, &layout, &model, &style);
    let browser = child(&node, "sources.folder_browser");
    let draft = child(browser, "sources.folder_row.1");

    assert_eq!(draft.role, AutomationRole::SearchField);
    assert_eq!(draft.label.as_deref(), Some("New folder"));
    assert_eq!(draft.value.as_deref(), Some("new folder"));
    assert_eq!(
        draft.metadata.get("kind").map(String::as_str),
        Some("create_draft")
    );
    assert_eq!(
        draft.metadata.get("input_error").map(String::as_str),
        Some("Folder already exists")
    );
    assert!(
        draft
            .available_actions
            .contains(&String::from("focus_folder_create_input"))
    );
}

#[test]
fn sidebar_automation_exposes_inline_folder_rename_row_metadata() {
    let layout = ShellLayout::build(Vector2::new(1440.0, 810.0));
    let style = style_for_layout(&layout);
    let mut model = AppModel::default();
    model.sources.folder_rows.push(FolderRowModel::new(
        "Root",
        String::new(),
        0,
        false,
        false,
        true,
        true,
        true,
    ));
    model.sources.folder_rows.push(FolderRowModel::rename_draft(
        1,
        String::from("drums"),
        String::from("Folder name"),
        None,
        true,
    ));
    let mut state = NativeShellState::new();

    let node = build_sidebar_automation(&mut state, &layout, &model, &style);
    let browser = child(&node, "sources.folder_browser");
    let draft = child(browser, "sources.folder_row.1");

    assert_eq!(draft.role, AutomationRole::SearchField);
    assert_eq!(draft.label.as_deref(), Some("Rename folder"));
    assert_eq!(draft.value.as_deref(), Some("drums"));
    assert_eq!(
        draft.metadata.get("kind").map(String::as_str),
        Some("rename_draft")
    );
    assert_eq!(
        draft
            .metadata
            .get("select_all_on_focus")
            .map(String::as_str),
        Some("true")
    );
    assert!(
        draft
            .available_actions
            .contains(&String::from("focus_folder_create_input"))
    );
}

#[test]
fn sidebar_automation_exposes_folder_flatten_toggle_metadata() {
    let layout = ShellLayout::build(Vector2::new(1440.0, 810.0));
    let style = style_for_layout(&layout);
    let mut model = AppModel::default();
    model.sources.folder_rows.push(FolderRowModel::new(
        "Root",
        String::new(),
        0,
        false,
        false,
        true,
        true,
        true,
    ));
    model.sources.can_toggle_show_all_folders = true;
    model.sources.can_toggle_flattened_view = true;
    model.sources.flattened_view = true;
    let mut state = NativeShellState::new();

    let node = build_sidebar_automation(&mut state, &layout, &model, &style);
    let browser = child(&node, "sources.folder_browser");
    let flatten = child(browser, "sources.folder_flatten_toggle");

    assert_eq!(flatten.role, AutomationRole::Button);
    assert_eq!(flatten.label.as_deref(), Some("Flattened view"));
    assert_eq!(flatten.value.as_deref(), Some("All descendants"));
    assert_eq!(
        flatten.available_actions,
        vec![String::from("toggle_folder_flattened_view")]
    );
    assert_eq!(
        browser.metadata.get("flattened_view").map(String::as_str),
        Some("all_descendants")
    );
}

#[test]
fn waveform_surface_smoke_includes_waveform_region() {
    let layout = ShellLayout::build(Vector2::new(1440.0, 810.0));
    let model = AppModel::default();
    let style = style_for_layout(&layout);
    let mut state = NativeShellState::new();
    let node = build_waveform_automation(&mut state, &layout, &model, &style);
    let region = child(&node, "waveform.region");
    assert_eq!(region.role, AutomationRole::WaveformRegion);
}

#[test]
fn browser_surface_smoke_includes_browser_panel_and_table() {
    let layout = ShellLayout::build(Vector2::new(1440.0, 810.0));
    let model = AppModel::default();
    let style = style_for_layout(&layout);
    let mut state = NativeShellState::new();
    let node = build_browser_automation(&mut state, &layout, &model, &style);
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
    let node = build_browser_automation(&mut state, &layout, &model, &style);
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
    let options = options_panel_automation(&layout, &model, &style).expect("options panel node");
    let prompt = prompt_automation(&layout, &model, &style).expect("prompt node");
    let progress = progress_automation(&layout, &model, &style).expect("progress node");
    assert_eq!(options.id.0, "overlay.options_panel");
    assert_eq!(prompt.id.0, "overlay.prompt");
    assert_eq!(progress.id.0, "overlay.progress");
}

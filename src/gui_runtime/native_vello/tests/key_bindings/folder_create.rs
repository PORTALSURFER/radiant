use super::*;

#[test]
fn hovered_folder_row_n_creates_under_hovered_folder() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());
    runner.model = Arc::new(AppModel {
        focus_context: crate::app::FocusContextModel::SourceFolders,
        sources: SourcesPanelModel {
            folder_rows: vec![
                crate::app::FolderRowModel::new("Root", "", 0, false, false, true, true, true)
                    .with_source_index(0),
                crate::app::FolderRowModel::new(
                    "Drums", "drums", 1, false, true, false, true, true,
                )
                .with_source_index(4),
            ],
            ..SourcesPanelModel::default()
        },
        ..AppModel::default()
    });
    runner.frame_state.model_dirty = false;
    runner
        .shell_state
        .set_hovered_folder_row_index_for_tests(Some(1));

    runner.handle_hotkey_press_for_tests(KeyCode::N);

    assert_eq!(
        runner.bridge.actions,
        vec![UiAction::StartNewFolderAtFolderRow { index: 4 }]
    );
}

#[test]
fn folder_create_append_text_emits_set_folder_create_input() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());
    runner.text_input_target = TextInputTarget::FolderCreate;
    runner.text_input_buffer = Some(String::from("dr"));
    runner.text_editor_state = Some(SingleLineTextEditorState::collapsed_at_end("dr"));

    assert!(runner.append_text("u"));

    assert_eq!(runner.text_input_buffer.as_deref(), Some("dru"));
    assert_eq!(
        runner.bridge.actions,
        vec![UiAction::SetFolderCreateInput {
            value: String::from("dru"),
        }]
    );
}

#[test]
fn r_hotkey_projects_folder_rename_draft_and_selects_all_text() {
    let bridge = ImmediateFolderCreateBridge {
        model: AppModel {
            focus_context: crate::app::FocusContextModel::SourceFolders,
            sources: SourcesPanelModel {
                focused_folder_row: Some(1),
                folder_rows: vec![
                    root_folder_row(),
                    crate::app::FolderRowModel::new("Drums", "", 1, false, true, false, true, true)
                        .with_source_index(1),
                ],
                ..SourcesPanelModel::default()
            },
            ..AppModel::default()
        },
        ..ImmediateFolderCreateBridge::default()
    };
    let mut runner = NativeVelloRunner::new(NativeRunOptions::default(), bridge);
    runner.model = runner.bridge.project_model();
    runner.frame_state.model_dirty = false;

    runner.handle_hotkey_press_for_tests(KeyCode::R);

    assert_eq!(runner.bridge.actions, vec![UiAction::StartFolderRename]);
    assert_eq!(runner.text_input_target, TextInputTarget::FolderCreate);
    assert!(matches!(
        runner
            .model
            .sources
            .folder_rows
            .iter()
            .find(|row| row.kind == crate::app::FolderRowKind::RenameDraft),
        Some(row) if row.input_value.as_deref() == Some("Drums")
    ));
    assert_eq!(
        runner
            .text_editor_state
            .as_ref()
            .map(|editor| editor.selection_range()),
        Some((0, "Drums".len()))
    );
}

#[test]
fn n_hotkey_projects_folder_create_draft_immediately() {
    let bridge = ImmediateFolderCreateBridge::with_root();
    let mut runner = NativeVelloRunner::new(NativeRunOptions::default(), bridge);
    runner.model = runner.bridge.project_model();
    runner.frame_state.model_dirty = false;

    runner.handle_hotkey_press_for_tests(KeyCode::N);

    assert_eq!(runner.bridge.actions, vec![UiAction::StartNewFolder]);
    assert_eq!(runner.text_input_target, TextInputTarget::FolderCreate);
    assert!(
        runner
            .model
            .sources
            .folder_rows
            .iter()
            .any(|row| row.kind == crate::app::FolderRowKind::CreateDraft)
    );
}

#[test]
fn enter_confirms_folder_create_and_refreshes_created_row_immediately() {
    let bridge = ImmediateFolderCreateBridge::with_root();
    let mut runner = NativeVelloRunner::new(NativeRunOptions::default(), bridge);
    runner.model = runner.bridge.project_model();
    runner.frame_state.model_dirty = false;

    runner.handle_hotkey_press_for_tests(KeyCode::N);
    assert!(runner.append_text("drums"));

    runner.handle_enter_for_tests();

    assert_eq!(
        runner.bridge.actions,
        vec![
            UiAction::StartNewFolder,
            UiAction::SetFolderCreateInput {
                value: String::from("drums"),
            },
            UiAction::ConfirmFolderCreate,
        ]
    );
    assert!(
        runner
            .model
            .sources
            .folder_rows
            .iter()
            .all(|row| row.kind != crate::app::FolderRowKind::CreateDraft)
    );
    assert!(
        runner
            .model
            .sources
            .folder_rows
            .iter()
            .any(|row| row.label == "drums")
    );
}

#[test]
fn enter_confirms_browser_duplicate_cleanup_when_browser_has_focus() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());
    runner.model = Arc::new(AppModel {
        focus_context: crate::app::FocusContextModel::SampleBrowser,
        browser: crate::app::BrowserPanelModel {
            duplicate_cleanup_active: true,
            ..crate::app::BrowserPanelModel::default()
        },
        ..AppModel::default()
    });
    runner.frame_state.model_dirty = false;

    runner.handle_enter_for_tests();

    assert_eq!(
        runner.bridge.actions,
        vec![UiAction::ConfirmBrowserDuplicateCleanup]
    );
}

#[test]
fn escape_cancels_folder_create_and_refreshes_model_immediately() {
    let bridge = ImmediateFolderCreateBridge::with_root();
    let mut runner = NativeVelloRunner::new(NativeRunOptions::default(), bridge);
    runner.model = runner.bridge.project_model();
    runner.frame_state.model_dirty = false;

    runner.handle_hotkey_press_for_tests(KeyCode::N);
    runner.handle_escape_for_tests();

    assert_eq!(
        runner.bridge.actions,
        vec![UiAction::StartNewFolder, UiAction::CancelFolderCreate]
    );
    assert!(
        runner
            .model
            .sources
            .folder_rows
            .iter()
            .all(|row| row.kind != crate::app::FolderRowKind::CreateDraft)
    );
    assert_eq!(runner.text_input_target, TextInputTarget::None);
}

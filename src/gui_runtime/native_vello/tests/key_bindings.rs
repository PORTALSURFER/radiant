use super::*;

fn resolved_action(key: KeyCode, modifiers: ModifiersState, model: &AppModel) -> Option<UiAction> {
    action_from_key(key, modifiers, model, None).action
}

#[test]
fn key_repeat_is_limited_to_plain_arrow_navigation_without_text_input() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());
    assert!(runner.allows_key_repeat(KeyCode::ArrowUp));
    assert!(runner.allows_key_repeat(KeyCode::ArrowDown));
    assert!(!runner.allows_key_repeat(KeyCode::Enter));

    runner.modifiers = ModifiersState::SHIFT;
    assert!(!runner.allows_key_repeat(KeyCode::ArrowDown));

    runner.modifiers = ModifiersState::default();
    runner.text_input_target = TextInputTarget::BrowserSearch;
    assert!(!runner.allows_key_repeat(KeyCode::ArrowDown));
}

#[test]
fn key_repeat_allows_shifted_arrow_steps_for_waveform_bpm_input() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());
    runner.text_input_target = TextInputTarget::WaveformBpm;
    runner.modifiers = ModifiersState::SHIFT;

    assert!(runner.allows_key_repeat(KeyCode::ArrowUp));
    assert!(runner.allows_key_repeat(KeyCode::ArrowDown));

    runner.modifiers = ModifiersState::SHIFT | ModifiersState::CONTROL;
    assert!(!runner.allows_key_repeat(KeyCode::ArrowUp));
}

#[test]
fn waveform_bpm_input_shift_arrow_steps_by_tenth() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());
    runner.text_input_target = TextInputTarget::WaveformBpm;
    runner.waveform_bpm_input_buffer = Some(String::from("120.0"));

    assert!(runner.step_waveform_bpm_input(1));
    assert_eq!(runner.waveform_bpm_input_buffer.as_deref(), Some("120.1"));
    assert_eq!(
        runner.bridge.actions,
        vec![UiAction::SetWaveformBpmValue { value_tenths: 1201 }]
    );
}

#[test]
fn g_prefix_routes_section_focus_commands() {
    let focus = AppModel::default();
    let first = action_from_key(KeyCode::G, ModifiersState::default(), &focus, None);
    assert_eq!(
        first.pending_chord,
        Some(crate::app::KeyPress::new(KeyCode::G))
    );
    assert!(first.action.is_none());

    let second = action_from_key(
        KeyCode::W,
        ModifiersState::default(),
        &focus,
        first.pending_chord,
    );
    assert_eq!(second.action, Some(UiAction::FocusWaveformPanel));
}

#[test]
fn explicit_focus_is_required_for_scope_specific_hotkeys() {
    let none = AppModel::default();
    assert_eq!(
        resolved_action(KeyCode::N, ModifiersState::default(), &none),
        None
    );
    assert_eq!(
        resolved_action(KeyCode::D, ModifiersState::default(), &none),
        None
    );
    assert_eq!(
        resolved_action(KeyCode::ArrowUp, ModifiersState::default(), &none),
        None
    );

    let browser = AppModel {
        focus_context: crate::app::FocusContextModel::SampleBrowser,
        ..AppModel::default()
    };
    assert_eq!(
        resolved_action(KeyCode::N, ModifiersState::default(), &browser),
        Some(UiAction::NormalizeFocusedBrowserSample)
    );
    assert_eq!(
        resolved_action(KeyCode::D, ModifiersState::default(), &browser),
        Some(UiAction::DeleteBrowserSelection)
    );
    assert_eq!(
        resolved_action(KeyCode::ArrowUp, ModifiersState::default(), &browser),
        Some(UiAction::MoveBrowserFocus { delta: -1 })
    );
    assert_eq!(
        resolved_action(KeyCode::ArrowDown, ModifiersState::default(), &browser),
        Some(UiAction::MoveBrowserFocus { delta: 1 })
    );
    assert_eq!(
        resolved_action(KeyCode::X, ModifiersState::default(), &browser),
        Some(UiAction::ToggleFocusedBrowserRowSelection)
    );

    let folders = AppModel {
        focus_context: crate::app::FocusContextModel::SourceFolders,
        ..AppModel::default()
    };
    assert_eq!(
        resolved_action(KeyCode::D, ModifiersState::default(), &folders),
        Some(UiAction::DeleteFocusedFolder)
    );
    assert_eq!(
        resolved_action(KeyCode::ArrowLeft, ModifiersState::default(), &folders),
        Some(UiAction::CollapseFocusedFolder)
    );
    assert_eq!(
        resolved_action(KeyCode::ArrowRight, ModifiersState::default(), &folders),
        Some(UiAction::ExpandFocusedFolder)
    );
    assert_eq!(
        resolved_action(KeyCode::ArrowUp, ModifiersState::default(), &folders),
        Some(UiAction::MoveFolderFocus { delta: -1 })
    );
    assert_eq!(
        resolved_action(KeyCode::ArrowDown, ModifiersState::default(), &folders),
        Some(UiAction::MoveFolderFocus { delta: 1 })
    );
    assert_eq!(
        resolved_action(KeyCode::X, ModifiersState::default(), &folders),
        Some(UiAction::ToggleFocusedFolderSelection)
    );

    let sources = AppModel {
        focus_context: crate::app::FocusContextModel::SourcesList,
        ..AppModel::default()
    };
    assert_eq!(
        resolved_action(KeyCode::R, ModifiersState::default(), &sources),
        Some(UiAction::ReloadFocusedSourceRow)
    );
}

#[test]
fn plain_s_routes_by_focus_between_browser_similarity_and_waveform_start_alignment() {
    let browser = AppModel {
        focus_context: crate::app::FocusContextModel::SampleBrowser,
        ..AppModel::default()
    };
    assert_eq!(
        resolved_action(KeyCode::S, ModifiersState::default(), &browser),
        Some(UiAction::ToggleFindSimilarFocusedSample)
    );

    let waveform = AppModel {
        focus_context: crate::app::FocusContextModel::Waveform,
        ..AppModel::default()
    };
    assert_eq!(
        resolved_action(KeyCode::S, ModifiersState::default(), &waveform),
        Some(UiAction::AlignWaveformStartToMarker)
    );
}

#[test]
fn waveform_hotkeys_resolve_by_focus_mode() {
    let waveform = AppModel {
        focus_context: crate::app::FocusContextModel::Waveform,
        ..AppModel::default()
    };
    assert_eq!(
        resolved_action(KeyCode::Enter, ModifiersState::default(), &waveform),
        Some(UiAction::CommitWaveformEditFades)
    );
    assert_eq!(
        resolved_action(KeyCode::E, ModifiersState::default(), &waveform),
        Some(UiAction::SaveWaveformSelectionToBrowser)
    );
    assert_eq!(
        resolved_action(KeyCode::E, ModifiersState::SHIFT, &waveform),
        Some(UiAction::SaveWaveformSelectionToBrowserWithKeep2)
    );
    assert_eq!(
        resolved_action(KeyCode::B, ModifiersState::default(), &waveform),
        Some(UiAction::ToggleBpmSnap)
    );
    assert_eq!(
        resolved_action(KeyCode::ArrowRight, ModifiersState::SHIFT, &waveform),
        Some(UiAction::SlideWaveformSelection {
            delta: 1,
            fine: true,
        })
    );
    assert_eq!(
        resolved_action(KeyCode::X, ModifiersState::default(), &waveform),
        Some(UiAction::ZoomWaveformFull)
    );
}

#[test]
fn text_input_targets_keep_plain_x_as_text_instead_of_selection_toggle() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());
    runner.frame_state.model_dirty = false;
    runner.model = Arc::new(AppModel {
        focus_context: crate::app::FocusContextModel::SampleBrowser,
        ..AppModel::default()
    });
    runner.text_input_target = TextInputTarget::BrowserSearch;
    runner.text_input_buffer = Some(String::from("dr"));
    runner.text_editor_state = Some(SingleLineTextEditorState::collapsed_at_end("dr"));

    runner.handle_character_key_for_tests(KeyCode::X, "x");

    assert_eq!(runner.text_input_buffer.as_deref(), Some("drx"));
    assert_eq!(
        runner.bridge.actions,
        vec![UiAction::SetBrowserSearch {
            query: String::from("drx"),
        }]
    );

    runner.model = Arc::new(AppModel {
        focus_context: crate::app::FocusContextModel::SourceFolders,
        ..AppModel::default()
    });
    runner.text_input_target = TextInputTarget::FolderSearch;
    runner.text_input_buffer = Some(String::from("ki"));
    runner.text_editor_state = Some(SingleLineTextEditorState::collapsed_at_end("ki"));

    runner.handle_character_key_for_tests(KeyCode::X, "x");

    assert_eq!(runner.text_input_buffer.as_deref(), Some("kix"));
    assert_eq!(
        runner.bridge.actions,
        vec![
            UiAction::SetBrowserSearch {
                query: String::from("drx"),
            },
            UiAction::SetFolderSearch {
                query: String::from("kix"),
            },
        ]
    );
}

#[test]
fn text_input_targets_consume_command_c_without_emitting_copy_selection_action() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());
    runner.frame_state.model_dirty = false;
    runner.model = Arc::new(AppModel {
        focus_context: crate::app::FocusContextModel::SampleBrowser,
        ..AppModel::default()
    });
    runner.text_input_target = TextInputTarget::BrowserSearch;
    runner.text_input_buffer = Some(String::from("drums"));
    runner.text_editor_state = Some(SingleLineTextEditorState::collapsed_at_end("drums"));
    runner.modifiers = ModifiersState::CONTROL;

    runner.handle_keyboard_input(winit::event::KeyEvent {
        physical_key: winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::KeyC),
        logical_key: winit::keyboard::Key::Character("c".into()),
        text: Some("c".into()),
        location: winit::keyboard::KeyLocation::Standard,
        state: winit::event::ElementState::Pressed,
        repeat: false,
        platform_specific: Default::default(),
    });

    assert!(runner.bridge.actions.is_empty());
    assert_eq!(runner.text_input_buffer.as_deref(), Some("drums"));
}

#[test]
fn folder_arrow_hotkeys_still_resolve_when_search_query_exists_but_tree_has_focus() {
    let mut folders = AppModel {
        focus_context: crate::app::FocusContextModel::SourceFolders,
        ..AppModel::default()
    };
    folders.sources.folder_search_query = String::from("dr");

    assert_eq!(
        resolved_action(KeyCode::ArrowLeft, ModifiersState::default(), &folders),
        Some(UiAction::CollapseFocusedFolder)
    );
    assert_eq!(
        resolved_action(KeyCode::ArrowRight, ModifiersState::default(), &folders),
        Some(UiAction::ExpandFocusedFolder)
    );
    assert_eq!(
        resolved_action(KeyCode::ArrowUp, ModifiersState::default(), &folders),
        Some(UiAction::MoveFolderFocus { delta: -1 })
    );
    assert_eq!(
        resolved_action(KeyCode::ArrowDown, ModifiersState::default(), &folders),
        Some(UiAction::MoveFolderFocus { delta: 1 })
    );
}

#[test]
fn key_bindings_respect_progress_cancelability_and_playback_shortcuts() {
    let mut model = AppModel::default();
    assert_eq!(
        resolved_action(KeyCode::P, ModifiersState::default(), &model),
        None
    );

    model.progress_overlay.cancelable = true;
    assert_eq!(
        resolved_action(KeyCode::P, ModifiersState::default(), &model),
        Some(UiAction::CancelProgress)
    );
    assert_eq!(
        resolved_action(KeyCode::Space, ModifiersState::default(), &model),
        Some(UiAction::PlayFromStart)
    );
}

#[test]
fn clicking_browser_search_field_focuses_text_input() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut shell_state = NativeShellState::new();
    let model = AppModel {
        focus_context: crate::app::FocusContextModel::SampleBrowser,
        ..AppModel::default()
    };
    let search_field = shell_state
        .browser_search_field_rect(&layout, &model)
        .expect("browser search field should be present");
    let point = Point::new(
        (search_field.min.x + search_field.max.x) * 0.5,
        (search_field.min.y + search_field.max.y) * 0.5,
    );

    assert_eq!(
        action_from_pointer(
            &layout,
            &model,
            &mut shell_state,
            point,
            ModifiersState::default(),
        ),
        Some(UiAction::FocusBrowserSearch)
    );
}

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

#[derive(Default)]
struct ImmediateFolderCreateBridge {
    actions: Vec<UiAction>,
    model: AppModel,
    project_calls: usize,
}

impl ImmediateFolderCreateBridge {
    fn with_root() -> Self {
        Self {
            model: AppModel {
                focus_context: crate::app::FocusContextModel::SourceFolders,
                sources: SourcesPanelModel {
                    folder_rows: vec![root_folder_row()],
                    ..SourcesPanelModel::default()
                },
                ..AppModel::default()
            },
            ..Self::default()
        }
    }

    fn set_draft(&mut self, value: String) {
        self.set_inline_draft(value, false);
    }

    fn set_inline_draft(&mut self, value: String, rename: bool) {
        let draft = if rename {
            crate::app::FolderRowModel::rename_draft(
                1,
                value.clone(),
                String::from("Folder name"),
                folder_create_error(&value),
                true,
            )
        } else {
            crate::app::FolderRowModel::create_draft(
                1,
                value.clone(),
                String::from("New folder name"),
                folder_create_error(&value),
                true,
            )
        };
        self.model.sources.folder_rows = vec![root_folder_row(), draft];
    }

    fn clear_draft(&mut self) {
        self.model.sources.folder_rows.retain(|row| row.is_root);
    }

    fn add_created_folder(&mut self, value: String) {
        self.model.sources.folder_rows = vec![
            root_folder_row(),
            crate::app::FolderRowModel::new(
                value.clone(),
                value,
                1,
                false,
                true,
                false,
                false,
                false,
            )
            .with_source_index(1),
        ];
    }
}

impl NativeAppBridge for ImmediateFolderCreateBridge {
    fn project_model(&mut self) -> Arc<AppModel> {
        self.project_calls = self.project_calls.saturating_add(1);
        Arc::new(self.model.clone())
    }

    fn reduce_action(&mut self, action: UiAction) {
        match &action {
            UiAction::StartNewFolder
            | UiAction::StartNewFolderAtFolderRow { .. }
            | UiAction::StartNewFolderAtRoot => self.set_draft(String::new()),
            UiAction::StartFolderRename => {
                let value = self
                    .model
                    .sources
                    .focused_folder_row
                    .and_then(|index| self.model.sources.folder_rows.get(index))
                    .map(|row| row.label.clone())
                    .unwrap_or_default();
                self.set_inline_draft(value, true);
            }
            UiAction::SetFolderCreateInput { value } => self.set_draft(value.clone()),
            UiAction::ConfirmFolderCreate => {
                let value = self
                    .model
                    .sources
                    .folder_rows
                    .iter()
                    .find(|row| row.kind == crate::app::FolderRowKind::CreateDraft)
                    .and_then(|row| row.input_value.clone())
                    .map(|value| value.trim().to_string())
                    .unwrap_or_default();
                if !value.is_empty() {
                    self.add_created_folder(value);
                }
            }
            UiAction::CancelFolderCreate => self.clear_draft(),
            _ => {}
        }
        self.actions.push(action);
    }
}

fn root_folder_row() -> crate::app::FolderRowModel {
    crate::app::FolderRowModel::new("Root", "", 0, false, false, true, true, true)
        .with_source_index(0)
}

fn folder_create_error(value: &str) -> Option<String> {
    value
        .trim()
        .is_empty()
        .then(|| String::from("Folder name cannot be empty"))
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

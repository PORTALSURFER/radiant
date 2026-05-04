use super::*;
use crate::compat_app_contract::FolderPaneIdModel;

#[test]
fn text_input_targets_keep_plain_x_as_text_instead_of_selection_toggle() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());
    runner.frame_state.model_dirty = false;
    runner.model = Arc::new(AppModel {
        focus_context: crate::compat_app_contract::FocusContextModel::ContentList,
        ..AppModel::default()
    });
    runner.text_input_target = TextInputTarget::BrowserSearch;
    runner.text_input_buffer = Some(String::from("it"));
    runner.text_editor_state = Some(SingleLineTextEditorState::collapsed_at_end("it"));

    runner.handle_character_key_for_tests(KeyCode::X, "x");

    assert_eq!(runner.text_input_buffer.as_deref(), Some("itx"));
    assert_eq!(
        runner.bridge.actions,
        vec![UiAction::SetContentSearch {
            query: String::from("itx"),
        }]
    );

    runner.model = Arc::new(AppModel {
        focus_context: crate::compat_app_contract::FocusContextModel::NavigationTree,
        ..AppModel::default()
    });
    runner.text_input_target = TextInputTarget::FolderSearch;
    runner.text_input_buffer = Some(String::from("gr"));
    runner.text_editor_state = Some(SingleLineTextEditorState::collapsed_at_end("gr"));

    runner.handle_character_key_for_tests(KeyCode::X, "x");

    assert_eq!(runner.text_input_buffer.as_deref(), Some("grx"));
    assert_eq!(
        runner.bridge.actions,
        vec![
            UiAction::SetContentSearch {
                query: String::from("itx"),
            },
            UiAction::SetFolderSearch {
                pane: Some(FolderPaneIdModel::Upper),
                query: String::from("grx"),
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
        focus_context: crate::compat_app_contract::FocusContextModel::ContentList,
        ..AppModel::default()
    });
    runner.text_input_target = TextInputTarget::BrowserSearch;
    runner.text_input_buffer = Some(String::from("query"));
    runner.text_editor_state = Some(SingleLineTextEditorState::collapsed_at_end("query"));
    runner.modifiers = ModifiersState::CONTROL;

    runner.handle_character_key_for_tests(KeyCode::C, "c");

    assert!(runner.bridge.actions.is_empty());
    assert_eq!(runner.text_input_buffer.as_deref(), Some("query"));
}

#[test]
fn clicking_browser_search_field_focuses_text_input() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut shell_state = NativeShellState::new();
    let model = AppModel {
        focus_context: crate::compat_app_contract::FocusContextModel::ContentList,
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
        Some(UiAction::FocusContentSearch)
    );
}

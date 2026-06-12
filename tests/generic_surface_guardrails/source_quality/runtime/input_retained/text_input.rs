use super::*;

#[test]
fn text_input_state_keeps_models_selection_navigation_and_editing_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let model = fs::read_to_string(manifest_dir.join("src/widgets/primitives/text_input/model.rs"))
        .expect("text input model root should be readable");
    let selection = fs::read_to_string(
        manifest_dir.join("src/widgets/primitives/text_input/model/selection.rs"),
    )
    .expect("text input selection model should be readable");
    let navigation = fs::read_to_string(
        manifest_dir.join("src/widgets/primitives/text_input/model/navigation.rs"),
    )
    .expect("text input navigation model should be readable");
    let word_boundary = fs::read_to_string(
        manifest_dir.join("src/widgets/primitives/text_input/model/word_boundary.rs"),
    )
    .expect("text input word-boundary helpers should be readable");
    let editing =
        fs::read_to_string(manifest_dir.join("src/widgets/primitives/text_input/model/editing.rs"))
            .expect("text input editing model should be readable");
    let widget_contract = fs::read_to_string(manifest_dir.join("src/widgets/contract/widget.rs"))
        .expect("widget contract should be readable");
    let focus_controller = fs::read_to_string(manifest_dir.join("src/runtime/controller/focus.rs"))
        .expect("runtime focus controller should be readable");
    let editing_command = fs::read_to_string(
        manifest_dir.join("src/widgets/primitives/text_input/model/editing/command.rs"),
    )
    .expect("text input edit command model should be readable");
    let editing_mutation = fs::read_to_string(
        manifest_dir.join("src/widgets/primitives/text_input/model/editing/mutation.rs"),
    )
    .expect("text input edit mutation model should be readable");
    let tests = fs::read_to_string(manifest_dir.join("src/widgets/primitives/text_input/tests.rs"))
        .expect("text input behavior test root should be readable");
    let widget_tests =
        fs::read_to_string(manifest_dir.join("src/widgets/primitives/text_input/tests/widget.rs"))
            .expect("text input widget interaction tests should be readable");
    let state_tests =
        fs::read_to_string(manifest_dir.join("src/widgets/primitives/text_input/tests/state.rs"))
            .expect("text input state behavior tests should be readable");
    let interaction_input =
        fs::read_to_string(manifest_dir.join("src/widgets/interaction/input.rs"))
            .expect("widget interaction input contract should be readable");
    let interaction_input_event =
        fs::read_to_string(manifest_dir.join("src/widgets/interaction/input/event.rs"))
            .expect("widget interaction input event contract should be readable");
    let text_edit_input =
        fs::read_to_string(manifest_dir.join("src/widgets/interaction/input/text_edit.rs"))
            .expect("widget text edit input contract should be readable");
    let native_text_edit = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/keyboard/text_edit.rs"),
    )
    .expect("native text edit keyboard routing should be readable");

    for required in [
        "mod editing;",
        "mod navigation;",
        "mod selection;",
        "mod word_boundary;",
    ] {
        assert!(
            model.contains(required),
            "text input model root should delegate `{required}`"
        );
    }
    assert!(
        model.contains("pub struct TextInputProps")
            && model.contains("pub struct TextInputState")
            && model.contains("pub struct TextInputEditResult")
            && model.contains("pub fn from_value")
            && !model.contains("TextEditCommand")
            && !model.contains("WidgetKey"),
        "text input model root should keep public state definitions separate from command handling"
    );
    assert!(
        selection.contains("pub fn selected_text")
            && selection.contains("pub fn selected_text_slice")
            && selection.contains("pub fn selection_range")
            && selection.contains("pub fn has_selection")
            && selection.contains("pub fn select_word_at")
            && selection.contains("word_range_at"),
        "text input selection queries should live in model/selection.rs"
    );
    assert!(
        widget_contract.contains("fn selected_text_slice(&self) -> Option<&str>")
            && widget_contract.contains("self.selected_text_slice().map(str::to_owned)")
            && focus_controller
                .contains("pub fn focused_text_selection_slice(&self) -> Option<&str>")
            && focus_controller.contains("focused_text_selection_slice().map(str::to_owned)"),
        "focused text selection inspection should preserve a borrowed path through the widget and runtime contracts"
    );
    assert!(
        navigation.contains("pub fn set_caret")
            && navigation.contains("fn move_left")
            && navigation.contains("fn move_right")
            && navigation.contains("fn move_word_left")
            && navigation.contains("fn move_word_right")
            && word_boundary.contains("pub(super) fn previous_word_boundary")
            && word_boundary.contains("pub(super) fn next_word_boundary")
            && word_boundary.contains("pub(super) fn word_range_at")
            && word_boundary.contains("pub(super) fn is_word_char")
            && word_boundary.contains("active_word_start")
            && !word_boundary.contains("Vec<char>")
            && !word_boundary.contains("collect::<")
            && !navigation.contains("fn is_word_char"),
        "text input caret movement should live in model/navigation.rs and word selection should avoid collecting the whole input"
    );
    assert!(
        editing.contains("mod command;")
            && editing.contains("mod mutation;")
            && !editing.contains("pub fn apply_edit_command")
            && !editing.contains("pub fn insert_text"),
        "text input editing root should delegate command dispatch and mutation mechanics"
    );
    assert!(
        editing_command.contains("pub fn apply_edit_command")
            && editing_command.contains("pub fn apply_key")
            && editing_command.contains("TextEditCommand")
            && editing_command.contains("TextEditCommand::MoveWordLeft")
            && editing_command.contains("TextEditCommand::MoveWordRight")
            && editing_command.contains("TextEditCommand::DeleteWordLeft")
            && editing_command.contains("TextEditCommand::DeleteWordRight")
            && editing_command.contains("WidgetKey")
            && !editing_mutation.contains("TextEditCommand")
            && !editing_mutation.contains("WidgetKey"),
        "text input edit command handling should live in model/editing/command.rs"
    );
    assert!(
        interaction_input.contains("pub use text_edit::TextEditCommand;")
            && interaction_input_event.contains("TextEdit(TextEditCommand)")
            && text_edit_input.contains("MoveWordLeft")
            && text_edit_input.contains("MoveWordRight")
            && text_edit_input.contains("DeleteWordLeft")
            && text_edit_input.contains("DeleteWordRight")
            && native_text_edit.contains("let word_navigation =")
            && native_text_edit.contains("TextEditCommand::MoveWordLeft")
            && native_text_edit.contains("TextEditCommand::MoveWordRight")
            && native_text_edit.contains("TextEditCommand::DeleteWordLeft")
            && native_text_edit.contains("TextEditCommand::DeleteWordRight"),
        "backend-neutral word navigation and deletion should be exposed by TextEditCommand and routed by the native adapter"
    );
    assert!(
        editing_mutation.contains("pub fn insert_text")
            && editing_mutation.contains("pub fn replace_selection")
            && editing_mutation.contains("pub(crate) fn delete_selected_text")
            && editing_mutation.contains("byte_index_for_char")
            && !editing_command.contains("byte_index_for_char"),
        "text input mutation mechanics should live in model/editing/mutation.rs"
    );
    assert!(
        tests.contains("mod widget;")
            && tests.contains("mod state;")
            && !tests.contains("fn text_input_editing_emits_changed_and_submitted_messages")
            && !tests.contains("fn text_input_state_applies_backend_neutral_editing_commands"),
        "text input behavior test root should index focused widget and state groups instead of owning all cases"
    );
    assert!(
        widget_tests.contains("fn text_input_editing_emits_changed_and_submitted_messages")
            && widget_tests
                .contains("fn text_input_pointer_drag_extends_selection_including_caret_character")
            && state_tests.contains("fn text_input_state_applies_backend_neutral_editing_commands")
            && state_tests.contains("fn text_input_state_exposes_borrowed_selected_text_slice")
            && state_tests.contains("fn text_input_state_can_clear_or_delete_active_selection")
            && state_tests.contains("fn text_input_state_moves_by_word_boundaries")
            && state_tests.contains("fn text_input_state_extends_selection_by_word_boundaries")
            && state_tests.contains("fn text_input_state_deletes_by_word_boundaries")
            && state_tests.contains("fn text_input_state_word_delete_removes_selection_first")
            && state_tests.contains("fn text_input_state_selects_word_at_character_index")
            && widget_tests.contains("fn text_input_double_click_selects_word_under_pointer"),
        "text input behavior tests should stay grouped by widget interaction and state editing concerns"
    );
}

#[test]
fn text_input_primitive_keeps_surface_builders_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/widgets/primitives/text_input.rs"))
        .expect("text-input primitive root should be readable");
    let builders =
        fs::read_to_string(manifest_dir.join("src/widgets/primitives/text_input/builders.rs"))
            .expect("text-input primitive builders should be readable");

    assert!(
        root.contains("mod builders;")
            && root.contains("pub struct TextInputWidget")
            && root.contains("impl Widget for TextInputWidget")
            && !root.contains("impl<Message> SurfaceNode<Message>")
            && !root.contains("impl<Message> WidgetMessageMapper<Message>"),
        "text-input primitive root should own widget behavior and delegate runtime builders"
    );
    assert!(
        builders.contains("impl<Message> SurfaceNode<Message>")
            && builders.contains("pub fn text_input(")
            && builders.contains("pub fn text_input_mapped(")
            && builders.contains("impl<Message> WidgetMessageMapper<Message>"),
        "text-input runtime builder helpers should live in text_input/builders.rs"
    );
}

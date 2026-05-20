use crate::widgets::interaction::TextEditCommand;

use super::super::TextInputState;

#[test]
fn text_input_state_applies_backend_neutral_editing_commands() {
    let mut state = TextInputState::from_value(String::from("alpha beta"));

    let result = state.apply_edit_command(
        TextEditCommand::MoveHome {
            extend_selection: false,
        },
        None,
    );
    assert!(!result.value_changed);
    assert!(result.selection_changed);

    for _ in 0..4 {
        let _ = state.apply_edit_command(
            TextEditCommand::MoveRight {
                extend_selection: true,
            },
            None,
        );
    }
    assert_eq!(state.selected_text().as_deref(), Some("alpha"));

    let result =
        state.apply_edit_command(TextEditCommand::InsertText(String::from("one\ntwo")), None);
    assert!(result.value_changed);
    assert!(result.selection_changed);
    assert_eq!(state.value, "onetwo beta");
    assert_eq!(state.caret, 6);
    assert_eq!(state.selection_anchor, 6);
}

#[test]
fn text_input_state_honors_character_limit_after_selection_replacement() {
    let mut state = TextInputState::from_value(String::from("abcd"));
    state.selection_anchor = 1;
    state.caret = 2;

    let result = state.insert_text("xyz", Some(4));

    assert!(result.value_changed);
    assert_eq!(state.value, "axyd");
    assert_eq!(state.caret, 3);
    assert_eq!(state.selection_anchor, 3);
}

#[test]
fn text_input_state_exposes_selection_replacement_helpers() {
    let mut state = TextInputState::from_value(String::from("alpha beta"));
    state.selection_anchor = 0;
    state.caret = 4;

    assert!(state.has_selection());
    assert_eq!(state.selected_text().as_deref(), Some("alpha"));

    let result = state.replace_selection("one\ntwo", None);

    assert!(result.value_changed);
    assert!(result.selection_changed);
    assert_eq!(state.value, "onetwo beta");
    assert_eq!(state.caret, 6);
    assert!(!state.has_selection());
}

#[test]
fn text_input_state_exposes_borrowed_selected_text_slice() {
    let mut state = TextInputState::from_value(String::from("aé日 beta"));
    state.selection_anchor = 1;
    state.caret = 2;

    assert_eq!(state.selected_text_slice(), Some("é日"));
    assert_eq!(state.selected_text().as_deref(), Some("é日"));

    state.clear_selection();

    assert_eq!(state.selected_text_slice(), None);
}

#[test]
fn text_input_state_can_clear_or_delete_active_selection() {
    let mut state = TextInputState::from_value(String::from("abcd"));
    state.selection_anchor = 1;
    state.caret = 2;

    state.clear_selection();

    assert!(!state.has_selection());
    assert_eq!(state.selection_range(), (2, 2));

    state.selection_anchor = 1;
    state.caret = 2;
    let result = state.delete_selection();

    assert!(result.value_changed);
    assert_eq!(state.value, "ad");
    assert_eq!(state.caret, 1);
    assert!(!state.has_selection());
}

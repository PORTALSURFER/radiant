use crate::widgets::interaction::TextEditCommand;

use super::super::TextInputState;

// OPT-857 text-input conformance checklist.
//
// Covered today:
// - single-line scalar caret movement and selection extension/collapse
// - word movement and word deletion
// - Home/End movement with and without selection extension
// - selection deletion and replacement
// - sanitized insertion, including newline stripping
// - character-limit replacement semantics
// - double-click word selection through widget tests
// - focused runtime keyboard routing through native runtime tests
//
// Explicitly not covered because Radiant does not yet expose the feature:
// - multiline Up/Down layout-aware navigation
// - undo/redo grouping
// - password masking mode
// - platform IME composition and bidirectional shaping behavior

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

    for _ in 0..5 {
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
fn text_input_state_home_end_and_collapse_follow_single_line_contract() {
    let mut state = TextInputState::from_value(String::from("alpha beta"));

    let result = state.apply_edit_command(
        TextEditCommand::MoveHome {
            extend_selection: true,
        },
        None,
    );

    assert!(!result.value_changed);
    assert!(result.selection_changed);
    assert_eq!(state.selected_text_slice(), Some("alpha beta"));

    let result = state.apply_edit_command(
        TextEditCommand::MoveRight {
            extend_selection: false,
        },
        None,
    );

    assert!(!result.value_changed);
    assert!(result.selection_changed);
    assert_eq!(state.caret, state.char_len());
    assert!(!state.has_selection());

    let result = state.apply_edit_command(
        TextEditCommand::MoveEnd {
            extend_selection: true,
        },
        None,
    );

    assert!(!result.value_changed);
    assert!(result.selection_changed);
    assert_eq!(state.selected_text_slice(), None);
}

#[test]
fn text_input_state_honors_character_limit_after_selection_replacement() {
    let mut state = TextInputState::from_value(String::from("abcd"));
    state.selection_anchor = 1;
    state.caret = 3;

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
    state.caret = 5;

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
    state.caret = 3;

    assert_eq!(state.selected_text_slice(), Some("é日"));
    assert_eq!(state.selected_text().as_deref(), Some("é日"));

    state.clear_selection();

    assert_eq!(state.selected_text_slice(), None);
}

#[test]
fn text_input_state_can_clear_or_delete_active_selection() {
    let mut state = TextInputState::from_value(String::from("abcd"));
    state.selection_anchor = 1;
    state.caret = 3;

    state.clear_selection();

    assert!(!state.has_selection());
    assert_eq!(state.selection_range(), (3, 3));

    state.selection_anchor = 1;
    state.caret = 3;
    let result = state.delete_selection();

    assert!(result.value_changed);
    assert_eq!(state.value, "ad");
    assert_eq!(state.caret, 1);
    assert!(!state.has_selection());
}

#[test]
fn text_input_state_moves_by_word_boundaries() {
    let mut state = TextInputState::from_value(String::from("alpha  beta_gamma.日文"));

    let result = state.apply_edit_command(
        TextEditCommand::MoveWordLeft {
            extend_selection: false,
        },
        None,
    );

    assert!(!result.value_changed);
    assert!(result.selection_changed);
    assert_eq!(state.caret, 18);

    let _ = state.apply_edit_command(
        TextEditCommand::MoveWordLeft {
            extend_selection: false,
        },
        None,
    );

    assert_eq!(state.caret, 7);

    let _ = state.apply_edit_command(
        TextEditCommand::MoveWordRight {
            extend_selection: false,
        },
        None,
    );

    assert_eq!(state.caret, 17);
}

#[test]
fn text_input_state_extends_selection_by_word_boundaries() {
    let mut state = TextInputState::from_value(String::from("alpha beta gamma"));
    state.set_caret(0, false);

    let _ = state.apply_edit_command(
        TextEditCommand::MoveWordRight {
            extend_selection: true,
        },
        None,
    );

    assert_eq!(state.selected_text_slice(), Some("alpha"));

    state.set_caret(state.char_len(), false);
    let _ = state.apply_edit_command(
        TextEditCommand::MoveWordLeft {
            extend_selection: true,
        },
        None,
    );

    assert_eq!(state.selected_text_slice(), Some("gamma"));
}

#[test]
fn text_input_state_moves_and_extends_word_selection_over_combining_graphemes() {
    let mut state = TextInputState::from_value(String::from("e\u{301} next"));
    state.caret = 1;
    state.selection_anchor = 1;

    let result = state.apply_edit_command(
        TextEditCommand::MoveWordLeft {
            extend_selection: false,
        },
        None,
    );
    assert!(!result.value_changed);
    assert_eq!(state.caret, 0);

    state.set_caret(0, false);
    let result = state.apply_edit_command(
        TextEditCommand::MoveWordRight {
            extend_selection: true,
        },
        None,
    );
    assert!(!result.value_changed);
    assert_eq!(state.caret, 2);
    assert_eq!(state.selection_range(), (0, 2));
    assert_eq!(state.selected_text_slice(), Some("e\u{301}"));

    let result = state.apply_edit_command(
        TextEditCommand::MoveWordRight {
            extend_selection: true,
        },
        None,
    );
    assert!(!result.value_changed);
    assert_eq!(state.caret, state.char_len());
    assert_eq!(state.selection_range(), (0, state.char_len()));
    assert_eq!(state.selected_text_slice(), Some("e\u{301} next"));
}

#[test]
fn text_input_state_deletes_by_word_boundaries() {
    let mut state = TextInputState::from_value(String::from("alpha  beta_gamma.日文"));
    state.set_caret(17, false);

    let result = state.apply_edit_command(TextEditCommand::DeleteWordLeft, None);

    assert!(result.value_changed);
    assert_eq!(state.value, "alpha  .日文");
    assert_eq!(state.caret, 7);
    assert!(!state.has_selection());

    let result = state.apply_edit_command(TextEditCommand::DeleteWordRight, None);

    assert!(result.value_changed);
    assert_eq!(state.value, "alpha  ");
    assert_eq!(state.caret, 7);
}

#[test]
fn text_input_state_word_delete_removes_selection_first() {
    let mut state = TextInputState::from_value(String::from("alpha beta gamma"));
    state.selection_anchor = 6;
    state.caret = 10;

    let result = state.apply_edit_command(TextEditCommand::DeleteWordLeft, None);

    assert!(result.value_changed);
    assert_eq!(state.value, "alpha  gamma");
    assert_eq!(state.caret, 6);
    assert!(!state.has_selection());
}

#[test]
fn text_input_state_word_deletion_canonicalizes_combining_carets() {
    let mut left = TextInputState::from_value(String::from("e\u{301} next"));
    left.caret = 1;
    left.selection_anchor = 1;
    let result = left.apply_edit_command(TextEditCommand::DeleteWordLeft, None);

    assert!(result.value_changed);
    assert_eq!(left.value, " next");
    assert_eq!(left.caret, 0);
    assert_eq!(left.selection_anchor, 0);

    let mut right_start = TextInputState::from_value(String::from("e\u{301} next"));
    right_start.caret = 0;
    right_start.selection_anchor = 0;
    let result = right_start.apply_edit_command(TextEditCommand::DeleteWordRight, None);

    assert!(result.value_changed);
    assert_eq!(right_start.value, " next");
    assert_eq!(right_start.caret, 0);
    assert_eq!(right_start.selection_anchor, 0);

    let mut right = TextInputState::from_value(String::from("e\u{301} next"));
    right.caret = 1;
    right.selection_anchor = 1;
    let result = right.apply_edit_command(TextEditCommand::DeleteWordRight, None);

    assert!(result.value_changed);
    assert_eq!(right.value, "e\u{301}");
    assert_eq!(right.caret, 2);
    assert_eq!(right.selection_anchor, 2);

    let mut stale = TextInputState::from_value(String::from("e\u{301}"));
    stale.caret = usize::MAX;
    stale.selection_anchor = usize::MAX;
    let result = stale.apply_edit_command(TextEditCommand::DeleteWordLeft, None);

    assert!(result.value_changed);
    assert!(stale.value.is_empty());
    assert_eq!(stale.caret, 0);
    assert_eq!(stale.selection_anchor, 0);
}

#[test]
fn text_input_state_word_deletion_keeps_zwj_and_non_bmp_words_whole() {
    let mut state = TextInputState::from_value(String::from(
        "क्\u{200d}ष👩\u{200d}\u{1f52c}\u{10400}\u{301}",
    ));
    let non_bmp_start = "क्\u{200d}ष👩\u{200d}\u{1f52c}".chars().count();
    state.caret = state.char_len();
    state.selection_anchor = state.caret;

    let result = state.apply_edit_command(TextEditCommand::DeleteWordLeft, None);

    assert!(result.value_changed);
    assert_eq!(state.value, "क्\u{200d}ष👩\u{200d}\u{1f52c}");
    assert_eq!(state.caret, non_bmp_start);

    state.caret = non_bmp_start;
    state.selection_anchor = non_bmp_start;
    let result = state.apply_edit_command(TextEditCommand::DeleteWordLeft, None);

    assert!(result.value_changed);
    assert!(state.value.is_empty());
    assert_eq!(state.caret, 0);
}

#[test]
fn text_input_state_selects_word_at_character_index() {
    let mut state = TextInputState::from_value(String::from("alpha  beta_gamma.日文"));

    assert!(state.select_word_at(9));
    assert_eq!(state.selected_text_slice(), Some("beta_gamma"));

    assert!(state.select_word_at(19));
    assert_eq!(state.selected_text_slice(), Some("日文"));

    assert!(!state.select_word_at(6));
    assert!(!state.has_selection());
    assert_eq!(state.caret, 6);
}

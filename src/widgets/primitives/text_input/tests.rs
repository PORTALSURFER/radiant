use crate::gui::types::{Point, Rect, Vector2};
use crate::widgets::interaction::{
    PointerButton, TextEditCommand, TextInputMessage, WidgetInput, WidgetKey,
};

use super::{TextInputState, TextInputWidget, WidgetSizing};

#[test]
fn text_input_editing_emits_changed_and_submitted_messages() {
    let mut input = TextInputWidget::new(
        7,
        "ab",
        WidgetSizing::new(Vector2::new(100.0, 28.0), Vector2::new(160.0, 28.0)),
    );
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(160.0, 28.0));
    let _ = input.handle_input(bounds, WidgetInput::FocusChanged(true));
    input.state.caret = 1;
    input.state.selection_anchor = 1;

    assert_eq!(
        input.handle_input(bounds, WidgetInput::Character('z')),
        Some(TextInputMessage::Changed {
            value: String::from("azb"),
        })
    );
    assert_eq!(input.state.caret, 2);

    assert_eq!(
        input.handle_input(bounds, WidgetInput::KeyPress(WidgetKey::Backspace)),
        Some(TextInputMessage::Changed {
            value: String::from("ab"),
        })
    );

    assert_eq!(
        input.handle_input(bounds, WidgetInput::KeyPress(WidgetKey::Enter)),
        Some(TextInputMessage::Submitted {
            value: String::from("ab"),
        })
    );
}

#[test]
fn text_input_selection_replaces_cuts_and_pastes_text() {
    let mut input = TextInputWidget::new(
        7,
        "alpha beta",
        WidgetSizing::new(Vector2::new(100.0, 28.0), Vector2::new(160.0, 28.0)),
    );
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(160.0, 28.0));
    let _ = input.handle_input(bounds, WidgetInput::FocusChanged(true));

    let _ = input.handle_input(
        bounds,
        WidgetInput::TextEdit(TextEditCommand::MoveHome {
            extend_selection: false,
        }),
    );
    for _ in 0..4 {
        let _ = input.handle_input(
            bounds,
            WidgetInput::TextEdit(TextEditCommand::MoveRight {
                extend_selection: true,
            }),
        );
    }

    assert_eq!(input.selected_text().as_deref(), Some("alpha"));
    assert_eq!(
        input.handle_input(
            bounds,
            WidgetInput::TextEdit(TextEditCommand::InsertText(String::from("one\ntwo"))),
        ),
        Some(TextInputMessage::Changed {
            value: String::from("onetwo beta"),
        })
    );

    let _ = input.handle_input(bounds, WidgetInput::TextEdit(TextEditCommand::SelectAll));
    assert_eq!(input.selected_text().as_deref(), Some("onetwo beta"));
    assert_eq!(
        input.handle_input(bounds, WidgetInput::TextEdit(TextEditCommand::CutSelection)),
        Some(TextInputMessage::Changed {
            value: String::new(),
        })
    );
}

#[test]
fn text_input_pointer_drag_extends_selection_including_caret_character() {
    let mut input = TextInputWidget::new(
        7,
        "abcdef",
        WidgetSizing::new(Vector2::new(100.0, 42.0), Vector2::new(180.0, 42.0)),
    );
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(180.0, 42.0));

    assert_eq!(
        input.handle_input(
            bounds,
            WidgetInput::PointerPress {
                position: Point::new(26.0, 20.0),
                button: PointerButton::Primary,
                modifiers: Default::default(),
            },
        ),
        None
    );
    assert_eq!(input.state.caret, 1);
    assert_eq!(
        input.handle_input(
            bounds,
            WidgetInput::PointerMove {
                position: Point::new(43.0, 20.0),
            },
        ),
        None
    );
    assert_eq!(input.state.caret, 3);
    assert_eq!(input.selected_text().as_deref(), Some("bcd"));
    assert_eq!(
        input.handle_input(
            bounds,
            WidgetInput::PointerRelease {
                position: Point::new(43.0, 20.0),
                button: PointerButton::Primary,
                modifiers: Default::default(),
            },
        ),
        None
    );
    assert!(!input.common.state.pressed);
}

#[test]
fn text_input_selection_range_clamps_stale_public_state() {
    let mut input = TextInputWidget::new(
        7,
        "abc",
        WidgetSizing::new(Vector2::new(100.0, 28.0), Vector2::new(160.0, 28.0)),
    );
    input.state.selection_anchor = usize::MAX;
    input.state.caret = 1;

    assert_eq!(input.selection_range(), (1, 3));
    assert_eq!(input.selected_text().as_deref(), Some("bc"));

    input.state.selection_anchor = 9;
    input.state.caret = 7;

    assert_eq!(input.selection_range(), (3, 3));
    assert_eq!(input.selected_text(), None);
}

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

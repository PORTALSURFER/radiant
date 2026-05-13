use crate::gui::types::{Point, Rect, Vector2};
use crate::widgets::interaction::{
    PointerButton, TextEditCommand, TextInputMessage, WidgetInput, WidgetKey,
};

use super::{TextInputWidget, WidgetSizing};

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

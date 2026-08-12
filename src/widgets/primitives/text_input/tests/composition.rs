use crate::gui::types::{Rect, Vector2};
use crate::layout::LayoutOutput;
use crate::runtime::PaintPrimitive;
use crate::theme::ThemeTokens;
use crate::widgets::contract::{Widget, WidgetSizing};
use crate::widgets::interaction::{
    CompositionRange, CompositionSample, CompositionSelectionState, TextInputMessage, WidgetInput,
    WidgetKey,
};

use super::super::TextInputWidget;

fn text_input(value: &str) -> TextInputWidget {
    TextInputWidget::new(
        7,
        value,
        WidgetSizing::new(Vector2::new(100.0, 28.0), Vector2::new(180.0, 28.0)),
    )
}

fn scalar_range(start: usize, end: usize, scalar_len: usize) -> CompositionRange {
    CompositionRange::new(start, end, scalar_len).expect("valid scalar range")
}

fn start(
    replacement: (usize, usize),
    selection: (usize, usize),
    scalar_len: usize,
) -> CompositionSample {
    CompositionSample::start(
        scalar_range(replacement.0, replacement.1, scalar_len),
        scalar_range(selection.0, selection.1, scalar_len),
    )
    .expect("valid composition start")
}

fn update(preedit: &str, selection: (usize, usize)) -> CompositionSample {
    CompositionSample::update(
        preedit,
        scalar_range(selection.0, selection.1, preedit.chars().count()),
    )
    .expect("valid composition update")
}

fn dispatch(input: &mut TextInputWidget, sample: CompositionSample) -> Option<TextInputMessage> {
    input
        .handle_composition_sample(sample)
        .and_then(|output| output.typed_cloned::<TextInputMessage>())
}

fn dispatch_hidden(input: &mut TextInputWidget, preedit: &str) -> Option<TextInputMessage> {
    input
        .handle_hidden_composition_update(preedit.to_owned(), None)
        .and_then(|output| output.typed_cloned::<TextInputMessage>())
}

fn focus(input: &mut TextInputWidget) {
    assert_eq!(
        input.handle_input(Rect::default(), WidgetInput::FocusChanged(true)),
        None
    );
}

#[test]
fn text_input_composition_replaces_preedit_and_commits_once() {
    let mut input = text_input("a");
    focus(&mut input);

    assert_eq!(dispatch(&mut input, start((0, 1), (0, 1), 1)), None);
    assert_eq!(dispatch(&mut input, update("あ", (1, 1))), None);
    assert_eq!(input.state.value, "あ");
    assert_eq!(input.state.selection_range(), (1, 1));

    assert_eq!(dispatch(&mut input, update("あい", (1, 2))), None);
    assert_eq!(input.state.value, "あい");
    assert_eq!(
        input.composition_preedit_selection(),
        Some(scalar_range(1, 2, 2))
    );

    assert_eq!(
        dispatch(&mut input, CompositionSample::commit("愛")),
        Some(TextInputMessage::Changed {
            value: String::from("愛"),
        })
    );
    assert_eq!(input.state.value, "愛");
    assert_eq!(input.state.selection_range(), (1, 1));
    assert!(!input.retains_managed_composition());
}

#[test]
fn text_input_composition_supports_empty_preedit_and_cancel_restores_selection() {
    let mut input = text_input("ab");
    focus(&mut input);

    assert_eq!(dispatch(&mut input, start((0, 2), (0, 2), 2)), None);
    assert_eq!(dispatch(&mut input, update("", (0, 0))), None);
    assert_eq!(input.state.value, "");
    assert_eq!(input.state.selection_range(), (0, 0));

    assert_eq!(dispatch(&mut input, CompositionSample::cancel()), None);
    assert_eq!(input.state.value, "ab");
    assert_eq!(input.state.selection_range(), (0, 2));
    assert!(!input.retains_managed_composition());
}

#[test]
fn text_input_composition_keeps_hidden_native_selection_absent() {
    let mut input = text_input("a");
    focus(&mut input);

    assert_eq!(dispatch(&mut input, start((0, 1), (0, 1), 1)), None);
    assert_eq!(
        input.composition_preedit_selection_state(),
        CompositionSelectionState::Unreported
    );
    assert_eq!(dispatch_hidden(&mut input, "あ"), None);
    assert_eq!(input.state.value, "あ");
    assert_eq!(input.composition_preedit_selection(), None);
    assert_eq!(
        input.composition_preedit_selection_state(),
        CompositionSelectionState::Hidden
    );

    assert_eq!(dispatch(&mut input, update("あい", (1, 1))), None);
    assert_eq!(
        input.composition_preedit_selection(),
        Some(scalar_range(1, 1, 2))
    );
    assert_eq!(
        input.composition_preedit_selection_state(),
        CompositionSelectionState::Visible(scalar_range(1, 1, 2))
    );
}

#[test]
fn hidden_composition_keeps_focus_but_zeroes_adornment_colors_until_visible_update() {
    let mut input = text_input("a");
    focus(&mut input);
    assert_eq!(dispatch(&mut input, start((0, 1), (0, 1), 1)), None);
    assert_eq!(dispatch(&mut input, update("あい", (0, 1))), None);

    let bounds = Rect::from_min_size(Default::default(), Vector2::new(180.0, 28.0));
    let paint = |input: &TextInputWidget| {
        let mut primitives = Vec::new();
        input.append_paint(
            &mut primitives,
            bounds,
            &LayoutOutput::default(),
            &ThemeTokens::default(),
        );
        primitives
            .into_iter()
            .find_map(|primitive| match primitive {
                PaintPrimitive::TextInput(input) => Some(input),
                _ => None,
            })
            .expect("text input should emit a paint primitive")
    };

    let visible = paint(&input);
    assert!(visible.focused);
    assert_ne!(visible.selection_color.a, 0);
    assert_ne!(visible.caret_color.a, 0);

    assert_eq!(dispatch_hidden(&mut input, "隠"), None);
    let hidden = paint(&input);
    assert!(hidden.focused);
    assert_eq!(hidden.selection_color.a, 0);
    assert_eq!(hidden.caret_color.a, 0);

    assert_eq!(dispatch(&mut input, update("隠れ", (1, 1))), None);
    let visible_again = paint(&input);
    assert!(visible_again.focused);
    assert_ne!(visible_again.selection_color.a, 0);
    assert_ne!(visible_again.caret_color.a, 0);
    assert_eq!(input.state.selection_range(), (1, 1));
}

#[test]
fn text_input_composition_allows_direct_commit_and_focus_loss_cancels() {
    let mut direct = text_input("ab");
    focus(&mut direct);
    assert_eq!(dispatch(&mut direct, start((1, 2), (1, 2), 2)), None);
    assert_eq!(
        dispatch(&mut direct, CompositionSample::commit("界")),
        Some(TextInputMessage::Changed {
            value: String::from("a界"),
        })
    );

    let mut focused = text_input("ab");
    focus(&mut focused);
    assert_eq!(dispatch(&mut focused, start((0, 2), (0, 2), 2)), None);
    assert_eq!(dispatch(&mut focused, update("あ", (1, 1))), None);
    assert_eq!(
        focused.handle_input(Rect::default(), WidgetInput::FocusChanged(false)),
        None
    );
    assert_eq!(focused.state.value, "ab");
    assert_eq!(focused.state.selection_range(), (0, 2));
    assert_eq!(
        dispatch(&mut focused, CompositionSample::commit("late")),
        None
    );
}

#[test]
fn text_input_composition_commits_empty_text_by_replacing_the_captured_range() {
    let mut input = text_input("abc");
    focus(&mut input);
    assert_eq!(dispatch(&mut input, start((1, 2), (1, 2), 3)), None);

    assert_eq!(
        dispatch(&mut input, CompositionSample::commit("")),
        Some(TextInputMessage::Changed {
            value: String::from("ac"),
        })
    );
    assert_eq!(input.state.value, "ac");
    assert_eq!(input.state.selection_range(), (1, 1));
}

#[test]
fn text_input_composition_commit_honors_scalar_limit_and_single_line_sanitization() {
    let mut limited = text_input("ab");
    limited.props.character_limit = Some(3);
    focus(&mut limited);
    assert_eq!(dispatch(&mut limited, start((0, 1), (0, 1), 2)), None);

    assert_eq!(
        dispatch(&mut limited, CompositionSample::commit("界文語")),
        Some(TextInputMessage::Changed {
            value: String::from("界文b"),
        })
    );
    assert_eq!(limited.state.value, "界文b");
    assert_eq!(limited.state.selection_range(), (2, 2));

    let mut sanitized = text_input("ab");
    focus(&mut sanitized);
    assert_eq!(dispatch(&mut sanitized, start((0, 1), (0, 1), 2)), None);

    assert_eq!(
        dispatch(
            &mut sanitized,
            CompositionSample::commit("x\r\ny\t\u{0000}z"),
        ),
        Some(TextInputMessage::Changed {
            value: String::from("xy zb"),
        })
    );
    assert_eq!(sanitized.state.value, "xy zb");
    assert_eq!(sanitized.state.selection_range(), (4, 4));
}

#[test]
fn text_input_composition_routes_nonmatching_keys_while_active() {
    let mut input = text_input("ab");
    focus(&mut input);
    assert_eq!(dispatch(&mut input, start((0, 1), (0, 1), 2)), None);
    assert_eq!(dispatch(&mut input, update("あ", (1, 1))), None);

    assert_eq!(
        input.handle_input(
            Rect::default(),
            WidgetInput::key_press(WidgetKey::ArrowLeft),
        ),
        None
    );
    assert_eq!(input.state.value, "あb");
    assert_eq!(input.state.selection_range(), (0, 0));
    assert!(input.retains_managed_composition());
}

#[test]
fn text_input_composition_preserves_compatible_reprojection_and_rejects_stale_start() {
    let mut previous = text_input("a");
    focus(&mut previous);
    assert_eq!(dispatch(&mut previous, start((0, 1), (0, 1), 1)), None);
    assert_eq!(dispatch(&mut previous, update("あ", (1, 1))), None);

    let mut current = text_input("a");
    focus(&mut current);
    current.synchronize_from_previous(&previous);
    assert_eq!(current.state.value, "あ");
    assert!(current.retains_managed_composition());
    assert_eq!(
        dispatch(&mut current, CompositionSample::commit("愛")),
        Some(TextInputMessage::Changed {
            value: String::from("愛"),
        },)
    );

    let mut stale = text_input("a");
    focus(&mut stale);
    assert_eq!(dispatch(&mut stale, start((0, 1), (0, 1), 2)), None);
    assert_eq!(stale.state.value, "a");
    assert!(!stale.retains_managed_composition());
}

use super::*;
use crate::gui::types::Rect;
use crate::widgets::interaction::NumericInteractionOwner;
use crate::widgets::{
    CompositionRange, CompositionSample, EditPhase, TextEditCommand, Widget, WidgetInput, WidgetKey,
};

fn scalar_range(start: usize, end: usize, scalar_len: usize) -> CompositionRange {
    CompositionRange::new(start, end, scalar_len).expect("composition range should be valid")
}

fn start(range: (usize, usize), scalar_len: usize) -> CompositionSample {
    start_with_selection(range, range, scalar_len)
}

fn start_with_selection(
    replacement: (usize, usize),
    selection: (usize, usize),
    scalar_len: usize,
) -> CompositionSample {
    CompositionSample::start(
        scalar_range(replacement.0, replacement.1, scalar_len),
        scalar_range(selection.0, selection.1, scalar_len),
    )
    .expect("composition start should be valid")
}

fn update(preedit: &str, selection: (usize, usize)) -> CompositionSample {
    CompositionSample::update(
        preedit,
        scalar_range(selection.0, selection.1, preedit.chars().count()),
    )
    .expect("composition update should be valid")
}

fn dispatch(
    input: &mut NumericInputWidget<u32, U32Codec, U32Adjustment>,
    sample: CompositionSample,
) -> Option<NumericInputEditBatch<u32>> {
    Widget::handle_composition_sample(input, sample).and_then(|output| output.typed_cloned())
}

#[test]
fn numeric_composition_keeps_preedit_local_and_does_not_parse_or_publish() {
    let (mut input, parse_calls) = super::u32_input_with_parse_calls();
    super::focus(&mut input);

    assert_eq!(dispatch(&mut input, start((0, 1), 1)), None);
    assert_eq!(
        input.interaction_gate.incumbent(),
        Some(NumericInteractionOwner::ImeComposition)
    );
    assert!(input.retains_managed_composition());

    assert_eq!(dispatch(&mut input, update("12", (1, 1))), None);
    assert_eq!(input.text_input.state.value, "12");
    assert_eq!(input.value, 7);
    assert_eq!(
        input.active.as_ref().map(|active| active.session.draft()),
        Some("7")
    );
    assert_eq!(parse_calls.get(), 0);
}

#[test]
fn numeric_composition_commit_reuses_text_sanitization_and_commits_once() {
    let mut input = super::u32_input();
    input.text_input.props.character_limit = Some(2);
    super::focus(&mut input);

    assert_eq!(dispatch(&mut input, start((0, 1), 1)), None);
    let batch = dispatch(&mut input, CompositionSample::commit("123\r\n"))
        .expect("valid committed numeric composition should emit one batch");

    assert_eq!(batch.events().len(), 2);
    assert_eq!(batch.events()[0].phase, EditPhase::Begin);
    assert_eq!(batch.events()[1].phase, EditPhase::Commit);
    assert_eq!(batch.events()[1].value, 12);
    assert_eq!(input.value, 12);
    assert_eq!(input.text_input.state.value, "12");
    assert!(!input.retains_managed_composition());
    assert_eq!(input.interaction_gate.incumbent(), None);
}

#[test]
fn invalid_composition_commit_becomes_correctable_text_edit() {
    let mut input = super::u32_input();
    super::focus(&mut input);

    assert_eq!(dispatch(&mut input, start((0, 1), 1)), None);
    assert_eq!(dispatch(&mut input, CompositionSample::commit("-")), None);
    assert_eq!(input.value, 7);
    assert_eq!(input.text_input.state.value, "-");
    assert!(!input.retains_managed_composition());
    assert_eq!(
        input.interaction_gate.incumbent(),
        Some(NumericInteractionOwner::TextEdit)
    );

    assert!(
        Widget::handle_input(
            &mut input,
            Rect::default(),
            WidgetInput::text_edit(TextEditCommand::SelectAll),
        )
        .is_none()
    );
    assert!(
        Widget::handle_input(
            &mut input,
            Rect::default(),
            WidgetInput::text_edit(TextEditCommand::InsertText(String::from("8"))),
        )
        .is_none()
    );
    assert_eq!(input.text_input.state.value, "8");

    let batch: NumericInputEditBatch<u32> = Widget::handle_input(
        &mut input,
        Rect::default(),
        WidgetInput::key_press(WidgetKey::Enter),
    )
    .and_then(|output| output.typed_cloned())
    .expect("corrected draft should commit");
    assert_eq!(batch.events()[1].phase, EditPhase::Commit);
    assert_eq!(batch.events()[1].value, 8);
    assert_eq!(input.value, 8);
}

#[test]
fn numeric_composition_cancel_and_focus_loss_restore_the_captured_selection() {
    let mut cancelled = super::u32_input();
    super::focus(&mut cancelled);
    assert_eq!(
        dispatch(&mut cancelled, start_with_selection((0, 1), (1, 1), 1),),
        None
    );
    assert_eq!(dispatch(&mut cancelled, update("12", (2, 2))), None);
    let batch = dispatch(&mut cancelled, CompositionSample::cancel())
        .expect("composition cancel should emit the numeric cancel lifecycle");
    super::assert_cancel_batch(&batch);
    assert_eq!(cancelled.value, 7);
    assert_eq!(cancelled.text_input.state.value, "7");
    assert_eq!(cancelled.text_input.state.selection_range(), (1, 1));
    assert_eq!(cancelled.interaction_gate.incumbent(), None);

    let mut focus_lost = super::u32_input();
    super::focus(&mut focus_lost);
    assert_eq!(
        dispatch(&mut focus_lost, start_with_selection((0, 1), (1, 1), 1),),
        None
    );
    assert_eq!(dispatch(&mut focus_lost, update("-", (1, 1))), None);
    assert!(
        Widget::handle_input(
            &mut focus_lost,
            Rect::default(),
            WidgetInput::FocusChanged(false),
        )
        .and_then(|output| output.typed_cloned())
        .inspect(|batch| {
            super::assert_cancel_batch(batch);
        })
        .is_some()
    );
    assert!(!focus_lost.text_input.common.state.focused);
    assert_eq!(focus_lost.text_input.state.value, "7");
    assert!(!focus_lost.retains_managed_composition());
}

#[test]
fn numeric_composition_keeps_nonmatching_keys_routable() {
    let mut input = super::u32_input();
    super::focus(&mut input);
    assert_eq!(dispatch(&mut input, start((0, 1), 1)), None);
    assert_eq!(dispatch(&mut input, update("12", (2, 2))), None);

    assert!(
        Widget::handle_input(
            &mut input,
            Rect::default(),
            WidgetInput::key_press(WidgetKey::ArrowLeft),
        )
        .is_none()
    );
    assert_eq!(input.text_input.state.selection_range(), (1, 1));
    assert!(input.retains_managed_composition());
}

#[test]
fn numeric_composition_survives_compatible_reprojection_and_cancels_incompatible_replacement() {
    let mut previous = super::u32_input();
    super::focus(&mut previous);
    assert_eq!(dispatch(&mut previous, start((0, 1), 1)), None);
    assert_eq!(dispatch(&mut previous, update("12", (2, 2))), None);

    let mut current = super::u32_input();
    super::focus(&mut current);
    current.synchronize_from_previous(&previous);
    assert_eq!(current.text_input.state.value, "12");
    assert!(current.retains_managed_composition());
    let batch = dispatch(&mut current, CompositionSample::commit("8"))
        .expect("compatible reprojection should retain the composition owner");
    assert_eq!(batch.events()[1].phase, EditPhase::Commit);
    assert_eq!(batch.events()[1].value, 8);

    let mut retiring = super::u32_input();
    super::focus(&mut retiring);
    assert_eq!(dispatch(&mut retiring, start((0, 1), 1)), None);
    assert_eq!(dispatch(&mut retiring, update("12", (2, 2))), None);
    let successor = super::u32_input_with_value(8);
    let cancelled = retiring
        .prepare_replacement(Some(&successor))
        .and_then(|output| output.typed_cloned())
        .expect("incompatible replacement should cancel composition");
    super::assert_cancel_batch(&cancelled);
    assert!(!retiring.retains_managed_composition());
    assert_eq!(retiring.text_input.state.value, "7");
}

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

fn dispatch_hidden(
    input: &mut NumericInputWidget<u32, U32Codec, U32Adjustment>,
    preedit: &str,
) -> Option<NumericInputEditBatch<u32>> {
    Widget::handle_hidden_composition_update(input, preedit.to_owned(), None)
        .and_then(|output| output.typed_cloned())
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
fn numeric_composition_keeps_hidden_native_selection_absent() {
    let mut input = super::u32_input();
    super::focus(&mut input);

    assert_eq!(dispatch(&mut input, start((0, 1), 1)), None);
    assert_eq!(
        input
            .composition
            .as_ref()
            .map(|composition| composition.preedit_selection),
        Some(CompositionSelectionState::Unreported)
    );
    assert_eq!(dispatch_hidden(&mut input, "12"), None);
    assert_eq!(input.text_input.state.value, "12");
    assert_eq!(
        input
            .composition
            .as_ref()
            .map(|composition| composition.preedit_selection),
        Some(CompositionSelectionState::Hidden)
    );
}

#[test]
fn numeric_hidden_composition_suppresses_embedded_text_input_adornments() {
    let mut input = super::u32_input();
    super::focus(&mut input);
    assert_eq!(dispatch(&mut input, start((0, 1), 1)), None);
    assert_eq!(dispatch(&mut input, update("12", (0, 1))), None);

    let bounds = Rect::from_min_size(
        Default::default(),
        crate::gui::types::Vector2::new(180.0, 28.0),
    );
    let paint = |input: &NumericInputWidget<u32, U32Codec, U32Adjustment>| {
        let mut primitives = Vec::new();
        Widget::append_paint(
            input,
            &mut primitives,
            bounds,
            &crate::layout::LayoutOutput::default(),
            &crate::theme::ThemeTokens::default(),
        );
        primitives
            .into_iter()
            .find_map(|primitive| match primitive {
                crate::runtime::PaintPrimitive::TextInput(input) => Some(input),
                _ => None,
            })
            .expect("numeric input should emit an embedded text input paint primitive")
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
    assert_ne!(visible_again.selection_color.a, 0);
    assert_ne!(visible_again.caret_color.a, 0);
}

#[test]
fn numeric_hidden_composition_context_paint_scales_and_preserves_owner_state() {
    let mut input = super::u32_input();
    super::focus(&mut input);
    assert_eq!(dispatch(&mut input, start((0, 1), 1)), None);
    assert_eq!(dispatch(&mut input, update("12", (0, 1))), None);

    let environment =
        crate::runtime::ResolvedEnvironment::from_snapshots(
            crate::runtime::WindowEnvironment::default(),
            std::sync::Arc::new(
                crate::application::ApplicationEnvironment::new(
                    crate::application::LocaleId::english(),
                )
                .with_text_scale(
                    crate::application::TextScale::new(1.5)
                        .expect("composition test scale should be valid"),
                ),
            ),
        );
    let bounds = Rect::from_min_size(
        crate::gui::types::Point::new(10.0, 20.0),
        crate::gui::types::Vector2::new(300.0, 60.0),
    );
    let layout = crate::layout::LayoutOutput::default();
    let theme = crate::theme::ThemeTokens::default();
    let state_before = input.text_input.state.clone();
    let composition_before = input.composition.clone();
    let owner_before = input.interaction_gate.incumbent();

    let paint = |input: &NumericInputWidget<u32, U32Codec, U32Adjustment>| {
        let mut primitives = Vec::new();
        let mut context = crate::widgets::WidgetPaintContext::new(
            &mut primitives,
            bounds,
            &layout,
            &theme,
            &environment,
        );
        Widget::append_paint_with_context(input, &mut context);
        primitives
            .into_iter()
            .find_map(|primitive| match primitive {
                crate::runtime::PaintPrimitive::TextInput(input) => Some(input),
                _ => None,
            })
            .expect("numeric context paint should emit text input")
    };

    let visible = paint(&input);
    assert_eq!(visible.font_size, 19.5);
    assert_eq!(visible.rect.min.x, 22.0);
    assert_eq!(visible.rect.min.y, 23.0);
    assert_eq!(visible.align, crate::runtime::PaintTextAlign::Left);
    assert_ne!(visible.selection_color.a, 0);
    assert_ne!(visible.caret_color.a, 0);
    assert_eq!(input.text_input.state, state_before);
    assert_eq!(input.composition, composition_before);
    assert_eq!(input.interaction_gate.incumbent(), owner_before);

    assert_eq!(dispatch_hidden(&mut input, "hidden"), None);
    let hidden = paint(&input);
    assert_eq!(hidden.font_size, 19.5);
    assert_eq!(hidden.rect.min.x, 22.0);
    assert_eq!(hidden.rect.min.y, 23.0);
    assert_eq!(hidden.align, crate::runtime::PaintTextAlign::Left);
    assert_eq!(hidden.selection_color.a, 0);
    assert_eq!(hidden.caret_color.a, 0);
    assert_eq!(input.interaction_gate.incumbent(), owner_before);
    assert_eq!(input.text_input.state.value, "hidden");
    assert_eq!(input.text_input.state.caret, state_before.caret);
    assert_eq!(
        input.text_input.state.selection_anchor,
        state_before.selection_anchor
    );
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

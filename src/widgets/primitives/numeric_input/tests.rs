use super::*;
use crate::widgets::interaction::NumericInteractionOwner;
use crate::{
    gui::{
        input::InputTimestamp,
        types::{Point, Vector2},
    },
    widgets::{
        EditPhase, InteractionSource, KeyboardModifier, KeyboardModifiers, NumericStep,
        NumericStepDirection, NumericStepModifiers, PointerModifiers, TextEditCommand,
    },
};
use std::{cell::Cell, fmt, rc::Rc};

#[derive(Debug, PartialEq)]
struct CodecError;

#[derive(Debug, PartialEq)]
struct AdjustmentError;

struct U32Codec {
    format_calls: Rc<Cell<usize>>,
    parse_calls: Rc<Cell<usize>>,
    fail_format: bool,
}

impl NumericCodec<u32> for U32Codec {
    type Error = CodecError;

    fn parse(&self, text: &str) -> NumericParseResult<u32> {
        self.parse_calls.set(self.parse_calls.get() + 1);
        if text.is_empty() || text == "-" {
            return NumericParseResult::Incomplete;
        }
        if text == "invalid" {
            return NumericParseResult::Invalid;
        }
        let Ok(value) = text.parse::<u32>() else {
            return NumericParseResult::Invalid;
        };
        if value <= 100 {
            NumericParseResult::Valid(value)
        } else {
            NumericParseResult::OutOfRange
        }
    }

    fn format_editable(&self, value: &u32, output: &mut dyn fmt::Write) -> Result<(), Self::Error> {
        self.format_calls.set(self.format_calls.get() + 1);
        if self.fail_format {
            return Err(CodecError);
        }
        write!(output, "{value}").map_err(|_| CodecError)
    }
}

struct U32Adjustment {
    inverse_calls: Rc<Cell<usize>>,
    step_calls: Rc<Cell<usize>>,
    fail_inverse: bool,
}

impl NumericAdjustment<u32> for U32Adjustment {
    type Error = AdjustmentError;

    fn normalized_to_value(&self, normalized: f32) -> Result<u32, Self::Error> {
        Ok((normalized * 100.0).round() as u32)
    }

    fn value_to_normalized(&self, value: &u32) -> Result<f32, Self::Error> {
        self.inverse_calls.set(self.inverse_calls.get() + 1);
        if self.fail_inverse {
            return Err(AdjustmentError);
        }
        Ok(*value as f32 / 100.0)
    }

    fn step(
        &self,
        value: &u32,
        direction: NumericStepDirection,
        step: NumericStep,
    ) -> Result<u32, Self::Error> {
        self.step_calls.set(self.step_calls.get() + 1);
        let amount = match step {
            NumericStep::Base => 1,
            NumericStep::Fine => 1,
            NumericStep::Coarse => 10,
        };
        Ok(match direction {
            NumericStepDirection::Decrease => value.saturating_sub(amount),
            NumericStepDirection::Increase => value.saturating_add(amount),
        })
    }

    fn scrub(
        &self,
        value: &u32,
        normalized_delta: f32,
        _step: NumericStep,
    ) -> Result<u32, Self::Error> {
        Ok(value.saturating_add(normalized_delta.max(0.0) as u32))
    }

    fn wheel(&self, value: &u32, delta: f32, _step: NumericStep) -> Result<u32, Self::Error> {
        Ok(value.saturating_add(delta.max(0.0) as u32))
    }
}

#[derive(Clone, Debug, PartialEq)]
struct Percent(f32);

struct PercentCodec;

impl NumericCodec<Percent> for PercentCodec {
    type Error = CodecError;

    fn parse(&self, text: &str) -> NumericParseResult<Percent> {
        text.parse::<f32>()
            .ok()
            .filter(|value| (0.0..=1.0).contains(value))
            .map_or(NumericParseResult::Invalid, |value| {
                NumericParseResult::Valid(Percent(value))
            })
    }

    fn format_editable(
        &self,
        value: &Percent,
        output: &mut dyn fmt::Write,
    ) -> Result<(), Self::Error> {
        write!(output, "{}", value.0).map_err(|_| CodecError)
    }
}

struct PercentAdjustment;

impl NumericAdjustment<Percent> for PercentAdjustment {
    type Error = AdjustmentError;

    fn normalized_to_value(&self, normalized: f32) -> Result<Percent, Self::Error> {
        Ok(Percent(normalized))
    }

    fn value_to_normalized(&self, value: &Percent) -> Result<f32, Self::Error> {
        Ok(value.0)
    }

    fn step(
        &self,
        value: &Percent,
        _direction: NumericStepDirection,
        _step: NumericStep,
    ) -> Result<Percent, Self::Error> {
        Ok(value.clone())
    }

    fn scrub(
        &self,
        value: &Percent,
        _normalized_delta: f32,
        _step: NumericStep,
    ) -> Result<Percent, Self::Error> {
        Ok(value.clone())
    }

    fn wheel(
        &self,
        value: &Percent,
        _delta: f32,
        _step: NumericStep,
    ) -> Result<Percent, Self::Error> {
        Ok(value.clone())
    }
}

#[derive(Clone, Debug, PartialEq)]
struct FrequencyHz(u32);

struct FrequencyCodec;

impl NumericCodec<FrequencyHz> for FrequencyCodec {
    type Error = CodecError;

    fn parse(&self, text: &str) -> NumericParseResult<FrequencyHz> {
        let Some(value) = text.strip_suffix(" Hz") else {
            return NumericParseResult::Invalid;
        };
        value
            .parse::<u32>()
            .ok()
            .filter(|value| (20..=20_000).contains(value))
            .map_or(NumericParseResult::OutOfRange, |value| {
                NumericParseResult::Valid(FrequencyHz(value))
            })
    }

    fn format_editable(
        &self,
        value: &FrequencyHz,
        output: &mut dyn fmt::Write,
    ) -> Result<(), Self::Error> {
        write!(output, "{} Hz", value.0).map_err(|_| CodecError)
    }
}

struct FrequencyAdjustment;

impl NumericAdjustment<FrequencyHz> for FrequencyAdjustment {
    type Error = AdjustmentError;

    fn normalized_to_value(&self, normalized: f32) -> Result<FrequencyHz, Self::Error> {
        Ok(FrequencyHz((20.0 + normalized * 19_980.0) as u32))
    }

    fn value_to_normalized(&self, value: &FrequencyHz) -> Result<f32, Self::Error> {
        Ok((value.0.saturating_sub(20) as f32) / 19_980.0)
    }

    fn step(
        &self,
        value: &FrequencyHz,
        _direction: NumericStepDirection,
        _step: NumericStep,
    ) -> Result<FrequencyHz, Self::Error> {
        Ok(value.clone())
    }

    fn scrub(
        &self,
        value: &FrequencyHz,
        _normalized_delta: f32,
        _step: NumericStep,
    ) -> Result<FrequencyHz, Self::Error> {
        Ok(value.clone())
    }

    fn wheel(
        &self,
        value: &FrequencyHz,
        _delta: f32,
        _step: NumericStep,
    ) -> Result<FrequencyHz, Self::Error> {
        Ok(value.clone())
    }
}

fn u32_input() -> NumericInputWidget<u32, U32Codec, U32Adjustment> {
    u32_input_with_parse_calls().0
}

fn u32_input_with_parse_calls() -> (
    NumericInputWidget<u32, U32Codec, U32Adjustment>,
    Rc<Cell<usize>>,
) {
    let parse_calls = Rc::new(Cell::new(0));
    NumericInputWidget::try_new(
        7,
        U32Codec {
            format_calls: Rc::new(Cell::new(0)),
            parse_calls: Rc::clone(&parse_calls),
            fail_format: false,
        },
        U32Adjustment {
            inverse_calls: Rc::new(Cell::new(0)),
            step_calls: Rc::new(Cell::new(0)),
            fail_inverse: false,
        },
        WidgetSizing::fixed(Vector2::new(120.0, 28.0)),
    )
    .map(|input| (input, parse_calls))
    .expect("u32 fixture should construct")
}

fn u32_input_with_step_calls() -> (
    NumericInputWidget<u32, U32Codec, U32Adjustment>,
    Rc<Cell<usize>>,
) {
    let step_calls = Rc::new(Cell::new(0));
    let input = NumericInputWidget::try_new(
        7,
        U32Codec {
            format_calls: Rc::new(Cell::new(0)),
            parse_calls: Rc::new(Cell::new(0)),
            fail_format: false,
        },
        U32Adjustment {
            inverse_calls: Rc::new(Cell::new(0)),
            step_calls: Rc::clone(&step_calls),
            fail_inverse: false,
        },
        WidgetSizing::fixed(Vector2::new(120.0, 28.0)),
    )
    .expect("u32 fixture should construct");
    (input, step_calls)
}

fn focus<T, C, A>(input: &mut NumericInputWidget<T, C, A>)
where
    T: Clone + PartialEq + 'static,
    C: NumericCodec<T> + 'static,
    A: NumericAdjustment<T> + 'static,
{
    assert!(
        Widget::handle_input(input, Rect::default(), WidgetInput::FocusChanged(true)).is_none()
    );
}

fn output<T: Clone + 'static>(
    output: Option<crate::widgets::WidgetOutput>,
) -> Option<NumericInputEditBatch<T>> {
    output.and_then(|output| output.typed_cloned())
}

fn replace_u32(input: &mut NumericInputWidget<u32, U32Codec, U32Adjustment>, text: &str) {
    focus(input);
    assert!(
        Widget::handle_input(
            input,
            Rect::default(),
            WidgetInput::text_edit(TextEditCommand::SelectAll),
        )
        .is_none()
    );
    assert!(
        Widget::handle_input(
            input,
            Rect::default(),
            WidgetInput::text_edit(TextEditCommand::InsertText(text.to_owned())),
        )
        .is_none()
    );
}

#[test]
fn construction_formats_generic_private_fixtures_and_validates_inverse() {
    let percent = NumericInputWidget::try_new(
        Percent(0.5),
        PercentCodec,
        PercentAdjustment,
        WidgetSizing::fixed(Vector2::new(120.0, 28.0)),
    )
    .expect("percent fixture should construct");
    assert_eq!(percent.text_input.state.value, "0.5");

    let frequency = NumericInputWidget::try_new(
        FrequencyHz(440),
        FrequencyCodec,
        FrequencyAdjustment,
        WidgetSizing::fixed(Vector2::new(120.0, 28.0)),
    )
    .expect("frequency fixture should construct");
    assert_eq!(frequency.text_input.state.value, "440 Hz");
}

#[test]
fn step_modifier_configuration_is_stored_and_cloned_without_consumption() {
    let mut input = u32_input();
    assert_eq!(input.step_modifiers, None);

    let policy = NumericStepModifiers::new(KeyboardModifier::Alt, KeyboardModifier::Control);
    input.set_step_modifiers(policy);
    assert_eq!(input.step_modifiers, Some(policy));
    assert_eq!(input.clone().step_modifiers, Some(policy));
}

#[test]
fn arrow_key_samples_remain_no_ops_with_default_and_override_policies() {
    let policies = [
        None,
        Some(NumericStepModifiers::MACOS_DEFAULT),
        Some(NumericStepModifiers::WINDOWS_LINUX_DEFAULT),
        Some(NumericStepModifiers::new(
            KeyboardModifier::Alt,
            KeyboardModifier::Control,
        )),
    ];

    for policy in policies {
        let (mut input, step_calls) = u32_input_with_step_calls();
        if let Some(policy) = policy {
            input.set_step_modifiers(policy);
        }
        assert_eq!(input.step_modifiers, policy);
        focus(&mut input);
        let before_text = input.text_input.state.clone();
        let before_focus = input.text_input.common.state;

        for (key, modifiers) in [
            (WidgetKey::ArrowUp, KeyboardModifiers::default()),
            (
                WidgetKey::ArrowDown,
                KeyboardModifiers {
                    command: true,
                    control: true,
                    shift: true,
                    alt: true,
                },
            ),
        ] {
            assert!(
                Widget::handle_input(
                    &mut input,
                    Rect::default(),
                    WidgetInput::KeyPress {
                        key,
                        modifiers,
                        repeat: false,
                        timestamp: None,
                    },
                )
                .is_none()
            );
            assert!(
                Widget::handle_input(
                    &mut input,
                    Rect::default(),
                    WidgetInput::KeyPress {
                        key,
                        modifiers,
                        repeat: true,
                        timestamp: None,
                    },
                )
                .is_none()
            );
            assert!(
                Widget::handle_input(
                    &mut input,
                    Rect::default(),
                    WidgetInput::KeyRelease {
                        key,
                        modifiers,
                        timestamp: None,
                    },
                )
                .is_none()
            );
        }

        assert_eq!(step_calls.get(), 0);
        assert_eq!(input.value, 7);
        assert_eq!(input.text_input.state, before_text);
        assert_eq!(input.text_input.common.state, before_focus);
        assert!(input.active.is_none());
        assert_eq!(input.interaction_gate.incumbent(), None);
    }
}

#[test]
fn construction_reports_codec_and_adjustment_failures_without_fallbacks() {
    let format_calls = Rc::new(Cell::new(0));
    let format_error = NumericInputWidget::try_new(
        7,
        U32Codec {
            format_calls: Rc::clone(&format_calls),
            parse_calls: Rc::new(Cell::new(0)),
            fail_format: true,
        },
        U32Adjustment {
            inverse_calls: Rc::new(Cell::new(0)),
            step_calls: Rc::new(Cell::new(0)),
            fail_inverse: false,
        },
        WidgetSizing::fixed(Vector2::new(120.0, 28.0)),
    )
    .expect_err("format failure should be explicit");
    assert_eq!(
        format_error,
        NumericInputConstructionError::CodecFormat { error: CodecError }
    );
    assert_eq!(format_calls.get(), 1);

    let inverse_calls = Rc::new(Cell::new(0));
    let inverse_error = NumericInputWidget::try_new(
        7,
        U32Codec {
            format_calls: Rc::new(Cell::new(0)),
            parse_calls: Rc::new(Cell::new(0)),
            fail_format: false,
        },
        U32Adjustment {
            inverse_calls: Rc::clone(&inverse_calls),
            step_calls: Rc::new(Cell::new(0)),
            fail_inverse: true,
        },
        WidgetSizing::fixed(Vector2::new(120.0, 28.0)),
    )
    .expect_err("inverse failure should be explicit");
    assert_eq!(
        inverse_error,
        NumericInputConstructionError::AdjustmentValueToNormalized {
            error: AdjustmentError
        }
    );
    assert_eq!(inverse_calls.get(), 1);
}

#[test]
fn draft_mutation_is_verbatim_has_no_typed_output_and_does_not_reformat_or_adjust() {
    let format_calls = Rc::new(Cell::new(0));
    let inverse_calls = Rc::new(Cell::new(0));
    let mut input = NumericInputWidget::try_new(
        7,
        U32Codec {
            format_calls: Rc::clone(&format_calls),
            parse_calls: Rc::new(Cell::new(0)),
            fail_format: false,
        },
        U32Adjustment {
            inverse_calls: Rc::clone(&inverse_calls),
            step_calls: Rc::new(Cell::new(0)),
            fail_inverse: false,
        },
        WidgetSizing::fixed(Vector2::new(120.0, 28.0)),
    )
    .expect("fixture should construct");
    replace_u32(&mut input, "invalid");
    assert_eq!(input.text_input.state.value, "invalid");
    assert!(input.active.is_some());
    assert_eq!(
        input.interaction_gate.incumbent(),
        Some(NumericInteractionOwner::TextEdit)
    );
    assert_eq!(format_calls.get(), 1);
    assert_eq!(inverse_calls.get(), 1);
    assert!(
        output::<u32>(Widget::handle_input(
            &mut input,
            Rect::default(),
            WidgetInput::key_press(WidgetKey::Enter),
        ))
        .is_none()
    );
    assert_eq!(input.text_input.state.value, "invalid");
    assert_eq!(
        input.interaction_gate.incumbent(),
        Some(NumericInteractionOwner::TextEdit)
    );
}

#[test]
fn denied_text_admission_precedes_policy_calls_and_preserves_widget_state() {
    let format_calls = Rc::new(Cell::new(0));
    let parse_calls = Rc::new(Cell::new(0));
    let inverse_calls = Rc::new(Cell::new(0));
    let mut input = NumericInputWidget::try_new(
        7,
        U32Codec {
            format_calls: Rc::clone(&format_calls),
            parse_calls: Rc::clone(&parse_calls),
            fail_format: false,
        },
        U32Adjustment {
            inverse_calls: Rc::clone(&inverse_calls),
            step_calls: Rc::new(Cell::new(0)),
            fail_inverse: false,
        },
        WidgetSizing::fixed(Vector2::new(120.0, 28.0)),
    )
    .expect("fixture should construct");
    focus(&mut input);
    input.text_input.state.caret = 0;
    input.text_input.state.selection_anchor = 1;
    let before_text = input.text_input.state.clone();
    let before_focus = input.text_input.common.state;
    assert!(
        input
            .interaction_gate
            .try_admit(NumericInteractionOwner::KeyboardAdjustment)
    );

    assert!(
        Widget::handle_input(
            &mut input,
            Rect::default(),
            WidgetInput::text_edit(TextEditCommand::InsertText("8".to_owned())),
        )
        .is_none()
    );

    assert_eq!(input.value, 7);
    assert_eq!(input.text_input.state, before_text);
    assert_eq!(input.text_input.common.state, before_focus);
    assert!(input.active.is_none());
    assert_eq!(parse_calls.get(), 0);
    assert_eq!(format_calls.get(), 1);
    assert_eq!(inverse_calls.get(), 1);
    assert_eq!(
        input.interaction_gate.incumbent(),
        Some(NumericInteractionOwner::KeyboardAdjustment)
    );
}

#[test]
fn first_mutation_admits_text_edit_and_continuation_keeps_one_session() {
    let mut input = u32_input();
    focus(&mut input);
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
            WidgetInput::text_edit(TextEditCommand::InsertText("8".to_owned())),
        )
        .is_none()
    );
    let transaction = input
        .active
        .as_ref()
        .expect("first mutation should start a session")
        .session
        .begin_event()
        .transaction;
    assert_eq!(
        input.interaction_gate.incumbent(),
        Some(NumericInteractionOwner::TextEdit)
    );

    assert!(
        Widget::handle_input(
            &mut input,
            Rect::default(),
            WidgetInput::text_edit(TextEditCommand::InsertText("9".to_owned())),
        )
        .is_none()
    );

    let active = input.active.as_ref().expect("continuation remains active");
    assert_eq!(active.session.begin_event().transaction, transaction);
    assert_eq!(active.session.draft(), "89");
    assert_eq!(input.text_input.state.value, "89");
    assert_eq!(
        input.interaction_gate.incumbent(),
        Some(NumericInteractionOwner::TextEdit)
    );
}

#[test]
fn no_op_first_mutation_releases_text_admission() {
    let mut input = u32_input();
    focus(&mut input);
    input.text_input.state.caret = 0;
    input.text_input.state.selection_anchor = 0;

    assert!(
        Widget::handle_input(
            &mut input,
            Rect::default(),
            WidgetInput::key_press(WidgetKey::Backspace),
        )
        .is_none()
    );

    assert_eq!(input.value, 7);
    assert_eq!(input.text_input.state.value, "7");
    assert!(input.active.is_none());
    assert_eq!(input.interaction_gate.incumbent(), None);
}

#[test]
fn incomplete_and_out_of_range_drafts_are_retained_without_terminal_output() {
    for draft in ["-", "101"] {
        let mut input = u32_input();
        replace_u32(&mut input, draft);
        assert_eq!(input.text_input.state.value, draft);
        assert!(input.active.is_some());
        assert!(
            output::<u32>(Widget::handle_input(
                &mut input,
                Rect::default(),
                WidgetInput::key_press(WidgetKey::Enter),
            ))
            .is_none()
        );
        assert_eq!(input.text_input.state.value, draft);
        assert_eq!(
            Widget::prepare_focus_loss(&mut input),
            FocusLossDecision::Veto
        );
    }
}

#[test]
fn valid_enter_emits_begin_then_commit_with_one_keyboard_transaction() {
    let mut input = u32_input();
    replace_u32(&mut input, "8");
    let commit_timestamp = Some(InputTimestamp::capture());
    let batch = output::<u32>(Widget::handle_input(
        &mut input,
        Rect::default(),
        WidgetInput::key_press_with_timestamp(WidgetKey::Enter, commit_timestamp),
    ))
    .expect("valid Enter should emit a batch");
    assert_eq!(batch.len(), 2);
    assert_eq!(batch.events()[0].phase, EditPhase::Begin);
    assert_eq!(batch.events()[1].phase, EditPhase::Commit);
    assert_eq!(batch.events()[0].transaction, batch.events()[1].transaction);
    assert_eq!(
        batch.events()[0].provenance.source(),
        InteractionSource::Keyboard
    );
    assert_eq!(
        batch.events()[1].provenance,
        InteractionProvenance::Keyboard {
            timestamp: commit_timestamp
        }
    );
    assert_eq!(batch.events()[0].value, 7);
    assert_eq!(batch.events()[1].value, 8);
    assert!(input.active.is_none());
    assert_eq!(input.interaction_gate.incumbent(), None);
}

#[test]
fn valid_focus_loss_commits_and_invalid_focus_loss_vetoes_idempotently() {
    let (mut valid, valid_parse_calls) = u32_input_with_parse_calls();
    replace_u32(&mut valid, "8");
    let parse_calls_after_draft = valid_parse_calls.get();
    assert_eq!(parse_calls_after_draft, 1);
    assert_eq!(
        Widget::prepare_focus_loss(&mut valid),
        FocusLossDecision::Allow
    );
    assert_eq!(valid_parse_calls.get(), parse_calls_after_draft);
    assert_eq!(
        Widget::prepare_focus_loss(&mut valid),
        FocusLossDecision::Allow
    );
    assert_eq!(valid_parse_calls.get(), parse_calls_after_draft);
    let batch = output::<u32>(Widget::handle_input(
        &mut valid,
        Rect::default(),
        WidgetInput::FocusChanged(false),
    ))
    .expect("valid focus loss should commit");
    assert_eq!(
        batch
            .events()
            .iter()
            .map(|event| event.phase)
            .collect::<Vec<_>>(),
        [EditPhase::Begin, EditPhase::Commit]
    );
    assert_eq!(valid_parse_calls.get(), parse_calls_after_draft);
    assert!(!valid.text_input.common.state.focused);
    assert_eq!(valid.interaction_gate.incumbent(), None);

    let (mut invalid, invalid_parse_calls) = u32_input_with_parse_calls();
    replace_u32(&mut invalid, "-");
    let parse_calls_after_invalid_draft = invalid_parse_calls.get();
    assert_eq!(parse_calls_after_invalid_draft, 1);
    assert_eq!(
        Widget::prepare_focus_loss(&mut invalid),
        FocusLossDecision::Veto
    );
    assert_eq!(invalid_parse_calls.get(), parse_calls_after_invalid_draft);
    assert_eq!(
        Widget::prepare_focus_loss(&mut invalid),
        FocusLossDecision::Veto
    );
    assert_eq!(invalid_parse_calls.get(), parse_calls_after_invalid_draft);
    assert!(
        Widget::handle_input(
            &mut invalid,
            Rect::default(),
            WidgetInput::FocusChanged(false),
        )
        .is_none()
    );
    assert_eq!(invalid_parse_calls.get(), parse_calls_after_invalid_draft);
    assert!(invalid.text_input.common.state.focused);
    assert_eq!(invalid.text_input.state.value, "-");
    assert_eq!(
        invalid.interaction_gate.incumbent(),
        Some(NumericInteractionOwner::TextEdit)
    );
}

#[test]
fn escape_emits_begin_cancel_and_restores_starting_value_and_draft() {
    let mut input = u32_input();
    replace_u32(&mut input, "8");
    let batch = output::<u32>(Widget::handle_input(
        &mut input,
        Rect::default(),
        WidgetInput::key_press(WidgetKey::Escape),
    ))
    .expect("Escape should cancel an active edit");
    assert_eq!(
        batch
            .events()
            .iter()
            .map(|event| event.phase)
            .collect::<Vec<_>>(),
        [EditPhase::Begin, EditPhase::Cancel]
    );
    assert_eq!(batch.events()[1].value, 7);
    assert_eq!(input.value, 7);
    assert_eq!(input.text_input.state.value, "7");
    assert!(input.text_input.common.state.focused);
    assert_eq!(input.interaction_gate.incumbent(), None);
    assert!(!Widget::preempts_host_shortcut_key(
        &input,
        WidgetKey::Escape
    ));
}

#[test]
fn same_value_reprojection_retains_draft_caret_selection_and_session_but_changed_value_resets() {
    let mut previous = u32_input();
    replace_u32(&mut previous, "8");
    previous.text_input.state.caret = 0;
    previous.text_input.state.selection_anchor = 1;

    let mut retained = u32_input();
    Widget::synchronize_from_previous(&mut retained, &previous);
    assert_eq!(retained.text_input.state.value, "8");
    assert_eq!(retained.text_input.state.caret, 0);
    assert_eq!(retained.text_input.state.selection_anchor, 1);
    assert!(retained.active.is_some());
    assert_eq!(
        retained.interaction_gate.incumbent(),
        Some(NumericInteractionOwner::TextEdit)
    );

    let mut changed = NumericInputWidget::try_new(
        9,
        U32Codec {
            format_calls: Rc::new(Cell::new(0)),
            parse_calls: Rc::new(Cell::new(0)),
            fail_format: false,
        },
        U32Adjustment {
            inverse_calls: Rc::new(Cell::new(0)),
            step_calls: Rc::new(Cell::new(0)),
            fail_inverse: false,
        },
        WidgetSizing::fixed(Vector2::new(120.0, 28.0)),
    )
    .expect("changed value fixture should construct");
    Widget::synchronize_from_previous(&mut changed, &previous);
    assert_eq!(changed.text_input.state.value, "9");
    assert!(changed.active.is_none());
    assert_eq!(changed.interaction_gate.incumbent(), None);

    let mut disabled = u32_input();
    disabled.text_input.common.state.disabled = true;
    Widget::synchronize_from_previous(&mut disabled, &previous);
    assert_eq!(disabled.text_input.state.value, "7");
    assert!(disabled.active.is_none());
    assert_eq!(disabled.interaction_gate.incumbent(), None);

    let mut read_only = u32_input();
    read_only.text_input.common.state.read_only = true;
    Widget::synchronize_from_previous(&mut read_only, &previous);
    assert_eq!(read_only.text_input.state.value, "7");
    assert!(read_only.active.is_none());

    let mut identity_reset = u32_input();
    replace_u32(&mut identity_reset, "8");
    identity_reset.text_input.common.id = 1;
    Widget::synchronize_from_previous(&mut identity_reset, &previous);
    assert!(identity_reset.active.is_none());
    assert_eq!(identity_reset.interaction_gate.incumbent(), None);
}

#[test]
fn inactive_reprojection_keeps_fresh_canonical_text_and_common_focus_state() {
    let mut previous = u32_input();
    focus(&mut previous);
    previous.text_input.state.value = String::from("007");
    previous.text_input.state.caret = 0;
    previous.text_input.state.selection_anchor = 2;

    let mut current = u32_input();
    Widget::synchronize_from_previous(&mut current, &previous);

    assert_eq!(current.text_input.state.value, "7");
    assert_eq!(current.text_input.state.caret, 1);
    assert_eq!(current.text_input.state.selection_anchor, 1);
    assert!(current.active.is_none());
    assert!(current.text_input.common.state.focused);
}

#[test]
fn selection_navigation_pointer_and_wheel_do_not_start_or_scrub_numeric_edits() {
    let mut input = u32_input();
    focus(&mut input);
    assert!(
        Widget::handle_input(
            &mut input,
            Rect::from_min_size(Point::default(), Vector2::new(120.0, 28.0)),
            WidgetInput::text_edit(TextEditCommand::MoveHome {
                extend_selection: false
            }),
        )
        .is_none()
    );
    assert!(input.active.is_none());
    assert!(
        Widget::handle_input(
            &mut input,
            Rect::from_min_size(Point::default(), Vector2::new(120.0, 28.0)),
            WidgetInput::primary_press(Point::new(0.0, 14.0)),
        )
        .is_none()
    );
    assert_eq!(input.text_input.state.caret, 0);
    assert!(input.active.is_none());
    assert!(
        Widget::handle_input(
            &mut input,
            Rect::from_min_size(Point::default(), Vector2::new(120.0, 28.0)),
            WidgetInput::plain_wheel(Point::new(40.0, 14.0), Vector2::new(0.0, 120.0)),
        )
        .is_none()
    );
    assert!(!Widget::accepts_wheel_input(&input));
    assert_eq!(input.text_input.state.value, "7");
}

#[test]
fn disabled_and_read_only_inputs_do_not_mutate_or_start_sessions() {
    let mut disabled = u32_input();
    disabled.text_input.common.state.focused = true;
    disabled.text_input.common.state.disabled = true;
    assert!(
        Widget::handle_input(&mut disabled, Rect::default(), WidgetInput::character('8'),)
            .is_none()
    );
    assert_eq!(disabled.text_input.state.value, "7");
    assert!(disabled.active.is_none());

    let mut read_only = u32_input();
    read_only.text_input.common.state.focused = true;
    read_only.text_input.common.state.read_only = true;
    assert!(
        Widget::handle_input(
            &mut read_only,
            Rect::default(),
            WidgetInput::text_edit(TextEditCommand::InsertText("8".to_owned())),
        )
        .is_none()
    );
    assert_eq!(read_only.text_input.state.value, "7");
    assert!(read_only.active.is_none());
}

#[test]
fn ui_local_non_clone_policies_are_accepted_by_the_consumer() {
    struct LocalCodec(Rc<Cell<usize>>);
    impl NumericCodec<u32> for LocalCodec {
        type Error = CodecError;

        fn parse(&self, text: &str) -> NumericParseResult<u32> {
            text.parse()
                .map_or(NumericParseResult::Invalid, NumericParseResult::Valid)
        }

        fn format_editable(
            &self,
            value: &u32,
            output: &mut dyn fmt::Write,
        ) -> Result<(), Self::Error> {
            self.0.set(self.0.get() + 1);
            write!(output, "{value}").map_err(|_| CodecError)
        }
    }

    struct LocalAdjustment(Rc<Cell<usize>>);
    impl NumericAdjustment<u32> for LocalAdjustment {
        type Error = AdjustmentError;

        fn normalized_to_value(&self, normalized: f32) -> Result<u32, Self::Error> {
            Ok(normalized as u32)
        }

        fn value_to_normalized(&self, value: &u32) -> Result<f32, Self::Error> {
            self.0.set(self.0.get() + 1);
            Ok(*value as f32)
        }

        fn step(
            &self,
            value: &u32,
            _: NumericStepDirection,
            _: NumericStep,
        ) -> Result<u32, Self::Error> {
            Ok(*value)
        }

        fn scrub(&self, value: &u32, _: f32, _: NumericStep) -> Result<u32, Self::Error> {
            Ok(*value)
        }

        fn wheel(&self, value: &u32, _: f32, _: NumericStep) -> Result<u32, Self::Error> {
            Ok(*value)
        }
    }

    let _ = NumericInputWidget::try_new(
        7,
        LocalCodec(Rc::new(Cell::new(0))),
        LocalAdjustment(Rc::new(Cell::new(0))),
        WidgetSizing::fixed(Vector2::new(120.0, 28.0)),
    )
    .expect("non-Clone, UI-local policies should construct");
    let _ = PointerModifiers::default();
}

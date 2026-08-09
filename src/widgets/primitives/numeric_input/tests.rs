use super::*;
use crate::widgets::interaction::{NumericInputInteraction, NumericInteractionOwner};
use crate::{
    gui::{
        input::InputTimestamp,
        types::{Point, Vector2},
    },
    widgets::{
        ButtonWidget, EditPhase, InteractionSource, KeyboardModifier, KeyboardModifiers,
        NumericStep, NumericStepDirection, NumericStepModifiers, PointerModifiers, TextEditCommand,
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
    u32_input_with_value(7)
}

fn u32_input_with_value(value: u32) -> NumericInputWidget<u32, U32Codec, U32Adjustment> {
    NumericInputWidget::try_new(
        value,
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
    .expect("u32 fixture should construct")
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

struct U32PolicyCalls {
    input: NumericInputWidget<u32, U32Codec, U32Adjustment>,
    format_calls: Rc<Cell<usize>>,
    parse_calls: Rc<Cell<usize>>,
    inverse_calls: Rc<Cell<usize>>,
    step_calls: Rc<Cell<usize>>,
}

fn u32_input_with_policy_calls() -> U32PolicyCalls {
    let format_calls = Rc::new(Cell::new(0));
    let parse_calls = Rc::new(Cell::new(0));
    let inverse_calls = Rc::new(Cell::new(0));
    let step_calls = Rc::new(Cell::new(0));
    let input = NumericInputWidget::try_new(
        7,
        U32Codec {
            format_calls: Rc::clone(&format_calls),
            parse_calls: Rc::clone(&parse_calls),
            fail_format: false,
        },
        U32Adjustment {
            inverse_calls: Rc::clone(&inverse_calls),
            step_calls: Rc::clone(&step_calls),
            fail_inverse: false,
        },
        WidgetSizing::fixed(Vector2::new(120.0, 28.0)),
    )
    .expect("u32 fixture should construct");
    U32PolicyCalls {
        input,
        format_calls,
        parse_calls,
        inverse_calls,
        step_calls,
    }
}

type U32StepCalls = (
    NumericInputWidget<u32, U32Codec, U32Adjustment>,
    Rc<Cell<usize>>,
    Rc<Cell<usize>>,
);

fn u32_input_with_step_calls() -> U32StepCalls {
    let step_calls = Rc::new(Cell::new(0));
    let format_calls = Rc::new(Cell::new(0));
    let input = NumericInputWidget::try_new(
        7,
        U32Codec {
            format_calls: Rc::clone(&format_calls),
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
    (input, step_calls, format_calls)
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

type CompleteU32Batch = NumericInputInteractionBatch<u32, AdjustmentError, CodecError>;

fn complete_output(output: Option<crate::widgets::WidgetOutput>) -> Option<CompleteU32Batch> {
    output.and_then(|output| output.typed_cloned())
}

fn complete_u32_input() -> NumericInputWidget<u32, U32Codec, U32Adjustment> {
    let mut input = u32_input();
    input.set_complete_output_mode();
    input
}

fn complete_edit(batch: &CompleteU32Batch) -> &NumericInputEditBatch<u32> {
    assert_eq!(batch.len(), 1);
    let [NumericInputInteraction::Edit(edit)] = batch.parts() else {
        panic!("complete TextEdit output should contain one outer Edit");
    };
    edit
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

fn active_u32_input() -> NumericInputWidget<u32, U32Codec, U32Adjustment> {
    let mut input = u32_input();
    replace_u32(&mut input, "8");
    input
}

fn assert_cancel_batch(batch: &NumericInputEditBatch<u32>) {
    assert_eq!(batch.len(), 2);
    assert_eq!(batch.events()[0].phase, EditPhase::Begin);
    assert_eq!(batch.events()[1].phase, EditPhase::Cancel);
    assert_eq!(batch.events()[0].transaction, batch.events()[1].transaction);
    assert_eq!(batch.events()[0].value, 7);
    assert_eq!(batch.events()[1].start_value, 7);
    assert_eq!(batch.events()[1].value, 7);
    assert_eq!(
        batch.events()[1].provenance,
        InteractionProvenance::Keyboard { timestamp: None }
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

    for complete in [false, true] {
        for policy in policies {
            let (mut input, step_calls, format_calls) = u32_input_with_step_calls();
            if complete {
                input.set_complete_output_mode();
            }
            if let Some(policy) = policy {
                input.set_step_modifiers(policy);
            }
            assert_eq!(input.step_modifiers, policy);
            focus(&mut input);
            let before_text = input.text_input.state.clone();
            let before_focus = input.text_input.common.state;
            let format_calls_before_arrows = format_calls.get();

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
            assert_eq!(format_calls.get(), format_calls_before_arrows);
            assert_eq!(input.value, 7);
            assert_eq!(input.text_input.state, before_text);
            assert_eq!(input.text_input.common.state, before_focus);
            assert!(input.active.is_none());
            assert_eq!(input.interaction_gate.incumbent(), None);
        }
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
    for complete in [false, true] {
        for draft in ["-", "101"] {
            let mut input = u32_input();
            if complete {
                input.set_complete_output_mode();
            }
            replace_u32(&mut input, draft);
            assert_eq!(input.text_input.state.value, draft);
            assert!(input.active.is_some());
            let terminal_output = Widget::handle_input(
                &mut input,
                Rect::default(),
                WidgetInput::key_press(WidgetKey::Enter),
            );
            if complete {
                assert!(complete_output(terminal_output).is_none());
            } else {
                assert!(output::<u32>(terminal_output).is_none());
            }
            assert_eq!(input.text_input.state.value, draft);
            assert_eq!(
                Widget::prepare_focus_loss(&mut input),
                FocusLossDecision::Veto
            );
        }
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
fn complete_mode_wraps_enter_and_focus_loss_in_one_unchanged_edit() {
    let mut enter = complete_u32_input();
    replace_u32(&mut enter, "8");
    let commit_timestamp = Some(InputTimestamp::capture());
    let enter_batch = complete_output(Widget::handle_input(
        &mut enter,
        Rect::default(),
        WidgetInput::key_press_with_timestamp(WidgetKey::Enter, commit_timestamp),
    ))
    .expect("complete Enter should emit one interaction envelope");
    let enter_edit = complete_edit(&enter_batch);
    assert_eq!(
        enter_edit
            .events()
            .iter()
            .map(|event| event.phase)
            .collect::<Vec<_>>(),
        [EditPhase::Begin, EditPhase::Commit]
    );
    assert_eq!(
        enter_edit.events()[0].transaction,
        enter_edit.events()[1].transaction
    );
    assert_eq!(enter_edit.events()[0].value, 7);
    assert_eq!(enter_edit.events()[1].value, 8);
    assert_eq!(
        enter_edit.events()[1].provenance,
        InteractionProvenance::Keyboard {
            timestamp: commit_timestamp
        }
    );

    let mut focus_loss = complete_u32_input();
    replace_u32(&mut focus_loss, "8");
    let focus_loss_batch = complete_output(Widget::handle_input(
        &mut focus_loss,
        Rect::default(),
        WidgetInput::FocusChanged(false),
    ))
    .expect("complete focus loss should emit one interaction envelope");
    let focus_loss_edit = complete_edit(&focus_loss_batch);
    assert_eq!(
        focus_loss_edit
            .events()
            .iter()
            .map(|event| event.phase)
            .collect::<Vec<_>>(),
        [EditPhase::Begin, EditPhase::Commit]
    );
    assert_eq!(
        focus_loss_edit.events()[0].transaction,
        focus_loss_edit.events()[1].transaction
    );
    assert_eq!(focus_loss_edit.events()[0].value, 7);
    assert_eq!(focus_loss_edit.events()[1].value, 8);
}

#[test]
fn complete_mode_wraps_escape_and_all_replacement_cancels_once() {
    let mut escape = complete_u32_input();
    replace_u32(&mut escape, "8");
    let escape_batch = complete_output(Widget::handle_input(
        &mut escape,
        Rect::default(),
        WidgetInput::key_press(WidgetKey::Escape),
    ))
    .expect("complete Escape should emit one interaction envelope");
    let escape_edit = complete_edit(&escape_batch);
    assert_eq!(escape_edit.events()[0].phase, EditPhase::Begin);
    assert_eq!(escape_edit.events()[1].phase, EditPhase::Cancel);
    assert_eq!(escape_edit.events()[1].value, 7);

    let mut removed = active_complete_u32_input();
    let removed_batch = complete_output(Widget::prepare_replacement(&mut removed, None))
        .expect("complete removal should emit one interaction envelope");
    assert_eq!(
        complete_edit(&removed_batch).events()[1].phase,
        EditPhase::Cancel
    );

    let mut changed_value = active_complete_u32_input();
    let changed_successor = u32_input_with_value(9);
    let changed_batch = complete_output(Widget::prepare_replacement(
        &mut changed_value,
        Some(&changed_successor as &dyn Widget),
    ))
    .expect("complete changed value should emit one interaction envelope");
    assert_eq!(
        complete_edit(&changed_batch).events()[1].phase,
        EditPhase::Cancel
    );

    let mut disabled = active_complete_u32_input();
    let mut disabled_successor = u32_input();
    disabled_successor.text_input.common.state.disabled = true;
    let disabled_batch = complete_output(Widget::prepare_replacement(
        &mut disabled,
        Some(&disabled_successor as &dyn Widget),
    ))
    .expect("complete disabled successor should emit one interaction envelope");
    assert_eq!(
        complete_edit(&disabled_batch).events()[1].phase,
        EditPhase::Cancel
    );

    let mut read_only = active_complete_u32_input();
    let mut read_only_successor = u32_input();
    read_only_successor.text_input.common.state.read_only = true;
    let read_only_batch = complete_output(Widget::prepare_replacement(
        &mut read_only,
        Some(&read_only_successor as &dyn Widget),
    ))
    .expect("complete read-only successor should emit one interaction envelope");
    assert_eq!(
        complete_edit(&read_only_batch).events()[1].phase,
        EditPhase::Cancel
    );

    let mut incompatible = active_complete_u32_input();
    let incompatible_successor = ButtonWidget::new(
        0,
        "replacement",
        WidgetSizing::fixed(Vector2::new(120.0, 28.0)),
    );
    let incompatible_batch = complete_output(Widget::prepare_replacement(
        &mut incompatible,
        Some(&incompatible_successor as &dyn Widget),
    ))
    .expect("complete incompatible successor should emit one interaction envelope");
    assert_eq!(
        complete_edit(&incompatible_batch).events()[1].phase,
        EditPhase::Cancel
    );
}

fn active_complete_u32_input() -> NumericInputWidget<u32, U32Codec, U32Adjustment> {
    let mut input = complete_u32_input();
    replace_u32(&mut input, "8");
    input
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
fn replacement_teardown_cancels_removed_incompatible_and_changed_value_once() {
    let mut removed = active_u32_input();
    let batch = output::<u32>(Widget::prepare_replacement(&mut removed, None))
        .expect("removal should cancel the active text edit");
    assert_cancel_batch(&batch);
    assert!(removed.active.is_none());
    assert_eq!(removed.text_input.state.value, "7");
    assert_eq!(removed.interaction_gate.incumbent(), None);

    let mut incompatible = active_u32_input();
    let successor = ButtonWidget::new(
        0,
        "replacement",
        WidgetSizing::fixed(Vector2::new(120.0, 28.0)),
    );
    let batch = output::<u32>(Widget::prepare_replacement(
        &mut incompatible,
        Some(&successor as &dyn Widget),
    ))
    .expect("incompatible replacement should cancel the active text edit");
    assert_cancel_batch(&batch);
    assert!(incompatible.active.is_none());
    assert_eq!(incompatible.text_input.state.value, "7");

    let mut changed_value = active_u32_input();
    let changed_successor = u32_input_with_value(9);
    let batch = output::<u32>(Widget::prepare_replacement(
        &mut changed_value,
        Some(&changed_successor as &dyn Widget),
    ))
    .expect("changed external value should cancel the active text edit");
    assert_cancel_batch(&batch);
    assert!(changed_value.active.is_none());
    assert_eq!(changed_value.text_input.state.value, "7");
}

#[test]
fn replacement_teardown_cancels_identity_disabled_and_read_only_successors() {
    let mut changed_identity = active_u32_input();
    let mut identity_successor = u32_input();
    Widget::common_mut(&mut identity_successor).id = 1;
    let batch = output::<u32>(Widget::prepare_replacement(
        &mut changed_identity,
        Some(&identity_successor as &dyn Widget),
    ))
    .expect("changed identity should cancel the active text edit");
    assert_cancel_batch(&batch);

    for read_only in [false, true] {
        let mut input = active_u32_input();
        let mut successor = u32_input();
        Widget::common_mut(&mut successor).state.disabled = !read_only;
        Widget::common_mut(&mut successor).state.read_only = read_only;
        let batch = output::<u32>(Widget::prepare_replacement(
            &mut input,
            Some(&successor as &dyn Widget),
        ))
        .expect("disabled or read-only successor should cancel the active text edit");
        assert_cancel_batch(&batch);
        assert!(input.active.is_none());
        assert_eq!(input.interaction_gate.incumbent(), None);
    }
}

#[test]
fn compatible_replacement_preserves_active_session_for_normal_sync() {
    let mut previous = active_u32_input();
    previous.text_input.state.caret = 0;
    previous.text_input.state.selection_anchor = 1;
    let successor = u32_input();

    assert!(Widget::prepare_replacement(&mut previous, Some(&successor as &dyn Widget),).is_none());
    assert!(previous.active.is_some());
    assert_eq!(
        previous.interaction_gate.incumbent(),
        Some(NumericInteractionOwner::TextEdit)
    );

    let mut synchronized = successor;
    Widget::synchronize_from_previous(&mut synchronized, &previous);
    assert_eq!(synchronized.text_input.state.value, "8");
    assert_eq!(synchronized.text_input.state.caret, 0);
    assert_eq!(synchronized.text_input.state.selection_anchor, 1);
    assert!(synchronized.active.is_some());
    assert_eq!(
        synchronized.interaction_gate.incumbent(),
        Some(NumericInteractionOwner::TextEdit)
    );
}

#[test]
fn output_mode_changes_retire_old_encoder_and_never_inherit_active_state() {
    let mut old_complete = active_complete_u32_input();
    let compatibility_successor = u32_input();
    let complete_cancel = complete_output(Widget::prepare_replacement(
        &mut old_complete,
        Some(&compatibility_successor as &dyn Widget),
    ))
    .expect("complete-to-compatibility change should retire through old mode");
    assert_eq!(
        complete_edit(&complete_cancel).events()[1].phase,
        EditPhase::Cancel
    );
    assert!(old_complete.active.is_none());

    let mut compatibility_current = compatibility_successor;
    Widget::synchronize_from_previous(&mut compatibility_current, &old_complete);
    assert!(compatibility_current.active.is_none());
    assert_eq!(compatibility_current.interaction_gate.incumbent(), None);

    let mut old_compatibility = active_u32_input();
    let complete_successor = complete_u32_input();
    let compatibility_cancel = output(Widget::prepare_replacement(
        &mut old_compatibility,
        Some(&complete_successor as &dyn Widget),
    ))
    .expect("compatibility-to-complete change should retire through old mode");
    assert_cancel_batch(&compatibility_cancel);
    assert!(old_compatibility.active.is_none());

    let mut complete_current = complete_successor;
    Widget::synchronize_from_previous(&mut complete_current, &old_compatibility);
    assert!(complete_current.active.is_none());
    assert_eq!(complete_current.interaction_gate.incumbent(), None);
}

#[test]
fn same_complete_mode_replacement_preserves_active_session() {
    let mut previous = active_complete_u32_input();
    let successor = complete_u32_input();

    assert!(Widget::prepare_replacement(&mut previous, Some(&successor as &dyn Widget),).is_none());
    let mut synchronized = successor;
    Widget::synchronize_from_previous(&mut synchronized, &previous);
    assert!(synchronized.active.is_some());
    assert_eq!(synchronized.text_input.state.value, "8");
    assert_eq!(
        synchronized.interaction_gate.incumbent(),
        Some(NumericInteractionOwner::TextEdit)
    );
}

#[test]
fn replacement_teardown_restores_invalid_incomplete_and_out_of_range_without_policy_calls() {
    for draft in ["invalid", "-", "101"] {
        let policy_calls = u32_input_with_policy_calls();
        let mut input = policy_calls.input;
        let format_calls = policy_calls.format_calls;
        let parse_calls = policy_calls.parse_calls;
        let inverse_calls = policy_calls.inverse_calls;
        let step_calls = policy_calls.step_calls;
        replace_u32(&mut input, draft);
        input.text_input.state.caret = 0;
        input.text_input.state.selection_anchor = 1;
        let calls_after_draft = (
            format_calls.get(),
            parse_calls.get(),
            inverse_calls.get(),
            step_calls.get(),
        );

        let batch = output::<u32>(Widget::prepare_replacement(&mut input, None))
            .expect("every non-valid draft should cancel during teardown");
        assert_cancel_batch(&batch);
        assert_eq!(input.value, 7);
        assert_eq!(input.text_input.state.value, "7");
        assert_eq!(input.text_input.state.caret, 1);
        assert_eq!(input.text_input.state.selection_anchor, 0);
        assert!(input.active.is_none());
        assert_eq!(input.interaction_gate.incumbent(), None);
        assert_eq!(
            (
                format_calls.get(),
                parse_calls.get(),
                inverse_calls.get(),
                step_calls.get(),
            ),
            calls_after_draft,
            "teardown must not consult codec or adjustment policy for {draft:?}"
        );
    }
}

#[test]
fn inactive_and_repeated_replacement_teardown_are_no_ops() {
    let mut inactive = u32_input();
    let before = inactive.text_input.state.clone();
    assert!(Widget::prepare_replacement(&mut inactive, None).is_none());
    assert_eq!(inactive.text_input.state, before);
    assert_eq!(inactive.interaction_gate.incumbent(), None);

    let mut active = active_u32_input();
    assert!(Widget::prepare_replacement(&mut active, None).is_some());
    assert!(Widget::prepare_replacement(&mut active, None).is_none());
    assert_eq!(active.value, 7);
    assert_eq!(active.text_input.state.value, "7");
    assert!(active.active.is_none());
    assert_eq!(active.interaction_gate.incumbent(), None);
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
fn complete_mode_disabled_and_read_only_inputs_are_silent() {
    let mut disabled = complete_u32_input();
    disabled.text_input.common.state.focused = true;
    disabled.text_input.common.state.disabled = true;
    assert!(
        complete_output(Widget::handle_input(
            &mut disabled,
            Rect::default(),
            WidgetInput::text_edit(TextEditCommand::InsertText("8".to_owned())),
        ))
        .is_none()
    );
    assert_eq!(disabled.text_input.state.value, "7");
    assert!(disabled.active.is_none());

    let mut read_only = complete_u32_input();
    read_only.text_input.common.state.focused = true;
    read_only.text_input.common.state.read_only = true;
    assert!(
        complete_output(Widget::handle_input(
            &mut read_only,
            Rect::default(),
            WidgetInput::text_edit(TextEditCommand::InsertText("8".to_owned())),
        ))
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

#[path = "tests/composition.rs"]
mod composition;

use super::*;
use crate::widgets::interaction::{NumericInputInteraction, NumericInteractionOwner};
use crate::{
    gui::{
        input::{InputSequence, InputSequenceRange, InputTimestamp},
        types::{Point, Vector2},
    },
    widgets::{
        ButtonWidget, EditPhase, InteractionSource, KeyboardModifier, KeyboardModifiers,
        NumericStep, NumericStepDirection, NumericStepModifiers, NumericWheelAttempt,
        PointerModifiers, TextEditCommand, WheelDelta, WheelPhase, WheelSample,
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
    fail_format_on_call: Option<usize>,
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
        if self.fail_format || self.fail_format_on_call == Some(self.format_calls.get()) {
            return Err(CodecError);
        }
        write!(output, "{value}").map_err(|_| CodecError)
    }
}

struct U32Adjustment {
    inverse_calls: Rc<Cell<usize>>,
    step_calls: Rc<Cell<usize>>,
    fail_inverse: bool,
    fail_step_on_call: Option<usize>,
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
        if self.fail_step_on_call == Some(self.step_calls.get()) {
            return Err(AdjustmentError);
        }
        let amount = match step {
            NumericStep::Base => 1,
            NumericStep::Fine => 2,
            NumericStep::Coarse => 10,
        };
        Ok(match direction {
            NumericStepDirection::Decrease => value.saturating_sub(amount),
            NumericStepDirection::Increase => value.saturating_add(amount).min(100),
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

struct PointerAdjustment {
    scrub_calls: Rc<Cell<usize>>,
    last_delta: Rc<Cell<f32>>,
    last_step: Rc<Cell<Option<NumericStep>>>,
    fail_scrub_on_call: Option<usize>,
    same_value: bool,
}

struct WheelAdjustment {
    wheel_calls: Rc<Cell<usize>>,
    last_delta: Rc<Cell<f32>>,
    last_step: Rc<Cell<Option<NumericStep>>>,
    fail_wheel_on_call: Option<usize>,
    same_value: bool,
}

impl NumericAdjustment<u32> for WheelAdjustment {
    type Error = AdjustmentError;

    fn normalized_to_value(&self, normalized: f32) -> Result<u32, Self::Error> {
        Ok((normalized * 100.0).round() as u32)
    }

    fn value_to_normalized(&self, value: &u32) -> Result<f32, Self::Error> {
        Ok(*value as f32 / 100.0)
    }

    fn step(
        &self,
        value: &u32,
        direction: NumericStepDirection,
        step: NumericStep,
    ) -> Result<u32, Self::Error> {
        let amount = match step {
            NumericStep::Base => 1,
            NumericStep::Fine => 2,
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
        _normalized_delta: f32,
        _step: NumericStep,
    ) -> Result<u32, Self::Error> {
        Ok(*value)
    }

    fn wheel(&self, value: &u32, delta: f32, step: NumericStep) -> Result<u32, Self::Error> {
        let call = self.wheel_calls.get() + 1;
        self.wheel_calls.set(call);
        self.last_delta.set(delta);
        self.last_step.set(Some(step));
        if self.fail_wheel_on_call == Some(call) {
            return Err(AdjustmentError);
        }
        if self.same_value {
            return Ok(*value);
        }
        let multiplier = match step {
            NumericStep::Base => 1.0,
            NumericStep::Fine => 2.0,
            NumericStep::Coarse => 10.0,
        };
        let amount = (delta * multiplier).round() as i64;
        if amount >= 0 {
            Ok(value.saturating_add(amount as u32))
        } else {
            Ok(value.saturating_sub(amount.unsigned_abs() as u32))
        }
    }
}

impl NumericAdjustment<u32> for PointerAdjustment {
    type Error = AdjustmentError;

    fn normalized_to_value(&self, normalized: f32) -> Result<u32, Self::Error> {
        Ok((normalized * 100.0).round() as u32)
    }

    fn value_to_normalized(&self, value: &u32) -> Result<f32, Self::Error> {
        Ok(*value as f32 / 100.0)
    }

    fn step(
        &self,
        value: &u32,
        direction: NumericStepDirection,
        step: NumericStep,
    ) -> Result<u32, Self::Error> {
        let amount = match step {
            NumericStep::Base => 1,
            NumericStep::Fine => 2,
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
        step: NumericStep,
    ) -> Result<u32, Self::Error> {
        let call = self.scrub_calls.get() + 1;
        self.scrub_calls.set(call);
        self.last_delta.set(normalized_delta);
        self.last_step.set(Some(step));
        if self.fail_scrub_on_call == Some(call) {
            return Err(AdjustmentError);
        }
        if self.same_value {
            return Ok(*value);
        }
        let amount = (normalized_delta.abs() * 10.0).floor() as u32
            * match step {
                NumericStep::Base => 1,
                NumericStep::Fine => 2,
                NumericStep::Coarse => 10,
            };
        Ok(if normalized_delta.is_sign_negative() {
            value.saturating_sub(amount)
        } else {
            value.saturating_add(amount)
        })
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
            fail_format_on_call: None,
        },
        U32Adjustment {
            inverse_calls: Rc::new(Cell::new(0)),
            step_calls: Rc::new(Cell::new(0)),
            fail_inverse: false,
            fail_step_on_call: None,
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
            fail_format_on_call: None,
        },
        U32Adjustment {
            inverse_calls: Rc::new(Cell::new(0)),
            step_calls: Rc::new(Cell::new(0)),
            fail_inverse: false,
            fail_step_on_call: None,
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
            fail_format_on_call: None,
        },
        U32Adjustment {
            inverse_calls: Rc::clone(&inverse_calls),
            step_calls: Rc::clone(&step_calls),
            fail_inverse: false,
            fail_step_on_call: None,
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
            fail_format_on_call: None,
        },
        U32Adjustment {
            inverse_calls: Rc::new(Cell::new(0)),
            step_calls: Rc::clone(&step_calls),
            fail_inverse: false,
            fail_step_on_call: None,
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

struct PointerU32Fixture {
    input: NumericInputWidget<u32, U32Codec, PointerAdjustment>,
    format_calls: Rc<Cell<usize>>,
    scrub_calls: Rc<Cell<usize>>,
    last_delta: Rc<Cell<f32>>,
    last_step: Rc<Cell<Option<NumericStep>>>,
}

fn pointer_u32_input(
    fail_scrub_on_call: Option<usize>,
    fail_format_on_call: Option<usize>,
    same_value: bool,
) -> PointerU32Fixture {
    let format_calls = Rc::new(Cell::new(0));
    let scrub_calls = Rc::new(Cell::new(0));
    let last_delta = Rc::new(Cell::new(0.0));
    let last_step = Rc::new(Cell::new(None));
    let mut input = NumericInputWidget::try_new(
        7,
        U32Codec {
            format_calls: Rc::clone(&format_calls),
            parse_calls: Rc::new(Cell::new(0)),
            fail_format: false,
            fail_format_on_call,
        },
        PointerAdjustment {
            scrub_calls: Rc::clone(&scrub_calls),
            last_delta: Rc::clone(&last_delta),
            last_step: Rc::clone(&last_step),
            fail_scrub_on_call,
            same_value,
        },
        WidgetSizing::fixed(Vector2::new(120.0, 28.0)),
    )
    .expect("pointer fixture should construct");
    input.set_scrub_policy(NumericScrubPolicy::default());
    input.set_complete_output_mode();
    PointerU32Fixture {
        input,
        format_calls,
        scrub_calls,
        last_delta,
        last_step,
    }
}

struct WheelU32Fixture {
    input: NumericInputWidget<u32, U32Codec, WheelAdjustment>,
    format_calls: Rc<Cell<usize>>,
    wheel_calls: Rc<Cell<usize>>,
    last_delta: Rc<Cell<f32>>,
    last_step: Rc<Cell<Option<NumericStep>>>,
}

fn wheel_u32_input(
    fail_wheel_on_call: Option<usize>,
    fail_format_on_call: Option<usize>,
    same_value: bool,
) -> WheelU32Fixture {
    let format_calls = Rc::new(Cell::new(0));
    let wheel_calls = Rc::new(Cell::new(0));
    let last_delta = Rc::new(Cell::new(0.0));
    let last_step = Rc::new(Cell::new(None));
    let mut input = NumericInputWidget::try_new(
        7,
        U32Codec {
            format_calls: Rc::clone(&format_calls),
            parse_calls: Rc::new(Cell::new(0)),
            fail_format: false,
            fail_format_on_call,
        },
        WheelAdjustment {
            wheel_calls: Rc::clone(&wheel_calls),
            last_delta: Rc::clone(&last_delta),
            last_step: Rc::clone(&last_step),
            fail_wheel_on_call,
            same_value,
        },
        WidgetSizing::fixed(Vector2::new(120.0, 28.0)),
    )
    .expect("wheel fixture should construct");
    input.set_wheel_policy(NumericWheelPolicy::default());
    input.set_complete_output_mode();
    WheelU32Fixture {
        input,
        format_calls,
        wheel_calls,
        last_delta,
        last_step,
    }
}

fn scrub_bounds() -> Rect {
    Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(100.0, 20.0))
}

fn wheel_bounds() -> Rect {
    Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 28.0))
}

fn exact_wheel_sample(
    delta: WheelDelta,
    phase: Option<WheelPhase>,
    modifiers: PointerModifiers,
    timestamp: Option<InputTimestamp>,
    sequence_range: Option<InputSequenceRange>,
) -> WheelSample {
    WheelSample::new_with_metadata(delta, phase, modifiers, timestamp, sequence_range)
        .expect("wheel fixture sample should be finite")
}

fn scrub_modifiers() -> PointerModifiers {
    PointerModifiers {
        alt: true,
        ..PointerModifiers::default()
    }
}

fn scrub_press(position: Point, timestamp: Option<InputTimestamp>) -> WidgetInput {
    WidgetInput::PointerPress {
        position,
        button: PointerButton::Primary,
        modifiers: scrub_modifiers(),
        timestamp,
    }
}

fn scrub_move(
    position: Point,
    modifiers: PointerModifiers,
    timestamp: Option<InputTimestamp>,
    sequence_range: Option<InputSequenceRange>,
) -> WidgetInput {
    WidgetInput::PointerMove {
        position,
        modifiers,
        timestamp,
        sequence_range,
    }
}

fn scrub_release(
    position: Point,
    modifiers: PointerModifiers,
    timestamp: Option<InputTimestamp>,
) -> WidgetInput {
    WidgetInput::PointerRelease {
        position,
        button: PointerButton::Primary,
        modifiers,
        timestamp,
    }
}

fn complete_output(output: Option<crate::widgets::WidgetOutput>) -> Option<CompleteU32Batch> {
    output.and_then(|output| output.typed_cloned())
}

fn complete_u32_input() -> NumericInputWidget<u32, U32Codec, U32Adjustment> {
    let mut input = u32_input();
    input.set_complete_output_mode();
    input
}

fn complete_keyboard_u32_input(
    policy: NumericStepModifiers,
) -> NumericInputWidget<u32, U32Codec, U32Adjustment> {
    let mut input = u32_input();
    input.set_step_modifiers(policy);
    input.set_complete_output_mode();
    input
}

fn complete_keyboard_u32_input_with_value(
    value: u32,
    policy: NumericStepModifiers,
) -> NumericInputWidget<u32, U32Codec, U32Adjustment> {
    let mut input = u32_input_with_value(value);
    input.set_step_modifiers(policy);
    input.set_complete_output_mode();
    input
}

fn scheduled_keyboard_u32_input(
    fail_step_on_call: Option<usize>,
    fail_format_on_call: Option<usize>,
) -> NumericInputWidget<u32, U32Codec, U32Adjustment> {
    NumericInputWidget::try_new(
        7,
        U32Codec {
            format_calls: Rc::new(Cell::new(0)),
            parse_calls: Rc::new(Cell::new(0)),
            fail_format: false,
            fail_format_on_call,
        },
        U32Adjustment {
            inverse_calls: Rc::new(Cell::new(0)),
            step_calls: Rc::new(Cell::new(0)),
            fail_inverse: false,
            fail_step_on_call,
        },
        WidgetSizing::fixed(Vector2::new(120.0, 28.0)),
    )
    .expect("scheduled keyboard fixture should construct")
}

fn complete_edit(batch: &CompleteU32Batch) -> &NumericInputEditBatch<u32> {
    assert_eq!(batch.len(), 1);
    let [NumericInputInteraction::Edit(edit)] = batch.parts() else {
        panic!("complete TextEdit output should contain one outer Edit");
    };
    edit
}

fn complete_wheel_edit(batch: &CompleteU32Batch) -> &NumericInputEditBatch<u32> {
    complete_edit(batch)
}

fn active_keyboard_u32_input() -> NumericInputWidget<u32, U32Codec, U32Adjustment> {
    let mut input = complete_keyboard_u32_input(NumericStepModifiers::new(
        KeyboardModifier::Shift,
        KeyboardModifier::Control,
    ));
    focus(&mut input);
    let initial = complete_output(Widget::handle_input(
        &mut input,
        Rect::default(),
        WidgetInput::key_press(WidgetKey::ArrowUp),
    ))
    .expect("keyboard fixture should start a transaction");
    assert_eq!(complete_edit(&initial).events().len(), 2);
    input
}

fn replace_u32<C, A>(input: &mut NumericInputWidget<u32, C, A>, text: &str)
where
    C: NumericCodec<u32> + 'static,
    A: NumericAdjustment<u32> + 'static,
{
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
fn arrow_key_samples_remain_no_ops_in_compatibility_and_without_policy() {
    for complete in [false, true] {
        let (mut input, step_calls, format_calls) = u32_input_with_step_calls();
        if complete {
            input.set_complete_output_mode();
        }
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
        assert!(input.keyboard.is_none());
        assert_eq!(input.interaction_gate.incumbent(), None);
    }

    let (mut input, step_calls, format_calls) = u32_input_with_step_calls();
    input.set_complete_output_mode();
    input.set_step_modifiers(NumericStepModifiers::MACOS_DEFAULT);
    focus(&mut input);
    assert!(
        Widget::handle_input(
            &mut input,
            Rect::default(),
            WidgetInput::key_press(WidgetKey::ArrowUp),
        )
        .is_some()
    );
    assert_eq!(step_calls.get(), 1);
    assert!(format_calls.get() > 1);
}

#[test]
fn explicit_keyboard_policies_select_base_fine_and_coarse_per_sample() {
    let cases = [
        (
            NumericStepModifiers::MACOS_DEFAULT,
            KeyboardModifiers::default(),
            8,
        ),
        (
            NumericStepModifiers::MACOS_DEFAULT,
            KeyboardModifiers {
                shift: true,
                ..KeyboardModifiers::default()
            },
            9,
        ),
        (
            NumericStepModifiers::MACOS_DEFAULT,
            KeyboardModifiers {
                command: true,
                ..KeyboardModifiers::default()
            },
            17,
        ),
        (
            NumericStepModifiers::WINDOWS_LINUX_DEFAULT,
            KeyboardModifiers {
                control: true,
                ..KeyboardModifiers::default()
            },
            17,
        ),
        (
            NumericStepModifiers::new(KeyboardModifier::Alt, KeyboardModifier::Control),
            KeyboardModifiers {
                alt: true,
                control: true,
                ..KeyboardModifiers::default()
            },
            9,
        ),
    ];

    for (policy, modifiers, expected) in cases {
        let mut input = complete_keyboard_u32_input(policy);
        focus(&mut input);
        assert!(
            Widget::handle_input(
                &mut input,
                Rect::default(),
                WidgetInput::KeyPress {
                    key: WidgetKey::ArrowUp,
                    modifiers,
                    repeat: false,
                    timestamp: None,
                },
            )
            .is_some()
        );
        assert_eq!(input.value, expected);
    }
}

#[test]
fn keyboard_transaction_preserves_changed_modifiers_timestamps_and_release() {
    let policy = NumericStepModifiers::new(KeyboardModifier::Shift, KeyboardModifier::Control);
    let mut input = complete_keyboard_u32_input(policy);
    focus(&mut input);
    let initial_timestamp = Some(InputTimestamp::capture());
    let repeat_timestamp = Some(InputTimestamp::capture());
    let release_timestamp = Some(InputTimestamp::capture());

    let initial = complete_output(Widget::handle_input(
        &mut input,
        Rect::default(),
        WidgetInput::KeyPress {
            key: WidgetKey::ArrowUp,
            modifiers: KeyboardModifiers::default(),
            repeat: false,
            timestamp: initial_timestamp,
        },
    ))
    .expect("effective initial step should emit Begin and Update");
    let initial_edit = complete_edit(&initial);
    assert_eq!(
        initial_edit
            .events()
            .iter()
            .map(|event| event.phase)
            .collect::<Vec<_>>(),
        [EditPhase::Begin, EditPhase::Update]
    );
    assert_eq!(initial_edit.events()[0].value, 7);
    assert_eq!(initial_edit.events()[1].value, 8);
    assert_eq!(
        initial_edit.events()[0].provenance,
        InteractionProvenance::Keyboard {
            timestamp: initial_timestamp
        }
    );
    assert_eq!(
        initial_edit.events()[1].provenance,
        InteractionProvenance::Keyboard {
            timestamp: initial_timestamp
        }
    );
    let transaction = initial_edit.transaction();
    assert_eq!(input.captured_focused_key(), Some(WidgetKey::ArrowUp));

    let repeat = complete_output(Widget::handle_input(
        &mut input,
        Rect::default(),
        WidgetInput::KeyPress {
            key: WidgetKey::ArrowUp,
            modifiers: KeyboardModifiers {
                shift: true,
                ..KeyboardModifiers::default()
            },
            repeat: true,
            timestamp: repeat_timestamp,
        },
    ))
    .expect("effective repeat should emit one Update");
    let repeat_edit = complete_edit(&repeat);
    assert_eq!(repeat_edit.events().len(), 1);
    assert_eq!(repeat_edit.events()[0].phase, EditPhase::Update);
    assert_eq!(repeat_edit.events()[0].transaction, transaction);
    assert_eq!(repeat_edit.events()[0].value, 10);
    assert_eq!(
        repeat_edit.events()[0].provenance,
        InteractionProvenance::Keyboard {
            timestamp: repeat_timestamp
        }
    );

    let release = complete_output(Widget::handle_input(
        &mut input,
        Rect::default(),
        WidgetInput::KeyRelease {
            key: WidgetKey::ArrowUp,
            modifiers: KeyboardModifiers {
                control: true,
                ..KeyboardModifiers::default()
            },
            timestamp: release_timestamp,
        },
    ))
    .expect("matching release should commit the current value");
    let release_edit = complete_edit(&release);
    assert_eq!(release_edit.events().len(), 1);
    assert_eq!(release_edit.events()[0].phase, EditPhase::Commit);
    assert_eq!(release_edit.events()[0].transaction, transaction);
    assert_eq!(release_edit.events()[0].value, 10);
    assert_eq!(
        release_edit.events()[0].provenance,
        InteractionProvenance::Keyboard {
            timestamp: release_timestamp
        }
    );
    assert_eq!(input.value, 10);
    assert_eq!(input.captured_focused_key(), None);
    assert_eq!(input.interaction_gate.incumbent(), None);
}

#[test]
fn unchanged_initial_and_repeat_steps_do_not_format_or_publish() {
    let mut initial =
        complete_keyboard_u32_input_with_value(100, NumericStepModifiers::MACOS_DEFAULT);
    focus(&mut initial);
    let format_calls = initial.codec.format_calls.get();
    assert!(
        Widget::handle_input(
            &mut initial,
            Rect::default(),
            WidgetInput::key_press(WidgetKey::ArrowUp),
        )
        .is_none()
    );
    assert_eq!(initial.codec.format_calls.get(), format_calls);
    assert_eq!(initial.value, 100);
    assert_eq!(initial.captured_focused_key(), None);
    assert_eq!(initial.interaction_gate.incumbent(), None);

    let mut boundary =
        complete_keyboard_u32_input_with_value(99, NumericStepModifiers::MACOS_DEFAULT);
    focus(&mut boundary);
    assert!(
        Widget::handle_input(
            &mut boundary,
            Rect::default(),
            WidgetInput::key_press(WidgetKey::ArrowUp),
        )
        .is_some()
    );
    let format_calls = boundary.codec.format_calls.get();
    assert!(
        Widget::handle_input(
            &mut boundary,
            Rect::default(),
            WidgetInput::KeyPress {
                key: WidgetKey::ArrowUp,
                modifiers: KeyboardModifiers::default(),
                repeat: true,
                timestamp: None,
            },
        )
        .is_none()
    );
    assert_eq!(boundary.codec.format_calls.get(), format_calls);
    assert_eq!(boundary.value, 100);
    assert_eq!(boundary.captured_focused_key(), Some(WidgetKey::ArrowUp));
    let commit = complete_output(Widget::handle_input(
        &mut boundary,
        Rect::default(),
        WidgetInput::key_release(WidgetKey::ArrowUp),
    ))
    .expect("unchanged repeat should retain the transaction through release");
    assert_eq!(complete_edit(&commit).events()[0].phase, EditPhase::Commit);
}

#[test]
fn initial_keyboard_failures_are_typed_and_leave_no_transaction_or_capture() {
    let mut step = scheduled_keyboard_u32_input(Some(1), None);
    step.set_step_modifiers(NumericStepModifiers::MACOS_DEFAULT);
    step.set_complete_output_mode();
    focus(&mut step);
    let step_output = complete_output(Widget::handle_input(
        &mut step,
        Rect::default(),
        WidgetInput::key_press(WidgetKey::ArrowUp),
    ))
    .expect("initial step failure should be emitted");
    assert_eq!(step_output.len(), 1);
    let NumericInputInteraction::StepFailed {
        attempt,
        direction,
        step: selected_step,
        provenance,
        cancelled,
        ..
    } = &step_output.parts()[0]
    else {
        panic!("initial step failure should be the only part");
    };
    assert_eq!(*attempt, NumericStepAttempt::Initial);
    assert_eq!(*direction, NumericStepDirection::Increase);
    assert_eq!(*selected_step, NumericStep::Base);
    assert_eq!(
        *provenance,
        InteractionProvenance::Keyboard { timestamp: None }
    );
    assert!(!cancelled);
    assert_eq!(step_output.parts()[0].step_error(), Some(&AdjustmentError));
    assert_eq!(step.value, 7);
    assert_eq!(step.text_input.state.value, "7");
    assert!(step.keyboard.is_none());
    assert_eq!(step.interaction_gate.incumbent(), None);

    let mut format = scheduled_keyboard_u32_input(None, Some(2));
    format.set_step_modifiers(NumericStepModifiers::MACOS_DEFAULT);
    format.set_complete_output_mode();
    focus(&mut format);
    let format_output = complete_output(Widget::handle_input(
        &mut format,
        Rect::default(),
        WidgetInput::key_press(WidgetKey::ArrowDown),
    ))
    .expect("initial format failure should be emitted");
    assert_eq!(format_output.len(), 1);
    let NumericInputInteraction::FormatFailed {
        attempt,
        direction,
        step: selected_step,
        provenance,
        cancelled,
        ..
    } = &format_output.parts()[0]
    else {
        panic!("initial format failure should be the only part");
    };
    assert_eq!(*attempt, NumericStepAttempt::Initial);
    assert_eq!(*direction, NumericStepDirection::Decrease);
    assert_eq!(*selected_step, NumericStep::Base);
    assert_eq!(
        *provenance,
        InteractionProvenance::Keyboard { timestamp: None }
    );
    assert!(!cancelled);
    assert_eq!(format_output.parts()[0].format_error(), Some(&CodecError));
    assert_eq!(format.value, 7);
    assert_eq!(format.text_input.state.value, "7");
    assert!(format.keyboard.is_none());
    assert_eq!(format.interaction_gate.incumbent(), None);
}

#[test]
fn repeat_failures_rollback_before_typed_failure_and_orphan_the_release() {
    for (fail_step_on_call, fail_format_on_call) in [(Some(2), None), (None, Some(3))] {
        let mut input = scheduled_keyboard_u32_input(fail_step_on_call, fail_format_on_call);
        input.set_step_modifiers(NumericStepModifiers::MACOS_DEFAULT);
        input.set_complete_output_mode();
        focus(&mut input);
        let initial_timestamp = Some(InputTimestamp::capture());
        let failure_timestamp = Some(InputTimestamp::capture());
        let initial = complete_output(Widget::handle_input(
            &mut input,
            Rect::default(),
            WidgetInput::key_press_with_timestamp(WidgetKey::ArrowUp, initial_timestamp),
        ))
        .expect("initial keyboard step should succeed");
        let transaction = complete_edit(&initial).transaction();

        let failed = complete_output(Widget::handle_input(
            &mut input,
            Rect::default(),
            WidgetInput::KeyPress {
                key: WidgetKey::ArrowUp,
                modifiers: KeyboardModifiers::default(),
                repeat: true,
                timestamp: failure_timestamp,
            },
        ))
        .expect("repeat failure should include rollback and failure");
        assert_eq!(failed.len(), 2);
        let [NumericInputInteraction::Edit(cancel), failure] = failed.parts() else {
            panic!("repeat failure should be ordered rollback then failure");
        };
        assert_eq!(cancel.events().len(), 1);
        assert_eq!(cancel.events()[0].phase, EditPhase::Cancel);
        assert_eq!(cancel.events()[0].transaction, transaction);
        assert_eq!(cancel.events()[0].value, 7);
        assert_eq!(
            cancel.events()[0].provenance,
            InteractionProvenance::Keyboard {
                timestamp: failure_timestamp
            }
        );
        match failure {
            NumericInputInteraction::StepFailed {
                attempt,
                cancelled,
                provenance,
                ..
            }
            | NumericInputInteraction::FormatFailed {
                attempt,
                cancelled,
                provenance,
                ..
            } => {
                assert_eq!(*attempt, NumericStepAttempt::Repeat);
                assert!(*cancelled);
                assert_eq!(
                    *provenance,
                    InteractionProvenance::Keyboard {
                        timestamp: failure_timestamp
                    }
                );
            }
            NumericInputInteraction::Edit(_)
            | NumericInputInteraction::ScrubFailed { .. }
            | NumericInputInteraction::PointerFormatFailed { .. }
            | NumericInputInteraction::WheelFailed { .. }
            | NumericInputInteraction::WheelFormatFailed { .. } => {
                panic!("repeat failure must be keyboard typed")
            }
        }
        assert_eq!(input.value, 7);
        assert_eq!(input.text_input.state.value, "7");
        assert!(input.keyboard.is_none());
        assert_eq!(input.captured_focused_key(), None);
        assert_eq!(input.interaction_gate.incumbent(), None);
        assert!(
            Widget::handle_input(
                &mut input,
                Rect::default(),
                WidgetInput::key_release(WidgetKey::ArrowUp),
            )
            .is_none()
        );
    }
}

#[test]
fn competing_and_orphan_keyboard_samples_do_not_mutate_or_reenter_host_path() {
    let mut input = active_keyboard_u32_input();
    let before_value = input.value;
    let before_text = input.text_input.state.clone();
    let before_step_calls = input.adjustment.step_calls.get();
    let before_format_calls = input.codec.format_calls.get();

    for sample in [
        WidgetInput::KeyPress {
            key: WidgetKey::ArrowDown,
            modifiers: KeyboardModifiers::default(),
            repeat: true,
            timestamp: None,
        },
        WidgetInput::KeyRelease {
            key: WidgetKey::ArrowDown,
            modifiers: KeyboardModifiers::default(),
            timestamp: None,
        },
    ] {
        assert!(Widget::handle_input(&mut input, Rect::default(), sample).is_none());
    }

    assert_eq!(input.value, before_value);
    assert_eq!(input.text_input.state, before_text);
    assert_eq!(input.adjustment.step_calls.get(), before_step_calls);
    assert_eq!(input.codec.format_calls.get(), before_format_calls);
    assert_eq!(input.captured_focused_key(), Some(WidgetKey::ArrowUp));

    let commit = complete_output(Widget::handle_input(
        &mut input,
        Rect::default(),
        WidgetInput::key_release(WidgetKey::ArrowUp),
    ))
    .expect("matching release should still commit after competing samples");
    assert_eq!(complete_edit(&commit).events()[0].phase, EditPhase::Commit);
    assert_eq!(input.captured_focused_key(), None);
    assert!(
        Widget::handle_input(
            &mut input,
            Rect::default(),
            WidgetInput::KeyPress {
                key: WidgetKey::ArrowUp,
                modifiers: KeyboardModifiers::default(),
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
            WidgetInput::key_release(WidgetKey::ArrowUp),
        )
        .is_none()
    );
}

#[test]
fn keyboard_admission_denies_text_and_other_numeric_owners_before_policy_calls() {
    let mut text = u32_input_with_policy_calls();
    text.input
        .set_step_modifiers(NumericStepModifiers::MACOS_DEFAULT);
    text.input.set_complete_output_mode();
    replace_u32(&mut text.input, "8");
    let text_before_value = text.input.value;
    let text_before_state = text.input.text_input.state.clone();
    let text_before_focus = text.input.text_input.common.state;
    let text_before_active = text.input.active.is_some();
    let text_before_keyboard = text.input.captured_focused_key();
    let text_before_format_calls = text.format_calls.get();
    let text_before_parse_calls = text.parse_calls.get();
    let text_before_inverse_calls = text.inverse_calls.get();
    let text_before_step_calls = text.step_calls.get();
    let text_before_owner = text.input.interaction_gate.incumbent();
    assert!(
        Widget::handle_input(
            &mut text.input,
            Rect::default(),
            WidgetInput::key_press(WidgetKey::ArrowUp),
        )
        .is_none()
    );
    assert_eq!(text.input.value, text_before_value);
    assert_eq!(text.input.text_input.state, text_before_state);
    assert_eq!(text.input.text_input.common.state, text_before_focus);
    assert_eq!(text.input.active.is_some(), text_before_active);
    assert_eq!(text.input.captured_focused_key(), text_before_keyboard);
    assert_eq!(text.format_calls.get(), text_before_format_calls);
    assert_eq!(text.parse_calls.get(), text_before_parse_calls);
    assert_eq!(text.inverse_calls.get(), text_before_inverse_calls);
    assert_eq!(text.step_calls.get(), text_before_step_calls);
    assert_eq!(text.input.interaction_gate.incumbent(), text_before_owner);

    let mut other = u32_input_with_policy_calls();
    other
        .input
        .set_step_modifiers(NumericStepModifiers::MACOS_DEFAULT);
    other.input.set_complete_output_mode();
    focus(&mut other.input);
    assert!(
        other
            .input
            .interaction_gate
            .try_admit(NumericInteractionOwner::PointerScrub)
    );
    let other_before_value = other.input.value;
    let other_before_state = other.input.text_input.state.clone();
    let other_before_focus = other.input.text_input.common.state;
    let other_before_active = other.input.active.is_some();
    let other_before_keyboard = other.input.captured_focused_key();
    let other_before_format_calls = other.format_calls.get();
    let other_before_parse_calls = other.parse_calls.get();
    let other_before_inverse_calls = other.inverse_calls.get();
    let other_before_step_calls = other.step_calls.get();
    let other_before_owner = other.input.interaction_gate.incumbent();
    assert!(
        Widget::handle_input(
            &mut other.input,
            Rect::default(),
            WidgetInput::key_press(WidgetKey::ArrowDown),
        )
        .is_none()
    );
    assert_eq!(other.input.value, other_before_value);
    assert_eq!(other.input.text_input.state, other_before_state);
    assert_eq!(other.input.text_input.common.state, other_before_focus);
    assert_eq!(other.input.active.is_some(), other_before_active);
    assert_eq!(other.input.captured_focused_key(), other_before_keyboard);
    assert_eq!(other.format_calls.get(), other_before_format_calls);
    assert_eq!(other.parse_calls.get(), other_before_parse_calls);
    assert_eq!(other.inverse_calls.get(), other_before_inverse_calls);
    assert_eq!(other.step_calls.get(), other_before_step_calls);
    assert_eq!(other.input.interaction_gate.incumbent(), other_before_owner);
}

#[test]
fn keyboard_escape_focus_loss_and_replacement_cancel_once_and_restore_snapshot() {
    let mut escape = active_keyboard_u32_input();
    escape.text_input.state.caret = 0;
    escape.text_input.state.selection_anchor = 1;
    let cancelled = complete_output(Widget::handle_input(
        &mut escape,
        Rect::default(),
        WidgetInput::key_press(WidgetKey::Escape),
    ))
    .expect("Escape should cancel keyboard adjustment");
    assert_eq!(
        complete_edit(&cancelled).events()[0].phase,
        EditPhase::Cancel
    );
    assert_eq!(escape.value, 7);
    assert_eq!(escape.text_input.state.value, "7");
    assert_eq!(escape.text_input.state.caret, 1);
    assert_eq!(escape.text_input.state.selection_anchor, 1);
    assert_eq!(escape.captured_focused_key(), None);
    assert_eq!(escape.interaction_gate.incumbent(), None);
    assert!(
        Widget::handle_input(
            &mut escape,
            Rect::default(),
            WidgetInput::key_press(WidgetKey::Escape),
        )
        .is_none()
    );

    let mut focus_loss = active_keyboard_u32_input();
    let cancelled = complete_output(Widget::handle_input(
        &mut focus_loss,
        Rect::default(),
        WidgetInput::FocusChanged(false),
    ))
    .expect("focus loss should cancel keyboard adjustment");
    assert_eq!(
        complete_edit(&cancelled).events()[0].phase,
        EditPhase::Cancel
    );
    assert!(!focus_loss.text_input.common.state.focused);
    assert_eq!(focus_loss.value, 7);
    assert!(
        Widget::handle_input(
            &mut focus_loss,
            Rect::default(),
            WidgetInput::FocusChanged(false),
        )
        .is_none()
    );

    let mut removed = active_keyboard_u32_input();
    let cancelled = complete_output(Widget::prepare_replacement(&mut removed, None))
        .expect("removal should cancel keyboard adjustment");
    assert_eq!(
        complete_edit(&cancelled).events()[0].phase,
        EditPhase::Cancel
    );
    assert!(Widget::prepare_replacement(&mut removed, None).is_none());
    assert_eq!(removed.value, 7);
    assert_eq!(removed.interaction_gate.incumbent(), None);
}

#[test]
fn compatible_keyboard_reprojection_preserves_state_and_capture() {
    let mut previous = active_keyboard_u32_input();
    previous.text_input.state.caret = 0;
    previous.text_input.state.selection_anchor = 1;
    let successor = complete_keyboard_u32_input_with_value(
        8,
        NumericStepModifiers::new(KeyboardModifier::Shift, KeyboardModifier::Control),
    );
    assert!(Widget::prepare_replacement(&mut previous, Some(&successor as &dyn Widget),).is_none());

    let mut synchronized = successor;
    Widget::synchronize_from_previous(&mut synchronized, &previous);
    assert_eq!(synchronized.value, 8);
    assert_eq!(synchronized.text_input.state.value, "8");
    assert_eq!(synchronized.text_input.state.caret, 0);
    assert_eq!(synchronized.text_input.state.selection_anchor, 1);
    assert_eq!(
        synchronized.captured_focused_key(),
        Some(WidgetKey::ArrowUp)
    );
    assert_eq!(
        synchronized.interaction_gate.incumbent(),
        Some(NumericInteractionOwner::KeyboardAdjustment)
    );

    let repeat = complete_output(Widget::handle_input(
        &mut synchronized,
        Rect::default(),
        WidgetInput::KeyPress {
            key: WidgetKey::ArrowUp,
            modifiers: KeyboardModifiers::default(),
            repeat: true,
            timestamp: None,
        },
    ))
    .expect("compatible reprojection should retain keyboard continuation");
    assert_eq!(complete_edit(&repeat).events()[0].phase, EditPhase::Update);
}

#[test]
fn incompatible_keyboard_reprojection_cancels_for_authority_mode_and_accessibility_boundaries() {
    let mut changed_value = active_keyboard_u32_input();
    let changed_value_successor = complete_keyboard_u32_input_with_value(
        9,
        NumericStepModifiers::new(KeyboardModifier::Shift, KeyboardModifier::Control),
    );
    let cancelled = complete_output(Widget::prepare_replacement(
        &mut changed_value,
        Some(&changed_value_successor as &dyn Widget),
    ))
    .expect("changed external authority should cancel");
    assert_eq!(
        complete_edit(&cancelled).events()[0].phase,
        EditPhase::Cancel
    );

    let mut changed_mode = active_keyboard_u32_input();
    let compatibility_successor = u32_input();
    let cancelled = complete_output(Widget::prepare_replacement(
        &mut changed_mode,
        Some(&compatibility_successor as &dyn Widget),
    ))
    .expect("mode replacement should use the retiring complete mapper");
    assert_eq!(
        complete_edit(&cancelled).events()[0].phase,
        EditPhase::Cancel
    );

    for read_only in [false, true] {
        let mut current = active_keyboard_u32_input();
        let mut successor = complete_keyboard_u32_input(NumericStepModifiers::new(
            KeyboardModifier::Shift,
            KeyboardModifier::Control,
        ));
        successor.text_input.common.state.disabled = !read_only;
        successor.text_input.common.state.read_only = read_only;
        let cancelled = complete_output(Widget::prepare_replacement(
            &mut current,
            Some(&successor as &dyn Widget),
        ))
        .expect("disablement and read-only should cancel keyboard adjustment");
        assert_eq!(
            complete_edit(&cancelled).events()[0].phase,
            EditPhase::Cancel
        );
        assert_eq!(current.value, 7);
        assert_eq!(current.interaction_gate.incumbent(), None);
    }
}

#[test]
fn navigation_keys_remain_text_navigation_when_keyboard_adjustment_is_configured() {
    let mut input = complete_keyboard_u32_input(NumericStepModifiers::MACOS_DEFAULT);
    focus(&mut input);
    input.text_input.state.caret = 1;
    input.text_input.state.selection_anchor = 1;
    for command in [
        TextEditCommand::MoveHome {
            extend_selection: false,
        },
        TextEditCommand::MoveEnd {
            extend_selection: false,
        },
    ] {
        assert!(
            Widget::handle_input(&mut input, Rect::default(), WidgetInput::text_edit(command),)
                .is_none()
        );
    }
    assert!(input.keyboard.is_none());
    assert!(input.active.is_none());
    assert_eq!(input.interaction_gate.incumbent(), None);
}

#[test]
fn keyboard_disablement_and_read_only_state_deny_initial_admission() {
    for read_only in [false, true] {
        let mut input = complete_keyboard_u32_input(NumericStepModifiers::MACOS_DEFAULT);
        input.text_input.common.state.focused = true;
        input.text_input.common.state.disabled = !read_only;
        input.text_input.common.state.read_only = read_only;
        assert!(
            Widget::handle_input(
                &mut input,
                Rect::default(),
                WidgetInput::key_press(WidgetKey::ArrowUp),
            )
            .is_none()
        );
        assert_eq!(input.value, 7);
        assert!(input.keyboard.is_none());
        assert_eq!(input.interaction_gate.incumbent(), None);
    }
}

#[test]
fn keyboard_widget_focus_routing_opt_in_tracks_only_active_complete_policy() {
    let mut inactive = complete_keyboard_u32_input(NumericStepModifiers::MACOS_DEFAULT);
    assert!(!Widget::participates_in_focused_key_routing(&inactive));
    focus(&mut inactive);
    assert!(Widget::participates_in_focused_key_routing(&inactive));

    let mut text_edit = complete_keyboard_u32_input(NumericStepModifiers::MACOS_DEFAULT);
    replace_u32(&mut text_edit, "8");
    assert_eq!(
        text_edit.interaction_gate.incumbent(),
        Some(NumericInteractionOwner::TextEdit)
    );
    assert!(Widget::participates_in_focused_key_routing(&text_edit));

    let mut pointer_scrub = complete_keyboard_u32_input(NumericStepModifiers::MACOS_DEFAULT);
    focus(&mut pointer_scrub);
    assert!(
        pointer_scrub
            .interaction_gate
            .try_admit(NumericInteractionOwner::PointerScrub)
    );
    assert!(Widget::participates_in_focused_key_routing(&pointer_scrub));

    assert_eq!(Widget::captured_focused_key(&inactive), None);
    let _ = Widget::handle_input(
        &mut inactive,
        Rect::default(),
        WidgetInput::key_press(WidgetKey::ArrowUp),
    );
    assert_eq!(
        Widget::captured_focused_key(&inactive),
        Some(WidgetKey::ArrowUp)
    );
    assert!(Widget::participates_in_focused_key_routing(&inactive));
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
            fail_format_on_call: None,
        },
        U32Adjustment {
            inverse_calls: Rc::new(Cell::new(0)),
            step_calls: Rc::new(Cell::new(0)),
            fail_inverse: false,
            fail_step_on_call: None,
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
            fail_format_on_call: None,
        },
        U32Adjustment {
            inverse_calls: Rc::clone(&inverse_calls),
            step_calls: Rc::new(Cell::new(0)),
            fail_inverse: true,
            fail_step_on_call: None,
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
            fail_format_on_call: None,
        },
        U32Adjustment {
            inverse_calls: Rc::clone(&inverse_calls),
            step_calls: Rc::new(Cell::new(0)),
            fail_inverse: false,
            fail_step_on_call: None,
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
            fail_format_on_call: None,
        },
        U32Adjustment {
            inverse_calls: Rc::clone(&inverse_calls),
            step_calls: Rc::new(Cell::new(0)),
            fail_inverse: false,
            fail_step_on_call: None,
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
            fail_format_on_call: None,
        },
        U32Adjustment {
            inverse_calls: Rc::new(Cell::new(0)),
            step_calls: Rc::new(Cell::new(0)),
            fail_inverse: false,
            fail_step_on_call: None,
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
fn pointer_scrub_admission_requires_explicit_complete_alt_primary_configuration() {
    let bounds = scrub_bounds();
    let mut fixture = pointer_u32_input(None, None, false);
    let alt_primary = scrub_press(Point::new(0.0, 10.0), None);
    assert_eq!(
        Widget::preflight_pointer_press(&fixture.input, bounds, &alt_primary),
        PointerPressAdmission::ManagedCapture
    );
    assert_eq!(
        Widget::preflight_pointer_press(
            &fixture.input,
            bounds,
            &WidgetInput::primary_press(Point::new(0.0, 10.0)),
        ),
        PointerPressAdmission::Legacy
    );
    assert_eq!(
        Widget::preflight_pointer_press(
            &fixture.input,
            bounds,
            &WidgetInput::pointer_press(
                Point::new(0.0, 10.0),
                PointerButton::Secondary,
                scrub_modifiers(),
            ),
        ),
        PointerPressAdmission::Legacy
    );

    focus(&mut fixture.input);
    assert!(
        Widget::handle_input(
            &mut fixture.input,
            bounds,
            WidgetInput::primary_press(Point::new(0.0, 10.0)),
        )
        .is_none()
    );
    assert!(fixture.input.pointer.is_none());
    assert_eq!(
        fixture.input.interaction_gate.incumbent(),
        None,
        "unmodified primary remains legacy text routing"
    );
    assert_eq!(fixture.scrub_calls.get(), 0);
    assert_eq!(fixture.format_calls.get(), 1);

    let mut unconfigured = complete_u32_input();
    assert_eq!(
        Widget::preflight_pointer_press(&unconfigured, bounds, &alt_primary),
        PointerPressAdmission::Legacy
    );
    focus(&mut unconfigured);
    assert!(Widget::handle_input(&mut unconfigured, bounds, alt_primary).is_none());
}

#[test]
fn pointer_scrub_first_effective_move_and_release_preserve_exact_metadata() {
    let bounds = scrub_bounds();
    let mut fixture = pointer_u32_input(None, None, false);
    focus(&mut fixture.input);
    let press_timestamp = Some(InputTimestamp::capture());
    assert!(
        Widget::handle_input(
            &mut fixture.input,
            bounds,
            scrub_press(Point::new(0.0, 10.0), press_timestamp),
        )
        .is_none()
    );
    assert_eq!(fixture.scrub_calls.get(), 0);
    assert_eq!(fixture.format_calls.get(), 1);
    assert_eq!(
        fixture.input.interaction_gate.incumbent(),
        Some(NumericInteractionOwner::PointerScrub)
    );

    let move_timestamp = Some(InputTimestamp::capture());
    let sequence_range = Some(InputSequenceRange::singleton(
        InputSequence::from_runtime_value(41),
    ));
    let move_modifiers = scrub_modifiers();
    let moved = complete_output(Widget::handle_input(
        &mut fixture.input,
        bounds,
        scrub_move(
            Point::new(100.0, 11.0),
            move_modifiers,
            move_timestamp,
            sequence_range,
        ),
    ))
    .expect("first effective pointer move should emit an edit");
    let edit = complete_edit(&moved);
    let [begin, update] = edit.events() else {
        panic!("first effective pointer move should batch Begin and Update");
    };
    assert_eq!(begin.phase, EditPhase::Begin);
    assert_eq!(begin.value, 7);
    assert_eq!(
        begin.provenance,
        InteractionProvenance::Pointer {
            modifiers: scrub_modifiers(),
            timestamp: press_timestamp,
            sequence_range: None,
        }
    );
    assert_eq!(update.phase, EditPhase::Update);
    assert_eq!(update.value, 17);
    assert_eq!(
        update.provenance,
        InteractionProvenance::Pointer {
            modifiers: move_modifiers,
            timestamp: move_timestamp,
            sequence_range,
        }
    );
    assert_eq!(fixture.scrub_calls.get(), 1);
    assert_eq!(fixture.last_delta.get(), 1.0);
    assert_eq!(fixture.last_step.get(), Some(NumericStep::Base));
    assert_eq!(fixture.format_calls.get(), 2);
    assert_eq!(fixture.input.text_input.state.value, "17");
    assert_eq!(fixture.input.text_input.state.caret, 2);
    assert_eq!(fixture.input.text_input.state.selection_anchor, 2);

    let release_timestamp = Some(InputTimestamp::capture());
    let release_modifiers = PointerModifiers {
        alt: true,
        shift: true,
        ..PointerModifiers::default()
    };
    let committed = complete_output(Widget::handle_input(
        &mut fixture.input,
        bounds,
        scrub_release(
            Point::new(100.0, 11.0),
            release_modifiers,
            release_timestamp,
        ),
    ))
    .expect("matching primary release should commit the active scrub");
    let commit = complete_edit(&committed).events();
    let [commit] = commit else {
        panic!("release should emit one Commit event");
    };
    assert_eq!(commit.phase, EditPhase::Commit);
    assert_eq!(commit.start_value, 7);
    assert_eq!(commit.value, 17);
    assert_eq!(
        commit.provenance,
        InteractionProvenance::Pointer {
            modifiers: release_modifiers,
            timestamp: release_timestamp,
            sequence_range: None,
        }
    );
    assert!(fixture.input.pointer.is_none());
    assert_eq!(fixture.input.interaction_gate.incumbent(), None);
}

#[test]
fn pointer_scrub_zero_and_subquantum_moves_are_allocation_free_no_ops_and_accumulate() {
    let bounds = scrub_bounds();
    let mut fixture = pointer_u32_input(None, None, false);
    focus(&mut fixture.input);
    assert!(
        Widget::handle_input(
            &mut fixture.input,
            bounds,
            scrub_press(Point::new(0.0, 10.0), None),
        )
        .is_none()
    );

    assert!(
        Widget::handle_input(
            &mut fixture.input,
            bounds,
            scrub_move(Point::new(0.0, 19.0), scrub_modifiers(), None, None),
        )
        .is_none()
    );
    assert_eq!(
        fixture.scrub_calls.get(),
        0,
        "zero horizontal movement is a policy no-op"
    );

    assert!(
        Widget::handle_input(
            &mut fixture.input,
            bounds,
            scrub_move(Point::new(5.0, 10.0), scrub_modifiers(), None, None),
        )
        .is_none()
    );
    assert_eq!(fixture.scrub_calls.get(), 1);
    assert_eq!(fixture.format_calls.get(), 1);
    assert_eq!(
        fixture.input.value, 7,
        "unchanged candidates do not advance the anchor"
    );
    assert_eq!(
        fixture.input.pointer.as_ref().unwrap().anchor_position.x,
        0.0
    );

    let moved = complete_output(Widget::handle_input(
        &mut fixture.input,
        bounds,
        scrub_move(Point::new(20.0, 10.0), scrub_modifiers(), None, None),
    ))
    .expect("the later move should accumulate from the original anchor");
    assert_eq!(complete_edit(&moved).events()[1].value, 9);
    assert_eq!(fixture.scrub_calls.get(), 2);
    assert_eq!(fixture.format_calls.get(), 2);
}

#[test]
fn pointer_scrub_modifier_changes_reanchor_and_fine_wins_over_coarse() {
    let bounds = scrub_bounds();
    let mut fixture = pointer_u32_input(None, None, false);
    focus(&mut fixture.input);
    assert!(
        Widget::handle_input(
            &mut fixture.input,
            bounds,
            scrub_press(Point::new(0.0, 10.0), None),
        )
        .is_none()
    );

    let base = complete_output(Widget::handle_input(
        &mut fixture.input,
        bounds,
        scrub_move(Point::new(100.0, 10.0), scrub_modifiers(), None, None),
    ))
    .expect("base move should start the scrub");
    assert_eq!(complete_edit(&base).events()[1].value, 17);
    assert_eq!(fixture.last_step.get(), Some(NumericStep::Base));
    let calls_after_base = fixture.scrub_calls.get();

    let fine_and_coarse = PointerModifiers {
        alt: true,
        shift: true,
        command: true,
    };
    assert!(
        Widget::handle_input(
            &mut fixture.input,
            bounds,
            scrub_move(Point::new(50.0, 10.0), fine_and_coarse, None, None),
        )
        .is_none()
    );
    assert_eq!(fixture.scrub_calls.get(), calls_after_base);
    assert_eq!(
        fixture.input.value, 17,
        "modifier changes do not jump the value"
    );
    assert_eq!(
        fixture.input.pointer.as_ref().unwrap().anchor_position.x,
        50.0
    );

    let fine = complete_output(Widget::handle_input(
        &mut fixture.input,
        bounds,
        scrub_move(Point::new(100.0, 10.0), fine_and_coarse, None, None),
    ))
    .expect("fine move should use the reanchored position");
    assert_eq!(complete_edit(&fine).events()[0].value, 27);
    assert_eq!(fixture.last_step.get(), Some(NumericStep::Fine));

    let coarse = PointerModifiers {
        alt: true,
        command: true,
        ..PointerModifiers::default()
    };
    assert!(
        Widget::handle_input(
            &mut fixture.input,
            bounds,
            scrub_move(Point::new(90.0, 10.0), coarse, None, None),
        )
        .is_none()
    );
    assert_eq!(fixture.input.value, 27);
    assert_eq!(fixture.last_step.get(), Some(NumericStep::Fine));
    let coarse_move = complete_output(Widget::handle_input(
        &mut fixture.input,
        bounds,
        scrub_move(Point::new(100.0, 10.0), coarse, None, None),
    ))
    .expect("coarse move should use the new coarse anchor");
    assert_eq!(complete_edit(&coarse_move).events()[0].value, 37);
    assert_eq!(fixture.last_step.get(), Some(NumericStep::Coarse));
}

#[test]
fn pointer_scrub_is_blocked_by_text_and_keyboard_owners_before_policy_calls() {
    let bounds = scrub_bounds();
    let mut text_owner = pointer_u32_input(None, None, false);
    replace_u32(&mut text_owner.input, "8");
    let press = scrub_press(Point::new(0.0, 10.0), None);
    assert_eq!(
        Widget::preflight_pointer_press(&text_owner.input, bounds, &press),
        PointerPressAdmission::Blocked
    );
    assert!(Widget::handle_input(&mut text_owner.input, bounds, press).is_none());
    assert_eq!(text_owner.scrub_calls.get(), 0);
    assert_eq!(text_owner.format_calls.get(), 1);
    assert_eq!(
        text_owner.input.interaction_gate.incumbent(),
        Some(NumericInteractionOwner::TextEdit)
    );

    let mut keyboard_owner = pointer_u32_input(None, None, false);
    keyboard_owner
        .input
        .set_step_modifiers(NumericStepModifiers::new(
            KeyboardModifier::Alt,
            KeyboardModifier::Control,
        ));
    focus(&mut keyboard_owner.input);
    assert!(
        complete_output(Widget::handle_input(
            &mut keyboard_owner.input,
            bounds,
            WidgetInput::key_press(WidgetKey::ArrowUp),
        ))
        .is_some()
    );
    let press = scrub_press(Point::new(0.0, 10.0), None);
    assert_eq!(
        Widget::preflight_pointer_press(&keyboard_owner.input, bounds, &press),
        PointerPressAdmission::Blocked
    );
    assert!(Widget::handle_input(&mut keyboard_owner.input, bounds, press).is_none());
    assert_eq!(keyboard_owner.scrub_calls.get(), 0);
    assert_eq!(
        keyboard_owner.input.interaction_gate.incumbent(),
        Some(NumericInteractionOwner::KeyboardAdjustment)
    );
}

#[test]
fn pointer_scrub_pending_and_active_cancellation_restore_the_exact_ui_snapshot() {
    let bounds = scrub_bounds();
    let mut pending = pointer_u32_input(None, None, false);
    focus(&mut pending.input);
    pending.input.text_input.state.caret = 0;
    pending.input.text_input.state.selection_anchor = 1;
    let snapshot = pending.input.text_input.state.clone();
    assert!(
        Widget::handle_input(
            &mut pending.input,
            bounds,
            scrub_press(Point::new(0.0, 10.0), None),
        )
        .is_none()
    );
    assert!(
        Widget::handle_input(
            &mut pending.input,
            bounds,
            WidgetInput::key_press(WidgetKey::Escape),
        )
        .is_none()
    );
    assert_eq!(pending.input.text_input.state, snapshot);
    assert_eq!(pending.input.value, 7);
    assert!(pending.input.pointer.is_none());
    assert_eq!(pending.input.interaction_gate.incumbent(), None);

    let mut escape = pointer_u32_input(None, None, false);
    focus(&mut escape.input);
    assert!(
        Widget::handle_input(
            &mut escape.input,
            bounds,
            scrub_press(Point::new(0.0, 10.0), None),
        )
        .is_none()
    );
    assert!(
        complete_output(Widget::handle_input(
            &mut escape.input,
            bounds,
            scrub_move(Point::new(100.0, 10.0), scrub_modifiers(), None, None),
        ))
        .is_some()
    );
    let cancel = complete_output(Widget::handle_input(
        &mut escape.input,
        bounds,
        WidgetInput::key_press(WidgetKey::Escape),
    ))
    .expect("active escape should emit one cancel");
    let [cancel] = complete_edit(&cancel).events() else {
        panic!("active pointer escape should emit one cancel event");
    };
    assert_eq!(cancel.phase, EditPhase::Cancel);
    assert_eq!(cancel.value, 7);
    assert_eq!(escape.input.value, 7);
    assert!(escape.input.pointer.is_none());
    assert_eq!(escape.input.interaction_gate.incumbent(), None);

    let mut focus_loss = pointer_u32_input(None, None, false);
    focus(&mut focus_loss.input);
    assert!(
        Widget::handle_input(
            &mut focus_loss.input,
            bounds,
            scrub_press(Point::new(0.0, 10.0), None),
        )
        .is_none()
    );
    assert!(
        complete_output(Widget::handle_input(
            &mut focus_loss.input,
            bounds,
            scrub_move(Point::new(100.0, 10.0), scrub_modifiers(), None, None),
        ))
        .is_some()
    );
    let cancelled = complete_output(Widget::handle_input(
        &mut focus_loss.input,
        bounds,
        WidgetInput::FocusChanged(false),
    ))
    .expect("focus loss should cancel the active pointer scrub");
    assert_eq!(
        complete_edit(&cancelled).events()[0].phase,
        EditPhase::Cancel
    );
    assert!(!focus_loss.input.text_input.common.state.focused);
    assert_eq!(focus_loss.input.value, 7);
    assert!(focus_loss.input.pointer.is_none());
}

#[test]
fn pointer_scrub_capture_loss_and_replacement_cancel_once() {
    let bounds = scrub_bounds();
    let mut capture_loss = pointer_u32_input(None, None, false);
    focus(&mut capture_loss.input);
    assert!(
        Widget::handle_input(
            &mut capture_loss.input,
            bounds,
            scrub_press(Point::new(0.0, 10.0), None),
        )
        .is_none()
    );
    assert!(
        complete_output(Widget::handle_input(
            &mut capture_loss.input,
            bounds,
            scrub_move(Point::new(100.0, 10.0), scrub_modifiers(), None, None),
        ))
        .is_some()
    );
    let cancelled = complete_output(Widget::handle_pointer_capture_cancelled(
        &mut capture_loss.input,
        bounds,
    ))
    .expect("capture loss should cancel the pointer transaction");
    assert_eq!(
        complete_edit(&cancelled).events()[0].phase,
        EditPhase::Cancel
    );
    assert_eq!(capture_loss.input.value, 7);
    assert_eq!(capture_loss.input.interaction_gate.incumbent(), None);
    assert!(Widget::handle_pointer_capture_cancelled(&mut capture_loss.input, bounds).is_none());

    let mut previous = pointer_u32_input(None, None, false);
    focus(&mut previous.input);
    assert!(
        Widget::handle_input(
            &mut previous.input,
            bounds,
            scrub_press(Point::new(0.0, 10.0), None),
        )
        .is_none()
    );
    assert!(
        complete_output(Widget::handle_input(
            &mut previous.input,
            bounds,
            scrub_move(Point::new(100.0, 10.0), scrub_modifiers(), None, None),
        ))
        .is_some()
    );
    let mut successor = pointer_u32_input(None, None, false);
    successor.input.value = previous.input.value;
    successor.input.text_input.state.value = previous.input.text_input.state.value.clone();
    assert!(
        Widget::prepare_replacement(&mut previous.input, Some(&successor.input as &dyn Widget),)
            .is_none()
    );
    let mut synchronized = successor;
    Widget::synchronize_from_previous(&mut synchronized.input, &previous.input);
    assert!(synchronized.input.pointer.is_some());
    assert_eq!(
        synchronized.input.interaction_gate.incumbent(),
        Some(NumericInteractionOwner::PointerScrub)
    );
    assert_eq!(synchronized.input.value, 17);

    let mut changed = pointer_u32_input(None, None, false);
    changed.input.value = 18;
    changed.input.text_input.state.value = "18".to_owned();
    let cancellation = complete_output(Widget::prepare_replacement(
        &mut synchronized.input,
        Some(&changed.input as &dyn Widget),
    ))
    .expect("changed external authority should cancel, not rebase");
    assert_eq!(
        complete_edit(&cancellation).events()[0].phase,
        EditPhase::Cancel
    );
    assert_eq!(synchronized.input.value, 7);
    assert!(synchronized.input.pointer.is_none());
    assert_eq!(synchronized.input.interaction_gate.incumbent(), None);
}

#[test]
fn pointer_scrub_initial_and_active_adjustment_failures_are_typed_and_rollback() {
    let bounds = scrub_bounds();
    let mut initial = pointer_u32_input(Some(1), None, false);
    focus(&mut initial.input);
    let timestamp = Some(InputTimestamp::capture());
    assert!(
        Widget::handle_input(
            &mut initial.input,
            bounds,
            scrub_press(Point::new(0.0, 10.0), None),
        )
        .is_none()
    );
    let failure = complete_output(Widget::handle_input(
        &mut initial.input,
        bounds,
        scrub_move(Point::new(100.0, 10.0), scrub_modifiers(), timestamp, None),
    ))
    .expect("initial adjustment failure should be typed");
    assert_eq!(failure.len(), 1);
    let [
        NumericInputInteraction::ScrubFailed {
            attempt,
            normalized_delta,
            step,
            provenance,
            cancelled,
            ..
        },
    ] = failure.parts()
    else {
        panic!("expected one initial pointer adjustment failure");
    };
    assert_eq!(*attempt, NumericScrubAttempt::Initial);
    assert_eq!(*normalized_delta, 1.0);
    assert_eq!(*step, NumericStep::Base);
    assert!(!*cancelled);
    assert_eq!(
        *provenance,
        InteractionProvenance::Pointer {
            modifiers: scrub_modifiers(),
            timestamp,
            sequence_range: None,
        }
    );
    assert_eq!(initial.input.value, 7);
    assert!(initial.input.pointer.is_none());
    assert_eq!(initial.input.interaction_gate.incumbent(), None);
    assert_eq!(initial.scrub_calls.get(), 1);
    assert_eq!(initial.format_calls.get(), 1);

    let mut active = pointer_u32_input(Some(2), None, false);
    focus(&mut active.input);
    assert!(
        Widget::handle_input(
            &mut active.input,
            bounds,
            scrub_press(Point::new(0.0, 10.0), None),
        )
        .is_none()
    );
    assert!(
        complete_output(Widget::handle_input(
            &mut active.input,
            bounds,
            scrub_move(Point::new(100.0, 10.0), scrub_modifiers(), None, None),
        ))
        .is_some()
    );
    let timestamp = Some(InputTimestamp::capture());
    let failure = complete_output(Widget::handle_input(
        &mut active.input,
        bounds,
        scrub_move(Point::new(0.0, 10.0), scrub_modifiers(), timestamp, None),
    ))
    .expect("active adjustment failure should cancel before the typed failure");
    assert_eq!(failure.len(), 2);
    let [
        NumericInputInteraction::Edit(cancel),
        NumericInputInteraction::ScrubFailed {
            attempt,
            normalized_delta,
            step,
            provenance,
            cancelled,
            ..
        },
    ] = failure.parts()
    else {
        panic!("expected Cancel followed by an update adjustment failure");
    };
    assert_eq!(cancel.events()[0].phase, EditPhase::Cancel);
    assert_eq!(cancel.events()[0].value, 7);
    assert_eq!(*attempt, NumericScrubAttempt::Update);
    assert_eq!(*normalized_delta, -1.0);
    assert_eq!(*step, NumericStep::Base);
    assert!(*cancelled);
    assert_eq!(
        *provenance,
        InteractionProvenance::Pointer {
            modifiers: scrub_modifiers(),
            timestamp,
            sequence_range: None,
        }
    );
    assert_eq!(active.input.value, 7);
    assert!(active.input.pointer.is_none());
    assert_eq!(active.input.interaction_gate.incumbent(), None);
}

#[test]
fn pointer_scrub_initial_and_active_format_failures_are_typed_and_rollback() {
    let bounds = scrub_bounds();
    let mut initial = pointer_u32_input(None, Some(2), false);
    focus(&mut initial.input);
    assert!(
        Widget::handle_input(
            &mut initial.input,
            bounds,
            scrub_press(Point::new(0.0, 10.0), None),
        )
        .is_none()
    );
    let failure = complete_output(Widget::handle_input(
        &mut initial.input,
        bounds,
        scrub_move(Point::new(100.0, 10.0), scrub_modifiers(), None, None),
    ))
    .expect("initial format failure should be typed");
    assert_eq!(failure.len(), 1);
    let [
        NumericInputInteraction::PointerFormatFailed {
            attempt,
            normalized_delta,
            step,
            cancelled,
            provenance,
            ..
        },
    ] = failure.parts()
    else {
        panic!("expected one initial pointer format failure");
    };
    assert_eq!(*attempt, NumericScrubAttempt::Initial);
    assert_eq!(*normalized_delta, 1.0);
    assert_eq!(*step, NumericStep::Base);
    assert!(!*cancelled);
    assert_eq!(provenance.source(), InteractionSource::Pointer);
    assert_eq!(initial.input.value, 7);
    assert!(initial.input.pointer.is_none());
    assert_eq!(initial.input.interaction_gate.incumbent(), None);

    let mut active = pointer_u32_input(None, Some(3), false);
    focus(&mut active.input);
    assert!(
        Widget::handle_input(
            &mut active.input,
            bounds,
            scrub_press(Point::new(0.0, 10.0), None),
        )
        .is_none()
    );
    assert!(
        complete_output(Widget::handle_input(
            &mut active.input,
            bounds,
            scrub_move(Point::new(100.0, 10.0), scrub_modifiers(), None, None),
        ))
        .is_some()
    );
    let timestamp = Some(InputTimestamp::capture());
    let failure = complete_output(Widget::handle_input(
        &mut active.input,
        bounds,
        scrub_move(Point::new(0.0, 10.0), scrub_modifiers(), timestamp, None),
    ))
    .expect("active format failure should cancel before the typed failure");
    assert_eq!(failure.len(), 2);
    let [
        NumericInputInteraction::Edit(cancel),
        NumericInputInteraction::PointerFormatFailed {
            attempt,
            normalized_delta,
            step,
            provenance,
            cancelled,
            ..
        },
    ] = failure.parts()
    else {
        panic!("expected Cancel followed by an update format failure");
    };
    assert_eq!(cancel.events()[0].phase, EditPhase::Cancel);
    assert_eq!(*attempt, NumericScrubAttempt::Update);
    assert_eq!(*normalized_delta, -1.0);
    assert_eq!(*step, NumericStep::Base);
    assert!(*cancelled);
    assert_eq!(provenance.source(), InteractionSource::Pointer);
    assert_eq!(active.input.value, 7);
    assert!(active.input.pointer.is_none());
    assert_eq!(active.input.interaction_gate.incumbent(), None);
}

#[test]
fn pointer_scrub_rejects_invalid_press_and_move_geometry_without_policy_calls() {
    let mut fixture = pointer_u32_input(None, None, false);
    focus(&mut fixture.input);
    let valid_position = Point::new(0.0, 10.0);
    let invalid_bounds = Rect::from_min_size(Point::default(), Vector2::new(0.0, 20.0));
    let press = scrub_press(valid_position, None);
    assert_eq!(
        Widget::preflight_pointer_press(&fixture.input, invalid_bounds, &press),
        PointerPressAdmission::Legacy
    );
    assert!(Widget::handle_input(&mut fixture.input, invalid_bounds, press).is_none());
    assert!(fixture.input.pointer.is_none());
    assert_eq!(fixture.scrub_calls.get(), 0);

    let bounds = scrub_bounds();
    assert!(
        Widget::handle_input(
            &mut fixture.input,
            bounds,
            scrub_press(valid_position, None),
        )
        .is_none()
    );
    assert!(
        Widget::handle_input(
            &mut fixture.input,
            bounds,
            scrub_move(Point::new(f32::NAN, 10.0), scrub_modifiers(), None, None),
        )
        .is_none()
    );
    assert!(
        Widget::handle_input(
            &mut fixture.input,
            bounds,
            scrub_move(Point::new(101.0, 10.0), scrub_modifiers(), None, None),
        )
        .is_none()
    );
    assert_eq!(fixture.scrub_calls.get(), 0);
    assert_eq!(fixture.format_calls.get(), 1);
    assert_eq!(fixture.input.value, 7);
    assert_eq!(
        fixture.input.pointer.as_ref().unwrap().anchor_position,
        valid_position
    );
}

#[test]
fn pointer_scrub_wheel_and_non_pointer_inputs_remain_unhandled() {
    let bounds = scrub_bounds();
    let mut fixture = pointer_u32_input(None, None, false);
    focus(&mut fixture.input);
    assert!(
        Widget::handle_input(
            &mut fixture.input,
            bounds,
            WidgetInput::Wheel {
                position: Point::new(10.0, 10.0),
                delta: Vector2::new(1.0, 2.0),
                modifiers: scrub_modifiers(),
                timestamp: Some(InputTimestamp::capture()),
                sequence_range: None,
            },
        )
        .is_none()
    );
    assert!(fixture.input.pointer.is_none());
    assert_eq!(fixture.scrub_calls.get(), 0);
    assert_eq!(fixture.format_calls.get(), 1);

    assert!(
        Widget::handle_input(
            &mut fixture.input,
            bounds,
            WidgetInput::PointerModifiersChanged {
                modifiers: scrub_modifiers(),
                timestamp: None,
            },
        )
        .is_none()
    );
    assert!(fixture.input.pointer.is_none());
    assert_eq!(fixture.scrub_calls.get(), 0);
}

#[test]
fn complete_wheel_consumes_exact_units_atomically_and_keeps_legacy_vectors_unhandled() {
    let bounds = wheel_bounds();
    let position = Point::new(40.0, 14.0);
    let timestamp = Some(InputTimestamp::capture());
    let sequence_range = Some(InputSequenceRange::singleton(
        InputSequence::from_runtime_value(17),
    ));
    let modifiers = PointerModifiers::default();
    let mut fixture = wheel_u32_input(None, None, false);
    focus(&mut fixture.input);
    assert!(Widget::accepts_wheel_input(&fixture.input));

    let atomic = complete_output(Widget::handle_wheel_sample(
        &mut fixture.input,
        bounds,
        position,
        exact_wheel_sample(
            WheelDelta::pixels(Vector2::new(0.0, 40.0)).unwrap(),
            None,
            modifiers,
            timestamp,
            sequence_range,
        ),
    ))
    .expect("an effective phase-less sample should be atomic");
    let edit = complete_wheel_edit(&atomic);
    assert_eq!(edit.events().len(), 3);
    assert_eq!(
        edit.events()
            .iter()
            .map(|event| event.phase)
            .collect::<Vec<_>>(),
        vec![EditPhase::Begin, EditPhase::Update, EditPhase::Commit]
    );
    assert!(
        edit.events()
            .iter()
            .all(|event| event.transaction == edit.transaction())
    );
    assert!(edit.events().iter().all(|event| {
        event.provenance
            == InteractionProvenance::Pointer {
                modifiers,
                timestamp,
                sequence_range,
            }
    }));
    assert_eq!(fixture.last_delta.get(), 1.0);
    assert_eq!(fixture.last_step.get(), Some(NumericStep::Base));
    assert_eq!(fixture.wheel_calls.get(), 1);
    assert_eq!(fixture.input.value, 8);

    let legacy_before = fixture.input.value;
    assert!(
        Widget::handle_input(
            &mut fixture.input,
            bounds,
            WidgetInput::Wheel {
                position,
                delta: Vector2::new(0.0, 40.0),
                modifiers,
                timestamp,
                sequence_range,
            },
        )
        .is_none()
    );
    assert_eq!(fixture.input.value, legacy_before);
    assert_eq!(fixture.wheel_calls.get(), 1);

    let fine = complete_output(Widget::handle_wheel_sample(
        &mut fixture.input,
        bounds,
        position,
        exact_wheel_sample(
            WheelDelta::lines(Vector2::new(0.0, 1.0)).unwrap(),
            Some(WheelPhase::Discrete),
            PointerModifiers {
                shift: true,
                ..PointerModifiers::default()
            },
            None,
            None,
        ),
    ))
    .expect("a discrete line sample should remain an exact atomic edit");
    assert_eq!(complete_wheel_edit(&fine).events().len(), 3);
    assert_eq!(fixture.last_delta.get(), 1.0);
    assert_eq!(fixture.last_step.get(), Some(NumericStep::Fine));
}

#[test]
fn complete_wheel_ignores_orphan_phaseful_samples_without_policy_or_state_changes() {
    let bounds = wheel_bounds();
    let position = Point::new(40.0, 14.0);
    let mut fixture = wheel_u32_input(None, None, false);
    focus(&mut fixture.input);
    let original_text = fixture.input.text_input.state.clone();

    for phase in [
        WheelPhase::Changed,
        WheelPhase::Ended,
        WheelPhase::Cancelled,
    ] {
        assert!(
            Widget::handle_wheel_sample(
                &mut fixture.input,
                bounds,
                position,
                exact_wheel_sample(
                    WheelDelta::pixels(Vector2::new(0.0, 40.0)).unwrap(),
                    Some(phase),
                    PointerModifiers::default(),
                    None,
                    None,
                ),
            )
            .is_none(),
            "orphan {phase:?} must remain unhandled"
        );
    }

    assert_eq!(fixture.wheel_calls.get(), 0);
    assert_eq!(fixture.format_calls.get(), 1);
    assert_eq!(fixture.input.value, 7);
    assert_eq!(fixture.input.text_input.state, original_text);
    assert!(fixture.input.wheel.is_none());
    assert_eq!(fixture.input.interaction_gate.incumbent(), None);
}

#[test]
fn complete_wheel_explicit_sequence_preserves_transaction_metadata_and_pending_end() {
    let bounds = wheel_bounds();
    let position = Point::new(40.0, 14.0);
    let started_timestamp = Some(InputTimestamp::capture());
    let changed_timestamp = Some(InputTimestamp::capture());
    let started_range = Some(InputSequenceRange::singleton(
        InputSequence::from_runtime_value(20),
    ));
    let changed_range = Some(InputSequenceRange::singleton(
        InputSequence::from_runtime_value(21),
    ));
    let modifiers = PointerModifiers::default();
    let mut fixture = wheel_u32_input(None, None, false);
    focus(&mut fixture.input);

    assert!(
        Widget::handle_wheel_sample(
            &mut fixture.input,
            bounds,
            position,
            exact_wheel_sample(
                WheelDelta::lines(Vector2::new(0.0, 1.0)).unwrap(),
                Some(WheelPhase::Started),
                modifiers,
                started_timestamp,
                started_range,
            ),
        )
        .is_none()
    );
    assert_eq!(
        fixture.input.interaction_gate.incumbent(),
        Some(NumericInteractionOwner::WheelSequence)
    );
    assert_eq!(fixture.wheel_calls.get(), 0);

    let first = complete_output(Widget::handle_wheel_sample(
        &mut fixture.input,
        bounds,
        position,
        exact_wheel_sample(
            WheelDelta::lines(Vector2::new(0.0, 1.0)).unwrap(),
            Some(WheelPhase::Changed),
            modifiers,
            changed_timestamp,
            changed_range,
        ),
    ))
    .expect("first changed sample should begin the retained edit");
    let first_edit = complete_wheel_edit(&first);
    assert_eq!(first_edit.events().len(), 2);
    assert_eq!(first_edit.events()[0].phase, EditPhase::Begin);
    assert_eq!(first_edit.events()[1].phase, EditPhase::Update);
    assert_eq!(
        first_edit.events()[0].provenance,
        InteractionProvenance::Pointer {
            modifiers,
            timestamp: started_timestamp,
            sequence_range: started_range,
        }
    );
    assert_eq!(
        first_edit.events()[1].provenance,
        InteractionProvenance::Pointer {
            modifiers,
            timestamp: changed_timestamp,
            sequence_range: changed_range,
        }
    );

    let second = complete_output(Widget::handle_wheel_sample(
        &mut fixture.input,
        bounds,
        position,
        exact_wheel_sample(
            WheelDelta::lines(Vector2::new(0.0, -1.0)).unwrap(),
            Some(WheelPhase::Changed),
            PointerModifiers {
                command: true,
                ..PointerModifiers::default()
            },
            None,
            None,
        ),
    ))
    .expect("a changed candidate should emit one update");
    let second_edit = complete_wheel_edit(&second);
    assert_eq!(second_edit.events().len(), 1);
    assert_eq!(second_edit.events()[0].phase, EditPhase::Update);
    assert_eq!(fixture.input.value, 0);
    assert_eq!(fixture.last_step.get(), Some(NumericStep::Coarse));

    let ended = complete_output(Widget::handle_wheel_sample(
        &mut fixture.input,
        bounds,
        position,
        exact_wheel_sample(
            WheelDelta::pixels(Vector2::new(0.0, 0.0)).unwrap(),
            Some(WheelPhase::Ended),
            modifiers,
            None,
            None,
        ),
    ))
    .expect("Ended should commit the active edit");
    let commit = complete_wheel_edit(&ended);
    assert_eq!(commit.events().len(), 1);
    assert_eq!(commit.events()[0].phase, EditPhase::Commit);
    assert!(matches!(
        commit.events()[0].provenance,
        InteractionProvenance::Pointer {
            timestamp: None,
            sequence_range: None,
            ..
        }
    ));
    assert_eq!(fixture.input.interaction_gate.incumbent(), None);

    let mut pending = wheel_u32_input(None, None, false);
    focus(&mut pending.input);
    assert!(
        Widget::handle_wheel_sample(
            &mut pending.input,
            bounds,
            position,
            exact_wheel_sample(
                WheelDelta::lines(Vector2::new(0.0, 1.0)).unwrap(),
                Some(WheelPhase::Started),
                modifiers,
                None,
                None,
            ),
        )
        .is_none()
    );
    assert!(
        Widget::handle_wheel_sample(
            &mut pending.input,
            bounds,
            position,
            exact_wheel_sample(
                WheelDelta::pixels(Vector2::new(0.0, 0.0)).unwrap(),
                Some(WheelPhase::Ended),
                modifiers,
                None,
                None,
            ),
        )
        .is_none()
    );
    assert_eq!(pending.wheel_calls.get(), 0);
    assert_eq!(pending.input.interaction_gate.incumbent(), None);
}

#[test]
fn complete_wheel_rejects_unusable_or_conflicting_samples_and_unchanged_atomic_values() {
    let bounds = wheel_bounds();
    let position = Point::new(40.0, 14.0);
    let mut fixture = wheel_u32_input(None, None, true);
    focus(&mut fixture.input);
    for sample in [
        exact_wheel_sample(
            WheelDelta::pixels(Vector2::new(0.0, 0.0)).unwrap(),
            None,
            PointerModifiers::default(),
            None,
            None,
        ),
        exact_wheel_sample(
            WheelDelta::pixels(Vector2::new(4.0, 0.0)).unwrap(),
            Some(WheelPhase::Discrete),
            PointerModifiers::default(),
            None,
            None,
        ),
        WheelSample::from_parts(
            WheelDelta::Pixels(Vector2::new(f32::NAN, 1.0)),
            None,
            PointerModifiers::default(),
            None,
            None,
        ),
    ] {
        assert!(
            Widget::handle_wheel_sample(&mut fixture.input, bounds, position, sample).is_none()
        );
    }
    assert_eq!(fixture.wheel_calls.get(), 0);
    assert_eq!(fixture.format_calls.get(), 1);
    assert_eq!(fixture.input.value, 7);

    assert!(
        fixture
            .input
            .interaction_gate
            .try_admit(NumericInteractionOwner::TextEdit)
    );
    assert!(
        Widget::handle_wheel_sample(
            &mut fixture.input,
            bounds,
            position,
            exact_wheel_sample(
                WheelDelta::lines(Vector2::new(0.0, 1.0)).unwrap(),
                None,
                PointerModifiers::default(),
                None,
                None,
            ),
        )
        .is_none()
    );
    assert_eq!(fixture.wheel_calls.get(), 0);
    assert_eq!(
        fixture.input.interaction_gate.incumbent(),
        Some(NumericInteractionOwner::TextEdit)
    );

    let mut unconfigured = u32_input();
    unconfigured.set_complete_output_mode();
    focus(&mut unconfigured);
    assert!(!Widget::accepts_wheel_input(&unconfigured));
    assert!(
        Widget::handle_wheel_sample(
            &mut unconfigured,
            bounds,
            position,
            exact_wheel_sample(
                WheelDelta::lines(Vector2::new(0.0, 1.0)).unwrap(),
                None,
                PointerModifiers::default(),
                None,
                None,
            ),
        )
        .is_none()
    );
}

#[test]
fn complete_wheel_initial_and_active_failures_are_typed_with_ordered_rollback() {
    let bounds = wheel_bounds();
    let position = Point::new(40.0, 14.0);
    let sample = |phase| {
        exact_wheel_sample(
            WheelDelta::lines(Vector2::new(0.0, 1.0)).unwrap(),
            phase,
            PointerModifiers::default(),
            None,
            None,
        )
    };

    let mut initial_adjustment = wheel_u32_input(Some(1), None, false);
    focus(&mut initial_adjustment.input);
    let failure = complete_output(Widget::handle_wheel_sample(
        &mut initial_adjustment.input,
        bounds,
        position,
        sample(None),
    ))
    .expect("initial wheel adjustment failure should be typed");
    assert_eq!(failure.len(), 1);
    let [
        NumericInputInteraction::WheelFailed {
            attempt,
            delta,
            step,
            provenance,
            cancelled,
            ..
        },
    ] = failure.parts()
    else {
        panic!("expected typed initial wheel adjustment failure");
    };
    assert_eq!(*attempt, NumericWheelAttempt::Initial);
    assert_eq!(*delta, 1.0);
    assert_eq!(*step, NumericStep::Base);
    assert_eq!(provenance.source(), InteractionSource::Pointer);
    assert!(!cancelled);
    assert_eq!(initial_adjustment.input.value, 7);
    assert_eq!(initial_adjustment.input.interaction_gate.incumbent(), None);

    let mut initial_format = wheel_u32_input(None, Some(2), false);
    focus(&mut initial_format.input);
    let failure = complete_output(Widget::handle_wheel_sample(
        &mut initial_format.input,
        bounds,
        position,
        sample(None),
    ))
    .expect("initial wheel format failure should be typed");
    assert!(matches!(
        failure.parts(),
        [NumericInputInteraction::WheelFormatFailed {
            attempt: NumericWheelAttempt::Initial,
            cancelled: false,
            ..
        }]
    ));
    assert_eq!(initial_format.input.value, 7);
    assert_eq!(initial_format.input.text_input.state.value, "7");

    let mut active_adjustment = wheel_u32_input(Some(2), None, false);
    focus(&mut active_adjustment.input);
    assert!(
        Widget::handle_wheel_sample(
            &mut active_adjustment.input,
            bounds,
            position,
            sample(Some(WheelPhase::Started)),
        )
        .is_none()
    );
    assert!(
        Widget::handle_wheel_sample(
            &mut active_adjustment.input,
            bounds,
            position,
            sample(Some(WheelPhase::Changed)),
        )
        .is_some()
    );
    let failure = complete_output(Widget::handle_wheel_sample(
        &mut active_adjustment.input,
        bounds,
        position,
        sample(Some(WheelPhase::Changed)),
    ))
    .expect("active wheel failure should include rollback");
    assert_eq!(failure.len(), 2);
    let [
        NumericInputInteraction::Edit(cancel),
        NumericInputInteraction::WheelFailed {
            attempt, cancelled, ..
        },
    ] = failure.parts()
    else {
        panic!("expected cancel before active wheel failure");
    };
    assert_eq!(cancel.events().len(), 1);
    assert_eq!(cancel.events()[0].phase, EditPhase::Cancel);
    assert_eq!(cancel.events()[0].value, 7);
    assert_eq!(*attempt, NumericWheelAttempt::Update);
    assert!(*cancelled);
    assert_eq!(active_adjustment.input.value, 7);
    assert_eq!(active_adjustment.input.text_input.state.value, "7");
    assert!(active_adjustment.input.wheel.is_none());
    assert_eq!(active_adjustment.input.interaction_gate.incumbent(), None);

    let mut active_format = wheel_u32_input(None, Some(3), false);
    focus(&mut active_format.input);
    assert!(
        Widget::handle_wheel_sample(
            &mut active_format.input,
            bounds,
            position,
            sample(Some(WheelPhase::Started)),
        )
        .is_none()
    );
    assert!(
        Widget::handle_wheel_sample(
            &mut active_format.input,
            bounds,
            position,
            sample(Some(WheelPhase::Changed)),
        )
        .is_some()
    );
    let failure = complete_output(Widget::handle_wheel_sample(
        &mut active_format.input,
        bounds,
        position,
        sample(Some(WheelPhase::Changed)),
    ))
    .expect("active wheel format failure should include rollback");
    assert!(matches!(
        failure.parts(),
        [
            NumericInputInteraction::Edit(cancel),
            NumericInputInteraction::WheelFormatFailed {
                attempt: NumericWheelAttempt::Update,
                cancelled: true,
                ..
            }
        ] if cancel.events()[0].phase == EditPhase::Cancel
    ));
    assert_eq!(active_format.input.value, 7);
    assert_eq!(active_format.input.text_input.state.value, "7");
}

#[test]
fn complete_wheel_escape_focus_loss_and_compatible_reprojection_restore_or_preserve_state() {
    let bounds = wheel_bounds();
    let position = Point::new(40.0, 14.0);
    let sample = |phase| {
        exact_wheel_sample(
            WheelDelta::lines(Vector2::new(0.0, 1.0)).unwrap(),
            phase,
            PointerModifiers::default(),
            None,
            None,
        )
    };

    let mut escape = wheel_u32_input(None, None, false);
    escape.input.set_selection(0, 1);
    focus(&mut escape.input);
    let original = escape.input.text_input.state.clone();
    assert!(
        Widget::handle_wheel_sample(
            &mut escape.input,
            bounds,
            position,
            sample(Some(WheelPhase::Started)),
        )
        .is_none()
    );
    assert!(
        Widget::handle_wheel_sample(
            &mut escape.input,
            bounds,
            position,
            sample(Some(WheelPhase::Changed)),
        )
        .is_some()
    );
    let cancel = complete_output(Widget::handle_input(
        &mut escape.input,
        bounds,
        WidgetInput::key_press(WidgetKey::Escape),
    ))
    .expect("Escape should cancel an active wheel edit");
    assert_eq!(
        complete_wheel_edit(&cancel).events()[0].phase,
        EditPhase::Cancel
    );
    assert_eq!(escape.input.value, 7);
    assert_eq!(escape.input.text_input.state, original);

    let mut focus_loss = wheel_u32_input(None, None, false);
    focus(&mut focus_loss.input);
    assert!(
        Widget::handle_wheel_sample(
            &mut focus_loss.input,
            bounds,
            position,
            sample(Some(WheelPhase::Started)),
        )
        .is_none()
    );
    assert!(
        Widget::handle_wheel_sample(
            &mut focus_loss.input,
            bounds,
            position,
            sample(Some(WheelPhase::Changed)),
        )
        .is_some()
    );
    let cancel = complete_output(Widget::handle_input(
        &mut focus_loss.input,
        bounds,
        WidgetInput::FocusChanged(false),
    ))
    .expect("focus loss should cancel an active wheel edit");
    assert_eq!(
        complete_wheel_edit(&cancel).events()[0].phase,
        EditPhase::Cancel
    );
    assert_eq!(focus_loss.input.value, 7);
    assert_eq!(focus_loss.input.interaction_gate.incumbent(), None);
    assert!(matches!(
        complete_wheel_edit(&cancel).events()[0].provenance,
        InteractionProvenance::Pointer {
            timestamp: None,
            sequence_range: None,
            ..
        }
    ));

    let mut reprojection = wheel_u32_input(None, None, false);
    focus(&mut reprojection.input);
    assert!(
        Widget::handle_wheel_sample(
            &mut reprojection.input,
            bounds,
            position,
            sample(Some(WheelPhase::Started)),
        )
        .is_none()
    );
    let previous = reprojection.input.clone();
    let mut successor = reprojection.input.clone();
    Widget::synchronize_from_previous(&mut successor, &previous);
    assert_eq!(
        successor.interaction_gate.incumbent(),
        Some(NumericInteractionOwner::WheelSequence)
    );
    assert!(
        Widget::handle_wheel_sample(
            &mut successor,
            bounds,
            position,
            sample(Some(WheelPhase::Changed)),
        )
        .is_some()
    );
    assert!(successor.wheel.is_some());
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

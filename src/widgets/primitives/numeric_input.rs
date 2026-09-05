//! Text-first generic numeric input built on the retained text-input primitive.

mod keyboard;
mod pointer;
mod wheel;

#[cfg(test)]
mod tests;

use std::{fmt, rc::Rc};

use self::keyboard::{
    KeyboardAdjustmentPolicy, KeyboardAdjustmentRequest, KeyboardAdjustmentState,
    complete_keyboard_adjustment_policy, direction_for_key, is_adjustment_key,
    no_keyboard_adjustment_policy,
};
use self::pointer::{
    PointerScrubOutputPolicy, PointerScrubRequest, PointerScrubState,
    complete_pointer_scrub_output_policy, normalized_delta, pointer_provenance, select_step,
    valid_geometry, without_activation_modifier,
};
use self::wheel::{
    WheelOutputPolicy, WheelRequest, WheelSequenceState, admits_position,
    complete_wheel_output_policy, is_configured as wheel_policy_is_configured, selected_step,
    wheel_delta, wheel_provenance,
};

use crate::{
    gui::types::{Point, Rect},
    layout::LayoutOutput,
    runtime::{PaintPrimitive, ResolvedEnvironment},
    theme::ThemeTokens,
    widgets::{
        CompositionRange, CompositionSample, CompositionSelectionState, EditEvent,
        FocusLossDecision, FocusedKeyDisposition, InteractionProvenance,
        NumericAccessibilityAction, NumericAccessibilityBlockOwner, NumericAccessibilityOutcome,
        NumericAccessibilityRejectedReason, NumericAdjustment, NumericCodec, NumericEditSession,
        NumericInputConstructionError, NumericInputEditBatch, NumericInputInteractionBatch,
        NumericParseResult, NumericScrubAttempt, NumericScrubPolicy, NumericStep,
        NumericStepAttempt, NumericStepDirection, NumericStepModifiers, NumericWheelAttempt,
        NumericWheelPolicy, PointerButton, PointerModifiers, PointerPressAdmission, TextAlign,
        TextBackgroundRole, TextColorRole, TextInputChrome, TextInputState, TextInputWidget,
        TextScaleParticipation, TextWrap, WheelPhase, WheelSample, Widget, WidgetCapabilities,
        WidgetInput, WidgetKey, WidgetOutput, WidgetPaintContext, WidgetPointerMotion,
        WidgetPointerMotionRevision, WidgetSemantics, WidgetSizing,
        interaction::{NumericInteractionGate, NumericInteractionOwner},
    },
};

type NumericInputOutputEncoder<T> = Rc<dyn Fn(NumericInputEditBatch<T>) -> WidgetOutput>;
type NumericAccessibilityActionHandler<T, C, A> = Rc<
    dyn Fn(&mut NumericInputWidget<T, C, A>, NumericAccessibilityAction) -> Option<WidgetOutput>,
>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NumericInputOutputMode {
    Compatibility,
    Complete,
}

fn encode_compatibility_output<T: Clone + 'static>(
    batch: NumericInputEditBatch<T>,
) -> WidgetOutput {
    WidgetOutput::typed(batch)
}

fn encode_complete_output<T, StepError, FormatError>(
    batch: NumericInputEditBatch<T>,
) -> WidgetOutput
where
    T: Clone + 'static,
    StepError: 'static,
    FormatError: 'static,
{
    WidgetOutput::typed(NumericInputInteractionBatch::<T, StepError, FormatError>::from_edit(batch))
}

fn compatibility_output_encoder<T: Clone + 'static>() -> NumericInputOutputEncoder<T> {
    Rc::new(encode_compatibility_output::<T>)
}

struct ActiveNumericEdit<T, C> {
    session: NumericEditSession<T>,
    codec: Rc<C>,
    draft_result: Option<NumericParseResult<T>>,
    start_text: String,
    start_caret: usize,
    start_selection_anchor: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct NumericInputComposition {
    original_value: String,
    replacement_range: CompositionRange,
    original_selection: CompositionRange,
    preedit: String,
    preedit_selection: CompositionSelectionState,
}

fn byte_index_for_char(value: &str, char_index: usize) -> usize {
    value
        .char_indices()
        .nth(char_index)
        .map_or(value.len(), |(byte_index, _)| byte_index)
}

fn composition_display_value(
    original_value: &str,
    replacement_range: CompositionRange,
    replacement: &str,
) -> String {
    let start = byte_index_for_char(original_value, replacement_range.start());
    let end = byte_index_for_char(original_value, replacement_range.end());
    let mut value = String::with_capacity(
        original_value
            .len()
            .saturating_sub(end.saturating_sub(start))
            + replacement.len(),
    );
    value.push_str(&original_value[..start]);
    value.push_str(replacement);
    value.push_str(&original_value[end..]);
    value
}

fn set_composition_selection(state: &mut TextInputState, selection: CompositionRange) {
    state.selection_anchor = selection.start();
    state.caret = selection.end();
}

fn composition_display_selection(
    replacement_range: CompositionRange,
    selection: CompositionRange,
) -> Option<CompositionRange> {
    let start = replacement_range.start() + selection.start();
    let end = replacement_range.start() + selection.end();
    let scalar_len = replacement_range
        .scalar_len()
        .saturating_sub(replacement_range.len())
        .saturating_add(selection.scalar_len());
    CompositionRange::new(start, end, scalar_len).ok()
}

impl<T: Clone, C> Clone for ActiveNumericEdit<T, C> {
    fn clone(&self) -> Self {
        Self {
            session: self.session.clone(),
            codec: Rc::clone(&self.codec),
            draft_result: self.draft_result.clone(),
            start_text: self.start_text.clone(),
            start_caret: self.start_caret,
            start_selection_anchor: self.start_selection_anchor,
        }
    }
}

/// UI-local numeric text consumer used by the application builder.
pub(crate) struct NumericInputWidget<T, C, A> {
    text_input: TextInputWidget,
    value: T,
    codec: Rc<C>,
    adjustment: Rc<A>,
    active: Option<ActiveNumericEdit<T, C>>,
    composition: Option<NumericInputComposition>,
    keyboard: Option<KeyboardAdjustmentState<T>>,
    pointer: Option<PointerScrubState<T>>,
    wheel: Option<WheelSequenceState<T>>,
    interaction_gate: NumericInteractionGate,
    step_modifiers: Option<NumericStepModifiers>,
    scrub_policy: Option<NumericScrubPolicy>,
    output_mode: NumericInputOutputMode,
    output_encoder: NumericInputOutputEncoder<T>,
    accessibility_action_handler: Option<NumericAccessibilityActionHandler<T, C, A>>,
    keyboard_policy: Rc<dyn KeyboardAdjustmentPolicy<T>>,
    pointer_policy: Option<Rc<dyn PointerScrubOutputPolicy<T>>>,
    wheel_policy: Option<NumericWheelPolicy>,
    wheel_output_policy: Option<Rc<dyn WheelOutputPolicy<T>>>,
}

impl<T, C, A> Clone for NumericInputWidget<T, C, A>
where
    T: Clone,
    C: 'static,
    A: 'static,
{
    fn clone(&self) -> Self {
        Self {
            text_input: self.text_input.clone(),
            value: self.value.clone(),
            codec: Rc::clone(&self.codec),
            adjustment: Rc::clone(&self.adjustment),
            active: self.active.clone(),
            composition: self.composition.clone(),
            keyboard: self.keyboard.clone(),
            pointer: self.pointer.clone(),
            wheel: self.wheel.clone(),
            interaction_gate: self.interaction_gate,
            step_modifiers: self.step_modifiers,
            scrub_policy: self.scrub_policy,
            output_mode: self.output_mode,
            output_encoder: Rc::clone(&self.output_encoder),
            accessibility_action_handler: self.accessibility_action_handler.as_ref().map(Rc::clone),
            keyboard_policy: Rc::clone(&self.keyboard_policy),
            pointer_policy: self.pointer_policy.as_ref().map(Rc::clone),
            wheel_policy: self.wheel_policy,
            wheel_output_policy: self.wheel_output_policy.as_ref().map(Rc::clone),
        }
    }
}

impl<T, C, A> fmt::Debug for NumericInputWidget<T, C, A>
where
    T: Clone + fmt::Debug,
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NumericInputWidget")
            .field("text_input", &self.text_input)
            .field("value", &self.value)
            .field(
                "active",
                &self.active.as_ref().map(|active| active.session.draft()),
            )
            .field("composition", &self.composition)
            .field(
                "keyboard",
                &self.keyboard.as_ref().map(|keyboard| keyboard.key),
            )
            .field(
                "pointer",
                &self.pointer.as_ref().map(|pointer| pointer.is_active()),
            )
            .field("wheel", &self.wheel.as_ref().map(|wheel| wheel.is_active()))
            .field("output_mode", &self.output_mode)
            .finish_non_exhaustive()
    }
}

impl<T, C, A> NumericInputWidget<T, C, A>
where
    T: Clone + PartialEq + 'static,
    C: NumericCodec<T> + 'static,
    A: NumericAdjustment<T> + 'static,
{
    pub(crate) fn try_new(
        value: T,
        codec: C,
        adjustment: A,
        sizing: WidgetSizing,
    ) -> Result<Self, NumericInputConstructionError<C::Error, A::Error>> {
        let codec = Rc::new(codec);
        let mut draft = String::new();
        codec
            .format_editable(&value, &mut draft)
            .map_err(|error| NumericInputConstructionError::CodecFormat { error })?;

        let adjustment = Rc::new(adjustment);
        adjustment.value_to_normalized(&value).map_err(|error| {
            NumericInputConstructionError::AdjustmentValueToNormalized { error }
        })?;

        Ok(Self {
            text_input: TextInputWidget::new(0, draft, sizing),
            value,
            codec,
            adjustment,
            active: None,
            composition: None,
            keyboard: None,
            pointer: None,
            wheel: None,
            interaction_gate: NumericInteractionGate::new(),
            step_modifiers: None,
            scrub_policy: None,
            output_mode: NumericInputOutputMode::Compatibility,
            output_encoder: compatibility_output_encoder(),
            accessibility_action_handler: None,
            keyboard_policy: no_keyboard_adjustment_policy(),
            pointer_policy: None,
            wheel_policy: None,
            wheel_output_policy: None,
        })
    }

    pub(crate) fn set_chrome(&mut self, chrome: TextInputChrome) {
        self.text_input.props.chrome = chrome;
    }

    pub(crate) fn set_selection(&mut self, anchor: usize, caret: usize) {
        self.text_input.state.selection_anchor = anchor;
        self.text_input.state.caret = caret;
    }

    pub(crate) fn select_all(&mut self) {
        let end = self.text_input.state.char_len();
        self.set_selection(0, end);
    }

    pub(crate) fn set_sizing(&mut self, sizing: WidgetSizing) {
        self.text_input.common.sizing = sizing;
    }

    pub(crate) fn set_step_modifiers(&mut self, policy: NumericStepModifiers) {
        self.step_modifiers = Some(policy);
    }

    pub(crate) fn set_scrub_policy(&mut self, policy: NumericScrubPolicy) {
        self.scrub_policy = Some(policy);
    }

    pub(crate) fn set_wheel_policy(&mut self, policy: NumericWheelPolicy) {
        self.wheel_policy = Some(policy);
    }

    pub(crate) fn set_compatibility_output_mode(&mut self) {
        self.output_mode = NumericInputOutputMode::Compatibility;
        self.output_encoder = compatibility_output_encoder();
        self.accessibility_action_handler = None;
        self.keyboard_policy = no_keyboard_adjustment_policy();
        self.pointer_policy = None;
        self.wheel_output_policy = None;
    }

    pub(crate) fn set_complete_output_mode(&mut self)
    where
        A::Error: 'static,
        C::Error: 'static,
    {
        self.output_mode = NumericInputOutputMode::Complete;
        self.output_encoder = Rc::new(encode_complete_output::<T, A::Error, C::Error>);
        self.accessibility_action_handler = None;
        self.keyboard_policy = complete_keyboard_adjustment_policy(
            Rc::clone(&self.codec),
            Rc::clone(&self.adjustment),
        );
        self.pointer_policy = Some(complete_pointer_scrub_output_policy(
            Rc::clone(&self.codec),
            Rc::clone(&self.adjustment),
        ));
        self.wheel_output_policy = Some(complete_wheel_output_policy(
            Rc::clone(&self.codec),
            Rc::clone(&self.adjustment),
        ));
    }

    pub(crate) fn set_accessibility_action_mode(&mut self)
    where
        A::Error: 'static,
        C::Error: 'static,
    {
        self.set_complete_output_mode();
        self.accessibility_action_handler = Some(Rc::new(|input, action| {
            NumericInputWidget::handle_accessibility_action(input, action).map(WidgetOutput::typed)
        }));
    }

    fn encode_output(&self, batch: NumericInputEditBatch<T>) -> WidgetOutput {
        (self.output_encoder)(batch)
    }

    /// Consume one discrete backend-neutral accessibility action locally.
    ///
    /// The runtime must resolve the current target, perform focus/authority
    /// admission, and reject stale, removed, or unmaterialized targets before
    /// calling this widget-local policy. This method never transfers focus,
    /// materializes a target, schedules work, or translates native action
    /// payloads. It is intentionally available only in complete interaction
    /// mode so typed adjustment and formatting outcomes remain aligned with
    /// the existing complete-mode policy; runtime output mapping remains a
    /// separate dispatch-boundary contract.
    #[allow(dead_code)]
    pub(crate) fn handle_accessibility_action(
        &mut self,
        action: NumericAccessibilityAction,
    ) -> Option<NumericAccessibilityOutcome<T, A::Error, C::Error>> {
        let rejected = |reason| NumericAccessibilityOutcome::Rejected {
            action: action.clone(),
            reason,
        };

        if self.output_mode != NumericInputOutputMode::Complete {
            return Some(rejected(
                NumericAccessibilityRejectedReason::UnsupportedAction,
            ));
        }
        if self.text_input.common.state.disabled {
            return Some(rejected(NumericAccessibilityRejectedReason::Disabled));
        }
        if self.text_input.common.state.read_only {
            return Some(rejected(NumericAccessibilityRejectedReason::ReadOnly));
        }
        if !self.text_input.common.state.focused {
            return Some(rejected(NumericAccessibilityRejectedReason::NotFocusable));
        }
        if let Some(owner) = self.interaction_gate.incumbent() {
            return Some(NumericAccessibilityOutcome::Blocked {
                owner: owner.into(),
            });
        }
        if !self
            .interaction_gate
            .try_admit(NumericInteractionOwner::AccessibilityEdit)
        {
            return self.interaction_gate.incumbent().map(|owner| {
                NumericAccessibilityOutcome::Blocked {
                    owner: owner.into(),
                }
            });
        }
        let owner = NumericInteractionOwner::AccessibilityEdit;

        let candidate = match &action {
            NumericAccessibilityAction::Increment => self.adjustment.step(
                &self.value,
                NumericStepDirection::Increase,
                NumericStep::Base,
            ),
            NumericAccessibilityAction::Decrement => self.adjustment.step(
                &self.value,
                NumericStepDirection::Decrease,
                NumericStep::Base,
            ),
            NumericAccessibilityAction::SetValueText(text) => match self.codec.parse(text) {
                NumericParseResult::Valid(value) => Ok(value),
                NumericParseResult::Incomplete => {
                    self.interaction_gate.release(owner);
                    return Some(rejected(NumericAccessibilityRejectedReason::Incomplete));
                }
                NumericParseResult::Invalid => {
                    self.interaction_gate.release(owner);
                    return Some(rejected(NumericAccessibilityRejectedReason::Invalid));
                }
                NumericParseResult::OutOfRange => {
                    self.interaction_gate.release(owner);
                    return Some(rejected(NumericAccessibilityRejectedReason::OutOfRange));
                }
            },
        };

        let candidate = match candidate {
            Ok(candidate) => candidate,
            Err(error) => {
                self.interaction_gate.release(owner);
                return Some(NumericAccessibilityOutcome::AdjustmentFailed {
                    action,
                    error: Rc::new(error),
                });
            }
        };

        if candidate == self.value {
            self.interaction_gate.release(owner);
            return Some(NumericAccessibilityOutcome::NoChange { action });
        }

        let mut draft = String::new();
        if let Err(error) = self.codec.format_editable(&candidate, &mut draft) {
            self.interaction_gate.release(owner);
            return Some(NumericAccessibilityOutcome::FormatFailed {
                action,
                error: Rc::new(error),
            });
        }

        let start_text = self.text_input.state.value.clone();
        let session = NumericEditSession::begin(
            self.value.clone(),
            start_text,
            InteractionProvenance::Accessibility,
        );
        let begin = session.begin_event().clone();
        let Some(update) = begin
            .clone()
            .update(candidate.clone(), InteractionProvenance::Accessibility)
        else {
            self.interaction_gate.release(owner);
            return None;
        };
        let Some(commit) = update
            .clone()
            .commit(candidate.clone(), InteractionProvenance::Accessibility)
        else {
            self.interaction_gate.release(owner);
            return None;
        };
        let Some(edit) = NumericInputEditBatch::from_events(&[begin, update, commit]) else {
            self.interaction_gate.release(owner);
            return None;
        };

        self.value = candidate;
        self.text_input.state.value = draft;
        let end = self.text_input.state.char_len();
        self.text_input.state.caret = end;
        self.text_input.state.selection_anchor = end;
        self.interaction_gate.release(owner);
        Some(NumericAccessibilityOutcome::Edit(edit))
    }

    fn is_editable(&self) -> bool {
        self.text_input.common.state.focused
            && !self.text_input.common.state.disabled
            && !self.text_input.common.state.read_only
    }

    fn scrub_is_configured(&self) -> bool {
        self.output_mode == NumericInputOutputMode::Complete
            && self.scrub_policy.is_some()
            && self.pointer_policy.is_some()
            && !self.text_input.common.state.disabled
            && !self.text_input.common.state.read_only
    }

    fn wheel_is_configured(&self) -> bool {
        self.output_mode == NumericInputOutputMode::Complete
            && wheel_policy_is_configured(self.wheel_policy)
            && self.wheel_output_policy.is_some()
    }

    fn default_pointer_provenance() -> InteractionProvenance {
        pointer_provenance(PointerModifiers::default(), None, None)
    }

    fn restore_pointer_start(&mut self, state: &PointerScrubState<T>) {
        debug_assert!(state.press_position.is_finite());
        self.value = state.start_value.clone();
        self.text_input.state.value = state.start_text.clone();
        self.text_input.state.caret = state.start_caret;
        self.text_input.state.selection_anchor = state.start_selection_anchor;
        self.text_input.common.state.pressed = false;
    }

    fn release_pointer_scrub(&mut self) {
        self.interaction_gate
            .release(NumericInteractionOwner::PointerScrub);
    }

    fn release_wheel_sequence(&mut self) {
        self.interaction_gate
            .release(NumericInteractionOwner::WheelSequence);
    }

    fn pointer_failure_rollback(
        state: &PointerScrubState<T>,
        provenance: InteractionProvenance,
    ) -> Option<NumericInputEditBatch<T>> {
        state
            .session
            .as_ref()?
            .begin_event()
            .clone()
            .cancel(provenance)
            .and_then(|cancel| NumericInputEditBatch::from_events(&[cancel]))
    }

    fn handle_pointer_press(
        &mut self,
        bounds: Rect,
        position: Point,
        button: PointerButton,
        modifiers: PointerModifiers,
        timestamp: Option<crate::gui::input::InputTimestamp>,
    ) -> Option<WidgetOutput> {
        let policy = self.scrub_policy?;
        if !self.scrub_is_configured()
            || !policy.admits(button, modifiers)
            || !valid_geometry(bounds, position)
            || self.interaction_gate.incumbent().is_some()
        {
            return None;
        }
        if !self
            .interaction_gate
            .try_admit(NumericInteractionOwner::PointerScrub)
        {
            return None;
        }

        let state = PointerScrubState::new(
            self.value.clone(),
            self.text_input.state.value.clone(),
            self.text_input.state.caret,
            self.text_input.state.selection_anchor,
            position,
            bounds,
            pointer_provenance(modifiers, timestamp, None),
            policy,
            modifiers,
        );
        self.text_input.common.state.focused = true;
        self.text_input.common.state.hovered = true;
        self.text_input.common.state.pressed = true;
        self.pointer = Some(state);
        None
    }

    fn handle_pointer_move(
        &mut self,
        position: Point,
        modifiers: PointerModifiers,
        timestamp: Option<crate::gui::input::InputTimestamp>,
        sequence_range: Option<crate::gui::input::InputSequenceRange>,
    ) -> Option<WidgetOutput> {
        let policy = self.pointer_policy.as_ref()?.clone();
        let mut state = self.pointer.take()?;
        if !valid_geometry(state.bounds, position) {
            self.pointer = Some(state);
            return None;
        }
        let Some(normalized_delta) =
            normalized_delta(state.bounds, state.anchor_position, position)
        else {
            self.pointer = Some(state);
            return None;
        };
        let scrub_modifiers = without_activation_modifier(modifiers, state.activation_modifier);
        let selected_step = select_step(scrub_modifiers);
        if selected_step != state.step {
            state.anchor_position = position;
            state.step = selected_step;
            self.pointer = Some(state);
            return None;
        }
        if normalized_delta == 0.0 {
            self.pointer = Some(state);
            return None;
        }

        let attempt = if state.is_active() {
            NumericScrubAttempt::Update
        } else {
            NumericScrubAttempt::Initial
        };
        let provenance = pointer_provenance(modifiers, timestamp, sequence_range);
        let candidate = match policy.scrub(PointerScrubRequest {
            value: &state.anchor_value,
            normalized_delta,
            step: state.step,
            attempt,
            provenance,
        }) {
            Ok(candidate) => candidate,
            Err(failure) => {
                let rollback = Self::pointer_failure_rollback(&state, provenance);
                let output = failure.into_output(rollback);
                self.restore_pointer_start(&state);
                self.pointer = None;
                self.release_pointer_scrub();
                return output;
            }
        };
        if candidate == state.anchor_value {
            self.pointer = Some(state);
            return None;
        }

        let mut draft = String::new();
        if let Err(failure) = policy.format(
            PointerScrubRequest {
                value: &candidate,
                normalized_delta,
                step: state.step,
                attempt,
                provenance,
            },
            &mut draft,
        ) {
            let rollback = Self::pointer_failure_rollback(&state, provenance);
            let output = failure.into_output(rollback);
            self.restore_pointer_start(&state);
            self.pointer = None;
            self.release_pointer_scrub();
            return output;
        }

        let edit = if state.session.is_none() {
            let session = crate::widgets::NumericEditSession::begin(
                state.start_value.clone(),
                state.start_text.clone(),
                state.press_provenance,
            );
            let begin = session.begin_event().clone();
            let Some(update) = begin.clone().update(candidate.clone(), provenance) else {
                self.pointer = Some(state);
                return None;
            };
            state.session = Some(session);
            NumericInputEditBatch::from_events(&[begin, update])
        } else {
            let Some(session) = state.session.as_ref() else {
                self.pointer = Some(state);
                return None;
            };
            let begin = session.begin_event().clone();
            let Some(update) = begin.update(candidate.clone(), provenance) else {
                self.pointer = Some(state);
                return None;
            };
            NumericInputEditBatch::from_events(&[update])
        };
        let Some(edit) = edit else {
            self.pointer = Some(state);
            return None;
        };

        state.anchor_position = position;
        state.anchor_value = candidate.clone();
        self.value = candidate;
        self.text_input.state.value = draft;
        let end = self.text_input.state.char_len();
        self.text_input.state.caret = end;
        self.text_input.state.selection_anchor = end;
        self.pointer = Some(state);
        Some(self.encode_output(edit))
    }

    fn handle_pointer_release(
        &mut self,
        button: PointerButton,
        modifiers: PointerModifiers,
        timestamp: Option<crate::gui::input::InputTimestamp>,
    ) -> Option<WidgetOutput> {
        if button != PointerButton::Primary {
            return None;
        }
        let mut state = self.pointer.take()?;
        self.text_input.common.state.pressed = false;
        let Some(session) = state.session.take() else {
            self.release_pointer_scrub();
            return None;
        };
        let provenance = pointer_provenance(modifiers, timestamp, None);
        let commit = match session.commit(state.anchor_value.clone(), provenance) {
            Ok(commit) => commit,
            Err(session) => {
                state.session = Some(session);
                self.pointer = Some(state);
                return None;
            }
        };
        let Some(edit) = NumericInputEditBatch::from_events(&[commit]) else {
            self.pointer = Some(state);
            return None;
        };
        self.release_pointer_scrub();
        Some(self.encode_output(edit))
    }

    fn cancel_pointer_scrub(&mut self, provenance: InteractionProvenance) -> Option<WidgetOutput> {
        let mut state = self.pointer.take()?;
        let cancel = state
            .session
            .take()
            .and_then(|session| session.cancel(provenance).ok())
            .and_then(|cancel| NumericInputEditBatch::from_events(&[cancel]));
        self.restore_pointer_start(&state);
        self.release_pointer_scrub();
        cancel.map(|edit| self.encode_output(edit))
    }

    fn default_wheel_provenance() -> InteractionProvenance {
        InteractionProvenance::Pointer {
            modifiers: PointerModifiers::default(),
            timestamp: None,
            sequence_range: None,
        }
    }

    fn restore_wheel_start(&mut self, state: &WheelSequenceState<T>) {
        self.value = state.start_value.clone();
        self.text_input.state.value = state.start_text.clone();
        self.text_input.state.caret = state.start_caret;
        self.text_input.state.selection_anchor = state.start_selection_anchor;
    }

    fn wheel_failure_rollback(
        state: &WheelSequenceState<T>,
        provenance: InteractionProvenance,
    ) -> Option<NumericInputEditBatch<T>> {
        state
            .session
            .as_ref()?
            .begin_event()
            .clone()
            .cancel(provenance)
            .and_then(|cancel| NumericInputEditBatch::from_events(&[cancel]))
    }

    fn handle_wheel_changed(&mut self, sample: WheelSample) -> Option<WidgetOutput> {
        let policy = self.wheel_output_policy.as_ref()?.clone();
        let mut state = self.wheel.take()?;
        let Some(delta) = wheel_delta(sample) else {
            self.wheel = Some(state);
            return None;
        };
        let step = selected_step(sample.modifiers());
        let attempt = if state.is_active() {
            NumericWheelAttempt::Update
        } else {
            NumericWheelAttempt::Initial
        };
        let provenance = wheel_provenance(sample);
        let candidate = match policy.wheel(WheelRequest {
            value: &self.value,
            delta,
            step,
            attempt,
            provenance,
        }) {
            Ok(candidate) => candidate,
            Err(failure) => {
                let rollback = Self::wheel_failure_rollback(&state, provenance);
                let output = failure.into_output(rollback);
                self.restore_wheel_start(&state);
                self.wheel = None;
                self.release_wheel_sequence();
                return output;
            }
        };
        if candidate == self.value {
            self.wheel = Some(state);
            return None;
        }

        let mut draft = String::new();
        if let Err(failure) = policy.format(
            WheelRequest {
                value: &candidate,
                delta,
                step,
                attempt,
                provenance,
            },
            &mut draft,
        ) {
            let rollback = Self::wheel_failure_rollback(&state, provenance);
            let output = failure.into_output(rollback);
            self.restore_wheel_start(&state);
            self.wheel = None;
            self.release_wheel_sequence();
            return output;
        }

        let Some(edit) = state.begin_update(candidate.clone(), provenance) else {
            self.wheel = Some(state);
            return None;
        };
        self.value = candidate;
        self.text_input.state.value = draft;
        let end = self.text_input.state.char_len();
        self.text_input.state.caret = end;
        self.text_input.state.selection_anchor = end;
        self.wheel = Some(state);
        Some(self.encode_output(edit))
    }

    fn handle_wheel_atomic(
        &mut self,
        bounds: Rect,
        position: Point,
        sample: WheelSample,
    ) -> Option<WidgetOutput> {
        if !self.wheel_is_configured()
            || !self.is_editable()
            || !admits_position(bounds, position)
            || self.interaction_gate.incumbent().is_some()
        {
            return None;
        }
        let delta = wheel_delta(sample)?;
        if !self
            .interaction_gate
            .try_admit(NumericInteractionOwner::WheelSequence)
        {
            return None;
        }

        let policy = match self.wheel_output_policy.as_ref().cloned() {
            Some(policy) => policy,
            None => {
                self.release_wheel_sequence();
                return None;
            }
        };
        let step = selected_step(sample.modifiers());
        let provenance = wheel_provenance(sample);
        let attempt = NumericWheelAttempt::Initial;
        let candidate = match policy.wheel(WheelRequest {
            value: &self.value,
            delta,
            step,
            attempt,
            provenance,
        }) {
            Ok(candidate) => candidate,
            Err(failure) => {
                let output = failure.into_output(None);
                self.release_wheel_sequence();
                return output;
            }
        };
        if candidate == self.value {
            self.release_wheel_sequence();
            return None;
        }

        let mut draft = String::new();
        if let Err(failure) = policy.format(
            WheelRequest {
                value: &candidate,
                delta,
                step,
                attempt,
                provenance,
            },
            &mut draft,
        ) {
            let output = failure.into_output(None);
            self.release_wheel_sequence();
            return output;
        }

        let mut state = WheelSequenceState::new(
            self.value.clone(),
            self.text_input.state.value.clone(),
            self.text_input.state.caret,
            self.text_input.state.selection_anchor,
            provenance,
        );
        let Some(begin_update) = state.begin_update(candidate.clone(), provenance) else {
            self.release_wheel_sequence();
            return None;
        };
        let [begin, update] = begin_update.events() else {
            self.release_wheel_sequence();
            return None;
        };
        let Some(session) = state.session.take() else {
            self.release_wheel_sequence();
            return None;
        };
        let Ok(commit) = session.commit(candidate.clone(), provenance) else {
            self.release_wheel_sequence();
            return None;
        };
        let Some(edit) =
            NumericInputEditBatch::from_events(&[begin.clone(), update.clone(), commit])
        else {
            self.release_wheel_sequence();
            return None;
        };
        self.value = candidate;
        self.text_input.state.value = draft;
        let end = self.text_input.state.char_len();
        self.text_input.state.caret = end;
        self.text_input.state.selection_anchor = end;
        self.release_wheel_sequence();
        Some(self.encode_output(edit))
    }

    fn commit_wheel_sequence(&mut self, sample: WheelSample) -> Option<WidgetOutput> {
        let mut state = self.wheel.take()?;
        let Some(session) = state.session.take() else {
            self.release_wheel_sequence();
            return None;
        };
        let provenance = wheel_provenance(sample);
        let commit = match session.commit(self.value.clone(), provenance) {
            Ok(commit) => commit,
            Err(session) => {
                state.session = Some(session);
                self.wheel = Some(state);
                return None;
            }
        };
        let Some(edit) = NumericInputEditBatch::from_events(&[commit]) else {
            self.wheel = Some(state);
            return None;
        };
        self.release_wheel_sequence();
        Some(self.encode_output(edit))
    }

    fn cancel_wheel_sequence(&mut self, provenance: InteractionProvenance) -> Option<WidgetOutput> {
        let state = self.wheel.take()?;
        let cancel = state
            .session
            .as_ref()
            .and_then(|session| session.begin_event().clone().cancel(provenance))
            .and_then(|cancel| NumericInputEditBatch::from_events(&[cancel]));
        self.restore_wheel_start(&state);
        self.release_wheel_sequence();
        cancel.map(|edit| self.encode_output(edit))
    }

    fn handle_exact_wheel_sample(
        &mut self,
        bounds: Rect,
        position: Point,
        sample: WheelSample,
    ) -> Option<WidgetOutput> {
        if self.wheel.is_some() && sample.phase() == Some(WheelPhase::Started) {
            let output = self.cancel_wheel_sequence(Self::default_wheel_provenance());
            if self.wheel_is_configured()
                && self.is_editable()
                && sample.is_valid()
                && admits_position(bounds, position)
                && wheel_delta(sample).is_some()
                && self.interaction_gate.incumbent().is_none()
                && self
                    .interaction_gate
                    .try_admit(NumericInteractionOwner::WheelSequence)
            {
                self.wheel = Some(WheelSequenceState::new(
                    self.value.clone(),
                    self.text_input.state.value.clone(),
                    self.text_input.state.caret,
                    self.text_input.state.selection_anchor,
                    wheel_provenance(sample),
                ));
            }
            return output;
        }
        if self.wheel.is_some() && !self.is_editable() {
            return self.cancel_wheel_sequence(Self::default_wheel_provenance());
        }
        if self.wheel.is_some() {
            return match sample.phase() {
                Some(WheelPhase::Changed) => self.handle_wheel_changed(sample),
                Some(WheelPhase::Ended) => self.commit_wheel_sequence(sample),
                Some(WheelPhase::Cancelled) => self.cancel_wheel_sequence(wheel_provenance(sample)),
                Some(WheelPhase::Started) | Some(WheelPhase::Discrete) | None => None,
            };
        }

        if !self.wheel_is_configured()
            || !self.is_editable()
            || !sample.is_valid()
            || !admits_position(bounds, position)
        {
            return None;
        }
        match sample.phase() {
            Some(WheelPhase::Started) => {
                if wheel_delta(sample).is_none()
                    || self.interaction_gate.incumbent().is_some()
                    || !self
                        .interaction_gate
                        .try_admit(NumericInteractionOwner::WheelSequence)
                {
                    return None;
                }
                self.wheel = Some(WheelSequenceState::new(
                    self.value.clone(),
                    self.text_input.state.value.clone(),
                    self.text_input.state.caret,
                    self.text_input.state.selection_anchor,
                    wheel_provenance(sample),
                ));
                None
            }
            Some(WheelPhase::Discrete) | None => self.handle_wheel_atomic(bounds, position, sample),
            Some(WheelPhase::Changed | WheelPhase::Ended | WheelPhase::Cancelled) => None,
        }
    }

    fn begin_text_edit_session(&mut self, timestamp: Option<crate::gui::input::InputTimestamp>) {
        let start_text = self.text_input.state.value.clone();
        self.active = Some(ActiveNumericEdit {
            session: NumericEditSession::begin(
                self.value.clone(),
                start_text.clone(),
                InteractionProvenance::Keyboard { timestamp },
            ),
            codec: Rc::clone(&self.codec),
            draft_result: None,
            start_text,
            start_caret: self.text_input.state.caret,
            start_selection_anchor: self.text_input.state.selection_anchor,
        });
    }

    fn update_active_draft(&mut self) {
        let draft = self.text_input.state.value.clone();
        if let Some(active) = self.active.as_mut() {
            active.session.replace_draft(draft.clone());
            active.draft_result = Some(active.codec.parse(&draft));
        }
    }

    fn terminal_batch(
        begin: EditEvent<T>,
        terminal: EditEvent<T>,
    ) -> Option<NumericInputEditBatch<T>> {
        NumericInputEditBatch::terminal(begin, terminal)
    }

    fn commit_active(
        &mut self,
        timestamp: Option<crate::gui::input::InputTimestamp>,
    ) -> Option<NumericInputEditBatch<T>> {
        self.commit_active_for_owner(timestamp, NumericInteractionOwner::TextEdit)
    }

    fn commit_active_for_owner(
        &mut self,
        timestamp: Option<crate::gui::input::InputTimestamp>,
        owner: NumericInteractionOwner,
    ) -> Option<NumericInputEditBatch<T>> {
        let active = self.active.take()?;
        let accepted = match active.draft_result.as_ref() {
            Some(NumericParseResult::Valid(value)) => value.clone(),
            Some(NumericParseResult::Incomplete)
            | Some(NumericParseResult::Invalid)
            | Some(NumericParseResult::OutOfRange)
            | None => {
                self.active = Some(active);
                return None;
            }
        };
        let begin = active.session.begin_event().clone();
        let commit = match active.session.commit(
            accepted.clone(),
            InteractionProvenance::Keyboard { timestamp },
        ) {
            Ok(event) => event,
            Err(session) => {
                self.active = Some(ActiveNumericEdit {
                    session,
                    codec: active.codec,
                    draft_result: active.draft_result,
                    start_text: active.start_text,
                    start_caret: active.start_caret,
                    start_selection_anchor: active.start_selection_anchor,
                });
                return None;
            }
        };
        let batch = Self::terminal_batch(begin, commit);
        if batch.is_some() {
            self.value = accepted;
            self.interaction_gate.release(owner);
        }
        batch
    }

    fn cancel_active(
        &mut self,
        timestamp: Option<crate::gui::input::InputTimestamp>,
    ) -> Option<NumericInputEditBatch<T>> {
        self.cancel_active_for_owner(timestamp, NumericInteractionOwner::TextEdit)
    }

    fn cancel_active_for_owner(
        &mut self,
        timestamp: Option<crate::gui::input::InputTimestamp>,
        owner: NumericInteractionOwner,
    ) -> Option<NumericInputEditBatch<T>> {
        let active = self.active.take()?;
        let begin = active.session.begin_event().clone();
        let cancel = match active
            .session
            .cancel(InteractionProvenance::Keyboard { timestamp })
        {
            Ok(event) => event,
            Err(session) => {
                self.active = Some(ActiveNumericEdit {
                    session,
                    codec: active.codec,
                    draft_result: active.draft_result,
                    start_text: active.start_text,
                    start_caret: active.start_caret,
                    start_selection_anchor: active.start_selection_anchor,
                });
                return None;
            }
        };
        self.value = cancel.value.clone();
        self.text_input.state.value = active.start_text;
        self.text_input.state.caret = active.start_caret;
        self.text_input.state.selection_anchor = active.start_selection_anchor;
        let batch = Self::terminal_batch(begin, cancel);
        if batch.is_some() {
            self.interaction_gate.release(owner);
        }
        batch
    }

    fn start_composition(
        &mut self,
        replacement_range: CompositionRange,
        selection: CompositionRange,
        timestamp: Option<crate::gui::input::InputTimestamp>,
    ) {
        if self.composition.is_some()
            || self.active.is_some()
            || self.keyboard.is_some()
            || self.pointer.is_some()
            || self.wheel.is_some()
            || self.interaction_gate.incumbent().is_some()
        {
            return;
        }
        let scalar_len = self.text_input.state.char_len();
        if !replacement_range.is_valid_for(scalar_len) || !selection.is_valid_for(scalar_len) {
            return;
        }
        if !self
            .interaction_gate
            .try_admit(NumericInteractionOwner::ImeComposition)
        {
            return;
        }

        let original_value = self.text_input.state.value.clone();
        set_composition_selection(&mut self.text_input.state, selection);
        self.begin_text_edit_session(timestamp);
        self.composition = Some(NumericInputComposition {
            original_value,
            replacement_range,
            original_selection: selection,
            preedit: String::new(),
            preedit_selection: CompositionSelectionState::Unreported,
        });
    }

    fn update_composition(&mut self, preedit: String, selection: CompositionSelectionState) {
        let Some(mut composition) = self.composition.take() else {
            return;
        };
        let display_selection = match selection {
            CompositionSelectionState::Visible(selection) => {
                if !selection.is_valid_for(preedit.chars().count()) {
                    self.composition = Some(composition);
                    return;
                }
                let Some(display_selection) =
                    composition_display_selection(composition.replacement_range, selection)
                else {
                    self.composition = Some(composition);
                    return;
                };
                Some(display_selection)
            }
            CompositionSelectionState::Unreported | CompositionSelectionState::Hidden => None,
        };

        composition.preedit = preedit;
        composition.preedit_selection = selection;
        self.text_input.state.value = composition_display_value(
            &composition.original_value,
            composition.replacement_range,
            &composition.preedit,
        );
        if let Some(display_selection) = display_selection {
            set_composition_selection(&mut self.text_input.state, display_selection);
        }
        self.composition = Some(composition);
    }

    fn commit_composition(
        &mut self,
        text: String,
        timestamp: Option<crate::gui::input::InputTimestamp>,
    ) -> Option<NumericInputEditBatch<T>> {
        let composition = self.composition.take()?;
        let mut committed_state = TextInputState::from_value(composition_display_value(
            &composition.original_value,
            composition.replacement_range,
            "",
        ));
        committed_state.set_caret(composition.replacement_range.start(), false);
        committed_state.insert_text(&text, self.text_input.props.character_limit);
        self.text_input.state = committed_state;
        self.update_active_draft();

        let batch =
            self.commit_active_for_owner(timestamp, NumericInteractionOwner::ImeComposition);
        if batch.is_none() {
            self.interaction_gate
                .release(NumericInteractionOwner::ImeComposition);
            if self.active.is_some() {
                let _ = self
                    .interaction_gate
                    .try_admit(NumericInteractionOwner::TextEdit);
            }
        }
        batch
    }

    fn cancel_composition(
        &mut self,
        timestamp: Option<crate::gui::input::InputTimestamp>,
    ) -> Option<NumericInputEditBatch<T>> {
        self.composition.take()?;
        let batch =
            self.cancel_active_for_owner(timestamp, NumericInteractionOwner::ImeComposition);
        if batch.is_none() {
            self.interaction_gate
                .release(NumericInteractionOwner::ImeComposition);
            if self.active.is_some() {
                let _ = self
                    .interaction_gate
                    .try_admit(NumericInteractionOwner::TextEdit);
            }
        }
        batch
    }

    fn dispatch_composition_sample(&mut self, sample: CompositionSample) -> Option<WidgetOutput> {
        if !sample.is_valid() {
            return None;
        }
        let batch = match sample {
            CompositionSample::Start {
                replacement_range,
                selection,
                timestamp,
            } if self.is_editable() => {
                self.start_composition(replacement_range, selection, timestamp);
                None
            }
            CompositionSample::Update {
                preedit, selection, ..
            } if self.is_editable() && self.composition.is_some() => {
                self.update_composition(preedit, CompositionSelectionState::Visible(selection));
                None
            }
            CompositionSample::Commit { text, timestamp }
                if self.is_editable() && self.composition.is_some() =>
            {
                self.commit_composition(text, timestamp)
            }
            CompositionSample::Cancel { timestamp } if self.composition.is_some() => {
                self.cancel_composition(timestamp)
            }
            _ => None,
        }?;
        Some(self.encode_output(batch))
    }

    fn handle_keyboard_initial(
        &mut self,
        key: WidgetKey,
        modifiers: crate::widgets::KeyboardModifiers,
        repeat: bool,
        timestamp: Option<crate::gui::input::InputTimestamp>,
    ) -> Option<WidgetOutput> {
        let direction = direction_for_key(key)?;
        if repeat
            || self.output_mode != NumericInputOutputMode::Complete
            || self.step_modifiers.is_none()
            || !self.is_editable()
            || self.active.is_some()
            || self.keyboard.is_some()
            || self.interaction_gate.incumbent().is_some()
        {
            return None;
        }

        if !self
            .interaction_gate
            .try_admit(NumericInteractionOwner::KeyboardAdjustment)
        {
            return None;
        }

        let Some(step_modifiers) = self.step_modifiers else {
            self.interaction_gate
                .release(NumericInteractionOwner::KeyboardAdjustment);
            return None;
        };
        let step = step_modifiers.select_step(modifiers);

        let provenance = InteractionProvenance::Keyboard { timestamp };
        let candidate = match self.keyboard_policy.step(KeyboardAdjustmentRequest {
            value: &self.value,
            direction,
            step,
            attempt: NumericStepAttempt::Initial,
            provenance,
            rollback: None,
        }) {
            Ok(candidate) => candidate,
            Err(output) => {
                self.interaction_gate
                    .release(NumericInteractionOwner::KeyboardAdjustment);
                return output;
            }
        };
        if candidate == self.value {
            self.interaction_gate
                .release(NumericInteractionOwner::KeyboardAdjustment);
            return None;
        }

        let mut draft = String::new();
        if let Err(output) = self.keyboard_policy.format(
            KeyboardAdjustmentRequest {
                value: &candidate,
                direction,
                step,
                attempt: NumericStepAttempt::Initial,
                provenance,
                rollback: None,
            },
            &mut draft,
        ) {
            self.interaction_gate
                .release(NumericInteractionOwner::KeyboardAdjustment);
            return output;
        }

        let start_text = self.text_input.state.value.clone();
        let keyboard = KeyboardAdjustmentState::new(
            self.value.clone(),
            start_text,
            key,
            timestamp,
            self.text_input.state.caret,
            self.text_input.state.selection_anchor,
        );
        let begin = keyboard.session.begin_event().clone();
        let Some(update) = keyboard.begin_update(candidate.clone(), timestamp) else {
            self.interaction_gate
                .release(NumericInteractionOwner::KeyboardAdjustment);
            return None;
        };
        let Some(edit) = NumericInputEditBatch::from_events(&[begin, update]) else {
            self.interaction_gate
                .release(NumericInteractionOwner::KeyboardAdjustment);
            return None;
        };
        let output = self.encode_output(edit);

        self.value = candidate;
        self.text_input.state.value = draft;
        let end = self.text_input.state.char_len();
        self.text_input.state.caret = end;
        self.text_input.state.selection_anchor = end;
        self.keyboard = Some(keyboard);
        Some(output)
    }

    fn handle_keyboard_repeat(
        &mut self,
        key: WidgetKey,
        modifiers: crate::widgets::KeyboardModifiers,
        timestamp: Option<crate::gui::input::InputTimestamp>,
    ) -> Option<WidgetOutput> {
        let active = self.keyboard.as_ref().cloned()?;
        if active.key != key {
            return None;
        }
        if !self.is_editable()
            || self.output_mode != NumericInputOutputMode::Complete
            || self.step_modifiers.is_none()
        {
            return self.cancel_keyboard(timestamp);
        }
        let direction = direction_for_key(key)?;
        let Some(step_modifiers) = self.step_modifiers else {
            return self.cancel_keyboard(timestamp);
        };
        let step = step_modifiers.select_step(modifiers);
        let provenance = InteractionProvenance::Keyboard { timestamp };
        let rollback = active
            .session
            .clone()
            .cancel(provenance)
            .ok()
            .and_then(|cancel| NumericInputEditBatch::from_events(&[cancel]));
        let candidate = match self.keyboard_policy.step(KeyboardAdjustmentRequest {
            value: &self.value,
            direction,
            step,
            attempt: NumericStepAttempt::Repeat,
            provenance,
            rollback: rollback.clone(),
        }) {
            Ok(candidate) => candidate,
            Err(output) => {
                return self.rollback_keyboard_failure(output, timestamp);
            }
        };
        if candidate == self.value {
            return None;
        }

        let mut draft = String::new();
        if let Err(output) = self.keyboard_policy.format(
            KeyboardAdjustmentRequest {
                value: &candidate,
                direction,
                step,
                attempt: NumericStepAttempt::Repeat,
                provenance,
                rollback,
            },
            &mut draft,
        ) {
            return self.rollback_keyboard_failure(output, timestamp);
        }

        let Some(update) = active.begin_update(candidate.clone(), timestamp) else {
            return self.cancel_keyboard(timestamp);
        };
        let Some(edit) = NumericInputEditBatch::from_events(&[update]) else {
            return self.cancel_keyboard(timestamp);
        };
        let output = self.encode_output(edit);

        self.value = candidate;
        self.text_input.state.value = draft;
        let end = self.text_input.state.char_len();
        self.text_input.state.caret = end;
        self.text_input.state.selection_anchor = end;
        Some(output)
    }

    fn cancel_keyboard(
        &mut self,
        timestamp: Option<crate::gui::input::InputTimestamp>,
    ) -> Option<WidgetOutput> {
        let active = self.keyboard.take()?;
        let start_text = active.start_text.clone();
        let start_caret = active.start_caret;
        let start_selection_anchor = active.start_selection_anchor;
        let cancel = match active.cancel(timestamp) {
            Ok(cancel) => cancel,
            Err(active) => {
                self.keyboard = Some(*active);
                return None;
            }
        };
        self.value = cancel.value.clone();
        self.text_input.state.value = start_text;
        self.text_input.state.caret = start_caret;
        self.text_input.state.selection_anchor = start_selection_anchor;
        self.interaction_gate
            .release(NumericInteractionOwner::KeyboardAdjustment);
        let edit = NumericInputEditBatch::from_events(&[cancel])?;
        Some(self.encode_output(edit))
    }

    fn rollback_keyboard_failure(
        &mut self,
        failure: Option<WidgetOutput>,
        timestamp: Option<crate::gui::input::InputTimestamp>,
    ) -> Option<WidgetOutput> {
        let active = self.keyboard.take()?;
        let start_text = active.start_text.clone();
        let start_caret = active.start_caret;
        let start_selection_anchor = active.start_selection_anchor;
        let cancel = match active.cancel(timestamp) {
            Ok(cancel) => cancel,
            Err(active) => {
                self.keyboard = Some(*active);
                return None;
            }
        };
        self.value = cancel.value.clone();
        self.text_input.state.value = start_text;
        self.text_input.state.caret = start_caret;
        self.text_input.state.selection_anchor = start_selection_anchor;
        self.interaction_gate
            .release(NumericInteractionOwner::KeyboardAdjustment);
        failure
    }

    fn handles_value_mutation(input: &WidgetInput) -> bool {
        match input {
            WidgetInput::Character { character, .. } => !character.is_control(),
            WidgetInput::KeyPress { key, .. } => {
                matches!(key, WidgetKey::Backspace | WidgetKey::Delete)
            }
            WidgetInput::TextEdit { command, .. } => matches!(
                command,
                crate::widgets::TextEditCommand::InsertText(_)
                    | crate::widgets::TextEditCommand::Backspace
                    | crate::widgets::TextEditCommand::Delete
                    | crate::widgets::TextEditCommand::DeleteWordLeft
                    | crate::widgets::TextEditCommand::DeleteWordRight
                    | crate::widgets::TextEditCommand::CutSelection
            ),
            _ => false,
        }
    }

    fn keyboard_timestamp(input: &WidgetInput) -> Option<crate::gui::input::InputTimestamp> {
        match input {
            WidgetInput::Character { timestamp, .. }
            | WidgetInput::TextEdit { timestamp, .. }
            | WidgetInput::KeyPress { timestamp, .. } => *timestamp,
            _ => None,
        }
    }

    fn handle_focus_loss(
        &mut self,
        bounds: Rect,
        environment: &ResolvedEnvironment,
    ) -> Option<NumericInputEditBatch<T>> {
        if self.composition.is_some() {
            let output = self.cancel_composition(None);
            let _ = Widget::handle_input_with_environment(
                &mut self.text_input,
                bounds,
                WidgetInput::FocusChanged(false),
                environment,
            );
            return output;
        }
        if self.active.is_some() {
            let output = self.commit_active(None);
            if output.is_some() {
                let _ = Widget::handle_input_with_environment(
                    &mut self.text_input,
                    bounds,
                    WidgetInput::FocusChanged(false),
                    environment,
                );
            }
            output
        } else {
            let _ = Widget::handle_input_with_environment(
                &mut self.text_input,
                bounds,
                WidgetInput::FocusChanged(false),
                environment,
            );
            None
        }
    }
}

impl<T, C, A> WidgetSemantics for NumericInputWidget<T, C, A>
where
    T: Clone + PartialEq + 'static,
    C: NumericCodec<T> + 'static,
    A: NumericAdjustment<T> + 'static,
{
    fn automation_role(&self) -> crate::gui::automation::AutomationRole {
        self.text_input.automation_role()
    }

    fn automation_label(&self) -> Option<String> {
        self.text_input.automation_label()
    }

    fn automation_value_text(&self) -> Option<String> {
        self.text_input.automation_value_text()
    }

    fn automation_available_actions(&self) -> Option<Vec<String>> {
        self.accessibility_action_handler.as_ref()?;
        let mut actions = Vec::with_capacity(4);
        if self.text_input.common.focus != crate::widgets::FocusBehavior::None
            && !self.text_input.common.state.disabled
        {
            actions.push(crate::gui::automation::AUTOMATION_ACTION_FOCUS.to_owned());
        }
        if !self.text_input.common.state.read_only && !self.text_input.common.state.disabled {
            actions.extend([
                crate::gui::automation::AUTOMATION_ACTION_INCREMENT.to_owned(),
                crate::gui::automation::AUTOMATION_ACTION_DECREMENT.to_owned(),
                crate::gui::automation::AUTOMATION_ACTION_SET_TEXT.to_owned(),
            ]);
        }
        Some(actions)
    }
}

impl<T, C, A> WidgetPointerMotion for NumericInputWidget<T, C, A>
where
    T: Clone + PartialEq + 'static,
    C: NumericCodec<T> + 'static,
    A: NumericAdjustment<T> + 'static,
{
    fn revision(&self) -> WidgetPointerMotionRevision {
        WidgetPointerMotionRevision::exact(false)
    }

    fn accepts_pointer_move(&self) -> bool {
        false
    }
}

impl<T, C, A> NumericInputWidget<T, C, A>
where
    T: Clone + PartialEq + 'static,
    C: NumericCodec<T> + 'static,
    A: NumericAdjustment<T> + 'static,
{
    fn handle_input_with_resolved_environment(
        &mut self,
        bounds: Rect,
        input: WidgetInput,
        environment: &ResolvedEnvironment,
    ) -> Option<crate::widgets::WidgetOutput> {
        if self.wheel.is_some() {
            match input {
                WidgetInput::KeyPress {
                    key: WidgetKey::Escape,
                    ..
                } => return self.cancel_wheel_sequence(Self::default_wheel_provenance()),
                WidgetInput::FocusChanged(false) => {
                    let output = self.cancel_wheel_sequence(Self::default_wheel_provenance());
                    let _ = Widget::handle_input_with_environment(
                        &mut self.text_input,
                        bounds,
                        WidgetInput::FocusChanged(false),
                        environment,
                    );
                    return output;
                }
                _ => return None,
            }
        }

        if self.pointer.is_some() {
            match input {
                WidgetInput::PointerMove {
                    position,
                    modifiers,
                    timestamp,
                    sequence_range,
                } => {
                    return self.handle_pointer_move(
                        position,
                        modifiers,
                        timestamp,
                        sequence_range,
                    );
                }
                WidgetInput::PointerRelease {
                    button,
                    modifiers,
                    timestamp,
                    ..
                } => return self.handle_pointer_release(button, modifiers, timestamp),
                WidgetInput::KeyPress {
                    key: WidgetKey::Escape,
                    ..
                } => return self.cancel_pointer_scrub(Self::default_pointer_provenance()),
                WidgetInput::FocusChanged(false) => {
                    let output = self.cancel_pointer_scrub(Self::default_pointer_provenance());
                    let _ = Widget::handle_input_with_environment(
                        &mut self.text_input,
                        bounds,
                        WidgetInput::FocusChanged(false),
                        environment,
                    );
                    return output;
                }
                _ => return None,
            }
        }

        if let WidgetInput::PointerPress {
            position,
            button,
            modifiers,
            timestamp,
        } = &input
            && self.scrub_policy.is_some_and(|policy| {
                self.output_mode == NumericInputOutputMode::Complete
                    && policy.admits(*button, *modifiers)
            })
        {
            return self.handle_pointer_press(bounds, *position, *button, *modifiers, *timestamp);
        }

        if matches!(&input, WidgetInput::FocusChanged(false)) {
            if self.active.is_some() {
                return self
                    .handle_focus_loss(bounds, environment)
                    .map(|batch| self.encode_output(batch));
            }
            if self.keyboard.is_some() {
                let output = self.cancel_keyboard(None);
                let _ = Widget::handle_input_with_environment(
                    &mut self.text_input,
                    bounds,
                    WidgetInput::FocusChanged(false),
                    environment,
                );
                return output;
            }
            let _ = Widget::handle_input_with_environment(
                &mut self.text_input,
                bounds,
                WidgetInput::FocusChanged(false),
                environment,
            );
            return None;
        }

        if self.composition.is_some()
            && matches!(
                &input,
                WidgetInput::KeyPress {
                    key: WidgetKey::Escape | WidgetKey::Enter,
                    ..
                }
            )
        {
            return None;
        }

        if self.keyboard.is_some() {
            match &input {
                WidgetInput::KeyPress {
                    key,
                    repeat: true,
                    modifiers,
                    timestamp,
                } if is_adjustment_key(*key) => {
                    return self.handle_keyboard_repeat(*key, *modifiers, *timestamp);
                }
                WidgetInput::KeyRelease { key, timestamp, .. }
                    if self
                        .keyboard
                        .as_ref()
                        .is_some_and(|active| active.key == *key) =>
                {
                    let active = self.keyboard.take()?;
                    let commit = match active.commit(self.value.clone(), *timestamp) {
                        Ok(commit) => commit,
                        Err(active) => {
                            self.keyboard = Some(*active);
                            return None;
                        }
                    };
                    self.interaction_gate
                        .release(NumericInteractionOwner::KeyboardAdjustment);
                    let edit = NumericInputEditBatch::from_events(&[commit])?;
                    return Some(self.encode_output(edit));
                }
                WidgetInput::KeyPress {
                    key: WidgetKey::Escape,
                    timestamp,
                    ..
                } => {
                    return self.cancel_keyboard(*timestamp);
                }
                _ => return None,
            }
        }

        if let WidgetInput::KeyPress {
            key,
            modifiers,
            repeat,
            timestamp,
        } = &input
            && is_adjustment_key(*key)
        {
            return self.handle_keyboard_initial(*key, *modifiers, *repeat, *timestamp);
        }

        match &input {
            WidgetInput::KeyPress {
                key: WidgetKey::Escape,
                timestamp,
                ..
            } if self.active.is_some() && self.composition.is_none() => {
                return self
                    .cancel_active(*timestamp)
                    .map(|batch| self.encode_output(batch));
            }
            WidgetInput::KeyPress {
                key: WidgetKey::Enter,
                timestamp,
                ..
            } if self.composition.is_none() => {
                return self
                    .commit_active(*timestamp)
                    .map(|batch| self.encode_output(batch));
            }
            _ => {}
        }

        let started_session =
            self.active.is_none() && self.is_editable() && Self::handles_value_mutation(&input);
        if started_session {
            if !self
                .interaction_gate
                .try_admit(NumericInteractionOwner::TextEdit)
            {
                return None;
            }
            self.begin_text_edit_session(Self::keyboard_timestamp(&input));
        }
        let _ =
            Widget::handle_input_with_environment(&mut self.text_input, bounds, input, environment);
        let value_changed = self
            .active
            .as_ref()
            .is_some_and(|active| active.session.draft() != self.text_input.state.value);
        if value_changed && self.composition.is_none() {
            self.update_active_draft();
        } else if started_session {
            self.active = None;
            self.interaction_gate
                .release(NumericInteractionOwner::TextEdit);
        }
        None
    }
}

impl<T, C, A> Widget for NumericInputWidget<T, C, A>
where
    T: Clone + PartialEq + 'static,
    C: NumericCodec<T> + 'static,
    A: NumericAdjustment<T> + 'static,
{
    fn focused_key_disposition(&self, key: WidgetKey) -> FocusedKeyDisposition {
        match key {
            WidgetKey::Home | WidgetKey::End => FocusedKeyDisposition::Consumed,
            WidgetKey::PageUp | WidgetKey::PageDown
                if self.interaction_gate.incumbent().is_some() =>
            {
                FocusedKeyDisposition::Consumed
            }
            WidgetKey::PageUp | WidgetKey::PageDown => FocusedKeyDisposition::Unhandled,
            _ => FocusedKeyDisposition::Consumed,
        }
    }

    fn common(&self) -> &crate::widgets::WidgetCommon {
        &self.text_input.common
    }

    fn common_mut(&mut self) -> &mut crate::widgets::WidgetCommon {
        &mut self.text_input.common
    }

    fn prepare_focus_loss(&mut self) -> FocusLossDecision {
        if self.pointer.is_some() {
            return FocusLossDecision::Allow;
        }
        if self.composition.is_some() {
            return FocusLossDecision::Allow;
        }
        let Some(active) = self.active.as_ref() else {
            return FocusLossDecision::Allow;
        };

        match active.draft_result.as_ref() {
            Some(NumericParseResult::Valid(_)) => FocusLossDecision::Allow,
            Some(NumericParseResult::Incomplete)
            | Some(NumericParseResult::Invalid)
            | Some(NumericParseResult::OutOfRange)
            | None => FocusLossDecision::Veto,
        }
    }

    fn supports_accessibility_action(&self, action: &NumericAccessibilityAction) -> bool {
        self.accessibility_action_handler.is_some()
            && matches!(
                action,
                NumericAccessibilityAction::Increment
                    | NumericAccessibilityAction::Decrement
                    | NumericAccessibilityAction::SetValueText(_)
            )
    }

    fn accessibility_action_owner(&self) -> Option<NumericAccessibilityBlockOwner> {
        self.interaction_gate
            .incumbent()
            .map(NumericAccessibilityBlockOwner::from)
    }

    fn handle_accessibility_action(
        &mut self,
        action: NumericAccessibilityAction,
    ) -> Option<WidgetOutput> {
        let handler = self.accessibility_action_handler.as_ref().map(Rc::clone)?;
        handler(self, action)
    }

    fn handle_input(
        &mut self,
        bounds: Rect,
        input: WidgetInput,
    ) -> Option<crate::widgets::WidgetOutput> {
        self.handle_input_with_resolved_environment(bounds, input, &ResolvedEnvironment::default())
    }

    fn handle_input_with_environment(
        &mut self,
        bounds: Rect,
        input: WidgetInput,
        environment: &ResolvedEnvironment,
    ) -> Option<crate::widgets::WidgetOutput> {
        self.handle_input_with_resolved_environment(bounds, input, environment)
    }

    fn text_scale_participation(&self) -> TextScaleParticipation {
        TextScaleParticipation::Scaled
    }

    fn layout_node_with_environment(
        &self,
        environment: &ResolvedEnvironment,
    ) -> crate::layout::LayoutNode {
        self.text_input.layout_node_with_environment(environment)
    }

    fn handle_wheel_sample(
        &mut self,
        bounds: Rect,
        position: Point,
        sample: WheelSample,
    ) -> Option<crate::widgets::WidgetOutput> {
        self.handle_exact_wheel_sample(bounds, position, sample)
    }

    fn synchronize_from_previous(&mut self, previous: &dyn Widget) {
        let Some(previous) = previous.as_any().downcast_ref::<Self>() else {
            self.active = None;
            self.composition = None;
            self.keyboard = None;
            self.pointer = None;
            self.wheel = None;
            self.interaction_gate = NumericInteractionGate::new();
            return;
        };
        if self.text_input.common.id != previous.text_input.common.id {
            self.active = None;
            self.composition = None;
            self.keyboard = None;
            self.pointer = None;
            self.wheel = None;
            self.interaction_gate = NumericInteractionGate::new();
            return;
        }

        let reset = self.value != previous.value
            || self.output_mode != previous.output_mode
            || self.step_modifiers != previous.step_modifiers
            || self.scrub_policy != previous.scrub_policy
            || self.wheel_policy != previous.wheel_policy
            || self.text_input.common.state.disabled
            || previous.text_input.common.state.disabled
            || self.text_input.common.state.read_only
            || previous.text_input.common.state.read_only;
        if reset {
            self.active = None;
            self.composition = None;
            self.keyboard = None;
            self.pointer = None;
            self.wheel = None;
            self.interaction_gate = NumericInteractionGate::new();
            return;
        }

        self.text_input.common.state = previous.text_input.common.state;
        if previous.pointer.is_some() {
            self.text_input.state = previous.text_input.state.clone();
            self.active = None;
            self.composition = None;
            self.keyboard = None;
            self.pointer = previous.pointer.clone();
            self.wheel = None;
            self.interaction_gate = previous.interaction_gate;
        } else if previous.wheel.is_some() {
            self.text_input.state = previous.text_input.state.clone();
            self.active = None;
            self.composition = None;
            self.keyboard = None;
            self.pointer = None;
            self.wheel = previous.wheel.clone();
            self.interaction_gate = previous.interaction_gate;
        } else if previous.active.is_some() {
            self.text_input.state = previous.text_input.state.clone();
            self.active = previous.active.clone();
            self.composition = previous.composition.clone();
            self.keyboard = None;
            self.pointer = None;
            self.wheel = None;
            self.interaction_gate = previous.interaction_gate;
        } else if previous.keyboard.is_some() {
            self.text_input.state = previous.text_input.state.clone();
            self.active = None;
            self.composition = None;
            self.keyboard = previous.keyboard.clone();
            self.pointer = None;
            self.wheel = None;
            self.interaction_gate = previous.interaction_gate;
        } else {
            self.active = None;
            self.composition = None;
            self.keyboard = None;
            self.pointer = None;
            self.wheel = None;
            self.interaction_gate = NumericInteractionGate::new();
        }
    }

    fn prepare_replacement(&mut self, successor: Option<&dyn Widget>) -> Option<WidgetOutput> {
        let compatible = successor
            .and_then(|successor| successor.as_any().downcast_ref::<Self>())
            .is_some_and(|successor| {
                self.text_input.common.id == successor.text_input.common.id
                    && self.value == successor.value
                    && self.output_mode == successor.output_mode
                    && self.step_modifiers == successor.step_modifiers
                    && self.scrub_policy == successor.scrub_policy
                    && self.wheel_policy == successor.wheel_policy
                    && !self.text_input.common.state.disabled
                    && !self.text_input.common.state.read_only
                    && !successor.text_input.common.state.disabled
                    && !successor.text_input.common.state.read_only
            });
        if compatible {
            return None;
        }

        if self.composition.is_some() {
            self.cancel_composition(None)
                .map(|batch| self.encode_output(batch))
        } else if self.keyboard.is_some() {
            self.cancel_keyboard(None)
        } else if self.pointer.is_some() {
            self.cancel_pointer_scrub(Self::default_pointer_provenance())
        } else if self.wheel.is_some() {
            self.cancel_wheel_sequence(Self::default_wheel_provenance())
        } else {
            self.cancel_active(None)
                .map(|batch| self.encode_output(batch))
        }
    }

    fn accepts_composition_input(&self) -> bool {
        // Runtime focus authority is checked separately. Keep this capability
        // true during refresh reconciliation, before focused widget state is
        // restored on the replacement surface.
        !self.text_input.common.state.disabled && !self.text_input.common.state.read_only
    }

    fn composition_start_context(
        &self,
    ) -> Option<crate::widgets::interaction::CompositionStartContext> {
        self.text_input.native_composition_start_context()
    }

    fn handle_composition_sample(&mut self, sample: CompositionSample) -> Option<WidgetOutput> {
        self.dispatch_composition_sample(sample)
    }

    fn handle_hidden_composition_update(
        &mut self,
        preedit: String,
        _timestamp: Option<crate::gui::input::InputTimestamp>,
    ) -> Option<WidgetOutput> {
        if !self.is_editable() || self.composition.is_none() {
            return None;
        }
        self.update_composition(preedit, CompositionSelectionState::Hidden);
        None
    }

    fn retains_managed_composition(&self) -> bool {
        self.composition.is_some()
    }

    fn accepts_text_input(&self) -> bool {
        self.is_editable()
    }

    fn accepts_wheel_input(&self) -> bool {
        self.output_mode == NumericInputOutputMode::Complete && self.wheel_policy.is_some()
    }

    fn retains_managed_wheel_sequence(&self) -> bool {
        self.wheel.is_some()
    }

    fn preempts_host_shortcut_key(&self, key: WidgetKey) -> bool {
        key == WidgetKey::Escape
            && (self.active.is_some()
                || self.keyboard.is_some()
                || self.pointer.is_some()
                || self.wheel.is_some())
            && self.is_editable()
    }

    fn participates_in_focused_key_routing(&self) -> bool {
        self.output_mode == NumericInputOutputMode::Complete
            && self.step_modifiers.is_some()
            && self.is_editable()
    }

    fn captured_focused_key(&self) -> Option<WidgetKey> {
        self.keyboard.as_ref().map(|active| active.key)
    }

    fn preflight_pointer_press(&self, bounds: Rect, input: &WidgetInput) -> PointerPressAdmission {
        let WidgetInput::PointerPress {
            position,
            button,
            modifiers,
            ..
        } = input
        else {
            return PointerPressAdmission::Legacy;
        };
        if !valid_geometry(bounds, *position) {
            return PointerPressAdmission::Legacy;
        }
        let Some(policy) = self.scrub_policy else {
            return PointerPressAdmission::Legacy;
        };
        if self.output_mode != NumericInputOutputMode::Complete
            || !policy.admits(*button, *modifiers)
        {
            return PointerPressAdmission::Legacy;
        }
        if self.interaction_gate.incumbent().is_some() {
            return PointerPressAdmission::Blocked;
        }
        if self.text_input.common.state.disabled || self.text_input.common.state.read_only {
            return PointerPressAdmission::Legacy;
        }
        PointerPressAdmission::ManagedCapture
    }

    fn retains_managed_pointer_capture(&self) -> bool {
        self.pointer.is_some()
    }

    fn handle_pointer_capture_cancelled(&mut self, _bounds: Rect) -> Option<WidgetOutput> {
        if self.pointer.is_some() {
            self.cancel_pointer_scrub(Self::default_pointer_provenance())
        } else {
            self.text_input.common.state.pressed = false;
            None
        }
    }

    fn capabilities(&self) -> WidgetCapabilities<'_> {
        WidgetCapabilities::new().semantics(self)
    }

    fn capabilities_v2(&self) -> crate::widgets::WidgetCapabilitiesV2<'_> {
        crate::widgets::WidgetCapabilitiesV2::new().with_pointer_motion(self)
    }

    fn selected_text_slice(&self) -> Option<&str> {
        self.text_input.selected_text_slice()
    }

    fn selected_text(&self) -> Option<String> {
        self.text_input.selected_text()
    }

    fn set_text_wrap(&mut self, wrap: TextWrap) -> bool {
        Widget::set_text_wrap(&mut self.text_input, wrap)
    }

    fn set_text_align(&mut self, align: TextAlign) -> bool {
        Widget::set_text_align(&mut self.text_input, align)
    }

    fn set_text_color(&mut self, color: TextColorRole) -> bool {
        Widget::set_text_color(&mut self.text_input, color)
    }

    fn set_text_background(&mut self, background: TextBackgroundRole) -> bool {
        Widget::set_text_background(&mut self.text_input, background)
    }

    fn set_text_inset(&mut self, inset: crate::layout::Vector2) -> bool {
        Widget::set_text_inset(&mut self.text_input, inset)
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        theme: &ThemeTokens,
    ) {
        self.text_input.append_paint_with_hidden_composition(
            primitives,
            bounds,
            theme,
            self.composition
                .as_ref()
                .is_some_and(|composition| composition.preedit_selection.is_hidden()),
        );
    }

    fn append_paint_with_context(&self, context: &mut WidgetPaintContext<'_>) {
        self.text_input
            .append_paint_with_context_hidden_composition(
                context,
                self.composition
                    .as_ref()
                    .is_some_and(|composition| composition.preedit_selection.is_hidden()),
            );
    }
}

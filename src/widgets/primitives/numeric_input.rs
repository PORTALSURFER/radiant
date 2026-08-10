//! Text-first generic numeric input built on the retained text-input primitive.

mod keyboard;
mod pointer;

#[cfg(test)]
mod tests;

use std::{fmt, rc::Rc};

use self::keyboard::{
    KeyboardAdjustmentPolicy, KeyboardAdjustmentRequest, KeyboardAdjustmentState,
    complete_keyboard_adjustment_policy, direction_for_key, is_adjustment_key,
    no_keyboard_adjustment_policy,
};

use crate::{
    gui::types::{Point, Rect},
    layout::LayoutOutput,
    runtime::PaintPrimitive,
    theme::ThemeTokens,
    widgets::{
        EditEvent, FocusBehavior, FocusLossDecision, InteractionProvenance, NumericAdjustment,
        NumericCodec, NumericEditSession, NumericInputConstructionError, NumericInputEditBatch,
        NumericInputInteractionBatch, NumericParseResult, NumericScrubAttempt, NumericScrubPolicy,
        NumericStepAttempt, NumericStepModifiers, PointerButton, PointerCapturePolicy,
        PointerModifiers, PointerPressPreflight, TextAlign, TextBackgroundRole, TextColorRole,
        TextInputChrome, TextInputWidget, TextWrap, Widget, WidgetCapabilities, WidgetInput,
        WidgetKey, WidgetOutput, WidgetSemantics, WidgetSizing,
        interaction::{NumericInteractionGate, NumericInteractionOwner},
    },
};

type NumericInputOutputEncoder<T> = Rc<dyn Fn(NumericInputEditBatch<T>) -> WidgetOutput>;

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
    keyboard: Option<KeyboardAdjustmentState<T>>,
    pointer: Option<pointer::PointerScrubState<T>>,
    interaction_gate: NumericInteractionGate,
    step_modifiers: Option<NumericStepModifiers>,
    scrub_policy: Option<NumericScrubPolicy>,
    output_mode: NumericInputOutputMode,
    output_encoder: NumericInputOutputEncoder<T>,
    pointer_policy: Rc<dyn pointer::PointerScrubInteractionPolicy<T>>,
    keyboard_policy: Rc<dyn KeyboardAdjustmentPolicy<T>>,
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
            keyboard: self.keyboard.clone(),
            pointer: self.pointer.clone(),
            interaction_gate: self.interaction_gate,
            step_modifiers: self.step_modifiers,
            scrub_policy: self.scrub_policy,
            output_mode: self.output_mode,
            output_encoder: Rc::clone(&self.output_encoder),
            pointer_policy: Rc::clone(&self.pointer_policy),
            keyboard_policy: Rc::clone(&self.keyboard_policy),
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
            .field(
                "keyboard",
                &self.keyboard.as_ref().map(|keyboard| keyboard.key),
            )
            .field("pointer", &self.pointer.as_ref().map(|_| "active"))
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
            keyboard: None,
            pointer: None,
            interaction_gate: NumericInteractionGate::new(),
            step_modifiers: None,
            scrub_policy: None,
            output_mode: NumericInputOutputMode::Compatibility,
            output_encoder: compatibility_output_encoder(),
            pointer_policy: pointer::no_pointer_scrub_interaction_policy(),
            keyboard_policy: no_keyboard_adjustment_policy(),
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

    pub(crate) fn set_compatibility_output_mode(&mut self) {
        self.output_mode = NumericInputOutputMode::Compatibility;
        self.output_encoder = compatibility_output_encoder();
        self.pointer_policy = pointer::no_pointer_scrub_interaction_policy();
        self.keyboard_policy = no_keyboard_adjustment_policy();
    }

    pub(crate) fn set_complete_output_mode(&mut self)
    where
        A::Error: 'static,
        C::Error: 'static,
    {
        self.output_mode = NumericInputOutputMode::Complete;
        self.output_encoder = Rc::new(encode_complete_output::<T, A::Error, C::Error>);
        self.pointer_policy = pointer::complete_pointer_scrub_interaction_policy(
            Rc::clone(&self.codec),
            Rc::clone(&self.adjustment),
        );
        self.keyboard_policy = complete_keyboard_adjustment_policy(
            Rc::clone(&self.codec),
            Rc::clone(&self.adjustment),
        );
    }

    fn encode_output(&self, batch: NumericInputEditBatch<T>) -> WidgetOutput {
        (self.output_encoder)(batch)
    }

    fn is_editable(&self) -> bool {
        self.text_input.common.state.focused
            && !self.text_input.common.state.disabled
            && !self.text_input.common.state.read_only
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
            self.interaction_gate
                .release(NumericInteractionOwner::TextEdit);
        }
        batch
    }

    fn cancel_active(
        &mut self,
        timestamp: Option<crate::gui::input::InputTimestamp>,
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
            self.interaction_gate
                .release(NumericInteractionOwner::TextEdit);
        }
        batch
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

    fn pointer_scrub_is_configured(&self) -> bool {
        self.output_mode == NumericInputOutputMode::Complete
            && self.scrub_policy.is_some()
            && self.text_input.common.focus != FocusBehavior::None
            && !self.text_input.common.state.disabled
            && !self.text_input.common.state.read_only
    }

    fn pointer_scrub_is_editable(&self) -> bool {
        self.pointer_scrub_is_configured() && self.is_editable()
    }

    fn encode_pointer_edit(&self, edit: NumericInputEditBatch<T>) -> Option<WidgetOutput> {
        self.pointer_policy.encode_edit(edit)
    }

    fn restore_pointer_snapshot(&mut self, state: &pointer::PointerScrubState<T>) {
        self.value = state.session.begin_event().value.clone();
        self.text_input.state.value = state.start_text.clone();
        self.text_input.state.caret = state.start_caret;
        self.text_input.state.selection_anchor = state.start_selection_anchor;
    }

    fn cancel_pointer_state(
        &mut self,
        state: pointer::PointerScrubState<T>,
    ) -> Option<WidgetOutput> {
        if !state.published_update {
            self.restore_pointer_snapshot(&state);
            self.interaction_gate
                .release(NumericInteractionOwner::PointerScrub);
            self.pointer = None;
            return None;
        }
        let start_text = state.start_text.clone();
        let start_caret = state.start_caret;
        let start_selection_anchor = state.start_selection_anchor;
        self.restore_pointer_snapshot(&state);
        let cancel = match state.cancel() {
            Ok(cancel) => cancel,
            Err(state) => {
                self.pointer = Some(state);
                return None;
            }
        };
        self.value = cancel.value.clone();
        self.text_input.state.value = start_text;
        self.text_input.state.caret = start_caret;
        self.text_input.state.selection_anchor = start_selection_anchor;
        self.interaction_gate
            .release(NumericInteractionOwner::PointerScrub);
        self.pointer = None;
        let edit = NumericInputEditBatch::from_events(&[cancel])?;
        self.encode_pointer_edit(edit)
    }

    fn pointer_failure(
        &mut self,
        state: pointer::PointerScrubState<T>,
        failure: Option<WidgetOutput>,
    ) -> Option<WidgetOutput> {
        self.restore_pointer_snapshot(&state);
        self.interaction_gate
            .release(NumericInteractionOwner::PointerScrub);
        self.pointer = None;
        failure
    }

    fn handle_pointer_press(
        &mut self,
        position: Point,
        button: PointerButton,
        modifiers: PointerModifiers,
        timestamp: Option<crate::gui::input::InputTimestamp>,
    ) -> Option<WidgetOutput> {
        let Some(policy) = self.scrub_policy else {
            return None;
        };
        if !self.pointer_scrub_is_editable()
            || !policy.qualifies(button, modifiers)
            || !self
                .interaction_gate
                .try_admit(NumericInteractionOwner::PointerScrub)
        {
            return None;
        }
        let step = policy.select_step(modifiers);
        self.pointer = Some(pointer::PointerScrubState::new(
            self.value.clone(),
            self.text_input.state.value.clone(),
            position,
            step,
            modifiers,
            timestamp,
            self.text_input.state.caret,
            self.text_input.state.selection_anchor,
        ));
        None
    }

    fn handle_pointer_move(
        &mut self,
        bounds: Rect,
        position: Point,
        modifiers: PointerModifiers,
        timestamp: Option<crate::gui::input::InputTimestamp>,
        sequence_range: Option<crate::gui::input::InputSequenceRange>,
    ) -> Option<WidgetOutput> {
        let Some(mut state) = self.pointer.take() else {
            return None;
        };
        if !self.pointer_scrub_is_editable() {
            return self.cancel_pointer_state(state);
        }
        let Some(policy) = self.scrub_policy else {
            self.pointer = Some(state);
            return None;
        };
        let Some(normalized_delta) = state.normalized_delta(bounds, position) else {
            self.pointer = Some(state);
            return None;
        };
        let step = policy.select_step(modifiers);
        if step != state.step {
            state.reanchor(position, self.value.clone(), step);
            self.pointer = Some(state);
            return None;
        }
        if normalized_delta == 0.0 {
            self.pointer = Some(state);
            return None;
        }

        let attempt = if state.published_update {
            NumericScrubAttempt::Update
        } else {
            NumericScrubAttempt::Initial
        };
        let provenance = InteractionProvenance::Pointer {
            modifiers,
            timestamp,
            sequence_range,
        };
        let rollback = state
            .published_update
            .then(|| state.rollback_batch())
            .flatten();
        let candidate = match self.pointer_policy.scrub(
            &state.anchor_value,
            normalized_delta,
            step,
            attempt,
            provenance,
            rollback.clone(),
        ) {
            Ok(candidate) => candidate,
            Err(output) => return self.pointer_failure(state, output),
        };
        if candidate == self.value {
            self.pointer = Some(state);
            return None;
        }

        let draft = match self.pointer_policy.format(
            &candidate,
            attempt,
            normalized_delta,
            step,
            provenance,
            rollback,
        ) {
            Ok(draft) => draft,
            Err(output) => return self.pointer_failure(state, output),
        };
        let Some(edit) = state.accept_update(candidate.clone(), draft, position, provenance) else {
            self.pointer = Some(state);
            return None;
        };
        self.value = candidate;
        self.text_input.state.value = state.session.draft().to_owned();
        let end = self.text_input.state.char_len();
        self.text_input.state.caret = end;
        self.text_input.state.selection_anchor = end;
        self.pointer = Some(state);
        self.encode_pointer_edit(edit)
    }

    fn handle_pointer_release(
        &mut self,
        button: PointerButton,
        modifiers: PointerModifiers,
        timestamp: Option<crate::gui::input::InputTimestamp>,
    ) -> Option<WidgetOutput> {
        let Some(state) = self.pointer.take() else {
            return None;
        };
        if button != PointerButton::Primary {
            self.pointer = Some(state);
            return None;
        }
        let commit = match state.commit(self.value.clone(), modifiers, timestamp) {
            Ok(commit) => commit,
            Err(state) => {
                self.pointer = Some(state);
                return None;
            }
        };
        self.interaction_gate
            .release(NumericInteractionOwner::PointerScrub);
        let edit = NumericInputEditBatch::from_events(&[commit])?;
        self.encode_pointer_edit(edit)
    }

    fn handle_pointer_press_preflight(&self, input: &WidgetInput) -> PointerPressPreflight {
        let WidgetInput::PointerPress {
            button, modifiers, ..
        } = input
        else {
            return PointerPressPreflight::Allow;
        };
        let Some(policy) = self.scrub_policy else {
            return PointerPressPreflight::Allow;
        };
        if !self.pointer_scrub_is_configured() || !policy.qualifies(*button, *modifiers) {
            return PointerPressPreflight::Allow;
        }
        self.interaction_gate
            .incumbent()
            .map(|_| PointerPressPreflight::Consume)
            .unwrap_or(PointerPressPreflight::Allow)
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

    fn handle_focus_loss(&mut self, bounds: Rect) -> Option<NumericInputEditBatch<T>> {
        if self.active.is_some() {
            let output = self.commit_active(None);
            if output.is_some() {
                let _ = self
                    .text_input
                    .handle_input(bounds, WidgetInput::FocusChanged(false));
            }
            output
        } else {
            let _ = self
                .text_input
                .handle_input(bounds, WidgetInput::FocusChanged(false));
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
}

impl<T, C, A> Widget for NumericInputWidget<T, C, A>
where
    T: Clone + PartialEq + 'static,
    C: NumericCodec<T> + 'static,
    A: NumericAdjustment<T> + 'static,
{
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

    fn preflight_pointer_press(&self, input: &WidgetInput) -> PointerPressPreflight {
        self.handle_pointer_press_preflight(input)
    }

    fn handle_input(
        &mut self,
        bounds: Rect,
        input: WidgetInput,
    ) -> Option<crate::widgets::WidgetOutput> {
        if matches!(&input, WidgetInput::FocusChanged(false)) {
            if let Some(state) = self.pointer.take() {
                let output = self.cancel_pointer_state(state);
                let _ = self
                    .text_input
                    .handle_input(bounds, WidgetInput::FocusChanged(false));
                return output;
            }
            if self.active.is_some() {
                return self
                    .handle_focus_loss(bounds)
                    .map(|batch| self.encode_output(batch));
            }
            if self.keyboard.is_some() {
                let output = self.cancel_keyboard(None);
                let _ = self
                    .text_input
                    .handle_input(bounds, WidgetInput::FocusChanged(false));
                return output;
            }
            let _ = self
                .text_input
                .handle_input(bounds, WidgetInput::FocusChanged(false));
            return None;
        }

        if self.pointer.is_some() {
            match &input {
                WidgetInput::PointerMove {
                    position,
                    modifiers,
                    timestamp,
                    sequence_range,
                } => {
                    return self.handle_pointer_move(
                        bounds,
                        *position,
                        *modifiers,
                        *timestamp,
                        *sequence_range,
                    );
                }
                WidgetInput::PointerRelease {
                    button,
                    modifiers,
                    timestamp,
                    ..
                } => return self.handle_pointer_release(*button, *modifiers, *timestamp),
                WidgetInput::PointerModifiersChanged { .. } => return None,
                WidgetInput::KeyPress {
                    key: WidgetKey::Escape,
                    ..
                } => {
                    return self
                        .pointer
                        .take()
                        .and_then(|state| self.cancel_pointer_state(state));
                }
                _ => return None,
            }
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

        if let WidgetInput::PointerPress {
            position,
            button,
            modifiers,
            timestamp,
        } = &input
            && self.pointer_scrub_is_configured()
            && self
                .scrub_policy
                .is_some_and(|policy| policy.qualifies(*button, *modifiers))
        {
            return self.handle_pointer_press(*position, *button, *modifiers, *timestamp);
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
            } if self.active.is_some() => {
                return self
                    .cancel_active(*timestamp)
                    .map(|batch| self.encode_output(batch));
            }
            WidgetInput::KeyPress {
                key: WidgetKey::Enter,
                timestamp,
                ..
            } => {
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
        let _ = self.text_input.handle_input(bounds, input);
        let value_changed = self
            .active
            .as_ref()
            .is_some_and(|active| active.session.draft() != self.text_input.state.value);
        if value_changed {
            self.update_active_draft();
        } else if started_session {
            self.active = None;
            self.interaction_gate
                .release(NumericInteractionOwner::TextEdit);
        }
        None
    }

    fn synchronize_from_previous(&mut self, previous: &dyn Widget) {
        let Some(previous) = previous.as_any().downcast_ref::<Self>() else {
            self.active = None;
            self.keyboard = None;
            self.pointer = None;
            self.interaction_gate = NumericInteractionGate::new();
            return;
        };
        if self.text_input.common.id != previous.text_input.common.id {
            self.active = None;
            self.keyboard = None;
            self.pointer = None;
            self.interaction_gate = NumericInteractionGate::new();
            return;
        }

        let reset = self.value != previous.value
            || self.output_mode != previous.output_mode
            || self.step_modifiers != previous.step_modifiers
            || self.scrub_policy != previous.scrub_policy
            || self.text_input.common.state.disabled
            || previous.text_input.common.state.disabled
            || self.text_input.common.state.read_only
            || previous.text_input.common.state.read_only;
        if reset {
            self.active = None;
            self.keyboard = None;
            self.pointer = None;
            self.interaction_gate = NumericInteractionGate::new();
            return;
        }

        self.text_input.common.state = previous.text_input.common.state;
        if previous.pointer.is_some() {
            self.text_input.state = previous.text_input.state.clone();
            self.active = None;
            self.keyboard = None;
            self.pointer = previous.pointer.clone();
            self.interaction_gate = previous.interaction_gate;
        } else if previous.active.is_some() {
            self.text_input.state = previous.text_input.state.clone();
            self.active = previous.active.clone();
            self.keyboard = None;
            self.pointer = None;
            self.interaction_gate = previous.interaction_gate;
        } else if previous.keyboard.is_some() {
            self.text_input.state = previous.text_input.state.clone();
            self.active = None;
            self.keyboard = previous.keyboard.clone();
            self.pointer = None;
            self.interaction_gate = previous.interaction_gate;
        } else {
            self.active = None;
            self.keyboard = None;
            self.pointer = None;
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
                    && !self.text_input.common.state.disabled
                    && !self.text_input.common.state.read_only
                    && !successor.text_input.common.state.disabled
                    && !successor.text_input.common.state.read_only
            });
        if compatible {
            return None;
        }

        if self.pointer.is_some() {
            self.pointer
                .take()
                .and_then(|state| self.cancel_pointer_state(state))
        } else if self.keyboard.is_some() {
            self.cancel_keyboard(None)
        } else {
            self.cancel_active(None)
                .map(|batch| self.encode_output(batch))
        }
    }

    fn accepts_text_input(&self) -> bool {
        self.is_editable()
    }

    fn preempts_host_shortcut_key(&self, key: WidgetKey) -> bool {
        key == WidgetKey::Escape
            && (self.active.is_some() || self.keyboard.is_some() || self.pointer.is_some())
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

    fn accepts_pointer_move(&self) -> bool {
        false
    }

    fn pointer_capture_policy(&self) -> PointerCapturePolicy {
        if self.pointer.is_some() {
            PointerCapturePolicy::Exclusive
        } else {
            PointerCapturePolicy::PassThrough
        }
    }

    fn handle_pointer_capture_cancelled(&mut self, bounds: Rect) -> Option<WidgetOutput> {
        if let Some(state) = self.pointer.take() {
            return self.cancel_pointer_state(state);
        }
        let _ = self.handle_input(bounds, WidgetInput::FocusChanged(false));
        None
    }

    fn capabilities(&self) -> WidgetCapabilities<'_> {
        WidgetCapabilities::new().semantics(self)
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
        layout: &LayoutOutput,
        theme: &ThemeTokens,
    ) {
        Widget::append_paint(&self.text_input, primitives, bounds, layout, theme);
    }
}

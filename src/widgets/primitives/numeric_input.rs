//! Text-first generic numeric input built on the retained text-input primitive.

#[cfg(test)]
mod tests;

use std::{fmt, rc::Rc};

use crate::{
    gui::types::Rect,
    layout::LayoutOutput,
    runtime::PaintPrimitive,
    theme::ThemeTokens,
    widgets::{
        EditEvent, FocusLossDecision, InteractionProvenance, NumericAdjustment, NumericCodec,
        NumericEditSession, NumericInputConstructionError, NumericInputEditBatch,
        NumericParseResult, NumericStepModifiers, TextAlign, TextBackgroundRole, TextColorRole,
        TextInputChrome, TextInputWidget, TextWrap, Widget, WidgetCapabilities, WidgetInput,
        WidgetKey, WidgetOutput, WidgetSemantics, WidgetSizing,
        interaction::{NumericInteractionGate, NumericInteractionOwner},
    },
};

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
    interaction_gate: NumericInteractionGate,
    step_modifiers: Option<NumericStepModifiers>,
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
            interaction_gate: self.interaction_gate,
            step_modifiers: self.step_modifiers,
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
            interaction_gate: NumericInteractionGate::new(),
            step_modifiers: None,
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

    fn handle_input(
        &mut self,
        bounds: Rect,
        input: WidgetInput,
    ) -> Option<crate::widgets::WidgetOutput> {
        if matches!(&input, WidgetInput::FocusChanged(false)) {
            let output = if self.active.is_some() {
                self.handle_focus_loss(bounds)
            } else {
                let _ = self
                    .text_input
                    .handle_input(bounds, WidgetInput::FocusChanged(false));
                None
            };
            return output.map(WidgetOutput::typed);
        }

        match &input {
            WidgetInput::KeyPress {
                key: WidgetKey::Escape,
                timestamp,
                ..
            } if self.active.is_some() => {
                return self.cancel_active(*timestamp).map(WidgetOutput::typed);
            }
            WidgetInput::KeyPress {
                key: WidgetKey::Enter,
                timestamp,
                ..
            } => {
                return self.commit_active(*timestamp).map(WidgetOutput::typed);
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
            self.interaction_gate = NumericInteractionGate::new();
            return;
        };
        if self.text_input.common.id != previous.text_input.common.id {
            self.active = None;
            self.interaction_gate = NumericInteractionGate::new();
            return;
        }

        let reset = self.value != previous.value
            || self.text_input.common.state.disabled
            || previous.text_input.common.state.disabled
            || self.text_input.common.state.read_only
            || previous.text_input.common.state.read_only;
        if reset {
            self.active = None;
            self.interaction_gate = NumericInteractionGate::new();
            return;
        }

        self.text_input.common.state = previous.text_input.common.state;
        if previous.active.is_some() {
            self.text_input.state = previous.text_input.state.clone();
            self.active = previous.active.clone();
            self.interaction_gate = previous.interaction_gate;
        } else {
            self.active = None;
            self.interaction_gate = NumericInteractionGate::new();
        }
    }

    fn accepts_text_input(&self) -> bool {
        self.is_editable()
    }

    fn preempts_host_shortcut_key(&self, key: WidgetKey) -> bool {
        key == WidgetKey::Escape && self.active.is_some() && self.is_editable()
    }

    fn accepts_pointer_move(&self) -> bool {
        false
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

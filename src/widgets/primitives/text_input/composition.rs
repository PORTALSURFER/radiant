//! Single-line text-input consumption of backend-neutral IME composition.

use super::editing_ops::byte_index_for_char;
use super::{TextInputMessage, TextInputState, TextInputWidget};
use crate::widgets::contract::Widget;
use crate::widgets::interaction::{CompositionRange, CompositionSample, CompositionSelectionState};

/// Widget-local composition state captured at `Start`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TextInputComposition {
    original_value: String,
    replacement_range: CompositionRange,
    original_selection: CompositionRange,
    preedit: String,
    preedit_selection: CompositionSelectionState,
}

pub(super) fn handle_sample(
    text_input: &mut TextInputWidget,
    sample: CompositionSample,
) -> Option<TextInputMessage> {
    if !text_input.accepts_editing_input() || !sample.is_valid() {
        return None;
    }
    text_input.reset_native_pointer_affinity();

    match sample {
        CompositionSample::Start {
            replacement_range,
            selection,
            ..
        } => text_input.start_composition(replacement_range, selection),
        CompositionSample::Update {
            preedit, selection, ..
        } => text_input.update_composition(preedit, CompositionSelectionState::Visible(selection)),
        CompositionSample::Commit { text, .. } => text_input.commit_composition(text),
        CompositionSample::Cancel { .. } => {
            text_input.cancel_composition();
            None
        }
    }
}

pub(super) fn handle_hidden_update(
    text_input: &mut TextInputWidget,
    preedit: String,
) -> Option<TextInputMessage> {
    if !text_input.accepts_editing_input() {
        return None;
    }
    text_input.reset_native_pointer_affinity();
    text_input.update_composition(preedit, CompositionSelectionState::Hidden)
}

impl TextInputWidget {
    pub(super) fn start_composition(
        &mut self,
        replacement_range: CompositionRange,
        selection: CompositionRange,
    ) -> Option<TextInputMessage> {
        if self.composition.is_some() {
            return None;
        }
        let scalar_len = self.state.char_len();
        if !replacement_range.is_valid_for(scalar_len) || !selection.is_valid_for(scalar_len) {
            return None;
        }
        self.composition = Some(TextInputComposition {
            original_value: self.state.value.clone(),
            replacement_range,
            original_selection: selection,
            preedit: String::new(),
            preedit_selection: CompositionSelectionState::Unreported,
        });
        set_state_selection(&mut self.state, selection);
        None
    }

    pub(super) fn update_composition(
        &mut self,
        preedit: String,
        selection: CompositionSelectionState,
    ) -> Option<TextInputMessage> {
        let mut composition = self.composition.take()?;
        let display_selection = match selection {
            CompositionSelectionState::Visible(selection) => {
                if !selection.is_valid_for(preedit.chars().count()) {
                    self.restore_composition(composition);
                    return None;
                }
                let Some(display_selection) =
                    display_selection(composition.replacement_range, selection)
                else {
                    self.restore_composition(composition);
                    return None;
                };
                Some(display_selection)
            }
            CompositionSelectionState::Unreported | CompositionSelectionState::Hidden => None,
        };

        composition.preedit = preedit;
        composition.preedit_selection = selection;
        self.state.value = display_value(
            &composition.original_value,
            composition.replacement_range,
            &composition.preedit,
        );
        if let Some(display_selection) = display_selection {
            set_state_selection(&mut self.state, display_selection);
        }
        self.composition = Some(composition);
        None
    }

    pub(super) fn commit_composition(&mut self, text: String) -> Option<TextInputMessage> {
        let composition = self.composition.take()?;
        let mut committed_state = TextInputState::from_value(display_value(
            &composition.original_value,
            composition.replacement_range,
            "",
        ));
        committed_state.set_caret(composition.replacement_range.start(), false);
        committed_state.insert_text(&text, self.props.character_limit);
        let value = committed_state.value.clone();
        self.state = committed_state;
        Some(TextInputMessage::Changed { value })
    }

    pub(super) fn cancel_composition(&mut self) {
        let Some(composition) = self.composition.take() else {
            return;
        };
        self.restore_composition(composition);
    }

    fn restore_composition(&mut self, composition: TextInputComposition) {
        self.state.value = composition.original_value;
        set_state_selection(&mut self.state, composition.original_selection);
    }

    pub(super) fn composition_hides_native_adornments(&self) -> bool {
        self.composition
            .as_ref()
            .is_some_and(|composition| composition.preedit_selection.is_hidden())
    }

    pub(super) fn committed_value_for_sync(&self) -> &str {
        self.composition
            .as_ref()
            .map_or(self.state.value.as_str(), |composition| {
                composition.original_value.as_str()
            })
    }

    #[cfg(test)]
    pub(super) fn composition_preedit_selection(&self) -> Option<CompositionRange> {
        self.composition
            .as_ref()
            .and_then(|composition| composition.preedit_selection.visible_range())
    }

    #[cfg(test)]
    pub(super) fn composition_preedit_selection_state(&self) -> CompositionSelectionState {
        self.composition
            .as_ref()
            .map_or(CompositionSelectionState::Unreported, |composition| {
                composition.preedit_selection
            })
    }

    pub(super) fn can_preserve_composition_with(&self, successor: Option<&dyn Widget>) -> bool {
        let Some(successor) =
            successor.and_then(|widget| widget.as_any().downcast_ref::<TextInputWidget>())
        else {
            return false;
        };
        if successor.common.id != self.common.id
            || successor.common.state.disabled
            || successor.common.state.read_only
        {
            return false;
        }
        match (self.props.revision, successor.props.revision) {
            (Some(previous), Some(current)) => current <= previous,
            (None, None) => successor.state.value == self.committed_value_for_sync(),
            (Some(_), None) | (None, Some(_)) => false,
        }
    }
}

fn display_value(
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

fn set_state_selection(state: &mut TextInputState, selection: CompositionRange) {
    state.selection_anchor = selection.start();
    state.caret = selection.end();
}

fn display_selection(
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

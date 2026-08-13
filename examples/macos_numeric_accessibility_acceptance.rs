//! macOS-only live acceptance harness for the ordinary native numeric-action contract.
//!
//! The native window is the acceptance surface: the numeric control is built
//! through the public application builder and complete NumericInput policy.
//! Automated tests below only inspect the production runtime projection; they
//! do not synthesize an accessibility element or claim live AppKit evidence.

#[cfg(any(target_os = "macos", test))]
use radiant::prelude::*;
#[cfg(any(target_os = "macos", test))]
use radiant::{
    application::{View, column, numeric_input, text},
    widgets::{
        EditPhase, NumericAccessibilityOutcome, NumericAdjustment, NumericCodec,
        NumericInputEditBatch, NumericInputInteraction, NumericInputInteractionBatch,
        NumericParseResult, NumericStep, NumericStepDirection,
    },
};
#[cfg(any(target_os = "macos", test))]
use std::fmt;

#[cfg(any(target_os = "macos", test))]
const NUMERIC_INPUT_ID: u64 = 20;

#[cfg(target_os = "macos")]
fn main() -> radiant::Result {
    app(AcceptanceState::default())
        .title("Radiant macOS Numeric Accessibility Acceptance")
        .size(640, 300)
        .min_size(520, 240)
        .view(project_surface)
        .update(update)
        .run()
}

#[cfg(not(target_os = "macos"))]
fn main() -> radiant::Result {
    Err("macos_numeric_accessibility_acceptance is macOS-only".to_owned())
}

#[cfg(any(target_os = "macos", test))]
#[derive(Clone, Copy, Debug, Default)]
struct AcceptanceCodec;

#[cfg(any(target_os = "macos", test))]
impl NumericCodec<f32> for AcceptanceCodec {
    type Error = &'static str;

    fn parse(&self, text: &str) -> NumericParseResult<f32> {
        if text.is_empty() || text == "-" || text.ends_with('.') {
            return NumericParseResult::Incomplete;
        }
        let Ok(value) = text.parse::<f32>() else {
            return NumericParseResult::Invalid;
        };
        if !value.is_finite() || !(0.0..=100.0).contains(&value) {
            NumericParseResult::OutOfRange
        } else {
            NumericParseResult::Valid(value)
        }
    }

    fn format_editable(
        &self,
        value: &f32,
        output: &mut dyn fmt::Write,
    ) -> std::result::Result<(), Self::Error> {
        write!(output, "{value:.2}").map_err(|_| "format")
    }
}

#[cfg(any(target_os = "macos", test))]
#[derive(Clone, Copy, Debug, Default)]
struct AcceptanceAdjustment;

#[cfg(any(target_os = "macos", test))]
impl NumericAdjustment<f32> for AcceptanceAdjustment {
    type Error = &'static str;

    fn normalized_to_value(&self, normalized: f32) -> std::result::Result<f32, Self::Error> {
        normalized
            .is_finite()
            .then_some(normalized.clamp(0.0, 1.0) * 100.0)
            .ok_or("normalized")
    }

    fn value_to_normalized(&self, value: &f32) -> std::result::Result<f32, Self::Error> {
        value
            .is_finite()
            .then_some((*value).clamp(0.0, 100.0) / 100.0)
            .ok_or("value")
    }

    fn step(
        &self,
        value: &f32,
        direction: NumericStepDirection,
        step: NumericStep,
    ) -> std::result::Result<f32, Self::Error> {
        let amount = match step {
            NumericStep::Base => 1.0,
            NumericStep::Fine => 0.1,
            NumericStep::Coarse => 10.0,
        };
        let signed = match direction {
            NumericStepDirection::Decrease => -amount,
            NumericStepDirection::Increase => amount,
        };
        value
            .is_finite()
            .then_some((*value + signed).clamp(0.0, 100.0))
            .ok_or("step")
    }

    fn scrub(
        &self,
        value: &f32,
        normalized_delta: f32,
        step: NumericStep,
    ) -> std::result::Result<f32, Self::Error> {
        let amount = match step {
            NumericStep::Base => 100.0,
            NumericStep::Fine => 10.0,
            NumericStep::Coarse => 100.0,
        };
        value
            .is_finite()
            .then_some((*value + normalized_delta * amount).clamp(0.0, 100.0))
            .ok_or("scrub")
    }

    fn wheel(
        &self,
        value: &f32,
        delta: f32,
        step: NumericStep,
    ) -> std::result::Result<f32, Self::Error> {
        self.scrub(value, delta / 120.0, step)
    }
}

#[cfg(any(target_os = "macos", test))]
#[derive(Clone, Debug)]
struct AcceptanceState {
    value: f32,
    status: String,
}

#[cfg(any(target_os = "macos", test))]
impl AcceptanceState {
    fn initial() -> Self {
        Self {
            value: 42.0,
            status: String::from("Ready for native AXIncrement and AXDecrement"),
        }
    }
}

#[cfg(any(target_os = "macos", test))]
impl Default for AcceptanceState {
    fn default() -> Self {
        Self::initial()
    }
}

#[cfg(any(target_os = "macos", test))]
type AcceptanceInteractionBatch = NumericInputInteractionBatch<f32, &'static str, &'static str>;

#[cfg(any(target_os = "macos", test))]
type AcceptanceAccessibilityOutcome = NumericAccessibilityOutcome<f32, &'static str, &'static str>;

#[cfg(any(target_os = "macos", test))]
#[derive(Debug)]
enum AcceptanceMessage {
    Interaction(AcceptanceInteractionBatch),
    Accessibility(AcceptanceAccessibilityOutcome),
}

#[cfg(any(target_os = "macos", test))]
fn project_surface(state: &AcceptanceState) -> View<AcceptanceMessage> {
    let control = match numeric_input(state.value, AcceptanceCodec, AcceptanceAdjustment) {
        Ok(builder) => builder.on_interaction_with_accessibility(
            AcceptanceMessage::Interaction,
            AcceptanceMessage::Accessibility,
        ),
        Err(error) => text(format!("Numeric input construction failed: {error:?}")),
    };

    column([
        text("Ordinary native NumericInput action acceptance")
            .primary()
            .fill_width(),
        text(
            "Use VoiceOver or an AX client to invoke only increment/decrement. The value below is application-owned and updates through the normal runtime dispatcher.",
        )
        .wrap()
        .fill_width(),
        control.id(NUMERIC_INPUT_ID).size(220.0, 36.0),
        text(format!("Current value: {:.2}", state.value)).fill_width(),
        text(format!("Status: {}", state.status))
            .wrap()
            .fill_width(),
    ])
    .padding(24.0)
    .spacing(14.0)
}

#[cfg(any(target_os = "macos", test))]
fn edit_value(edit: &NumericInputEditBatch<f32>) -> Option<f32> {
    edit.events()
        .iter()
        .rev()
        .find(|event| event.phase == EditPhase::Commit)
        .or_else(|| edit.events().last())
        .map(|event| event.value)
}

#[cfg(any(target_os = "macos", test))]
fn interaction_value(batch: &AcceptanceInteractionBatch) -> Option<f32> {
    batch.parts().iter().rev().find_map(|part| match part {
        NumericInputInteraction::Edit(edit) => edit_value(edit),
        _ => None,
    })
}

#[cfg(any(target_os = "macos", test))]
fn update(state: &mut AcceptanceState, message: AcceptanceMessage) {
    match message {
        AcceptanceMessage::Interaction(batch) => {
            if let Some(value) = interaction_value(&batch) {
                state.value = value;
            }
            state.status = String::from("Normal numeric interaction received");
        }
        AcceptanceMessage::Accessibility(outcome) => {
            if let AcceptanceAccessibilityOutcome::Edit(edit) = &outcome
                && let Some(value) = edit_value(edit)
            {
                state.value = value;
            }
            state.status = format!("Native numeric outcome: {outcome:?}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use radiant::{gui::automation::AutomationRole, layout::Vector2, runtime::SurfaceRuntime};

    #[test]
    fn production_stack_publishes_one_materialized_numeric_target() {
        let bridge = radiant::app(AcceptanceState::default())
            .view(project_surface)
            .update(update)
            .into_bridge();
        let runtime = SurfaceRuntime::new(bridge, Vector2::new(640.0, 300.0));
        let target = runtime
            .automation_target_snapshot()
            .targets
            .into_iter()
            .find(|target| target.id.0 == NUMERIC_INPUT_ID.to_string())
            .expect("the production numeric input should be an ordinary target");

        assert_eq!(target.role, AutomationRole::TextInput);
        assert!(target.enabled);
        assert!(target.focusable);
        assert_eq!(target.value.as_deref(), Some("42.00"));
        assert!(
            target
                .available_actions
                .iter()
                .any(|action| action == "increment")
        );
        assert!(
            target
                .available_actions
                .iter()
                .any(|action| action == "decrement")
        );
        assert!(
            target
                .authority
                .is_some_and(|authority| authority.materialized)
        );
    }

    #[test]
    fn application_update_reduces_a_native_edit_outcome() {
        let mut state = AcceptanceState::default();
        let provenance = radiant::widgets::InteractionProvenance::Accessibility;
        let begin = radiant::widgets::EditEvent::begin(42.0, provenance);
        let commit = begin
            .clone()
            .commit(43.0, provenance)
            .expect("the matching accessibility provenance should commit");
        let edit = NumericInputEditBatch::from_events(&[begin, commit])
            .expect("the widget's validated batch shape should be accepted");
        update(
            &mut state,
            AcceptanceMessage::Accessibility(AcceptanceAccessibilityOutcome::Edit(edit)),
        );

        assert_eq!(state.value, 43.0);
    }
}

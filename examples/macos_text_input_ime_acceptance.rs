//! Checked macOS live acceptance harness for the shipped single-line text input IME path.
//!
//! The primary window is the live acceptance surface. Its instructions require
//! an actual Japanese IME interaction, including candidate-panel and caret
//! observation. The deterministic tests below inspect only the production
//! runtime projection. This harness does not claim live AppKit evidence.

#[cfg(any(target_os = "macos", test))]
use radiant::prelude::*;
#[cfg(any(target_os = "macos", test))]
use radiant::widgets::{TextInputMessage, WidgetId};
#[cfg(test)]
use radiant::{
    layout::Vector2,
    runtime::{
        PaintPrimitive, PaintTextInput, SurfaceRuntime, UiSurface, declarative_owned_runtime_bridge,
    },
    theme::ThemeTokens,
    widgets::{CompositionRange, CompositionSample},
};
#[cfg(any(target_os = "macos", test))]
const TEXT_INPUT_ID: WidgetId = 20;
#[cfg(any(target_os = "macos", test))]
const STATUS_ID: WidgetId = 21;
#[cfg(any(target_os = "macos", test))]
const COMMITTED_VALUE_ID: WidgetId = 22;

#[cfg(target_os = "macos")]
fn main() -> radiant::Result {
    radiant::app(AcceptanceState::default())
        .title("Radiant macOS Japanese IME Acceptance")
        .size(820, 560)
        .min_size(640, 440)
        .view(project_surface)
        .on_startup(|_, context| context.focus(TEXT_INPUT_ID))
        .update(update)
        .run()
}

#[cfg(not(target_os = "macos"))]
fn main() -> radiant::Result {
    Err("macos_text_input_ime_acceptance is macOS-only".to_owned())
}

#[cfg(any(target_os = "macos", test))]
#[derive(Clone, Debug)]
struct AcceptanceState {
    value: String,
    changed_messages: usize,
    status: String,
}

#[cfg(any(target_os = "macos", test))]
impl AcceptanceState {
    fn with_value(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            changed_messages: 0,
            status: String::from("Ready; no committed text change"),
        }
    }
}

#[cfg(any(target_os = "macos", test))]
impl Default for AcceptanceState {
    fn default() -> Self {
        Self::with_value("日本語を入力")
    }
}

#[cfg(any(target_os = "macos", test))]
#[derive(Clone, Debug, PartialEq, Eq)]
enum AcceptanceMessage {
    TextInput(TextInputMessage),
}

#[cfg(any(target_os = "macos", test))]
fn project_surface(state: &AcceptanceState) -> View<AcceptanceMessage> {
    column([
        text("macOS Japanese IME acceptance")
            .primary()
            .fill_width(),
        text(
            "Live macOS step: keep this primary window focused, select the Japanese Hiragana IME, and type romaji such as kanji. Observe the underlined preedit, the candidate panel, and the caret location. Choose a candidate or press Return to commit, then confirm one committed application-state change. Start another composition and press Escape to cancel; the committed value and change count must stay unchanged. Start a composition again, switch focus to another application or window, and confirm focus loss restores the last committed value without a change."
        )
        .wrap()
        .fill_width(),
        text_input(state.value.clone())
            .select_all()
            .placeholder("Japanese IME text")
            .message_event(AcceptanceMessage::TextInput)
            .id(TEXT_INPUT_ID)
            .fill_width(),
        text(format!(
            "Committed application value: {:?}",
            state.value
        ))
        .id(COMMITTED_VALUE_ID)
        .fill_width(),
        text(format!(
            "Status: {} | changed messages: {}",
            state.status, state.changed_messages
        ))
        .id(STATUS_ID)
        .wrap()
        .fill_width(),
        text(
            "Native boundary: the existing runtime publishes the focused caret area through Winit's set_ime_cursor_area path. This harness does not report AppKit candidate-panel evidence until the live steps above are performed."
        )
        .wrap()
        .fill_width(),
    ])
    .padding(24.0)
    .spacing(16.0)
}

#[cfg(any(target_os = "macos", test))]
fn update(state: &mut AcceptanceState, message: AcceptanceMessage) {
    match message {
        AcceptanceMessage::TextInput(TextInputMessage::Changed { value }) => {
            state.value = value;
            state.changed_messages = state.changed_messages.saturating_add(1);
            state.status = String::from("One committed text change received");
        }
        AcceptanceMessage::TextInput(TextInputMessage::Submitted { value }) => {
            state.status = format!("Submit observed for committed value {value:?}");
        }
        AcceptanceMessage::TextInput(TextInputMessage::CompletionRequested { value }) => {
            state.status = format!("Completion requested for committed value {value:?}");
        }
    }
}

#[cfg(test)]
type AcceptanceBridge = radiant::runtime::DeclarativeOwnedRuntimeBridge<
    AcceptanceState,
    AcceptanceMessage,
    fn(&mut AcceptanceState) -> UiSurface<AcceptanceMessage>,
    fn(&mut AcceptanceState, AcceptanceMessage),
>;

#[cfg(test)]
fn project_test_surface(state: &mut AcceptanceState) -> UiSurface<AcceptanceMessage> {
    project_surface(state).into_surface()
}

#[cfg(test)]
fn reduce_test_message(state: &mut AcceptanceState, message: AcceptanceMessage) {
    update(state, message);
}

#[cfg(test)]
fn test_runtime(value: &str) -> SurfaceRuntime<AcceptanceBridge, AcceptanceMessage> {
    let project: fn(&mut AcceptanceState) -> UiSurface<AcceptanceMessage> = project_test_surface;
    let reduce: fn(&mut AcceptanceState, AcceptanceMessage) = reduce_test_message;
    let bridge =
        declarative_owned_runtime_bridge(AcceptanceState::with_value(value), project, reduce);
    SurfaceRuntime::new(bridge, Vector2::new(820.0, 560.0))
}

#[cfg(test)]
fn start_sample(start: usize, end: usize, scalar_len: usize) -> CompositionSample {
    let range = CompositionRange::new(start, end, scalar_len).expect("valid composition range");
    CompositionSample::start(range, range).expect("matching composition ranges")
}

#[cfg(test)]
fn painted_input(runtime: &SurfaceRuntime<AcceptanceBridge, AcceptanceMessage>) -> PaintTextInput {
    runtime
        .paint_plan(&ThemeTokens::default())
        .primitives
        .into_iter()
        .find_map(|primitive| match primitive {
            PaintPrimitive::TextInput(input) if input.widget_id == TEXT_INPUT_ID => Some(input),
            _ => None,
        })
        .expect("the acceptance surface should project one text input")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preedit_stays_widget_local_without_application_state_change() {
        let mut runtime = test_runtime("abc");
        assert!(runtime.focus_widget(TEXT_INPUT_ID));
        assert_eq!(runtime.bridge().state().value, "abc");
        assert_eq!(runtime.bridge().state().changed_messages, 0);

        assert_eq!(
            runtime.dispatch_focused_composition_sample(start_sample(0, 3, 3)),
            Some(TEXT_INPUT_ID)
        );
        assert_eq!(
            runtime.dispatch_focused_composition_sample(
                CompositionSample::update(
                    "かな",
                    CompositionRange::new(0, 2, 2).expect("valid preedit selection"),
                )
                .expect("valid preedit update"),
            ),
            Some(TEXT_INPUT_ID)
        );

        assert_eq!(painted_input(&runtime).state.value, "かな");
        assert_eq!(runtime.bridge().state().value, "abc");
        assert_eq!(runtime.bridge().state().changed_messages, 0);
        assert_eq!(
            runtime.bridge().state().status,
            "Ready; no committed text change"
        );
    }

    #[test]
    fn commit_replaces_once_and_emits_one_changed_message() {
        let mut runtime = test_runtime("abc");
        assert!(runtime.focus_widget(TEXT_INPUT_ID));
        assert_eq!(
            runtime.dispatch_focused_composition_sample(start_sample(1, 2, 3)),
            Some(TEXT_INPUT_ID)
        );
        assert_eq!(
            runtime.dispatch_focused_composition_sample(
                CompositionSample::update(
                    "かな",
                    CompositionRange::new(0, 2, 2).expect("valid preedit selection"),
                )
                .expect("valid preedit update"),
            ),
            Some(TEXT_INPUT_ID)
        );
        assert_eq!(runtime.bridge().state().value, "abc");
        assert_eq!(runtime.bridge().state().changed_messages, 0);

        assert_eq!(
            runtime.dispatch_focused_composition_sample(CompositionSample::commit("日")),
            Some(TEXT_INPUT_ID)
        );

        assert_eq!(runtime.bridge().state().value, "a日c");
        assert_eq!(runtime.bridge().state().changed_messages, 1);
        assert_eq!(
            runtime.bridge().state().status,
            "One committed text change received"
        );
        assert_eq!(painted_input(&runtime).state.value, "a日c");
    }

    #[test]
    fn disabled_cancel_restores_committed_state_without_a_change() {
        let mut runtime = test_runtime("abc");
        assert!(runtime.focus_widget(TEXT_INPUT_ID));
        assert_eq!(
            runtime.dispatch_focused_composition_sample(start_sample(0, 3, 3)),
            Some(TEXT_INPUT_ID)
        );
        assert_eq!(
            runtime.dispatch_focused_composition_sample(
                CompositionSample::update(
                    "かな",
                    CompositionRange::new(0, 2, 2).expect("valid preedit selection"),
                )
                .expect("valid preedit update"),
            ),
            Some(TEXT_INPUT_ID)
        );

        // Native Ime::Disabled maps to this public cancel path. This test
        // stops at runtime projection and does not fabricate AppKit evidence.
        // Cancel has no application message, so the public dispatch result is
        // intentionally unused after the focused owner restores its state.
        let _ = runtime.dispatch_focused_composition_sample(CompositionSample::cancel());

        assert_eq!(painted_input(&runtime).state.value, "abc");
        assert_eq!(runtime.bridge().state().value, "abc");
        assert_eq!(runtime.bridge().state().changed_messages, 0);
        assert_eq!(
            runtime.bridge().state().status,
            "Ready; no committed text change"
        );
    }

    #[test]
    fn focus_loss_restores_committed_state_without_a_change() {
        let mut runtime = test_runtime("abc");
        assert!(runtime.focus_widget(TEXT_INPUT_ID));
        assert_eq!(
            runtime.dispatch_focused_composition_sample(start_sample(0, 3, 3)),
            Some(TEXT_INPUT_ID)
        );
        assert_eq!(
            runtime.dispatch_focused_composition_sample(
                CompositionSample::update(
                    "かな",
                    CompositionRange::new(0, 2, 2).expect("valid preedit selection"),
                )
                .expect("valid preedit update"),
            ),
            Some(TEXT_INPUT_ID)
        );

        runtime.clear_focus();

        assert_eq!(runtime.focused_widget(), None);
        assert_eq!(painted_input(&runtime).state.value, "abc");
        assert_eq!(runtime.bridge().state().value, "abc");
        assert_eq!(runtime.bridge().state().changed_messages, 0);
        assert_eq!(
            runtime.bridge().state().status,
            "Ready; no committed text change"
        );
    }

    #[test]
    fn focused_text_input_publishes_the_caret_area_source_for_native_ime() {
        let mut runtime = test_runtime("abc");
        assert!(runtime.focus_widget(TEXT_INPUT_ID));

        let input = painted_input(&runtime);

        // This proves only the focused, finite production projection consumed
        // by the existing native set_ime_cursor_area path; it is not live
        // AppKit candidate-panel evidence.
        assert!(input.focused);
        assert!(input.rect.width().is_finite() && input.rect.width() > 0.0);
        assert!(input.rect.height().is_finite() && input.rect.height() > 0.0);
        assert!(input.state.caret <= input.state.value.chars().count());
    }
}

//! macOS-only live acceptance harness for the public devtools overlay contract.

#[cfg(any(target_os = "macos", test))]
use radiant::prelude::*;
#[cfg(any(target_os = "macos", test))]
use radiant::runtime::DevtoolsOverlayOptions;

const PRIMARY_ACTION_ID: u64 = 10;
const SECONDARY_ACTION_ID: u64 = 11;
const INPUT_ID: u64 = 12;
const TOGGLE_ID: u64 = 13;
const STATUS_ID: u64 = 14;
/// Maximum number of Unicode scalar values retained from the text input.
const MAX_INPUT_CHARS: usize = 64;

#[cfg(target_os = "macos")]
fn main() -> radiant::Result {
    radiant::app(AcceptanceState::default())
        .title("Radiant macOS Devtools Acceptance")
        .size(1120, 680)
        .min_size(860, 460)
        .devtools_overlay(DevtoolsOverlayOptions::enabled())
        .view(project_surface)
        .update(update)
        .run()
}

#[cfg(not(target_os = "macos"))]
fn main() -> radiant::Result {
    Err("macos_devtools_acceptance is macOS-only".to_owned())
}

#[cfg(any(target_os = "macos", test))]
#[derive(Clone, Debug)]
struct AcceptanceState {
    input: String,
    enabled: bool,
    action_count: u32,
    last_action: String,
}

#[cfg(any(target_os = "macos", test))]
impl AcceptanceState {
    fn initial() -> Self {
        Self {
            input: String::from("Edit this text"),
            enabled: true,
            action_count: 0,
            last_action: String::from("Ready for pointer, focus, resize, and editing checks"),
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
#[derive(Clone, Debug, PartialEq, Eq)]
enum AcceptanceMessage {
    PrimaryAction,
    SecondaryAction,
    InputChanged(String),
    ToggleChanged(bool),
}

#[cfg(any(target_os = "macos", test))]
fn project_surface(state: &AcceptanceState) -> View<AcceptanceMessage> {
    column([
        text("macOS live devtools acceptance")
            .primary()
            .id(1)
            .fill_width(),
        text(
            "The inspector is observational runtime paint only: use the normal controls below, then resize the window. Move the pointer across controls and use Tab/Shift-Tab to verify that hit testing and keyboard focus remain ordinary.",
        )
        .wrap()
        .id(2)
        .fill_width(),
        row([
            button("Primary action")
                .primary()
                .message(AcceptanceMessage::PrimaryAction)
                .id(PRIMARY_ACTION_ID)
                .size(150.0, 36.0),
            button("Secondary action")
                .message(AcceptanceMessage::SecondaryAction)
                .id(SECONDARY_ACTION_ID)
                .size(170.0, 36.0),
        ])
        .spacing(12.0)
        .id(3),
        row([
            text("Text input").size(100.0, 36.0),
            text_input(state.input.clone())
                .message(AcceptanceMessage::InputChanged)
                .id(INPUT_ID)
                .fill_width(),
        ])
        .fill_width()
        .spacing(12.0)
        .id(4),
        row([
            toggle("Enabled", state.enabled)
                .message(AcceptanceMessage::ToggleChanged)
                .id(TOGGLE_ID),
            text(format!(
                "Actions: {} | enabled: {}",
                state.action_count, state.enabled
            ))
            .fill_width()
            .id(5),
        ])
        .fill_width()
        .spacing(16.0)
        .id(6),
        text(format!("Last interaction: {}", state.last_action))
            .wrap()
            .id(STATUS_ID)
            .fill_width(),
        text(
            "Acceptance matrix: overlay visible at startup; hover selection and selected-node metadata change over each control; clicking and editing the text input reports focus; Tab traversal changes focus without overlay interference; resizing updates the selected bounds while controls continue to work.",
        )
        .wrap()
        .id(7)
        .fill_width(),
    ])
    .padding(24.0)
    .spacing(16.0)
}

#[cfg(any(target_os = "macos", test))]
fn update(state: &mut AcceptanceState, message: AcceptanceMessage) {
    match message {
        AcceptanceMessage::PrimaryAction => {
            state.action_count = state.action_count.saturating_add(1);
            state.last_action = String::from("Primary action activated");
        }
        AcceptanceMessage::SecondaryAction => {
            state.action_count = state.action_count.saturating_add(1);
            state.last_action = String::from("Secondary action activated");
        }
        AcceptanceMessage::InputChanged(value) => {
            state.input = value.chars().take(MAX_INPUT_CHARS).collect();
            state.last_action = String::from("Text input edited");
        }
        AcceptanceMessage::ToggleChanged(enabled) => {
            state.enabled = enabled;
            state.last_action = String::from("Toggle changed");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use radiant::{layout::Vector2, runtime::SurfaceRuntime};

    #[test]
    fn acceptance_builder_uses_the_existing_enabled_overlay_option() {
        let _builder = radiant::app(AcceptanceState::default())
            .devtools_overlay(DevtoolsOverlayOptions::enabled());
    }

    #[test]
    fn acceptance_surface_exposes_focusable_controls_and_one_text_input() {
        let bridge = radiant::app(AcceptanceState::default())
            .view(project_surface)
            .update(update)
            .into_bridge();
        let runtime = SurfaceRuntime::new(bridge, Vector2::new(1120.0, 680.0));
        let snapshot = runtime.devtools_snapshot();

        for node_id in [PRIMARY_ACTION_ID, SECONDARY_ACTION_ID, TOGGLE_ID, INPUT_ID] {
            assert!(
                snapshot.root.find_node(node_id).is_some(),
                "acceptance control {node_id} should be present in the inspector tree"
            );
        }
        let input = snapshot
            .root
            .find_node(INPUT_ID)
            .expect("acceptance text input should be present");
        assert!(input.widget.as_ref().is_some_and(|widget| widget.focusable));
        assert_eq!(
            input
                .widget
                .as_ref()
                .and_then(|widget| widget.semantics.value_text.as_deref()),
            Some("Edit this text")
        );
    }

    #[test]
    fn input_state_is_bounded_by_the_documented_character_limit() {
        let mut state = AcceptanceState::default();
        let input = format!("{}overflow", "é".repeat(MAX_INPUT_CHARS));

        update(&mut state, AcceptanceMessage::InputChanged(input));

        assert_eq!(state.input.chars().count(), MAX_INPUT_CHARS);
        assert_eq!(state.input, "é".repeat(MAX_INPUT_CHARS));
    }
}

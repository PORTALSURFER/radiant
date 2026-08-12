//! Structural guardrails for the bounded composition foundation.

use std::{fs, path::PathBuf};

#[test]
fn composition_stays_qualified_and_additive_to_legacy_input_models() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let interaction_input =
        fs::read_to_string(manifest_dir.join("src/widgets/interaction/input/composition.rs"))
            .expect("composition input module should be readable");
    let runtime_kernel =
        fs::read_to_string(manifest_dir.join("src/runtime/controller/composition.rs"))
            .expect("composition runtime module should be readable");
    let runtime_state =
        fs::read_to_string(manifest_dir.join("src/runtime/controller/interaction_state.rs"))
            .expect("runtime interaction state should be readable");
    let widget_contract = fs::read_to_string(manifest_dir.join("src/widgets/contract/widget.rs"))
        .expect("widget contract should be readable");
    let widget_input =
        fs::read_to_string(manifest_dir.join("src/widgets/interaction/input/event.rs"))
            .expect("legacy WidgetInput model should be readable");
    let event_model =
        fs::read_to_string(manifest_dir.join("src/runtime/controller/events/model.rs"))
            .expect("legacy Event model should be readable");
    let prelude = fs::read_to_string(manifest_dir.join("src/prelude/widgets.rs"))
        .expect("common widget prelude should be readable");

    assert!(interaction_input.contains("pub enum CompositionSample"));
    assert!(interaction_input.contains("pub struct CompositionRange"));
    assert!(interaction_input.contains("pub(crate) enum CompositionSelectionState"));
    assert!(runtime_kernel.contains("pub fn dispatch_composition_sample"));
    assert!(runtime_kernel.contains("dispatch_hidden_composition_update"));
    assert!(runtime_kernel.contains("RuntimeManagedCompositionState::Active"));
    assert!(widget_contract.contains("handle_hidden_composition_update"));
    assert!(runtime_state.contains("RuntimeManagedCompositionState"));
    assert!(runtime_state.contains("Idle"));
    assert!(runtime_state.contains("Active"));
    assert!(runtime_state.contains("Blocked"));
    assert!(!widget_input.contains("CompositionSample"));
    assert!(!event_model.contains("CompositionSample"));
    assert!(!prelude.contains("CompositionSample"));
}

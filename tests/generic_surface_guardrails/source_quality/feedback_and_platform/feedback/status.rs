use super::*;

#[test]
fn status_line_entries_use_named_parts_for_source_and_message() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source_path = manifest_dir.join("src/gui/feedback/status/line.rs");
    let source = fs::read_to_string(&source_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));
    let source_tests =
        fs::read_to_string(manifest_dir.join("src/gui/feedback/status/line/tests.rs"))
            .expect("status line tests should be readable");
    let status = fs::read_to_string(manifest_dir.join("src/gui/feedback/status.rs"))
        .expect("feedback status module should be readable");
    let recovery = fs::read_to_string(manifest_dir.join("src/gui/feedback/status/recovery.rs"))
        .expect("feedback recovery status module should be readable");
    let health = fs::read_to_string(manifest_dir.join("src/gui/feedback/status/health.rs"))
        .expect("feedback health status module should be readable");
    let drag_overlay =
        fs::read_to_string(manifest_dir.join("src/gui/feedback/status/drag_overlay.rs"))
            .expect("feedback drag-overlay status module should be readable");
    let update = fs::read_to_string(manifest_dir.join("src/gui/feedback/status/update.rs"))
        .expect("feedback update status module should be readable");
    let prompt = fs::read_to_string(manifest_dir.join("src/gui/feedback/status/prompt.rs"))
        .expect("feedback prompt status module should be readable");
    let tests = fs::read_to_string(manifest_dir.join("src/gui/feedback/status/tests.rs"))
        .expect("feedback status tests should be readable");
    let feedback = fs::read_to_string(manifest_dir.join("src/gui/feedback.rs"))
        .expect("feedback module should be readable");
    let prelude = public_prelude_source(&manifest_dir);

    assert!(
        source.contains("pub struct StatusLineEntryParts")
            && source.contains("pub fn from_parts(parts: StatusLineEntryParts) -> Self")
            && source.contains("#[path = \"line/tests.rs\"]")
            && !source.contains("fn status_line_log_keeps_latest_bounded_message"),
        "status-line entries should expose named parts for source and message text while delegating behavior tests"
    );
    assert!(
        source_tests.contains("fn status_line_log_keeps_latest_bounded_message")
            && source_tests.contains("fn status_line_log_exposes_borrowed_latest_line")
            && source_tests.contains("fn status_line_entry_supports_named_parts_construction"),
        "status-line behavior tests should live in status/line/tests.rs"
    );
    assert!(
        source.contains("pub fn latest_line(&self) -> &str")
            && source.contains("self.latest_line().to_owned()")
            && !source.contains("unwrap_or_else(|| \"system: Ready\".to_string())"),
        "status-line logs should expose a borrowed latest-line path while keeping the owned compatibility helper"
    );
    assert!(
        source.contains("Self::from_parts(StatusLineEntryParts {")
            && status.contains("StatusLineEntryParts")
            && feedback.contains("StatusLineEntryParts")
            && !prelude.contains("StatusLineEntryParts"),
        "status-line entry parts should remain available from gui ownership without entering the common prelude"
    );
    for required in [
        "mod drag_overlay;",
        "mod health;",
        "mod prompt;",
        "mod recovery;",
        "mod update;",
        "#[path = \"status/tests.rs\"]",
        "pub use drag_overlay::DragOverlay;",
        "pub use health::HealthState;",
        "pub use prompt::{ConfirmPrompt, PromptIntent};",
        "pub use recovery::RecoverySummary;",
        "pub use update::{UpdatePanel, UpdateStatus};",
    ] {
        assert!(
            status.contains(required),
            "feedback status facade should delegate `{required}`"
        );
    }
    assert!(
        !status.contains("pub struct RecoverySummary")
            && !status.contains("pub enum HealthState")
            && !status.contains("pub struct DragOverlay")
            && !status.contains("pub struct UpdatePanel")
            && !status.contains("pub struct ConfirmPrompt")
            && !status.contains("fn recovery_summary_defaults_to_idle_and_empty"),
        "feedback status root should re-export focused models and delegate behavior tests instead of owning them"
    );
    assert!(
        recovery.contains("pub struct RecoverySummary")
            && health.contains("pub enum HealthState")
            && drag_overlay.contains("pub struct DragOverlay")
            && update.contains("pub enum UpdateStatus")
            && update.contains("pub struct UpdatePanel")
            && prompt.contains("pub enum PromptIntent")
            && prompt.contains("pub struct ConfirmPrompt"),
        "feedback status models should live in their focused status child modules"
    );
    assert!(
        tests.contains("fn recovery_summary_defaults_to_idle_and_empty")
            && tests.contains("fn prompt_intent_exposes_generic_confirmation_categories"),
        "feedback status behavior tests should live in status/tests.rs"
    );
}

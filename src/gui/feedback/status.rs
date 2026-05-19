mod drag_overlay;
mod health;
mod line;
mod prompt;
mod recovery;
mod update;

pub use drag_overlay::DragOverlay;
pub use health::HealthState;
pub use line::{StatusLineEntry, StatusLineEntryParts, StatusLineLog};
pub use prompt::{ConfirmPrompt, PromptIntent};
pub use recovery::RecoverySummary;
pub use update::{UpdatePanel, UpdateStatus};

#[cfg(test)]
mod tests {
    use super::{
        ConfirmPrompt, DragOverlay, HealthState, PromptIntent, RecoverySummary, UpdatePanel,
        UpdateStatus,
    };

    #[test]
    fn recovery_summary_defaults_to_idle_and_empty() {
        let summary = RecoverySummary::default();

        assert!(!summary.in_progress);
        assert_eq!(summary.entry_count, 0);
        assert_eq!(summary.retained_count, 0);
    }

    #[test]
    fn health_state_defaults_to_healthy() {
        assert_eq!(HealthState::default(), HealthState::Healthy);
    }

    #[test]
    fn drag_overlay_defaults_to_inactive_and_unanchored() {
        let overlay = DragOverlay::default();

        assert!(!overlay.active);
        assert_eq!(overlay.label, "");
        assert_eq!(overlay.target_label, "");
        assert!(!overlay.valid_target);
        assert_eq!(overlay.pointer_x, None);
        assert_eq!(overlay.pointer_y, None);
    }

    #[test]
    fn update_panel_defaults_to_idle_without_release_metadata() {
        let panel = UpdatePanel::default();

        assert_eq!(panel.status, UpdateStatus::Idle);
        assert_eq!(panel.status_label, "");
        assert_eq!(panel.action_hint_label, "");
        assert_eq!(panel.release_notes_label, "");
        assert_eq!(panel.available_version_label, None);
        assert_eq!(panel.available_url, None);
        assert_eq!(panel.last_error, None);
    }

    #[test]
    fn confirm_prompt_defaults_to_hidden_without_host_kind() {
        let prompt = ConfirmPrompt::<u8>::default();

        assert!(!prompt.visible);
        assert_eq!(prompt.kind, None);
        assert_eq!(prompt.title, "");
        assert_eq!(prompt.message, "");
        assert_eq!(prompt.confirm_label, "");
        assert_eq!(prompt.cancel_label, "");
        assert_eq!(prompt.target_label, None);
        assert_eq!(prompt.input_value, None);
        assert_eq!(prompt.input_placeholder, None);
        assert_eq!(prompt.input_error, None);
    }

    #[test]
    fn prompt_intent_exposes_generic_confirmation_categories() {
        let intents = [
            PromptIntent::DestructiveOperation,
            PromptIntent::RenameContent,
            PromptIntent::RenameNavigationItem,
            PromptIntent::CreateNavigationItem,
            PromptIntent::RestoreRetainedItems,
            PromptIntent::PurgeRetainedItems,
            PromptIntent::EditConfiguration,
        ];

        assert_eq!(intents.len(), 7);
    }
}

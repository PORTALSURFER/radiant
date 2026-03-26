use super::{UiAction, UiActionFamily};

#[test]
fn ui_action_family_preserves_bridge_groupings() {
    assert_eq!(
        UiAction::ToggleTransport.family(),
        UiActionFamily::Transport
    );
    assert_eq!(
        UiAction::PlayFromWaveformCursor.family(),
        UiActionFamily::Transport
    );
    assert_eq!(
        UiAction::PlayWaveformAtPrecise {
            position_nanos: 330_000_000,
        }
        .family(),
        UiActionFamily::Transport
    );
    assert_eq!(UiAction::FocusFolderSearch.family(), UiActionFamily::Focus);
    assert_eq!(
        UiAction::DeleteFocusedFolder.family(),
        UiActionFamily::Sources
    );
    assert_eq!(
        UiAction::RestoreRetainedFolderDeletes.family(),
        UiActionFamily::Sources
    );
    assert_eq!(
        UiAction::ToggleBrowserRowSelection { visible_row: 4 }.family(),
        UiActionFamily::Browser
    );
    assert_eq!(UiAction::ConfirmPrompt.family(), UiActionFamily::Prompt);
    assert_eq!(
        UiAction::SetWaveformBpmValue { value_tenths: 1200 }.family(),
        UiActionFamily::Options
    );
    assert_eq!(
        UiAction::ZoomWaveformFull.family(),
        UiActionFamily::Waveform
    );
    assert_eq!(UiAction::Undo.family(), UiActionFamily::History);
    assert_eq!(UiAction::DismissUpdate.family(), UiActionFamily::Update);
}

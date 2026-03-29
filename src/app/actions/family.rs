#![cfg(test)]

use super::UiAction;

/// Internal ownership buckets for the centralized [`UiAction`] compatibility surface.
///
/// The runtime keeps one top-level action enum so hosts can inspect the whole
/// bridge contract in one place. This family classification gives local code
/// and tests a stable way to group actions without splitting that contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum UiActionFamily {
    Column,
    Transport,
    Focus,
    Sources,
    Browser,
    Prompt,
    Options,
    Waveform,
    History,
    Update,
}

impl UiAction {
    /// Return the high-level ownership family for this action.
    pub(crate) const fn family(&self) -> UiActionFamily {
        match self {
            Self::SelectColumn { .. } | Self::MoveColumn { .. } => UiActionFamily::Column,
            Self::ToggleTransport
            | Self::PlayCompareAnchor
            | Self::PlayFromStart
            | Self::PlayFromCurrentPlayhead
            | Self::PlayFromWaveformCursor
            | Self::PlayWaveformAtPrecise { .. }
            | Self::HandleEscape => UiActionFamily::Transport,
            Self::FocusBrowserPanel
            | Self::FocusSourcesPanel
            | Self::FocusWaveformPanel
            | Self::FocusFolderPanel
            | Self::FocusLoadedSampleInBrowser
            | Self::FocusBrowserSearch
            | Self::BlurBrowserSearch
            | Self::OpenAddSourceDialog
            | Self::OpenOptionsMenu
            | Self::CloseOptionsPanel
            | Self::PickTrashFolder
            | Self::OpenTrashFolder
            | Self::FocusFolderSearch
            | Self::SetFolderSearch { .. } => UiActionFamily::Focus,
            Self::SelectSourceRow { .. }
            | Self::FocusSourceRow { .. }
            | Self::MoveSourceFocus { .. }
            | Self::ReloadFocusedSourceRow
            | Self::HardSyncFocusedSourceRow
            | Self::OpenFocusedSourceFolder
            | Self::RemoveFocusedSourceRow
            | Self::ReloadSourceRow { .. }
            | Self::HardSyncSourceRow { .. }
            | Self::OpenSourceFolderRow { .. }
            | Self::RemoveSourceRow { .. }
            | Self::FocusFolderRow { .. }
            | Self::ActivateFolderRow { .. }
            | Self::ToggleFolderRowExpanded { .. }
            | Self::ExpandFocusedFolder
            | Self::CollapseFocusedFolder
            | Self::ToggleFocusedFolderSelection
            | Self::MoveFolderFocus { .. }
            | Self::StartNewFolder
            | Self::StartNewFolderAtFolderRow { .. }
            | Self::StartNewFolderAtRoot
            | Self::FocusFolderCreateInput
            | Self::SetFolderCreateInput { .. }
            | Self::ConfirmFolderCreate
            | Self::CancelFolderCreate
            | Self::StartFolderRename
            | Self::DeleteFocusedFolder
            | Self::RestoreRetainedFolderDeletes
            | Self::PurgeRetainedFolderDeletes
            | Self::ToggleShowAllFolders
            | Self::ToggleFolderFlattenedView
            | Self::ClearFolderDeleteRecoveryLog => UiActionFamily::Sources,
            Self::MoveBrowserFocus { .. }
            | Self::SetBrowserViewStart { .. }
            | Self::FocusBrowserRow { .. }
            | Self::SetCompareAnchorFromFocusedBrowserSample
            | Self::CommitFocusedBrowserRow
            | Self::SaveWaveformSelectionToBrowser
            | Self::SaveWaveformSelectionToBrowserWithKeep2
            | Self::ToggleBrowserRowSelection { .. }
            | Self::StartBrowserSampleDrag { .. }
            | Self::UpdateBrowserSampleDrag { .. }
            | Self::FinishBrowserSampleDrag
            | Self::ExtendBrowserSelectionToRow { .. }
            | Self::AddRangeBrowserSelection { .. }
            | Self::ExtendBrowserSelectionFromFocus { .. }
            | Self::AddRangeBrowserSelectionFromFocus { .. }
            | Self::ToggleFocusedBrowserRowSelection
            | Self::SelectAllBrowserRows
            | Self::SetBrowserSearch { .. }
            | Self::ToggleBrowserRatingFilter { .. }
            | Self::ToggleRandomNavigationMode
            | Self::FocusPreviousBrowserHistory
            | Self::FocusNextBrowserHistory
            | Self::ToggleFindSimilarFocusedSample
            | Self::PlayRandomSample
            | Self::PlayPreviousRandomSample
            | Self::CopySelectionToClipboard
            | Self::AdjustSelectedBrowserRating { .. }
            | Self::SetBrowserTab { .. }
            | Self::FocusMapSample { .. } => UiActionFamily::Browser,
            Self::SetPromptInput { .. }
            | Self::StartBrowserRename
            | Self::ConfirmBrowserRename
            | Self::CancelBrowserRename
            | Self::TagBrowserSelection { .. }
            | Self::DeleteBrowserSelection
            | Self::NormalizeFocusedBrowserSample
            | Self::NormalizeWaveformSelectionOrSample
            | Self::CropWaveformSelection
            | Self::CropWaveformSelectionToNewSample
            | Self::TrimWaveformSelection
            | Self::ConfirmPrompt
            | Self::CancelPrompt
            | Self::CancelProgress => UiActionFamily::Prompt,
            Self::SetInputMonitoringEnabled { .. }
            | Self::SetAdvanceAfterRatingEnabled { .. }
            | Self::SetDestructiveYoloMode { .. }
            | Self::SetInvertWaveformScroll { .. }
            | Self::ToggleLoopPlayback
            | Self::ToggleLoopLock
            | Self::ToggleTransientMarkers
            | Self::ToggleBpmSnap
            | Self::ToggleHotkeyOverlay
            | Self::CopyStatusLog
            | Self::OpenFeedbackIssuePrompt
            | Self::MoveTrashedSamplesToFolder
            | Self::SetWaveformChannelView { .. }
            | Self::SetNormalizedAuditionEnabled { .. }
            | Self::SetBpmSnapEnabled { .. }
            | Self::SetRelativeBpmGridEnabled { .. }
            | Self::AdjustWaveformBpm { .. }
            | Self::SetWaveformBpmValue { .. }
            | Self::SetTransientSnapEnabled { .. }
            | Self::SetTransientMarkersEnabled { .. }
            | Self::SetSliceModeEnabled { .. }
            | Self::SetVolume { .. }
            | Self::CommitVolumeSetting => UiActionFamily::Options,
            Self::SeekWaveformPrecise { .. }
            | Self::SetWaveformCursorPrecise { .. }
            | Self::SeekWaveform { .. }
            | Self::SetWaveformCursor { .. }
            | Self::BeginWaveformSelectionAt { .. }
            | Self::BeginWaveformCircularSlide { .. }
            | Self::UpdateWaveformCircularSlide { .. }
            | Self::FinishWaveformCircularSlide
            | Self::SetWaveformSelectionRange { .. }
            | Self::SetWaveformSelectionRangeSmartScale { .. }
            | Self::SetWaveformEditSelectionRange { .. }
            | Self::CommitWaveformEditFades
            | Self::DetectWaveformExactDuplicateSlices
            | Self::CleanWaveformExactDuplicateSlices
            | Self::SetWaveformEditFadeInEnd { .. }
            | Self::SetWaveformEditFadeInMuteStart { .. }
            | Self::SetWaveformEditFadeInCurve { .. }
            | Self::SetWaveformEditFadeOutStart { .. }
            | Self::SetWaveformEditFadeOutMuteEnd { .. }
            | Self::SetWaveformEditFadeOutCurve { .. }
            | Self::FinishWaveformEditFadeDrag
            | Self::StartWaveformSelectionDrag { .. }
            | Self::UpdateWaveformSelectionDrag { .. }
            | Self::FinishWaveformSelectionDrag
            | Self::FinishWaveformSelectionRangeDrag
            | Self::FinishWaveformSelectionSmartScaleDrag
            | Self::BeginWaveformSelectionShift { .. }
            | Self::BeginWaveformEditSelectionShift { .. }
            | Self::FinishWaveformEditSelectionDrag
            | Self::ClearWaveformSelection
            | Self::ClearWaveformEditSelection
            | Self::ClearWaveformSelections
            | Self::DetectWaveformSilenceSlices
            | Self::SetWaveformViewCenter { .. }
            | Self::ZoomWaveform { .. }
            | Self::ZoomWaveformToSelection
            | Self::ZoomWaveformFull => UiActionFamily::Waveform,
            Self::ReverseWaveformSelection
            | Self::FadeWaveformSelectionLeftToRight
            | Self::FadeWaveformSelectionRightToLeft
            | Self::MuteWaveformSelection
            | Self::DeleteSelectedSliceMarkers
            | Self::ToggleWaveformSliceSelection { .. }
            | Self::AuditionWaveformDuplicateSlice { .. }
            | Self::ToggleWaveformDuplicateSliceExemption { .. }
            | Self::MoveWaveformSliceFocus { .. }
            | Self::ToggleFocusedWaveformSliceExportMark
            | Self::AlignWaveformStartToMarker
            | Self::DeleteLoadedWaveformSample
            | Self::SlideWaveformSelection { .. } => UiActionFamily::Waveform,
            Self::Undo | Self::Redo => UiActionFamily::History,
            Self::CheckForUpdates
            | Self::OpenUpdateLink
            | Self::InstallUpdate
            | Self::DismissUpdate => UiActionFamily::Update,
        }
    }
}

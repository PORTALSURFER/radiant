use super::*;

pub(super) fn action_from_key(
    key: KeyCode,
    modifiers: ModifiersState,
    model: &AppModel,
) -> Option<UiAction> {
    if model.confirm_prompt.visible {
        let confirm_enabled = model
            .confirm_prompt
            .input_error
            .as_ref()
            .is_none_or(|error| error.trim().is_empty());
        return match key {
            KeyCode::Enter if confirm_enabled => Some(UiAction::ConfirmPrompt),
            KeyCode::C => Some(UiAction::CancelPrompt),
            _ => None,
        };
    }
    if model.options_panel.visible {
        return None;
    }
    let shift = modifiers.shift_key();
    let command = modifiers.control_key() || modifiers.super_key();
    match key {
        KeyCode::ArrowLeft => Some(UiAction::MoveColumn { delta: -1 }),
        KeyCode::ArrowRight => Some(UiAction::MoveColumn { delta: 1 }),
        KeyCode::ArrowUp => {
            if matches!(
                model.focus_context,
                crate::app::FocusContextModel::SourceFolders
            ) {
                Some(UiAction::MoveFolderFocus { delta: -1 })
            } else if shift && command {
                Some(UiAction::AddRangeBrowserSelectionFromFocus { delta: -1 })
            } else if shift {
                Some(UiAction::ExtendBrowserSelectionFromFocus { delta: -1 })
            } else {
                Some(UiAction::MoveBrowserFocus { delta: -1 })
            }
        }
        KeyCode::ArrowDown => {
            if matches!(
                model.focus_context,
                crate::app::FocusContextModel::SourceFolders
            ) {
                Some(UiAction::MoveFolderFocus { delta: 1 })
            } else if shift && command {
                Some(UiAction::AddRangeBrowserSelectionFromFocus { delta: 1 })
            } else if shift {
                Some(UiAction::ExtendBrowserSelectionFromFocus { delta: 1 })
            } else {
                Some(UiAction::MoveBrowserFocus { delta: 1 })
            }
        }
        KeyCode::Num1 => Some(UiAction::SelectColumn { index: 0 }),
        KeyCode::Num2 => Some(UiAction::SelectColumn { index: 1 }),
        KeyCode::Num3 => Some(UiAction::SelectColumn { index: 2 }),
        KeyCode::A => Some(UiAction::SelectAllBrowserRows),
        KeyCode::B => Some(UiAction::StartNewFolder),
        KeyCode::C => match model.focus_context {
            crate::app::FocusContextModel::Waveform if shift => {
                Some(UiAction::CropWaveformSelectionToNewSample)
            }
            crate::app::FocusContextModel::Waveform => Some(UiAction::CropWaveformSelection),
            _ => None,
        },
        KeyCode::D => Some(UiAction::DeleteBrowserSelection),
        KeyCode::Enter => {
            if matches!(model.focus_context, crate::app::FocusContextModel::Waveform) {
                Some(UiAction::SaveWaveformSelectionToBrowser)
            } else {
                Some(UiAction::CommitFocusedBrowserRow)
            }
        }
        KeyCode::F => Some(UiAction::FocusBrowserSearch),
        KeyCode::G => Some(UiAction::DeleteFocusedFolder),
        KeyCode::I => Some(UiAction::StartBrowserRename),
        KeyCode::L => Some(UiAction::ToggleLoopPlayback),
        KeyCode::M => Some(UiAction::ZoomWaveformToSelection),
        KeyCode::N => match model.focus_context {
            crate::app::FocusContextModel::Waveform => {
                Some(UiAction::NormalizeWaveformSelectionOrSample)
            }
            crate::app::FocusContextModel::SampleBrowser | crate::app::FocusContextModel::None => {
                Some(UiAction::NormalizeFocusedBrowserSample)
            }
            _ => None,
        },
        KeyCode::OpenBracket => Some(UiAction::TagBrowserSelection {
            target: crate::app::BrowserTagTarget::Trash,
        }),
        KeyCode::P => model
            .progress_overlay
            .cancelable
            .then_some(UiAction::CancelProgress),
        KeyCode::CloseBracket => Some(UiAction::TagBrowserSelection {
            target: crate::app::BrowserTagTarget::Keep,
        }),
        KeyCode::Slash => Some(UiAction::ZoomWaveformFull),
        KeyCode::Quote => Some(UiAction::FocusFolderSearch),
        KeyCode::R => Some(UiAction::Redo),
        KeyCode::S => Some(UiAction::FocusSourcesPanel),
        KeyCode::Space => {
            if command {
                Some(UiAction::PlayFromCurrentPlayhead)
            } else {
                Some(UiAction::PlayFromStart)
            }
        }
        KeyCode::T => match model.focus_context {
            crate::app::FocusContextModel::Waveform => Some(UiAction::TrimWaveformSelection),
            _ => None,
        },
        KeyCode::U => Some(if shift {
            UiAction::Redo
        } else {
            UiAction::Undo
        }),
        KeyCode::W => Some(UiAction::FocusWaveformPanel),
        KeyCode::X => Some(UiAction::TagBrowserSelection {
            target: crate::app::BrowserTagTarget::Trash,
        }),
        KeyCode::Y => Some(UiAction::TagBrowserSelection {
            target: crate::app::BrowserTagTarget::Keep,
        }),
        KeyCode::Z => match model.focus_context {
            crate::app::FocusContextModel::Waveform => Some(UiAction::ZoomWaveformToSelection),
            _ => Some(UiAction::StartFolderRename),
        },
        _ => None,
    }
}

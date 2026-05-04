use super::*;

fn resolved_action(key: KeyCode, modifiers: ModifiersState, model: &AppModel) -> Option<UiAction> {
    action_from_key(key, modifiers, model, None, default_hotkey_resolver).action
}

pub(super) fn default_hotkey_resolver(
    pending_chord: Option<crate::compat_app_contract::KeyPress>,
    press: crate::compat_app_contract::KeyPress,
    focus: crate::compat_app_contract::FocusContextModel,
) -> crate::compat_app_contract::HotkeyResolution {
    if let Some(first) = pending_chord {
        if first == crate::compat_app_contract::KeyPress::new(KeyCode::G) {
            let action = match press.key {
                KeyCode::W => Some(UiAction::FocusWaveformPanel),
                KeyCode::B => Some(UiAction::FocusContentPanel),
                KeyCode::T => Some(UiAction::FocusFolderPanel { pane: None }),
                KeyCode::S => Some(UiAction::FocusSourcesPanel),
                _ => None,
            };
            return crate::compat_app_contract::HotkeyResolution {
                handled: true,
                pending_chord: None,
                action,
            };
        }
        return crate::compat_app_contract::HotkeyResolution {
            action: None,
            handled: true,
            pending_chord: None,
        };
    }

    if press == crate::compat_app_contract::KeyPress::new(KeyCode::G) {
        return crate::compat_app_contract::HotkeyResolution {
            action: None,
            handled: true,
            pending_chord: Some(press),
        };
    }

    let action = match focus {
        crate::compat_app_contract::FocusContextModel::None => global_hotkey_action(press),
        crate::compat_app_contract::FocusContextModel::ContentList => {
            browser_hotkey_action(press).or_else(|| global_hotkey_action(press))
        }
        crate::compat_app_contract::FocusContextModel::Timeline => {
            waveform_hotkey_action(press).or_else(|| global_hotkey_action(press))
        }
        crate::compat_app_contract::FocusContextModel::NavigationTree => {
            folder_hotkey_action(press).or_else(|| global_hotkey_action(press))
        }
        crate::compat_app_contract::FocusContextModel::NavigationList => {
            sources_hotkey_action(press).or_else(|| global_hotkey_action(press))
        }
    };

    crate::compat_app_contract::HotkeyResolution {
        handled: action.is_some(),
        pending_chord: None,
        action,
    }
}

fn global_hotkey_action(press: crate::compat_app_contract::KeyPress) -> Option<UiAction> {
    match press {
        press if press == crate::compat_app_contract::KeyPress::new(KeyCode::Space) => {
            Some(UiAction::PlayFromStart)
        }
        press if press == crate::compat_app_contract::KeyPress::with_shift(KeyCode::Space) => {
            Some(UiAction::PlayCompareAnchor)
        }
        press if press == crate::compat_app_contract::KeyPress::with_command(KeyCode::Space) => {
            Some(UiAction::PlayFromCurrentPlayhead)
        }
        _ => None,
    }
}

fn browser_hotkey_action(press: crate::compat_app_contract::KeyPress) -> Option<UiAction> {
    match press {
        press if press == crate::compat_app_contract::KeyPress::new(KeyCode::N) => {
            Some(UiAction::NormalizeFocusedContentItem)
        }
        press if press == crate::compat_app_contract::KeyPress::new(KeyCode::D) => {
            Some(UiAction::DeleteBrowserSelection)
        }
        press if press == crate::compat_app_contract::KeyPress::new(KeyCode::ArrowUp) => {
            Some(UiAction::MoveContentFocus { delta: -1 })
        }
        press if press == crate::compat_app_contract::KeyPress::new(KeyCode::ArrowDown) => {
            Some(UiAction::MoveContentFocus { delta: 1 })
        }
        press if press == crate::compat_app_contract::KeyPress::new(KeyCode::ArrowLeft) => {
            Some(UiAction::FocusPreviousBrowserHistory)
        }
        press if press == crate::compat_app_contract::KeyPress::new(KeyCode::ArrowRight) => {
            Some(UiAction::FocusNextBrowserHistory)
        }
        press if press == crate::compat_app_contract::KeyPress::new(KeyCode::X) => {
            Some(UiAction::ToggleFocusedContentRowSelection)
        }
        press if press == crate::compat_app_contract::KeyPress::new(KeyCode::S) => {
            Some(UiAction::ToggleFindSimilarFocusedContent)
        }
        press if press == crate::compat_app_contract::KeyPress::new(KeyCode::Semicolon) => {
            Some(UiAction::ToggleContentMark)
        }
        _ => None,
    }
}

fn folder_hotkey_action(press: crate::compat_app_contract::KeyPress) -> Option<UiAction> {
    match press {
        press if press == crate::compat_app_contract::KeyPress::new(KeyCode::D) => {
            Some(UiAction::DeleteFocusedFolder)
        }
        press if press == crate::compat_app_contract::KeyPress::new(KeyCode::N) => {
            Some(UiAction::StartNewFolder)
        }
        press if press == crate::compat_app_contract::KeyPress::new(KeyCode::R) => {
            Some(UiAction::StartFolderRename)
        }
        press if press == crate::compat_app_contract::KeyPress::new(KeyCode::ArrowLeft) => {
            Some(UiAction::CollapseFocusedFolder)
        }
        press if press == crate::compat_app_contract::KeyPress::new(KeyCode::ArrowRight) => {
            Some(UiAction::ExpandFocusedFolder)
        }
        press if press == crate::compat_app_contract::KeyPress::new(KeyCode::ArrowUp) => {
            Some(UiAction::MoveFolderFocus { delta: -1 })
        }
        press if press == crate::compat_app_contract::KeyPress::new(KeyCode::ArrowDown) => {
            Some(UiAction::MoveFolderFocus { delta: 1 })
        }
        press if press == crate::compat_app_contract::KeyPress::new(KeyCode::X) => {
            Some(UiAction::ToggleFocusedFolderSelection)
        }
        _ => None,
    }
}

fn sources_hotkey_action(press: crate::compat_app_contract::KeyPress) -> Option<UiAction> {
    match press {
        press if press == crate::compat_app_contract::KeyPress::new(KeyCode::R) => {
            Some(UiAction::ReloadFocusedSourceRow)
        }
        _ => None,
    }
}

fn waveform_hotkey_action(press: crate::compat_app_contract::KeyPress) -> Option<UiAction> {
    match press {
        press if press == crate::compat_app_contract::KeyPress::new(KeyCode::S) => {
            Some(UiAction::AlignWaveformStartToMarker)
        }
        press if press == crate::compat_app_contract::KeyPress::new(KeyCode::Enter) => {
            Some(UiAction::CommitWaveformEditFades)
        }
        press if press == crate::compat_app_contract::KeyPress::new(KeyCode::E) => {
            Some(UiAction::SaveWaveformSelectionToBrowser)
        }
        press if press == crate::compat_app_contract::KeyPress::with_shift(KeyCode::E) => {
            Some(UiAction::SaveWaveformSelectionToBrowserWithKeep2)
        }
        press if press == crate::compat_app_contract::KeyPress::new(KeyCode::B) => {
            Some(UiAction::ToggleBpmSnap)
        }
        press if press == crate::compat_app_contract::KeyPress::with_shift(KeyCode::ArrowRight) => {
            Some(UiAction::SlideWaveformSelection {
                delta: 1,
                fine: true,
            })
        }
        press if press == crate::compat_app_contract::KeyPress::new(KeyCode::X) => {
            Some(UiAction::ZoomWaveformFull)
        }
        _ => None,
    }
}

#[derive(Default)]
struct RecordingBridge {
    actions: Vec<UiAction>,
}

impl NativeAppBridge for RecordingBridge {
    fn project_model(&mut self) -> Arc<AppModel> {
        Arc::new(AppModel::default())
    }

    fn resolve_hotkey_press(
        &mut self,
        pending_chord: Option<crate::compat_app_contract::KeyPress>,
        press: crate::compat_app_contract::KeyPress,
        focus: crate::compat_app_contract::FocusContextModel,
    ) -> crate::compat_app_contract::HotkeyResolution {
        default_hotkey_resolver(pending_chord, press, focus)
    }

    fn reduce_action(&mut self, action: UiAction) {
        self.actions.push(action);
    }
}

#[derive(Default)]
struct ImmediateFolderCreateBridge {
    actions: Vec<UiAction>,
    model: AppModel,
    project_calls: usize,
}

impl ImmediateFolderCreateBridge {
    fn with_root() -> Self {
        Self {
            model: AppModel {
                focus_context: crate::compat_app_contract::FocusContextModel::NavigationTree,
                sources: SourcesPanelModel {
                    tree_rows: vec![root_folder_row()].into(),
                    ..SourcesPanelModel::default()
                },
                ..AppModel::default()
            },
            ..Self::default()
        }
    }

    fn set_draft(&mut self, value: String) {
        self.set_inline_draft(value, false);
    }

    fn set_inline_draft(&mut self, value: String, rename: bool) {
        let draft = if rename {
            crate::compat_app_contract::FolderRowModel::rename_draft(
                1,
                value.clone(),
                String::from("Folder name"),
                folder_create_error(&value),
                true,
            )
        } else {
            crate::compat_app_contract::FolderRowModel::create_draft(
                1,
                value.clone(),
                String::from("New folder name"),
                folder_create_error(&value),
                true,
            )
        };
        self.model.sources.tree_rows = vec![root_folder_row(), draft].into();
    }

    fn clear_draft(&mut self) {
        self.model
            .sources
            .tree_rows
            .make_mut()
            .retain(|row| row.is_root);
    }

    fn add_created_folder(&mut self, value: String) {
        self.model.sources.tree_rows = vec![
            root_folder_row(),
            crate::compat_app_contract::FolderRowModel::new(
                value.clone(),
                value,
                1,
                false,
                true,
                false,
                false,
                false,
            )
            .with_backing_index(1),
        ]
        .into();
    }
}

impl NativeAppBridge for ImmediateFolderCreateBridge {
    fn project_model(&mut self) -> Arc<AppModel> {
        self.project_calls = self.project_calls.saturating_add(1);
        Arc::new(self.model.clone())
    }

    fn resolve_hotkey_press(
        &mut self,
        pending_chord: Option<crate::compat_app_contract::KeyPress>,
        press: crate::compat_app_contract::KeyPress,
        focus: crate::compat_app_contract::FocusContextModel,
    ) -> crate::compat_app_contract::HotkeyResolution {
        default_hotkey_resolver(pending_chord, press, focus)
    }

    fn reduce_action(&mut self, action: UiAction) {
        match &action {
            UiAction::StartNewFolder
            | UiAction::StartNewFolderAtFolderRow { .. }
            | UiAction::StartNewFolderAtRoot => self.set_draft(String::new()),
            UiAction::StartFolderRename => {
                let value = self
                    .model
                    .sources
                    .focused_tree_row
                    .and_then(|index| self.model.sources.tree_rows.get(index))
                    .map(|row| row.label.clone())
                    .unwrap_or_default();
                self.set_inline_draft(value, true);
            }
            UiAction::SetFolderCreateInput { value } => self.set_draft(value.clone()),
            UiAction::ConfirmFolderCreate => {
                let value = self
                    .model
                    .sources
                    .tree_rows
                    .iter()
                    .find(|row| row.kind == crate::compat_app_contract::FolderRowKind::CreateDraft)
                    .and_then(|row| row.input_value.clone())
                    .map(|value| value.trim().to_string())
                    .unwrap_or_default();
                if !value.is_empty() {
                    self.add_created_folder(value);
                }
            }
            UiAction::CancelFolderCreate => self.clear_draft(),
            _ => {}
        }
        self.actions.push(action);
    }
}

fn root_folder_row() -> crate::compat_app_contract::FolderRowModel {
    crate::compat_app_contract::FolderRowModel::new("Root", "", 0, false, false, true, true, true)
        .with_backing_index(0)
}

fn folder_create_error(value: &str) -> Option<String> {
    value
        .trim()
        .is_empty()
        .then(|| String::from("Folder name cannot be empty"))
}

mod focus;
mod folder_create;
mod repeat;
mod text_input;

use super::*;

fn resolved_action(key: KeyCode, modifiers: ModifiersState, model: &AppModel) -> Option<UiAction> {
    action_from_key(key, modifiers, model, None).action
}

#[derive(Default)]
struct RecordingBridge {
    actions: Vec<UiAction>,
}

impl NativeAppBridge for RecordingBridge {
    fn project_model(&mut self) -> Arc<AppModel> {
        Arc::new(AppModel::default())
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
                focus_context: crate::app::FocusContextModel::SourceFolders,
                sources: SourcesPanelModel {
                    folder_rows: vec![root_folder_row()].into(),
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
            crate::app::FolderRowModel::rename_draft(
                1,
                value.clone(),
                String::from("Folder name"),
                folder_create_error(&value),
                true,
            )
        } else {
            crate::app::FolderRowModel::create_draft(
                1,
                value.clone(),
                String::from("New folder name"),
                folder_create_error(&value),
                true,
            )
        };
        self.model.sources.folder_rows = vec![root_folder_row(), draft].into();
    }

    fn clear_draft(&mut self) {
        self.model.sources.folder_rows.make_mut().retain(|row| row.is_root);
    }

    fn add_created_folder(&mut self, value: String) {
        self.model.sources.folder_rows = vec![
            root_folder_row(),
            crate::app::FolderRowModel::new(
                value.clone(),
                value,
                1,
                false,
                true,
                false,
                false,
                false,
            )
            .with_source_index(1),
        ]
        .into();
    }
}

impl NativeAppBridge for ImmediateFolderCreateBridge {
    fn project_model(&mut self) -> Arc<AppModel> {
        self.project_calls = self.project_calls.saturating_add(1);
        Arc::new(self.model.clone())
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
                    .focused_folder_row
                    .and_then(|index| self.model.sources.folder_rows.get(index))
                    .map(|row| row.label.clone())
                    .unwrap_or_default();
                self.set_inline_draft(value, true);
            }
            UiAction::SetFolderCreateInput { value } => self.set_draft(value.clone()),
            UiAction::ConfirmFolderCreate => {
                let value = self
                    .model
                    .sources
                    .folder_rows
                    .iter()
                    .find(|row| row.kind == crate::app::FolderRowKind::CreateDraft)
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

fn root_folder_row() -> crate::app::FolderRowModel {
    crate::app::FolderRowModel::new("Root", "", 0, false, false, true, true, true)
        .with_source_index(0)
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

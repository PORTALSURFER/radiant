use super::super::*;

impl BrowserState {
    pub(crate) fn select_folder(&mut self, id: impl Into<String>) {
        self.selected_folder = id.into();
        self.selected_file = None;
        self.context_file = None;
        self.context_position = None;
        self.cancel_renames();
    }

    pub(crate) fn activate_folder(&mut self, id: impl Into<String>) {
        let id = id.into();
        self.context_folder = None;
        self.context_file = None;
        self.context_position = None;
        self.cancel_renames();
        if !self.folder_has_children(&id) {
            self.select_folder(id);
            return;
        }
        if !self.is_expanded(&id) {
            self.expanded_folders.insert(id.clone());
            self.select_folder(id);
        } else if self.selected_folder == id {
            self.expanded_folders.remove(&id);
        } else {
            self.select_folder(id);
        }
    }

    #[cfg(test)]
    pub(crate) fn toggle_folder(&mut self, id: impl Into<String>) {
        let id = id.into();
        if self.folder_has_children(&id) && !self.expanded_folders.remove(&id) {
            self.expanded_folders.insert(id);
        }
    }

    pub(crate) fn select_file_id(&mut self, id: String) {
        self.selected_file = Some(id);
        self.context_folder = None;
        self.context_file = None;
        self.context_column = None;
        self.context_position = None;
        self.cancel_renames();
    }

    pub(crate) fn open_context_menu_at(&mut self, id: String, position: radiant::layout::Point) {
        self.context_folder = Some(id);
        self.context_file = None;
        self.context_column = None;
        self.context_position = Some(position);
        self.cancel_renames();
    }

    pub(crate) fn close_context_menu(&mut self) {
        self.context_folder = None;
        self.context_position = None;
    }

    pub(crate) fn open_file_context_menu_at(
        &mut self,
        id: String,
        position: radiant::layout::Point,
    ) {
        self.selected_file = Some(id.clone());
        self.context_file = Some(id);
        self.context_folder = None;
        self.context_column = None;
        self.context_position = Some(position);
        self.cancel_renames();
    }

    pub(crate) fn close_file_context_menu(&mut self) {
        self.context_file = None;
        self.context_position = None;
    }

    pub(crate) fn open_column_context_menu_at(
        &mut self,
        id: String,
        position: radiant::layout::Point,
    ) {
        self.context_column = Some(id);
        self.context_file = None;
        self.context_folder = None;
        self.context_position = Some(position);
        self.cancel_renames();
    }

    pub(crate) fn close_column_context_menu(&mut self) {
        self.context_column = None;
        self.context_position = None;
    }

    pub(crate) fn cancel_renames(&mut self) {
        self.cancel_folder_rename();
        self.cancel_file_rename();
    }

    pub(crate) fn cancel_folder_rename(&mut self) {
        self.rename_folder = None;
        self.rename_draft.clear();
    }

    pub(crate) fn cancel_file_rename(&mut self) {
        self.rename_file = None;
        self.file_rename_draft.clear();
    }
}

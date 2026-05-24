use super::super::*;

impl BrowserState {
    pub(crate) fn select_folder(&mut self, id: impl Into<String>) {
        self.selection.selected_folder = id.into();
        self.selection.selected_file = None;
        self.context.context_file = None;
        self.context.context_position = None;
        self.cancel_renames();
    }

    pub(crate) fn activate_folder(&mut self, id: impl Into<String>) {
        let id = id.into();
        self.context.context_folder = None;
        self.context.context_file = None;
        self.context.context_position = None;
        self.cancel_renames();
        if !self.folder_has_children(&id) {
            self.select_folder(id);
            return;
        }
        if !self.is_expanded(&id) {
            self.tree.expanded_folders.insert(id.clone());
            self.select_folder(id);
        } else if self.selection.selected_folder == id {
            self.tree.expanded_folders.remove(&id);
        } else {
            self.select_folder(id);
        }
    }

    #[cfg(test)]
    pub(crate) fn toggle_folder(&mut self, id: impl Into<String>) {
        let id = id.into();
        if self.folder_has_children(&id) && !self.tree.expanded_folders.remove(&id) {
            self.tree.expanded_folders.insert(id);
        }
    }

    pub(crate) fn select_file_id(&mut self, id: String) {
        self.selection.selected_file = Some(id);
        self.context.context_folder = None;
        self.context.context_file = None;
        self.context.context_column = None;
        self.context.context_position = None;
        self.cancel_renames();
    }

    pub(crate) fn open_context_menu_at(&mut self, id: String, position: radiant::layout::Point) {
        self.context.context_folder = Some(id);
        self.context.context_file = None;
        self.context.context_column = None;
        self.context.context_position = Some(position);
        self.cancel_renames();
    }

    pub(crate) fn close_context_menu(&mut self) {
        self.context.context_folder = None;
        self.context.context_position = None;
    }

    pub(crate) fn open_file_context_menu_at(
        &mut self,
        id: String,
        position: radiant::layout::Point,
    ) {
        self.selection.selected_file = Some(id.clone());
        self.context.context_file = Some(id);
        self.context.context_folder = None;
        self.context.context_column = None;
        self.context.context_position = Some(position);
        self.cancel_renames();
    }

    pub(crate) fn close_file_context_menu(&mut self) {
        self.context.context_file = None;
        self.context.context_position = None;
    }

    pub(crate) fn open_column_context_menu_at(
        &mut self,
        id: String,
        position: radiant::layout::Point,
    ) {
        self.context.context_column = Some(id);
        self.context.context_file = None;
        self.context.context_folder = None;
        self.context.context_position = Some(position);
        self.cancel_renames();
    }

    pub(crate) fn close_column_context_menu(&mut self) {
        self.context.context_column = None;
        self.context.context_position = None;
    }

    pub(crate) fn cancel_renames(&mut self) {
        self.cancel_folder_rename();
        self.cancel_file_rename();
    }

    pub(crate) fn cancel_folder_rename(&mut self) {
        self.rename.folder = None;
        self.rename.folder_draft.clear();
    }

    pub(crate) fn cancel_file_rename(&mut self) {
        self.rename.file = None;
        self.rename.file_draft.clear();
    }
}

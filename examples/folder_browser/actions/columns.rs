use super::super::*;
use radiant::application::{DetailsSort, SortDirection};

impl BrowserState {
    pub(crate) fn sort_by(&mut self, column_id: String) {
        if self.columns.sort.column_id == column_id {
            self.columns.sort.direction = self.columns.sort.direction.toggled();
        } else {
            self.columns.sort = DetailsSort::new(column_id, SortDirection::Ascending);
        }
    }

    pub(crate) fn toggle_file_column(&mut self, column_id: String) {
        let visible_count = self
            .columns
            .file_columns
            .iter()
            .filter(|column| column.visible)
            .count();
        let Some(column) = self
            .columns
            .file_columns
            .iter_mut()
            .find(|column| column.id == column_id)
        else {
            return;
        };
        if column.id == "name" {
            self.status = String::from("Name column stays visible");
            return;
        }
        if column.visible && visible_count <= 1 {
            self.status = String::from("Keep at least one column visible");
            return;
        }
        column.visible = !column.visible;
        if !column.visible && self.columns.sort.column_id == column.id {
            self.columns.sort = DetailsSort::new("name", SortDirection::Ascending);
        }
        self.context.context_column = Some(column.id.clone());
    }

    pub(crate) fn reset_file_columns(&mut self) {
        self.columns.file_columns = default_file_columns();
        self.columns.sort = DetailsSort::new("name", SortDirection::Ascending);
        self.context.context_column = None;
        self.status = String::from("Reset file columns");
    }

    pub(crate) fn resize_file_column(&mut self, column_id: String, message: ui::DragHandleMessage) {
        let current_width = self
            .columns
            .file_columns
            .iter()
            .find(|column| column.id == column_id && column.visible)
            .map(|column| column.width);
        let batch = radiant::application::update_details_column_resize_edit(
            &mut self.columns.resize,
            &column_id,
            message,
            current_width,
            MIN_FILE_COLUMN_WIDTH,
            MAX_FILE_COLUMN_WIDTH,
        );
        if let Some(update) = batch.and_then(|batch| batch.width_update())
            && let Some(column) = self
                .columns
                .file_columns
                .iter_mut()
                .find(|column| column.id == update.column_id)
        {
            column.width = update.width;
        }
    }

    pub(crate) fn resize_tree(&mut self, message: ui::DragHandleMessage) {
        match message {
            ui::DragHandleMessage::Started { position, .. }
            | ui::DragHandleMessage::Moved { position, .. }
            | ui::DragHandleMessage::Ended { position, .. } => {
                self.tree.tree_width =
                    (position.x - SPLITTER_OFFSET).clamp(MIN_TREE_WIDTH, MAX_TREE_WIDTH);
            }
            ui::DragHandleMessage::Cancelled { .. } => {}
            ui::DragHandleMessage::DoubleActivate { .. } => {}
        }
    }
}

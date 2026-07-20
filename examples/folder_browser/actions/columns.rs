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
        match message {
            ui::DragHandleMessage::Started { origin, .. } => {
                if let Some(column) = self
                    .columns
                    .file_columns
                    .iter()
                    .find(|column| column.id == column_id)
                {
                    self.columns.resize = Some(ColumnResize {
                        column_id,
                        start_x: origin.x,
                        start_width: column.width,
                    });
                }
            }
            ui::DragHandleMessage::Moved { position }
            | ui::DragHandleMessage::Ended { position } => {
                let Some(resize) = self.columns.resize.clone() else {
                    return;
                };
                if let Some(column) = self
                    .columns
                    .file_columns
                    .iter_mut()
                    .find(|column| column.id == resize.column_id)
                {
                    column.width = (resize.start_width + position.x - resize.start_x)
                        .clamp(MIN_FILE_COLUMN_WIDTH, MAX_FILE_COLUMN_WIDTH);
                }
                if matches!(message, ui::DragHandleMessage::Ended { .. }) {
                    self.columns.resize = None;
                }
            }
            ui::DragHandleMessage::Cancelled { .. } => {
                self.columns.resize = None;
            }
            ui::DragHandleMessage::DoubleActivate { .. } => {}
        }
    }

    pub(crate) fn resize_tree(&mut self, message: ui::DragHandleMessage) {
        match message {
            ui::DragHandleMessage::Started { position, .. }
            | ui::DragHandleMessage::Moved { position }
            | ui::DragHandleMessage::Ended { position } => {
                self.tree.tree_width =
                    (position.x - SPLITTER_OFFSET).clamp(MIN_TREE_WIDTH, MAX_TREE_WIDTH);
            }
            ui::DragHandleMessage::Cancelled { .. } => {}
            ui::DragHandleMessage::DoubleActivate { .. } => {}
        }
    }
}

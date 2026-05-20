use super::super::*;

impl BrowserState {
    pub(crate) fn sort_by(&mut self, column_id: String) {
        if self.sort.column_id == column_id {
            self.sort.direction = self.sort.direction.toggled();
        } else {
            self.sort = ui::DetailsSort::new(column_id, ui::SortDirection::Ascending);
        }
    }

    pub(crate) fn toggle_file_column(&mut self, column_id: String) {
        let visible_count = self
            .file_columns
            .iter()
            .filter(|column| column.visible)
            .count();
        let Some(column) = self
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
        if !column.visible && self.sort.column_id == column.id {
            self.sort = ui::DetailsSort::new("name", ui::SortDirection::Ascending);
        }
        self.context_column = Some(column.id.clone());
    }

    pub(crate) fn reset_file_columns(&mut self) {
        self.file_columns = default_file_columns();
        self.sort = ui::DetailsSort::new("name", ui::SortDirection::Ascending);
        self.context_column = None;
        self.status = String::from("Reset file columns");
    }

    pub(crate) fn resize_file_column(&mut self, column_id: String, message: ui::DragHandleMessage) {
        match message {
            ui::DragHandleMessage::Started { position } => {
                if let Some(column) = self
                    .file_columns
                    .iter()
                    .find(|column| column.id == column_id)
                {
                    self.column_resize = Some(ColumnResize {
                        column_id,
                        start_x: position.x,
                        start_width: column.width,
                    });
                }
            }
            ui::DragHandleMessage::Moved { position }
            | ui::DragHandleMessage::Ended { position } => {
                let Some(resize) = self.column_resize.clone() else {
                    return;
                };
                if let Some(column) = self
                    .file_columns
                    .iter_mut()
                    .find(|column| column.id == resize.column_id)
                {
                    column.width = (resize.start_width + position.x - resize.start_x)
                        .clamp(MIN_FILE_COLUMN_WIDTH, MAX_FILE_COLUMN_WIDTH);
                }
                if matches!(message, ui::DragHandleMessage::Ended { .. }) {
                    self.column_resize = None;
                }
            }
        }
    }

    pub(crate) fn resize_tree(&mut self, message: ui::DragHandleMessage) {
        match message {
            ui::DragHandleMessage::Started { position }
            | ui::DragHandleMessage::Moved { position }
            | ui::DragHandleMessage::Ended { position } => {
                self.tree_width =
                    (position.x - SPLITTER_OFFSET).clamp(MIN_TREE_WIDTH, MAX_TREE_WIDTH);
            }
        }
    }
}

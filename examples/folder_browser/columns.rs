//! Details-list column configuration for the folder browser example.

#[derive(Clone, Debug)]
pub(super) struct FileColumn {
    pub(super) id: String,
    pub(super) label: String,
    pub(super) width: f32,
    pub(super) visible: bool,
}

pub(super) fn default_file_columns() -> Vec<FileColumn> {
    vec![
        file_column("name", "Name", 190.0, true),
        file_column("size", "Size", 78.0, true),
        file_column("kind", "Type", 132.0, true),
        file_column("modified", "Modified", 112.0, true),
        file_column("extension", "Extension", 92.0, false),
        file_column("path", "Path", 260.0, false),
    ]
}

fn file_column(id: &str, label: &str, width: f32, visible: bool) -> FileColumn {
    FileColumn {
        id: id.to_owned(),
        label: label.to_owned(),
        width,
        visible,
    }
}

use crate::model::{FileEntry, FolderEntry};
use std::path::Path;

use super::{file_extension, file_label};

pub(crate) fn demo_root_folder() -> FolderEntry {
    FolderEntry {
        id: String::from("demo-root"),
        name: String::from("Resource sandbox"),
        children: vec![
            FolderEntry {
                id: String::from("demo-root/design"),
                name: String::from("Design"),
                children: vec![FolderEntry {
                    id: String::from("demo-root/design/archive"),
                    name: String::from("Archive"),
                    children: Vec::new(),
                    files: vec![demo_file("demo-root/design/archive/notes.md", 2048, 2)],
                }],
                files: vec![
                    demo_file("demo-root/design/palette.json", 1536, 0),
                    demo_file("demo-root/design/wireframe.svg", 4096, 1),
                ],
            },
            FolderEntry {
                id: String::from("demo-root/engineering"),
                name: String::from("Engineering"),
                children: Vec::new(),
                files: vec![
                    demo_file("demo-root/engineering/app.rs", 8192, 3),
                    demo_file("demo-root/engineering/config.toml", 512, 4),
                ],
            },
        ],
        files: vec![demo_file("demo-root/readme.txt", 1024, 5)],
    }
}

fn demo_file(id: &str, size_bytes: u64, modified_rank: u64) -> FileEntry {
    let path = Path::new(id);
    FileEntry {
        id: id.to_owned(),
        name: file_label(path),
        kind: if path.extension().is_some() {
            file_kind_label(path)
        } else {
            String::from("File")
        },
        size: format_demo_size(size_bytes),
        size_bytes,
        modified: if modified_rank == 0 {
            String::from("Today")
        } else {
            format!("{modified_rank} days")
        },
        modified_rank,
    }
}

fn file_kind_label(path: &Path) -> String {
    match file_extension(path).to_ascii_lowercase().as_str() {
        "wav" | "aiff" | "flac" | "mp3" => String::from("Audio"),
        "rs" | "toml" | "md" | "txt" | "json" => String::from("Text"),
        "png" | "jpg" | "jpeg" | "svg" => String::from("Image"),
        _ => String::from("File"),
    }
}

fn format_demo_size(bytes: u64) -> String {
    if bytes >= 1024 {
        format!("{} KB", bytes / 1024)
    } else {
        format!("{bytes} B")
    }
}

//! Sample tree and details data for the tree/details example.

use radiant::prelude::DetailsRow;

pub(super) fn tree_children(id: &str) -> Option<&'static [(&'static str, &'static str)]> {
    match id {
        "workspace" => Some(&[("ui", "UI"), ("audio", "Audio"), ("docs", "Docs")]),
        "ui" => Some(&[("design", "Design"), ("widgets", "Widgets")]),
        "audio" => Some(&[("analysis", "Analysis"), ("playback", "Playback")]),
        _ => None,
    }
}

pub(super) fn detail_rows_for(id: &str) -> Vec<DetailsRow> {
    match id {
        "design" => vec![
            DetailsRow::new("palette", ["palette.rs", "Rust", "Ready"]),
            DetailsRow::new("tokens", ["tokens.rs", "Rust", "Ready"]),
            DetailsRow::new("spacing", ["spacing.md", "Markdown", "Draft"]),
        ],
        "widgets" => vec![
            DetailsRow::new("tree", ["tree_list.rs", "Rust", "New"]),
            DetailsRow::new("details", ["details_list.rs", "Rust", "New"]),
            DetailsRow::new("button", ["button.rs", "Rust", "Stable"]),
        ],
        "analysis" => vec![
            DetailsRow::new("onsets", ["onsets.rs", "Rust", "Ready"]),
            DetailsRow::new("tempo", ["tempo.rs", "Rust", "Draft"]),
        ],
        "playback" => vec![
            DetailsRow::new("transport", ["transport.rs", "Rust", "Ready"]),
            DetailsRow::new("looping", ["looping.rs", "Rust", "Ready"]),
        ],
        "docs" => vec![
            DetailsRow::new("api", ["API.md", "Markdown", "Draft"]),
            DetailsRow::new("readme", ["README.md", "Markdown", "Ready"]),
        ],
        _ => vec![
            DetailsRow::new("overview", ["overview.txt", "Text", "Ready"]),
            DetailsRow::new("notes", ["notes.txt", "Text", "Draft"]),
        ],
    }
}

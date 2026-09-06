//! Headless public-API fixture for controlled editor documents and shared geometry.

use radiant::{
    application::{TextEditorDocument, TextEditorEdit, text_editor},
    gui::{
        text_layout::paragraph::{
            ClusterCaretOffset, ParagraphBaseDirection, ParagraphGeometry, ParagraphGeometryInput,
            ParagraphGeometryKey, ShapedLogicalCluster,
        },
        types::Rect,
    },
};
use std::sync::Arc;

enum Message {
    Edit(TextEditorEdit),
}

fn main() {
    let document = TextEditorDocument::new(Arc::<str>::from("kick")).expect("bounded document");
    let snapshot = document.snapshot();
    let view = text_editor(snapshot.clone())
        .wrap(true)
        .font_size(14.0)
        .message(Message::Edit);
    // The host handles `Message::Edit(edit)` by calling `document.apply(&edit)`.
    let _ = view;
    let _geometry = fixture_geometry(document.snapshot().text());
}

fn fixture_geometry(text: &str) -> ParagraphGeometry {
    let clusters = text
        .char_indices()
        .map(|(start, character)| {
            let end = start + character.len_utf8();
            ShapedLogicalCluster {
                bytes: start..end,
                advance: 8.0,
                bidi_level: 0,
                safe_break_after: character == ' ',
                carets: vec![
                    ClusterCaretOffset {
                        byte_offset: 0,
                        x: 0.0,
                    },
                    ClusterCaretOffset {
                        byte_offset: character.len_utf8() as u32,
                        x: 8.0,
                    },
                ],
            }
        })
        .collect();
    ParagraphGeometry::build(ParagraphGeometryInput {
        key: ParagraphGeometryKey(1),
        source: Arc::from(text),
        clusters,
        base_direction: ParagraphBaseDirection::Ltr,
        wrap_width: Rect::from_size(160.0, 80.0).width(),
        line_height: 18.0,
    })
    .expect("deterministic fixture geometry")
}

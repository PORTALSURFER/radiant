use super::{TextEditorDelta, TextEditorSelection, TextEditorWidget};
use crate::{
    gui::{
        text_layout::paragraph::{CaretAffinity, ParagraphCaret},
        types::Point,
    },
    widgets::{TextEditCommand, WidgetOutput},
};
use std::{ops::Range, sync::Arc};
use unicode_segmentation::UnicodeSegmentation;

pub(super) fn previous_boundary(text: &str, at: usize) -> usize {
    text.grapheme_indices(true)
        .map(|(byte, _)| byte)
        .take_while(|byte| *byte < at)
        .last()
        .unwrap_or(0)
}
pub(super) fn next_boundary(text: &str, at: usize) -> usize {
    text.grapheme_indices(true)
        .map(|(byte, _)| byte)
        .find(|byte| *byte > at)
        .unwrap_or(text.len())
}
fn word_boundary(text: &str, at: usize, forward: bool) -> usize {
    if forward {
        text.unicode_word_indices()
            .map(|(start, word)| start + word.len())
            .find(|end| *end > at)
            .unwrap_or(text.len())
    } else {
        text.unicode_word_indices()
            .map(|(start, _)| start)
            .take_while(|start| *start < at)
            .last()
            .unwrap_or(0)
    }
}
impl TextEditorWidget {
    pub(super) fn move_caret(
        &mut self,
        caret: ParagraphCaret,
        extend: bool,
    ) -> Option<WidgetOutput> {
        let old = self.selection();
        let next = TextEditorSelection {
            anchor: if extend { old.anchor } else { caret.byte },
            caret: caret.byte,
            affinity: caret.affinity,
        };
        if old == next && !self.groups.is_active() {
            return None;
        }
        self.emit(TextEditorDelta::Selection, next)
    }
    pub(super) fn move_logical(&mut self, byte: usize, extend: bool) -> Option<WidgetOutput> {
        self.preferred_x = None;
        self.move_caret(
            ParagraphCaret {
                byte,
                affinity: CaretAffinity::Downstream,
            },
            extend,
        )
    }
    pub(super) fn replace_range(
        &mut self,
        range: Range<usize>,
        text: Arc<str>,
    ) -> Option<WidgetOutput> {
        self.replace_range_grouped(
            range,
            text,
            crate::widgets::interaction::TextEditKind::Typing,
        )
    }
    fn replace_range_grouped(
        &mut self,
        range: Range<usize>,
        text: Arc<str>,
        kind: crate::widgets::interaction::TextEditKind,
    ) -> Option<WidgetOutput> {
        if self.snapshot.is_composing() || range.end > self.text().len() || range.start > range.end
        {
            return None;
        }
        let length = self
            .text()
            .len()
            .checked_sub(range.len())?
            .checked_add(text.len())?;
        if length > super::MAX_TEXT_EDITOR_BYTES {
            return None;
        }
        let mut result = String::with_capacity(length);
        result.push_str(self.text().get(..range.start)?);
        result.push_str(&text);
        result.push_str(self.text().get(range.end..)?);
        let inserted_end = range.start + text.len();
        // Inserting next to combining/ZWJ text can merge graphemes across the edit.
        let caret = result
            .grapheme_indices(true)
            .map(|(at, _)| at)
            .find(|at| *at >= inserted_end)
            .unwrap_or(result.len());
        let delta = if text.is_empty() {
            TextEditorDelta::Delete { range }
        } else if range.is_empty() {
            TextEditorDelta::Insert {
                at: range.start,
                text,
            }
        } else {
            TextEditorDelta::Replace { range, text }
        };
        self.preferred_x = None;
        self.emit_grouped(delta, TextEditorSelection::caret(caret), Some(kind), None)
    }
    pub(super) fn edit_command(&mut self, command: TextEditCommand) -> Option<WidgetOutput> {
        if self.snapshot.is_composing() {
            return None;
        }
        use crate::widgets::interaction::TextEditKind;
        let selected = self.selection();
        let range = selected.range();
        let at = selected.caret;
        match command {
            TextEditCommand::InsertText(text) => self.replace_range(range, text.into()),
            TextEditCommand::PasteText(text) => {
                self.replace_range_grouped(range, text.into(), TextEditKind::Clipboard)
            }
            TextEditCommand::CutSelection if !range.is_empty() => {
                self.replace_range_grouped(range, Arc::from(""), TextEditKind::Clipboard)
            }
            TextEditCommand::Backspace => {
                let range = if range.is_empty() {
                    previous_boundary(self.text(), at)..at
                } else {
                    range
                };
                (!range.is_empty())
                    .then(|| {
                        self.replace_range_grouped(
                            range,
                            Arc::from(""),
                            TextEditKind::BackwardDelete,
                        )
                    })
                    .flatten()
            }
            TextEditCommand::Delete => {
                let range = if range.is_empty() {
                    at..next_boundary(self.text(), at)
                } else {
                    range
                };
                (!range.is_empty())
                    .then(|| {
                        self.replace_range_grouped(
                            range,
                            Arc::from(""),
                            TextEditKind::ForwardDelete,
                        )
                    })
                    .flatten()
            }
            TextEditCommand::DeleteWordLeft => {
                let range = if range.is_empty() {
                    word_boundary(self.text(), at, false)..at
                } else {
                    range
                };
                self.replace_range_grouped(range, Arc::from(""), TextEditKind::BackwardDelete)
            }
            TextEditCommand::DeleteWordRight => {
                let range = if range.is_empty() {
                    at..word_boundary(self.text(), at, true)
                } else {
                    range
                };
                self.replace_range_grouped(range, Arc::from(""), TextEditKind::ForwardDelete)
            }
            TextEditCommand::MoveLeft { extend_selection } => self.move_logical(
                if !extend_selection && !range.is_empty() {
                    range.start
                } else {
                    previous_boundary(self.text(), at)
                },
                extend_selection,
            ),
            TextEditCommand::MoveRight { extend_selection } => self.move_logical(
                if !extend_selection && !range.is_empty() {
                    range.end
                } else {
                    next_boundary(self.text(), at)
                },
                extend_selection,
            ),
            TextEditCommand::MoveWordLeft { extend_selection } => {
                self.move_logical(word_boundary(self.text(), at, false), extend_selection)
            }
            TextEditCommand::MoveWordRight { extend_selection } => {
                self.move_logical(word_boundary(self.text(), at, true), extend_selection)
            }
            TextEditCommand::MoveHome { extend_selection } => {
                self.move_line_end(false, extend_selection)
            }
            TextEditCommand::MoveEnd { extend_selection } => {
                self.move_line_end(true, extend_selection)
            }
            TextEditCommand::SelectAll => self.emit(
                TextEditorDelta::Selection,
                TextEditorSelection {
                    anchor: 0,
                    caret: self.text().len(),
                    affinity: CaretAffinity::Upstream,
                },
            ),
            _ => None,
        }
    }
    pub(super) fn move_vertical(&mut self, lines: f32, extend: bool) -> Option<WidgetOutput> {
        let receipt = self.current_geometry()?;
        let selection = self.display_selection()?;
        let point = receipt.geometry().caret(ParagraphCaret {
            byte: selection.caret,
            affinity: selection.affinity,
        })?;
        let preferred = self.preferred_x.unwrap_or(point.x);
        let caret = receipt.geometry().hit_test(Point::new(
            preferred,
            point.y + lines * receipt.request().line_height + receipt.request().line_height * 0.5,
        ));
        self.preferred_x = Some(preferred);
        self.move_caret(self.source_caret(caret)?, extend)
    }
    pub(super) fn move_line_end(&mut self, end: bool, extend: bool) -> Option<WidgetOutput> {
        let receipt = self.current_geometry()?;
        let selection = self.display_selection()?;
        let point = receipt.geometry().caret(ParagraphCaret {
            byte: selection.caret,
            affinity: selection.affinity,
        })?;
        let index = (point.y / receipt.request().line_height).floor().max(0.0) as usize;
        let line = receipt.geometry().lines().get(index)?;
        let caret = ParagraphCaret {
            byte: if end {
                line.bytes.end
            } else {
                line.bytes.start
            },
            affinity: if end {
                CaretAffinity::Upstream
            } else {
                CaretAffinity::Downstream
            },
        };
        self.preferred_x = None;
        self.move_caret(self.source_caret(caret)?, extend)
    }
    pub(super) fn move_paragraph(&mut self, forward: bool, extend: bool) -> Option<WidgetOutput> {
        let at = self.selection().caret;
        let boundaries = std::iter::once(0)
            .chain(
                self.text()
                    .grapheme_indices(true)
                    .filter(|(_, s)| {
                        matches!(
                            *s,
                            "\r\n"
                                | "\r"
                                | "\n"
                                | "\u{b}"
                                | "\u{c}"
                                | "\u{85}"
                                | "\u{2028}"
                                | "\u{2029}"
                        )
                    })
                    .map(|(at, s)| at + s.len()),
            )
            .chain(std::iter::once(self.text().len()));
        let byte = if forward {
            boundaries
                .into_iter()
                .find(|byte| *byte > at)
                .unwrap_or(self.text().len())
        } else {
            boundaries.take_while(|byte| *byte < at).last().unwrap_or(0)
        };
        self.move_logical(byte, extend)
    }
}

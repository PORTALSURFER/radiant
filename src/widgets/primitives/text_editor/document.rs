//! Bounded application-owned text and exact-revision replacement deltas.
use crate::gui::text_layout::paragraph::CaretAffinity;
use std::{
    ops::Range,
    rc::{Rc, Weak},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};
use unicode_segmentation::UnicodeSegmentation;

/// Maximum UTF-8 bytes accepted by an editor document.
pub const MAX_TEXT_EDITOR_BYTES: usize = 1024 * 1024;
/// Maximum extended grapheme clusters accepted by an editor document.
pub const MAX_TEXT_EDITOR_GRAPHEMES: usize = 65_536;
static NEXT_OWNER: AtomicU64 = AtomicU64::new(1);
static NEXT_REVISION: AtomicU64 = AtomicU64::new(1);

fn next_revision() -> Result<TextEditorRevision, TextEditorError> {
    NEXT_REVISION
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
            value.checked_add(1)
        })
        .map(TextEditorRevision)
        .map_err(|_| TextEditorError::Exhausted)
}

/// Monotonic content authority belonging to one document.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TextEditorRevision(u64);
impl TextEditorRevision {
    /// Read the document's content revision.
    pub const fn value(self) -> u64 {
        self.0
    }
}
/// UTF-8 byte selection with explicit anchor and active caret.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextEditorSelection {
    /// Anchor byte boundary.
    pub anchor: usize,
    /// Active caret byte boundary.
    pub caret: usize,
    /// Visual side at a soft wrap or bidirectional boundary.
    pub affinity: CaretAffinity,
}
impl Default for TextEditorSelection {
    fn default() -> Self {
        Self::caret(0)
    }
}
impl TextEditorSelection {
    /// A collapsed caret at a byte boundary.
    pub const fn caret(byte: usize) -> Self {
        Self {
            anchor: byte,
            caret: byte,
            affinity: CaretAffinity::Downstream,
        }
    }
    /// Ordered selected byte range.
    pub fn range(self) -> Range<usize> {
        self.anchor.min(self.caret)..self.anchor.max(self.caret)
    }
}
/// A bounded edit or authority update was rejected without changing the document.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextEditorError {
    /// Text exceeds the byte or grapheme bound.
    TooLarge,
    /// Range or selection is not an ordered, in-bounds grapheme boundary.
    InvalidRange,
    /// The delta belongs to another or retired document.
    WrongOwner,
    /// The delta does not follow the current exact content revision.
    StaleRevision,
    /// A monotonic identity or revision cannot advance.
    Exhausted,
    /// The operation is incompatible with the active composition lifecycle.
    InvalidComposition,
}
impl std::fmt::Display for TextEditorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::TooLarge => "editor text exceeds its bound",
            Self::InvalidRange => "editor range is not a valid grapheme range",
            Self::WrongOwner => "editor document owner is not current",
            Self::StaleRevision => "editor revision is stale",
            Self::Exhausted => "editor identity or revision is exhausted",
            Self::InvalidComposition => "editor composition lifecycle is invalid",
        })
    }
}
impl std::error::Error for TextEditorError {}
/// Logical text mutation; offsets are UTF-8 byte boundaries in the preceding revision.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TextEditorDelta {
    /// Change logical selection without mutating committed text.
    Selection,
    /// Advance one transient composition lifecycle without product history policy.
    Composition(TextEditorCompositionDelta),
    /// Insert text at one boundary.
    Insert {
        /// Insertion boundary.
        at: usize,
        /// Inserted text.
        text: Arc<str>,
    },
    /// Delete an ordered grapheme range.
    Delete {
        /// Deleted byte range.
        range: Range<usize>,
    },
    /// Replace an ordered grapheme range.
    Replace {
        /// Replaced byte range.
        range: Range<usize>,
        /// Replacement text.
        text: Arc<str>,
    },
}
/// Backend-neutral composition operations. Offsets are UTF-8 grapheme boundaries.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TextEditorCompositionDelta {
    /// Begin replacing a range in the committed text.
    Start {
        /// Committed replacement range.
        range: Range<usize>,
    },
    /// Replace transient preedit text. The edit's selection addresses displayed text.
    Update {
        /// Preedit text, retained independently from committed text.
        text: Arc<str>,
    },
    /// Commit text into the original replacement range and end composition.
    Commit {
        /// Final replacement text.
        text: Arc<str>,
    },
    /// Discard preedit and restore committed text.
    Cancel,
}
#[derive(Clone, Debug)]
struct CompositionState {
    range: Range<usize>,
    display: Arc<str>,
    original_selection: TextEditorSelection,
}
/// Exact-owner edit delivered through the application's ordinary message path.
#[derive(Clone, Debug)]
pub struct TextEditorEdit {
    owner: Weak<()>,
    source_id: u64,
    expected: TextEditorRevision,
    next: TextEditorRevision,
    delta: TextEditorDelta,
    selection: TextEditorSelection,
}
impl TextEditorEdit {
    /// Revision against which this delta was produced.
    pub const fn expected_revision(&self) -> TextEditorRevision {
        self.expected
    }
    /// Revision after successful application.
    pub const fn resulting_revision(&self) -> TextEditorRevision {
        self.next
    }
    /// Logical text operation.
    pub fn delta(&self) -> &TextEditorDelta {
        &self.delta
    }
    /// Selection in the resulting text.
    pub const fn selection(&self) -> TextEditorSelection {
        self.selection
    }
}
/// Immutable content snapshot. Retaining it does not retain its document owner.
#[derive(Clone, Debug)]
pub struct TextEditorSnapshot {
    owner: Weak<()>,
    pub(crate) source_id: u64,
    revision: TextEditorRevision,
    text: Arc<str>,
    selection: TextEditorSelection,
    composition: Option<CompositionState>,
}
impl TextEditorSnapshot {
    /// Current selection in displayed text.
    pub const fn selection(&self) -> TextEditorSelection {
        self.selection
    }
    /// Whether a transient composition is active.
    pub fn is_composing(&self) -> bool {
        self.composition.is_some()
    }
    /// Project preedit text without changing committed document text.
    pub fn display_text(&self) -> Arc<str> {
        self.composition
            .as_ref()
            .map_or_else(|| self.text.clone(), |state| state.display.clone())
    }

    pub(crate) fn composition_range(&self) -> Option<Range<usize>> {
        self.composition.as_ref().map(|value| value.range.clone())
    }
    pub(crate) fn composition_original_selection(&self) -> Option<TextEditorSelection> {
        self.composition
            .as_ref()
            .map(|value| value.original_selection)
    }
    pub(crate) fn same_owner(&self, other: &Self) -> bool {
        self.source_id == other.source_id && Weak::ptr_eq(&self.owner, &other.owner)
    }
    /// UTF-8 content including hard line breaks.
    pub fn text(&self) -> &str {
        &self.text
    }
    /// Current content authority.
    pub const fn revision(&self) -> TextEditorRevision {
        self.revision
    }
    pub(crate) fn edit(
        &self,
        delta: TextEditorDelta,
        selection: TextEditorSelection,
    ) -> Result<TextEditorEdit, TextEditorError> {
        if self.owner.strong_count() == 0 {
            return Err(TextEditorError::WrongOwner);
        }
        let next = next_revision()?;
        let result = self.transition(&delta, selection)?;
        let selection = result.selection;
        validate_selection(&result.display_text(), selection)?;
        Ok(TextEditorEdit {
            owner: self.owner.clone(),
            source_id: self.source_id,
            expected: self.revision,
            next,
            delta,
            selection,
        })
    }
    pub(crate) fn after(&self, edit: &TextEditorEdit) -> Result<Self, TextEditorError> {
        if !Weak::ptr_eq(&self.owner, &edit.owner)
            || self.owner.strong_count() == 0
            || self.source_id != edit.source_id
        {
            return Err(TextEditorError::WrongOwner);
        }
        if self.revision != edit.expected || edit.next <= self.revision {
            return Err(TextEditorError::StaleRevision);
        }
        let mut result = self.transition(&edit.delta, edit.selection)?;
        validate_selection(&result.display_text(), result.selection)?;
        result.revision = edit.next;
        Ok(result)
    }
    fn transition(
        &self,
        delta: &TextEditorDelta,
        selection: TextEditorSelection,
    ) -> Result<Self, TextEditorError> {
        let mut next = self.clone();
        match delta {
            TextEditorDelta::Selection => {}
            TextEditorDelta::Composition(change) => match change {
                TextEditorCompositionDelta::Start { range } => {
                    if next.composition.is_some() {
                        return Err(TextEditorError::InvalidComposition);
                    }
                    validate_range(&next.text, range)?;
                    next.composition = Some(CompositionState {
                        range: range.clone(),
                        display: self.text.clone(),
                        original_selection: self.selection,
                    });
                }
                TextEditorCompositionDelta::Update { text } => {
                    let state = next
                        .composition
                        .as_mut()
                        .ok_or(TextEditorError::InvalidComposition)?;
                    state.display = replace(
                        &next.text,
                        &TextEditorDelta::Replace {
                            range: state.range.clone(),
                            text: text.clone(),
                        },
                    )?;
                }
                TextEditorCompositionDelta::Commit { text } => {
                    let state = next
                        .composition
                        .take()
                        .ok_or(TextEditorError::InvalidComposition)?;
                    next.text = replace(
                        &next.text,
                        &TextEditorDelta::Replace {
                            range: state.range,
                            text: text.clone(),
                        },
                    )?;
                }
                TextEditorCompositionDelta::Cancel => {
                    next.composition
                        .take()
                        .ok_or(TextEditorError::InvalidComposition)?;
                }
            },
            _ => {
                if next.composition.is_some() {
                    return Err(TextEditorError::InvalidComposition);
                }
                next.text = replace(&next.text, delta)?;
            }
        }
        next.selection = match delta {
            // Native composition selections are scalar-indexed, whereas editor
            // selections are constrained to extended-grapheme boundaries. A
            // preedit or committed replacement can join a neighboring grapheme
            // across the replacement seam, so normalize against the resulting
            // displayed text rather than rejecting an otherwise valid IME update.
            TextEditorDelta::Composition(
                TextEditorCompositionDelta::Update { .. }
                | TextEditorCompositionDelta::Commit { .. },
            ) => normalize_composition_selection(next.display_text().as_ref(), selection)?,
            _ => selection,
        };
        Ok(next)
    }
}
/// Application-owned bounded document. The framework does not own product history or persistence.
pub struct TextEditorDocument {
    owner: Rc<()>,
    source_id: u64,
    revision: TextEditorRevision,
    text: Arc<str>,
    selection: TextEditorSelection,
    composition: Option<CompositionState>,
}
impl TextEditorDocument {
    /// Create a document with a fresh authority revision, preserving hard line breaks.
    pub fn new(text: impl Into<Arc<str>>) -> Result<Self, TextEditorError> {
        let text = text.into();
        validate_text(&text)?;
        let source_id = NEXT_OWNER
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |v| v.checked_add(1))
            .map_err(|_| TextEditorError::Exhausted)?;
        Ok(Self {
            owner: Rc::new(()),
            source_id,
            revision: next_revision()?,
            text,
            selection: TextEditorSelection::default(),
            composition: None,
        })
    }
    /// Read current UTF-8 content.
    pub fn text(&self) -> &str {
        &self.text
    }
    /// Read the current authority revision.
    pub const fn revision(&self) -> TextEditorRevision {
        self.revision
    }
    /// Create an immutable input for a controlled editor.
    pub fn snapshot(&self) -> TextEditorSnapshot {
        TextEditorSnapshot {
            owner: Rc::downgrade(&self.owner),
            source_id: self.source_id,
            revision: self.revision,
            text: self.text.clone(),
            selection: self.selection,
            composition: self.composition.clone(),
        }
    }
    /// Apply an exact-owner delta once. Failed edits leave text and revision untouched.
    pub fn apply(&mut self, edit: &TextEditorEdit) -> Result<TextEditorRevision, TextEditorError> {
        let next = self.snapshot().after(edit)?;
        self.text = next.text;
        self.revision = next.revision;
        self.selection = next.selection;
        self.composition = next.composition;
        Ok(self.revision)
    }
    /// Publish a newer external authority, including an explicit same-value reset.
    pub fn set_text(
        &mut self,
        text: impl Into<Arc<str>>,
    ) -> Result<TextEditorRevision, TextEditorError> {
        let text = text.into();
        validate_text(&text)?;
        let next = next_revision()?;
        self.text = text;
        self.revision = next;
        self.selection = TextEditorSelection::default();
        self.composition = None;
        Ok(self.revision)
    }
}
fn boundary(text: &str, offset: usize) -> bool {
    offset == text.len() || text.grapheme_indices(true).any(|(at, _)| at == offset)
}
fn validate_text(text: &str) -> Result<(), TextEditorError> {
    if text.len() > MAX_TEXT_EDITOR_BYTES
        || text
            .graphemes(true)
            .take(MAX_TEXT_EDITOR_GRAPHEMES + 1)
            .count()
            > MAX_TEXT_EDITOR_GRAPHEMES
    {
        Err(TextEditorError::TooLarge)
    } else {
        Ok(())
    }
}
fn validate_selection(text: &str, selection: TextEditorSelection) -> Result<(), TextEditorError> {
    if boundary(text, selection.anchor) && boundary(text, selection.caret) {
        Ok(())
    } else {
        Err(TextEditorError::InvalidRange)
    }
}

fn normalize_composition_selection(
    text: &str,
    selection: TextEditorSelection,
) -> Result<TextEditorSelection, TextEditorError> {
    if selection.anchor > text.len()
        || selection.caret > text.len()
        || !text.is_char_boundary(selection.anchor)
        || !text.is_char_boundary(selection.caret)
    {
        return Err(TextEditorError::InvalidRange);
    }
    let floor_boundary = |offset| {
        text.grapheme_indices(true)
            .map(|(at, _)| at)
            .chain(std::iter::once(text.len()))
            .take_while(|at| *at <= offset)
            .last()
            .unwrap_or(0)
    };
    let ceiling_boundary = |offset| {
        text.grapheme_indices(true)
            .map(|(at, _)| at)
            .chain(std::iter::once(text.len()))
            .find(|at| *at >= offset)
            .unwrap_or(text.len())
    };
    let (anchor, caret) = if selection.anchor == selection.caret {
        // A native scalar caret inside a grapheme advances to its downstream
        // editor boundary, preserving continued composition after that cluster.
        let boundary = ceiling_boundary(selection.caret);
        (boundary, boundary)
    } else if selection.anchor < selection.caret {
        (
            floor_boundary(selection.anchor),
            ceiling_boundary(selection.caret),
        )
    } else {
        (
            ceiling_boundary(selection.anchor),
            floor_boundary(selection.caret),
        )
    };
    Ok(TextEditorSelection {
        anchor,
        caret,
        affinity: selection.affinity,
    })
}
fn validate_range(source: &str, range: &Range<usize>) -> Result<(), TextEditorError> {
    if range.start > range.end
        || range.end > source.len()
        || !boundary(source, range.start)
        || !boundary(source, range.end)
    {
        Err(TextEditorError::InvalidRange)
    } else {
        Ok(())
    }
}
fn replace(source: &str, delta: &TextEditorDelta) -> Result<Arc<str>, TextEditorError> {
    let (range, text) = match delta {
        TextEditorDelta::Selection | TextEditorDelta::Composition(_) => {
            return Err(TextEditorError::InvalidComposition);
        }
        TextEditorDelta::Insert { at, text } => (*at..*at, text.as_ref()),
        TextEditorDelta::Delete { range } => (range.clone(), ""),
        TextEditorDelta::Replace { range, text } => (range.clone(), text.as_ref()),
    };
    if range.start > range.end
        || range.end > source.len()
        || !boundary(source, range.start)
        || !boundary(source, range.end)
    {
        return Err(TextEditorError::InvalidRange);
    }
    let size = source.len() - (range.end - range.start);
    let size = size
        .checked_add(text.len())
        .filter(|size| *size <= MAX_TEXT_EDITOR_BYTES)
        .ok_or(TextEditorError::TooLarge)?;
    let mut result = String::with_capacity(size);
    result.push_str(&source[..range.start]);
    result.push_str(text);
    result.push_str(&source[range.end..]);
    validate_text(&result)?;
    Ok(result.into())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ordered_edits_apply_once_and_foreign_or_stale_edits_are_inert() {
        let mut doc = TextEditorDocument::new("a\nb").unwrap();
        let edit = doc
            .snapshot()
            .edit(
                TextEditorDelta::Insert {
                    at: 1,
                    text: Arc::from("!"),
                },
                TextEditorSelection::caret(2),
            )
            .unwrap();
        let mut other = TextEditorDocument::new("a\nb").unwrap();
        assert!(doc.snapshot().same_owner(&doc.snapshot()));
        assert!(!doc.snapshot().same_owner(&other.snapshot()));
        assert_eq!(other.apply(&edit), Err(TextEditorError::WrongOwner));
        doc.apply(&edit).unwrap();
        assert_eq!(doc.text(), "a!\nb");
        assert_eq!(doc.apply(&edit), Err(TextEditorError::StaleRevision));
    }
    #[test]
    fn grapheme_ranges_and_resulting_selection_are_validated() {
        let doc = TextEditorDocument::new("a\u{301}b").unwrap();
        assert_eq!(
            doc.snapshot()
                .edit(
                    TextEditorDelta::Delete { range: 1..3 },
                    TextEditorSelection::caret(0)
                )
                .unwrap_err(),
            TextEditorError::InvalidRange
        );
        assert_eq!(
            doc.snapshot()
                .edit(
                    TextEditorDelta::Insert {
                        at: 3,
                        text: Arc::from("x")
                    },
                    TextEditorSelection::caret(1)
                )
                .unwrap_err(),
            TextEditorError::InvalidRange
        );
    }
    #[test]
    fn external_authority_invalidates_unapplied_delta_and_preserves_newlines() {
        let mut doc = TextEditorDocument::new("a\r\n").unwrap();
        let edit = doc
            .snapshot()
            .edit(
                TextEditorDelta::Insert {
                    at: 0,
                    text: Arc::from("x"),
                },
                TextEditorSelection::caret(1),
            )
            .unwrap();
        doc.set_text("a\r\n").unwrap();
        assert_eq!(doc.apply(&edit), Err(TextEditorError::StaleRevision));
        assert_eq!(doc.text(), "a\r\n");
    }
    #[test]
    fn snapshot_does_not_keep_retired_document_alive() {
        let snapshot = TextEditorDocument::new("a").unwrap().snapshot();
        assert_eq!(
            snapshot
                .edit(
                    TextEditorDelta::Delete { range: 0..1 },
                    TextEditorSelection::caret(0)
                )
                .unwrap_err(),
            TextEditorError::WrongOwner
        );
    }
    #[test]
    fn ordered_selection_and_composition_keep_committed_text_until_commit() {
        let mut doc = TextEditorDocument::new("ab\ncd").unwrap();
        let steps = [
            (
                TextEditorDelta::Selection,
                TextEditorSelection::caret(1),
                "ab\ncd",
                "ab\ncd",
            ),
            (
                TextEditorDelta::Composition(TextEditorCompositionDelta::Start { range: 1..2 }),
                TextEditorSelection::caret(1),
                "ab\ncd",
                "ab\ncd",
            ),
            (
                TextEditorDelta::Composition(TextEditorCompositionDelta::Update {
                    text: Arc::from("漢"),
                }),
                TextEditorSelection::caret(4),
                "ab\ncd",
                "a漢\ncd",
            ),
            (
                TextEditorDelta::Composition(TextEditorCompositionDelta::Commit {
                    text: Arc::from("漢字"),
                }),
                TextEditorSelection::caret(7),
                "a漢字\ncd",
                "a漢字\ncd",
            ),
        ];
        for (delta, selection, committed, displayed) in steps {
            let edit = doc.snapshot().edit(delta, selection).unwrap();
            doc.apply(&edit).unwrap();
            assert_eq!(doc.text(), committed);
            assert_eq!(doc.snapshot().display_text().as_ref(), displayed);
            assert_eq!(doc.snapshot().selection(), selection);
            assert_eq!(doc.apply(&edit), Err(TextEditorError::StaleRevision));
        }
        assert!(!doc.snapshot().is_composing());
    }
    #[test]
    fn external_authority_fences_ahead_of_optimistic_edits_and_composition() {
        let mut doc = TextEditorDocument::new("a").unwrap();
        let first = doc
            .snapshot()
            .edit(
                TextEditorDelta::Insert {
                    at: 1,
                    text: Arc::from("b"),
                },
                TextEditorSelection::caret(2),
            )
            .unwrap();
        let optimistic = doc.snapshot().after(&first).unwrap();
        let second = optimistic
            .edit(
                TextEditorDelta::Insert {
                    at: 2,
                    text: Arc::from("c"),
                },
                TextEditorSelection::caret(3),
            )
            .unwrap();
        let authority = doc.set_text("reset").unwrap();
        assert!(authority > second.resulting_revision());
        assert_eq!(doc.apply(&first), Err(TextEditorError::StaleRevision));
        assert_eq!(doc.apply(&second), Err(TextEditorError::StaleRevision));
    }
    #[test]
    fn composition_cancel_restores_text_and_invalid_sequences_are_inert() {
        let mut doc = TextEditorDocument::new("ab").unwrap();
        assert_eq!(
            doc.snapshot()
                .edit(
                    TextEditorDelta::Composition(TextEditorCompositionDelta::Cancel),
                    TextEditorSelection::caret(0)
                )
                .unwrap_err(),
            TextEditorError::InvalidComposition
        );
        let start = doc
            .snapshot()
            .edit(
                TextEditorDelta::Composition(TextEditorCompositionDelta::Start { range: 0..1 }),
                TextEditorSelection::caret(0),
            )
            .unwrap();
        doc.apply(&start).unwrap();
        let update = doc
            .snapshot()
            .edit(
                TextEditorDelta::Composition(TextEditorCompositionDelta::Update {
                    text: Arc::from("é"),
                }),
                TextEditorSelection::caret(2),
            )
            .unwrap();
        doc.apply(&update).unwrap();
        assert_eq!(doc.snapshot().display_text().as_ref(), "éb");
        let snapshot = doc.snapshot();
        let cancel = snapshot
            .edit(
                TextEditorDelta::Composition(TextEditorCompositionDelta::Cancel),
                snapshot.composition_original_selection().unwrap(),
            )
            .unwrap();
        doc.apply(&cancel).unwrap();
        assert_eq!(doc.snapshot().display_text().as_ref(), "ab");
        assert!(!doc.snapshot().is_composing());
    }
    #[test]
    fn bounds_fail_without_mutation() {
        let mut doc = TextEditorDocument::new("a").unwrap();
        let before = doc.revision();
        assert_eq!(
            doc.set_text("x".repeat(MAX_TEXT_EDITOR_GRAPHEMES + 1)),
            Err(TextEditorError::TooLarge)
        );
        assert_eq!(doc.text(), "a");
        assert_eq!(doc.revision(), before);
    }
}

//! Privacy and revocable authority foundations for text editing.

use std::{
    fmt,
    sync::{
        Arc, Weak,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
};

use unicode_segmentation::UnicodeSegmentation;

/// Privacy treatment for text presented by a widget.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TextPrivacy {
    /// Text may participate in ordinary copy and automation behavior.
    #[default]
    Public,
    /// Text must use the supplied restrictive policy.
    Secret(TextSecretPolicy),
}

/// Explicit opt-ins for operations on secret text.
///
/// Copy and automation are both denied by default. Callers must opt in to each
/// operation deliberately before a future integration may permit it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct TextSecretPolicy {
    copy_allowed: bool,
    automation_allowed: bool,
}

impl TextSecretPolicy {
    /// Create a policy that denies copy and automation.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            copy_allowed: false,
            automation_allowed: false,
        }
    }

    /// Allow copy operations for this secret text.
    #[must_use]
    pub const fn allow_copy(mut self) -> Self {
        self.copy_allowed = true;
        self
    }

    /// Allow automation operations for this secret text.
    #[must_use]
    pub const fn allow_automation(mut self) -> Self {
        self.automation_allowed = true;
        self
    }

    /// Return whether copy operations are explicitly allowed.
    #[must_use]
    pub const fn copy_allowed(self) -> bool {
        self.copy_allowed
    }

    /// Return whether automation operations are explicitly allowed.
    #[must_use]
    pub const fn automation_allowed(self) -> bool {
        self.automation_allowed
    }
}

#[derive(Debug)]
struct AuthorityGeneration {
    generation: AtomicU64,
    cancelled: AtomicBool,
}

/// Owner that issues and revokes authority to edit one text value.
///
/// Advancing the owner invalidates every earlier receipt. If the generation
/// counter would overflow, the owner permanently fails closed and no receipt
/// remains current.
pub struct TextEditAuthorityOwner {
    generation: Arc<AuthorityGeneration>,
}

impl Default for TextEditAuthorityOwner {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for TextEditAuthorityOwner {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TextEditAuthorityOwner")
            .field(
                "generation",
                &self.generation.generation.load(Ordering::Acquire),
            )
            .field(
                "cancelled",
                &self.generation.cancelled.load(Ordering::Acquire),
            )
            .finish()
    }
}

impl TextEditAuthorityOwner {
    /// Create an owner at its initial generation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            generation: Arc::new(AuthorityGeneration {
                generation: AtomicU64::new(0),
                cancelled: AtomicBool::new(false),
            }),
        }
    }

    /// Issue a weak receipt for the current generation.
    ///
    /// The receipt does not keep this owner alive.
    #[must_use]
    pub fn authority(&self) -> Option<TextEditAuthority> {
        if self.generation.cancelled.load(Ordering::Acquire) {
            return None;
        }

        Some(TextEditAuthority {
            generation: Arc::downgrade(&self.generation),
            issued_generation: self.generation.generation.load(Ordering::Acquire),
        })
    }

    /// Advance to a fresh generation and issue its receipt.
    ///
    /// All earlier receipts become cancelled. Overflow permanently cancels the
    /// owner and returns no replacement receipt.
    #[must_use]
    pub fn advance(&self) -> Option<TextEditAuthority> {
        if self.generation.cancelled.load(Ordering::Acquire) {
            return None;
        }

        let advanced = self.generation.generation.fetch_update(
            Ordering::AcqRel,
            Ordering::Acquire,
            |current| current.checked_add(1),
        );
        match advanced {
            Ok(_) => self.authority(),
            Err(_) => {
                self.generation.cancelled.store(true, Ordering::Release);
                None
            }
        }
    }

    /// Return whether `authority` was issued by this owner and is still current.
    #[must_use]
    pub fn is_current(&self, authority: &TextEditAuthority) -> bool {
        if self.generation.cancelled.load(Ordering::Acquire) {
            return false;
        }

        let Some(receipt_generation) = authority.generation.upgrade() else {
            return false;
        };
        Arc::ptr_eq(&self.generation, &receipt_generation)
            && authority.issued_generation == self.generation.generation.load(Ordering::Acquire)
    }
}

/// Immutable, weak authority receipt for one text-edit generation.
///
/// This receipt is `Send`, so a worker can check [`Self::is_cancelled`] without
/// owning a UI object. It never keeps its issuing owner alive.
#[derive(Clone)]
pub struct TextEditAuthority {
    generation: Weak<AuthorityGeneration>,
    issued_generation: u64,
}

impl fmt::Debug for TextEditAuthority {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TextEditAuthority")
            .field("generation", &self.issued_generation)
            .field("cancelled", &self.is_cancelled())
            .finish()
    }
}

impl TextEditAuthority {
    /// Return whether this receipt can no longer authorize work.
    ///
    /// This is safe to call from a `Send` worker and returns `true` after the
    /// owner is dropped, advanced, or has failed closed on generation overflow.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        let Some(generation) = self.generation.upgrade() else {
            return true;
        };
        generation.cancelled.load(Ordering::Acquire)
            || generation.generation.load(Ordering::Acquire) != self.issued_generation
    }
}

const MAX_SECRET_TEXT_BYTES: usize = 1024 * 1024;
const MAX_SECRET_TEXT_UNITS: usize = 65_536;
const MASK: char = '•';

/// The unit used to mask secret text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SecretTextUnit {
    /// Mask each Unicode scalar value. A CRLF sequence is two scalar units.
    Scalar,
    /// Mask each extended grapheme cluster. A CRLF sequence is one unit.
    ExtendedGrapheme,
}

/// Reason a secret-text mapping could not be built.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SecretTextMappingError {
    /// The source exceeded the one-mebibyte bound.
    SourceTooLong,
    /// The selected unitization exceeded the bounded unit count.
    TooManyUnits,
}

/// Bounded, text-free source/display offset mapping for secret text.
pub(crate) struct SecretTextMapping {
    masked: Arc<str>,
    source_boundaries: Box<[usize]>,
    display_boundaries: Box<[usize]>,
}

impl fmt::Debug for SecretTextMapping {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SecretTextMapping")
            .field("masked_bytes", &self.masked.len())
            .field("units", &(self.source_boundaries.len() - 1))
            .finish()
    }
}

impl SecretTextMapping {
    /// Build a bounded masked mapping without retaining `source`.
    pub(crate) fn new(source: &str, unit: SecretTextUnit) -> Result<Self, SecretTextMappingError> {
        if source.len() > MAX_SECRET_TEXT_BYTES {
            return Err(SecretTextMappingError::SourceTooLong);
        }

        let ranges: Vec<(usize, usize)> = match unit {
            SecretTextUnit::Scalar => source
                .char_indices()
                .map(|(start, character)| (start, start + character.len_utf8()))
                .collect(),
            SecretTextUnit::ExtendedGrapheme => UnicodeSegmentation::grapheme_indices(source, true)
                .map(|(start, grapheme)| (start, start + grapheme.len()))
                .collect(),
        };
        if ranges.len() > MAX_SECRET_TEXT_UNITS {
            return Err(SecretTextMappingError::TooManyUnits);
        }

        let mut masked = String::with_capacity(ranges.len() * MASK.len_utf8());
        let mut source_boundaries = Vec::with_capacity(ranges.len() + 1);
        let mut display_boundaries = Vec::with_capacity(ranges.len() + 1);
        source_boundaries.push(0);
        display_boundaries.push(0);

        for (start, end) in ranges {
            let source_unit = &source[start..end];
            if is_hard_separator(source_unit) {
                masked.push('\n');
            } else {
                masked.push(MASK);
            }
            source_boundaries.push(end);
            display_boundaries.push(masked.len());
        }

        Ok(Self {
            masked: Arc::from(masked),
            source_boundaries: source_boundaries.into_boxed_slice(),
            display_boundaries: display_boundaries.into_boxed_slice(),
        })
    }

    /// Return the retained masked display text.
    pub(crate) fn masked(&self) -> &str {
        &self.masked
    }

    /// Map an exact source byte boundary to its display byte boundary.
    pub(crate) fn source_to_display_byte(&self, source_offset: usize) -> Option<usize> {
        exact_boundary(
            &self.source_boundaries,
            &self.display_boundaries,
            source_offset,
        )
    }

    /// Map an exact display byte boundary to its source byte boundary.
    pub(crate) fn display_to_source_byte(&self, display_offset: usize) -> Option<usize> {
        exact_boundary(
            &self.display_boundaries,
            &self.source_boundaries,
            display_offset,
        )
    }
}

fn exact_boundary(boundaries: &[usize], mapped: &[usize], offset: usize) -> Option<usize> {
    boundaries
        .binary_search(&offset)
        .ok()
        .map(|index| mapped[index])
}

fn is_hard_separator(unit: &str) -> bool {
    matches!(
        unit,
        "\r\n" | "\r" | "\n" | "\u{000B}" | "\u{000C}" | "\u{0085}" | "\u{2028}" | "\u{2029}"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn privacy_defaults_are_public_and_secret_opt_ins_are_explicit() {
        assert_eq!(TextPrivacy::default(), TextPrivacy::Public);
        let policy = TextSecretPolicy::default();
        assert!(!policy.copy_allowed());
        assert!(!policy.automation_allowed());
        let policy = policy.allow_copy().allow_automation();
        assert!(policy.copy_allowed());
        assert!(policy.automation_allowed());
    }

    #[test]
    fn authority_receipt_is_weak_and_advancing_cancels_old_work() {
        let owner = TextEditAuthorityOwner::new();
        let first = owner.authority().expect("live owner issues a receipt");
        assert!(owner.is_current(&first));
        assert!(!first.is_cancelled());

        let second = owner.advance().expect("advance issues a fresh receipt");
        assert!(first.is_cancelled());
        assert!(!owner.is_current(&first));
        assert!(owner.is_current(&second));

        drop(owner);
        assert!(second.is_cancelled());
    }

    #[test]
    fn foreign_receipts_are_never_current() {
        let first = TextEditAuthorityOwner::new();
        let second = TextEditAuthorityOwner::new();
        let receipt = first.authority().expect("live owner issues a receipt");

        assert!(!second.is_current(&receipt));
        assert!(first.is_current(&receipt));
    }

    #[test]
    fn grapheme_mapping_keeps_combining_and_zwj_sequences_together() {
        let source = "e\u{301}👩\u{200d}💻";
        let mapping = SecretTextMapping::new(source, SecretTextUnit::ExtendedGrapheme)
            .expect("bounded source");

        assert_eq!(mapping.masked(), "••");
        assert_eq!(mapping.source_to_display_byte(0), Some(0));
        assert_eq!(
            mapping.source_to_display_byte("e\u{301}".len()),
            Some(MASK.len_utf8())
        );
        assert_eq!(mapping.source_to_display_byte(1), None);
        assert_eq!(mapping.display_to_source_byte(1), None);
        assert_eq!(
            mapping.display_to_source_byte(MASK.len_utf8()),
            Some("e\u{301}".len())
        );
    }

    #[test]
    fn hard_breaks_normalize_and_crlf_unitization_is_mode_specific() {
        let source = "a\r\nb\r\u{000B}\u{000C}\u{0085}\u{2028}\u{2029}";
        let scalar =
            SecretTextMapping::new(source, SecretTextUnit::Scalar).expect("bounded source");
        let grapheme = SecretTextMapping::new(source, SecretTextUnit::ExtendedGrapheme)
            .expect("bounded source");

        assert_eq!(scalar.masked(), "•\n\n•\n\n\n\n\n\n");
        assert_eq!(grapheme.masked(), "•\n•\n\n\n\n\n\n");
        assert_eq!(scalar.source_to_display_byte(2), Some(MASK.len_utf8() + 1));
        assert_eq!(grapheme.source_to_display_byte(2), None);
    }

    #[test]
    fn source_and_unit_bounds_are_enforced() {
        let too_many_units = "a".repeat(MAX_SECRET_TEXT_UNITS + 1);
        assert!(matches!(
            SecretTextMapping::new(&too_many_units, SecretTextUnit::Scalar),
            Err(SecretTextMappingError::TooManyUnits)
        ));
        let too_many_bytes = "a".repeat(MAX_SECRET_TEXT_BYTES + 1);
        assert!(matches!(
            SecretTextMapping::new(&too_many_bytes, SecretTextUnit::Scalar),
            Err(SecretTextMappingError::SourceTooLong)
        ));
    }

    #[test]
    fn debug_output_redacts_secret_text_and_addresses() {
        let source = "do not disclose";
        let mapping =
            SecretTextMapping::new(source, SecretTextUnit::Scalar).expect("bounded source");
        let owner = TextEditAuthorityOwner::new();
        let authority = owner.authority().expect("live owner issues a receipt");

        for debug in [
            format!("{mapping:?}"),
            format!("{owner:?}"),
            format!("{authority:?}"),
        ] {
            assert!(!debug.contains(source));
            assert!(!debug.contains("0x"));
        }
    }
}

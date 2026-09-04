//! Deterministic application text localization primitives.

use super::TextContent;
use std::{collections::HashMap, fmt, sync::Arc};

/// Maximum number of distinct missing-key diagnostics retained per collector.
pub const MAX_LOCALIZATION_DIAGNOSTICS: usize = 128;

/// Canonical, validated locale identifier.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct LocaleId(Arc<str>);

impl LocaleId {
    /// Construct a locale identifier from ASCII language and region segments.
    pub fn new(value: impl AsRef<str>) -> Result<Self, LocaleIdError> {
        let value = value.as_ref();
        if value.is_empty() {
            return Err(LocaleIdError::Empty);
        }
        if value.len() > 32 {
            return Err(LocaleIdError::TooLong);
        }
        if !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
        {
            return Err(LocaleIdError::InvalidCharacter);
        }
        let canonical = value.replace('_', "-").to_ascii_lowercase();
        if canonical.split('-').any(str::is_empty) {
            return Err(LocaleIdError::InvalidCharacter);
        }
        Ok(Self(Arc::from(canonical)))
    }

    /// Return the canonical English locale.
    pub fn english() -> Self {
        Self(Arc::from("en"))
    }

    /// Return the canonical locale bytes.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for LocaleId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Validation failure for a locale identifier.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocaleIdError {
    /// No locale bytes were supplied.
    Empty,
    /// Locale identifiers are bounded to keep diagnostics stable.
    TooLong,
    /// A locale contains a non-ASCII-alphanumeric separator or bad edge.
    InvalidCharacter,
}

impl fmt::Display for LocaleIdError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Empty => "locale identifier cannot be empty",
            Self::TooLong => "locale identifier is too long",
            Self::InvalidCharacter => "locale identifier contains an invalid character",
        })
    }
}

impl std::error::Error for LocaleIdError {}

/// Stable application key with an optional source-language fallback.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct TextKey {
    key: Arc<str>,
    source_fallback: Option<TextContent>,
}

impl TextKey {
    /// Construct a key with the source-language fallback visible when no
    /// catalog entry exists.
    pub fn new(key: impl AsRef<str>, source_fallback: impl Into<TextContent>) -> Self {
        Self {
            key: Arc::from(key.as_ref()),
            source_fallback: Some(source_fallback.into()),
        }
    }

    /// Construct a key with no source fallback.
    pub fn without_source_fallback(key: impl AsRef<str>) -> Self {
        Self {
            key: Arc::from(key.as_ref()),
            source_fallback: None,
        }
    }

    /// Return the stable key bytes.
    pub fn as_str(&self) -> &str {
        &self.key
    }

    /// Return the optional source-language fallback.
    pub fn source_fallback(&self) -> Option<&TextContent> {
        self.source_fallback.as_ref()
    }
}

/// Outcome of deterministic localized text resolution.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum LocalizationOutcome {
    /// The requested locale contained the key.
    Exact,
    /// An explicit application fallback contained the key.
    ExplicitFallback,
    /// The key's source-language fallback was used.
    SourceFallback,
    /// No value was available; the resolved content is empty.
    Missing,
}

/// Text plus the resolution evidence needed by visible and accessibility
/// consumers to share exactly the same bytes.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct LocalizedText {
    content: TextContent,
    requested_locale: Option<LocaleId>,
    resolved_locale: Option<LocaleId>,
    outcome: LocalizationOutcome,
    catalog_generation: u64,
}

impl LocalizedText {
    /// Return the resolved display bytes.
    pub fn as_str(&self) -> &str {
        self.content.as_str()
    }

    /// Return the requested locale, if a chain was supplied.
    pub fn requested_locale(&self) -> Option<&LocaleId> {
        self.requested_locale.as_ref()
    }

    /// Return the locale that supplied the catalog value.
    pub fn resolved_locale(&self) -> Option<&LocaleId> {
        self.resolved_locale.as_ref()
    }

    /// Return the deterministic resolution outcome.
    pub const fn outcome(&self) -> LocalizationOutcome {
        self.outcome
    }

    /// Borrow the shared text content used by visible and semantic output.
    pub fn content(&self) -> &TextContent {
        &self.content
    }

    /// Clone the shared display value for a second visible or semantic owner.
    pub fn to_content(&self) -> TextContent {
        self.content.clone()
    }

    /// Consume the resolution and return its shared display value.
    pub fn into_content(self) -> TextContent {
        self.content
    }

    /// Return the catalog generation used for this resolution.
    pub const fn catalog_generation(&self) -> u64 {
        self.catalog_generation
    }
}

impl AsRef<str> for LocalizedText {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for LocalizedText {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Immutable locale/key text catalog.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TextCatalog {
    generation: u64,
    entries: HashMap<(LocaleId, Arc<str>), TextContent>,
}

/// One stable missing-key diagnostic identity.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct MissingTextDiagnostic {
    catalog_generation: u64,
    requested_locale: Option<LocaleId>,
    key: Arc<str>,
}

impl MissingTextDiagnostic {
    /// Return the catalog generation in which the key was missing.
    pub const fn catalog_generation(&self) -> u64 {
        self.catalog_generation
    }

    /// Return the requested locale, if one was supplied.
    pub fn requested_locale(&self) -> Option<&LocaleId> {
        self.requested_locale.as_ref()
    }

    /// Return the missing key.
    pub fn key(&self) -> &str {
        &self.key
    }
}

/// Bounded stable collector for missing localization entries.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LocalizationDiagnostics {
    entries: Vec<MissingTextDiagnostic>,
}

impl LocalizationDiagnostics {
    /// Record one missing result, preserving insertion order and deduplicating
    /// repeated reports for the same catalog/locale/key identity.
    pub fn record(&mut self, key: &TextKey, resolved: &LocalizedText) {
        if resolved.outcome != LocalizationOutcome::Missing
            || self.entries.len() >= MAX_LOCALIZATION_DIAGNOSTICS
        {
            return;
        }
        let diagnostic = MissingTextDiagnostic {
            catalog_generation: resolved.catalog_generation,
            requested_locale: resolved.requested_locale.clone(),
            key: Arc::clone(&key.key),
        };
        if !self.entries.contains(&diagnostic) {
            self.entries.push(diagnostic);
        }
    }

    /// Return retained diagnostics in stable insertion order.
    pub fn entries(&self) -> &[MissingTextDiagnostic] {
        &self.entries
    }
}

impl TextCatalog {
    /// Set the catalog generation used for cache and diagnostic identity.
    pub const fn with_generation(mut self, generation: u64) -> Self {
        self.generation = generation;
        self
    }

    /// Add or replace one locale/key entry.
    pub fn insert(mut self, locale: LocaleId, key: TextKey, value: impl Into<TextContent>) -> Self {
        self.entries
            .insert((locale, Arc::clone(&key.key)), value.into());
        self
    }

    /// Return the catalog generation.
    pub const fn generation(&self) -> u64 {
        self.generation
    }

    /// Resolve a key through the supplied ordered chain.
    pub fn resolve(&self, key: &TextKey, chain: &[LocaleId]) -> LocalizedText {
        let requested_locale = chain.first().cloned();
        for (index, locale) in chain.iter().enumerate() {
            if let Some(content) = self.entries.get(&(locale.clone(), Arc::clone(&key.key))) {
                return LocalizedText {
                    content: content.clone(),
                    requested_locale,
                    resolved_locale: Some(locale.clone()),
                    outcome: if index == 0 {
                        LocalizationOutcome::Exact
                    } else {
                        LocalizationOutcome::ExplicitFallback
                    },
                    catalog_generation: self.generation,
                };
            }
        }
        if let Some(content) = &key.source_fallback {
            return LocalizedText {
                content: content.clone(),
                requested_locale,
                resolved_locale: None,
                outcome: LocalizationOutcome::SourceFallback,
                catalog_generation: self.generation,
            };
        }
        LocalizedText {
            content: TextContent::from(""),
            requested_locale,
            resolved_locale: None,
            outcome: LocalizationOutcome::Missing,
            catalog_generation: self.generation,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{LocaleId, LocaleIdError, LocalizationOutcome, TextCatalog, TextKey};

    #[test]
    fn locale_ids_are_canonical_and_validated() {
        for (input, expected) in [("EN_us", "en-us"), ("ZH_Hant-TW", "zh-hant-tw")] {
            assert_eq!(LocaleId::new(input).unwrap().as_str(), expected);
        }

        for input in [
            "_", "-", "_en", "en_", "-en", "en-", "en__US", "en--US", "en-_US", "en_-US",
        ] {
            assert_eq!(LocaleId::new(input), Err(LocaleIdError::InvalidCharacter));
        }
        assert_eq!(LocaleId::new(""), Err(LocaleIdError::Empty));
        assert_eq!(LocaleId::new("en/"), Err(LocaleIdError::InvalidCharacter));
        assert_eq!(LocaleId::new("a".repeat(33)), Err(LocaleIdError::TooLong));
    }

    #[test]
    fn source_and_missing_outcomes_are_explicit_and_stable() {
        let source = TextKey::new("save", "Save");
        let resolved = TextCatalog::default().resolve(&source, &[LocaleId::english()]);
        assert_eq!(resolved.outcome(), LocalizationOutcome::SourceFallback);
        assert_eq!(resolved.as_str(), "Save");

        let missing = TextKey::without_source_fallback("missing");
        assert_eq!(
            TextCatalog::default()
                .resolve(&missing, &[LocaleId::english()])
                .outcome(),
            LocalizationOutcome::Missing
        );
    }

    #[test]
    fn missing_diagnostics_are_deduplicated_by_generation_locale_and_key() {
        let key = TextKey::without_source_fallback("missing");
        let catalog = TextCatalog::default().with_generation(9);
        let resolved = catalog.resolve(&key, &[LocaleId::english()]);
        let mut diagnostics = super::LocalizationDiagnostics::default();
        diagnostics.record(&key, &resolved);
        diagnostics.record(&key, &resolved);

        assert_eq!(diagnostics.entries().len(), 1);
        assert_eq!(diagnostics.entries()[0].catalog_generation(), 9);
        assert_eq!(diagnostics.entries()[0].key(), "missing");
    }
}

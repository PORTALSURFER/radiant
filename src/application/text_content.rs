//! Static and shared text accepted by application-facing builders.

use crate::runtime::PaintText;
use std::{fmt, ops::Deref, sync::Arc};

/// Immutable display text for Radiant application builders.
///
/// String literals remain allocation-free through projection. Owned strings
/// move into shared storage once, and an existing [`Arc<str>`] keeps its shared
/// allocation. Editable text-input values intentionally continue to use owned
/// `String` state instead of this immutable display payload.
#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct TextContent(PaintText);

impl TextContent {
    /// Return the display text as a borrowed string slice.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// Return true when this content has no bytes.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Return true when this content preserves process-lifetime static storage.
    pub const fn is_static(&self) -> bool {
        self.0.is_static()
    }

    pub(crate) fn into_paint_text(self) -> PaintText {
        self.0
    }
}

impl From<&'static str> for TextContent {
    fn from(value: &'static str) -> Self {
        Self(PaintText::from_static(value))
    }
}

impl From<String> for TextContent {
    fn from(value: String) -> Self {
        Self(PaintText::from(value))
    }
}

impl From<&String> for TextContent {
    fn from(value: &String) -> Self {
        Self(PaintText::from(value))
    }
}

impl From<Arc<str>> for TextContent {
    fn from(value: Arc<str>) -> Self {
        Self(PaintText::from(value))
    }
}

impl AsRef<str> for TextContent {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Deref for TextContent {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl fmt::Display for TextContent {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl PartialEq<&str> for TextContent {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl PartialEq<TextContent> for &str {
    fn eq(&self, other: &TextContent) -> bool {
        *self == other.as_str()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn literals_owned_strings_and_arcs_keep_their_intended_storage() {
        fn assert_send_sync<T: Send + Sync>() {}

        assert_send_sync::<TextContent>();
        let literal = TextContent::from("Ready");
        let owned = TextContent::from(String::from("Owned"));
        let shared: Arc<str> = Arc::from("Shared");
        let shared_content = TextContent::from(Arc::clone(&shared));

        assert!(literal.is_static());
        assert!(!owned.is_static());
        assert!(!shared_content.is_static());
        assert_eq!(literal, "Ready");
        assert_eq!(owned.as_str(), "Owned");
        assert_eq!(shared_content.as_str(), shared.as_ref());
        assert!(shared_content.0.shares_storage_with(&shared));
    }
}

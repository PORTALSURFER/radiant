use std::{fmt, path::PathBuf, sync::Arc};

/// Native text/font policy used by backend runtime adapters.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NativeTextOptions {
    /// Host-provided font bytes checked before file and native fallback fonts.
    pub embedded_fonts: Vec<EmbeddedFont>,
    /// Host-preferred font files checked before environment and system fallbacks.
    pub font_paths: Vec<PathBuf>,
}

impl NativeTextOptions {
    /// Add embedded TTF/OTF font bytes checked before file and native fallback fonts.
    pub fn embedded_font(mut self, font: impl Into<EmbeddedFont>) -> Self {
        self.embedded_fonts.push(font.into());
        self
    }

    /// Add a preferred font file checked after embedded fonts and before native fallbacks.
    pub fn font_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.font_paths.push(path.into());
        self
    }
}

/// Application-owned font bytes that can be bundled into the executable.
///
/// This is intended for `include_bytes!(...)` style packaging where the
/// application should not depend on an installed font file at runtime.
#[derive(Clone, PartialEq, Eq)]
pub struct EmbeddedFont {
    bytes: Arc<[u8]>,
    index: u32,
}

impl EmbeddedFont {
    /// Create an embedded font from static bytes, typically `include_bytes!`.
    pub fn from_static(bytes: &'static [u8]) -> Self {
        Self::from_bytes(bytes)
    }

    /// Create an embedded font from owned bytes.
    pub fn from_bytes(bytes: impl AsRef<[u8]>) -> Self {
        Self {
            bytes: Arc::from(bytes.as_ref()),
            index: 0,
        }
    }

    /// Set the font index for collection files.
    pub fn with_index(mut self, index: u32) -> Self {
        self.index = index;
        self
    }

    /// Borrow the embedded font bytes.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub(crate) fn shared_bytes(&self) -> Arc<[u8]> {
        Arc::clone(&self.bytes)
    }

    /// Return the font index used for this embedded font.
    pub const fn index(&self) -> u32 {
        self.index
    }
}

impl From<&'static [u8]> for EmbeddedFont {
    fn from(bytes: &'static [u8]) -> Self {
        Self::from_static(bytes)
    }
}

impl From<Vec<u8>> for EmbeddedFont {
    fn from(bytes: Vec<u8>) -> Self {
        Self {
            bytes: Arc::from(bytes),
            index: 0,
        }
    }
}

impl fmt::Debug for EmbeddedFont {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("EmbeddedFont")
            .field("len", &self.bytes.len())
            .field("index", &self.index)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::EmbeddedFont;
    use std::sync::Arc;

    #[test]
    fn embedded_font_shared_bytes_reuses_storage() {
        let font = EmbeddedFont::from_static(b"font bytes");
        let first = font.shared_bytes();
        let second = font.shared_bytes();

        assert!(Arc::ptr_eq(&first, &second));
    }
}

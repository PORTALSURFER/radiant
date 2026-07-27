use std::{fmt, path::PathBuf, sync::Arc};

/// Native text/font policy used by backend runtime adapters.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NativeTextOptions {
    /// Ordered host-provided font faces in the per-glyph fallback stack.
    pub embedded_fonts: Vec<EmbeddedFont>,
    /// Ordered host-preferred files used after embedded faces and before
    /// environment/system candidates; the first valid path may seed an empty
    /// stack and later paths are loaded lazily in the fallback stack.
    pub font_paths: Vec<PathBuf>,
}

impl NativeTextOptions {
    /// Append embedded TTF/OTF bytes to the per-glyph fallback stack.
    pub fn embedded_font(mut self, font: impl Into<EmbeddedFont>) -> Self {
        self.embedded_fonts.push(font.into());
        self
    }

    /// Append a preferred font file, loaded lazily after embedded faces.
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

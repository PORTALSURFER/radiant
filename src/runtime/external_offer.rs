//! Checked owned representations for untrusted incoming external offers.
//!
//! This module only bounds and retains portable offer data. It does not read
//! files, canonicalize paths, parse URLs, decode MIME data, or approve a MIME
//! type for application use.

use std::{
    error::Error,
    fmt,
    path::{Path, PathBuf},
};

/// Maximum number of file paths or URLs in one external offer.
pub const MAX_EXTERNAL_OFFER_ITEMS: usize = 128;
/// Maximum encoded bytes in an individual file path or URL.
pub const MAX_EXTERNAL_OFFER_ITEM_BYTES: usize = 64 * 1024;
/// Maximum UTF-8 bytes in a text offer.
pub const MAX_EXTERNAL_OFFER_TEXT_BYTES: usize = 1024 * 1024;
/// Maximum bytes in a MIME offer body.
pub const MAX_EXTERNAL_OFFER_MIME_BYTES: usize = 8 * 1024 * 1024;
/// Maximum retained payload bytes in one external offer.
pub const MAX_EXTERNAL_OFFER_TOTAL_BYTES: usize = 8 * 1024 * 1024;
/// Maximum ASCII bytes in a MIME type name.
pub const MAX_EXTERNAL_OFFER_MIME_NAME_BYTES: usize = 127;

/// Untrusted, owned data supplied by a native external-offer adapter.
///
/// This type intentionally has no bounds. Convert it with
/// [`OwnedExternalOffer::try_new`] before admitting it to the runtime
/// ingress path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExternalOfferData {
    /// Files offered by the platform. Paths remain untrusted and are not
    /// canonicalized or read here.
    Files(Vec<PathBuf>),
    /// URL strings offered by the platform. They remain unparsed and untrusted.
    Urls(Vec<String>),
    /// UTF-8 text offered by the platform.
    Text(String),
    /// Arbitrary bytes with a syntactically valid MIME type name.
    ///
    /// A valid name is not approval to decode or consume that MIME type.
    Mime {
        /// Untrusted ASCII MIME type name, without parameters.
        name: String,
        /// Owned untrusted representation bytes.
        bytes: Vec<u8>,
    },
}

/// Portable representation kind retained by an owned external offer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExternalOfferKind {
    /// Filesystem paths.
    Files,
    /// URL strings.
    Urls,
    /// UTF-8 text.
    Text,
    /// Named MIME bytes.
    Mime,
}

/// Small data-free summary available for fast offer-acceptance decisions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExternalOfferMetadata {
    kind: ExternalOfferKind,
    item_count: usize,
    bytes: usize,
    mime_type: Option<String>,
}

impl ExternalOfferMetadata {
    /// Return the portable representation kind.
    pub const fn kind(&self) -> ExternalOfferKind {
        self.kind
    }

    /// Return the number of paths, URLs, or single scalar representation.
    pub const fn item_count(&self) -> usize {
        self.item_count
    }

    /// Return retained payload bytes, excluding MIME type-name metadata.
    pub const fn bytes(&self) -> usize {
        self.bytes
    }

    /// Return the MIME type name for MIME data.
    ///
    /// The presence of a syntactically valid name does not approve that type
    /// for decoding or application use.
    pub fn mime_type(&self) -> Option<&str> {
        self.mime_type.as_deref()
    }
}

/// Rejection returned while bounding an untrusted external offer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExternalOfferValidationError {
    /// A file or URL collection had no entries.
    EmptyCollection,
    /// A file or URL collection exceeded [`MAX_EXTERNAL_OFFER_ITEMS`].
    TooManyItems,
    /// A file path or URL was empty or contained an embedded NUL byte.
    InvalidItem,
    /// A file path or URL exceeded [`MAX_EXTERNAL_OFFER_ITEM_BYTES`].
    ItemTooLarge,
    /// Text exceeded [`MAX_EXTERNAL_OFFER_TEXT_BYTES`].
    TextTooLarge,
    /// MIME bytes exceeded [`MAX_EXTERNAL_OFFER_MIME_BYTES`].
    MimeBytesTooLarge,
    /// The MIME type name exceeded [`MAX_EXTERNAL_OFFER_MIME_NAME_BYTES`].
    MimeNameTooLarge,
    /// The MIME type name was not an ASCII `type/subtype` token pair.
    InvalidMimeName,
    /// Retained payload bytes exceeded [`MAX_EXTERNAL_OFFER_TOTAL_BYTES`].
    TotalTooLarge,
}

impl fmt::Display for ExternalOfferValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::EmptyCollection => "external offer collection is empty",
            Self::TooManyItems => "external offer has too many items",
            Self::InvalidItem => "external offer contains an invalid item",
            Self::ItemTooLarge => "external offer item is too large",
            Self::TextTooLarge => "external offer text is too large",
            Self::MimeBytesTooLarge => "external offer MIME bytes are too large",
            Self::MimeNameTooLarge => "external offer MIME type name is too large",
            Self::InvalidMimeName => "external offer MIME type name is invalid",
            Self::TotalTooLarge => "external offer total is too large",
        };
        f.write_str(message)
    }
}

impl Error for ExternalOfferValidationError {}

/// Validated, immutable, owned data that may cross an external-offer ingress.
///
/// Access to the retained untrusted representation is borrow-only. Construction
/// performs no file I/O, URL parsing, network access, or content decoding.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OwnedExternalOffer {
    data: ExternalOfferData,
    metadata: ExternalOfferMetadata,
}

impl OwnedExternalOffer {
    /// Validate and retain one untrusted owned offer without cloning its payload.
    pub fn try_new(data: ExternalOfferData) -> Result<Self, ExternalOfferValidationError> {
        let metadata = validate(&data)?;
        Ok(Self { data, metadata })
    }

    /// Borrow the retained, still-untrusted offer data.
    pub const fn data(&self) -> &ExternalOfferData {
        &self.data
    }

    /// Borrow the data-free summary for quick acceptance decisions.
    pub const fn metadata(&self) -> &ExternalOfferMetadata {
        &self.metadata
    }
}

impl TryFrom<ExternalOfferData> for OwnedExternalOffer {
    type Error = ExternalOfferValidationError;

    fn try_from(data: ExternalOfferData) -> Result<Self, Self::Error> {
        Self::try_new(data)
    }
}

fn validate(
    data: &ExternalOfferData,
) -> Result<ExternalOfferMetadata, ExternalOfferValidationError> {
    match data {
        ExternalOfferData::Files(paths) => {
            let bytes = validate_items(paths.iter().map(PathBuf::as_path))?;
            Ok(metadata(ExternalOfferKind::Files, paths.len(), bytes, None))
        }
        ExternalOfferData::Urls(urls) => {
            let bytes = validate_items(urls.iter().map(String::as_str))?;
            Ok(metadata(ExternalOfferKind::Urls, urls.len(), bytes, None))
        }
        ExternalOfferData::Text(text) => {
            if text.len() > MAX_EXTERNAL_OFFER_TEXT_BYTES {
                return Err(ExternalOfferValidationError::TextTooLarge);
            }
            validate_total(text.len())?;
            Ok(metadata(ExternalOfferKind::Text, 1, text.len(), None))
        }
        ExternalOfferData::Mime { name, bytes } => {
            validate_mime_name(name)?;
            if bytes.len() > MAX_EXTERNAL_OFFER_MIME_BYTES {
                return Err(ExternalOfferValidationError::MimeBytesTooLarge);
            }
            validate_total(bytes.len())?;
            Ok(metadata(
                ExternalOfferKind::Mime,
                1,
                bytes.len(),
                Some(name.clone()),
            ))
        }
    }
}

fn metadata(
    kind: ExternalOfferKind,
    item_count: usize,
    bytes: usize,
    mime_type: Option<String>,
) -> ExternalOfferMetadata {
    ExternalOfferMetadata {
        kind,
        item_count,
        bytes,
        mime_type,
    }
}

fn validate_items<'a, Item>(
    items: impl Iterator<Item = Item>,
) -> Result<usize, ExternalOfferValidationError>
where
    Item: ExternalOfferItem<'a>,
{
    let mut count = 0_usize;
    let mut total = 0_usize;
    for item in items {
        count = count
            .checked_add(1)
            .ok_or(ExternalOfferValidationError::TooManyItems)?;
        if count > MAX_EXTERNAL_OFFER_ITEMS {
            return Err(ExternalOfferValidationError::TooManyItems);
        }
        let bytes = item.encoded_bytes();
        if bytes.len() > MAX_EXTERNAL_OFFER_ITEM_BYTES {
            return Err(ExternalOfferValidationError::ItemTooLarge);
        }
        if bytes.is_empty() || bytes.contains(&0) {
            return Err(ExternalOfferValidationError::InvalidItem);
        }
        total = total
            .checked_add(bytes.len())
            .ok_or(ExternalOfferValidationError::TotalTooLarge)?;
        validate_total(total)?;
    }
    if count == 0 {
        return Err(ExternalOfferValidationError::EmptyCollection);
    }
    Ok(total)
}

trait ExternalOfferItem<'a> {
    fn encoded_bytes(self) -> &'a [u8];
}

impl<'a> ExternalOfferItem<'a> for &'a Path {
    fn encoded_bytes(self) -> &'a [u8] {
        self.as_os_str().as_encoded_bytes()
    }
}

impl<'a> ExternalOfferItem<'a> for &'a str {
    fn encoded_bytes(self) -> &'a [u8] {
        self.as_bytes()
    }
}

fn validate_total(total: usize) -> Result<(), ExternalOfferValidationError> {
    (total <= MAX_EXTERNAL_OFFER_TOTAL_BYTES)
        .then_some(())
        .ok_or(ExternalOfferValidationError::TotalTooLarge)
}

pub(super) fn validate_mime_name(name: &str) -> Result<(), ExternalOfferValidationError> {
    if name.len() > MAX_EXTERNAL_OFFER_MIME_NAME_BYTES {
        return Err(ExternalOfferValidationError::MimeNameTooLarge);
    }
    let Some((type_, subtype)) = name.split_once('/') else {
        return Err(ExternalOfferValidationError::InvalidMimeName);
    };
    if type_.is_empty()
        || subtype.is_empty()
        || subtype.contains('/')
        || !type_.bytes().all(is_mime_token_byte)
        || !subtype.bytes().all(is_mime_token_byte)
    {
        return Err(ExternalOfferValidationError::InvalidMimeName);
    }
    Ok(())
}

fn is_mime_token_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric()
        || matches!(
            byte,
            b'!' | b'#'..=b'\'' | b'*' | b'+' | b'-' | b'.' | b'^' | b'_' | b'`' | b'|' | b'~'
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_send<T: Send>() {}

    #[test]
    fn checked_offer_is_send_and_retains_immutable_data() {
        assert_send::<OwnedExternalOffer>();
        let offer = OwnedExternalOffer::try_new(ExternalOfferData::Mime {
            name: String::from("application/octet-stream"),
            bytes: vec![1, 2, 3],
        })
        .unwrap();

        assert_eq!(offer.metadata().kind(), ExternalOfferKind::Mime);
        assert_eq!(offer.metadata().item_count(), 1);
        assert_eq!(offer.metadata().bytes(), 3);
        assert_eq!(
            offer.metadata().mime_type(),
            Some("application/octet-stream")
        );
        assert_eq!(
            offer.data(),
            &ExternalOfferData::Mime {
                name: String::from("application/octet-stream"),
                bytes: vec![1, 2, 3],
            }
        );
    }

    #[test]
    fn accepts_exact_text_and_mime_boundaries() {
        assert!(
            OwnedExternalOffer::try_new(ExternalOfferData::Text(
                "x".repeat(MAX_EXTERNAL_OFFER_TEXT_BYTES)
            ))
            .is_ok()
        );
        assert_eq!(
            OwnedExternalOffer::try_new(ExternalOfferData::Text(
                "x".repeat(MAX_EXTERNAL_OFFER_TEXT_BYTES + 1)
            )),
            Err(ExternalOfferValidationError::TextTooLarge)
        );
        assert!(
            OwnedExternalOffer::try_new(ExternalOfferData::Mime {
                name: String::from("application/octet-stream"),
                bytes: vec![0; MAX_EXTERNAL_OFFER_MIME_BYTES],
            })
            .is_ok()
        );
        assert_eq!(
            OwnedExternalOffer::try_new(ExternalOfferData::Mime {
                name: String::from("application/octet-stream"),
                bytes: vec![0; MAX_EXTERNAL_OFFER_MIME_BYTES + 1],
            }),
            Err(ExternalOfferValidationError::MimeBytesTooLarge)
        );
    }

    #[test]
    fn bounds_file_and_url_collections_before_total_overflow() {
        let item = "x".repeat(MAX_EXTERNAL_OFFER_ITEM_BYTES);
        let urls = vec![item.clone(); MAX_EXTERNAL_OFFER_ITEMS];
        let offer = OwnedExternalOffer::try_new(ExternalOfferData::Urls(urls)).unwrap();
        assert_eq!(offer.metadata().bytes(), MAX_EXTERNAL_OFFER_TOTAL_BYTES);

        assert_eq!(
            OwnedExternalOffer::try_new(ExternalOfferData::Urls(vec![
                String::from("x");
                MAX_EXTERNAL_OFFER_ITEMS + 1
            ])),
            Err(ExternalOfferValidationError::TooManyItems)
        );
        assert_eq!(
            OwnedExternalOffer::try_new(ExternalOfferData::Urls(vec![
                "x".repeat(MAX_EXTERNAL_OFFER_ITEM_BYTES + 1)
            ])),
            Err(ExternalOfferValidationError::ItemTooLarge)
        );
    }

    #[test]
    fn rejects_empty_and_nul_containing_collection_items() {
        assert_eq!(
            OwnedExternalOffer::try_new(ExternalOfferData::Files(vec![])),
            Err(ExternalOfferValidationError::EmptyCollection)
        );
        assert_eq!(
            OwnedExternalOffer::try_new(ExternalOfferData::Urls(vec![String::new()])),
            Err(ExternalOfferValidationError::InvalidItem)
        );
        assert_eq!(
            OwnedExternalOffer::try_new(ExternalOfferData::Urls(vec![String::from("a\0b")])),
            Err(ExternalOfferValidationError::InvalidItem)
        );
    }

    #[test]
    fn permits_empty_text_and_mime_bodies() {
        assert!(OwnedExternalOffer::try_new(ExternalOfferData::Text(String::new())).is_ok());
        assert!(
            OwnedExternalOffer::try_new(ExternalOfferData::Mime {
                name: String::from("text/plain"),
                bytes: vec![],
            })
            .is_ok()
        );
    }

    #[cfg(unix)]
    #[test]
    fn retains_non_utf8_paths_without_lossy_conversion() {
        use std::os::unix::ffi::OsStringExt;

        let path = PathBuf::from(std::ffi::OsString::from_vec(vec![b'/', 0xff]));
        let offer =
            OwnedExternalOffer::try_new(ExternalOfferData::Files(vec![path.clone()])).unwrap();
        assert_eq!(offer.data(), &ExternalOfferData::Files(vec![path]));
    }

    #[test]
    fn rejects_invalid_mime_names_without_approving_valid_names() {
        for name in [
            "text",
            "text/",
            "/plain",
            "text/plain; charset=utf-8",
            "text/pl ain",
            "text/pl\nain",
        ] {
            assert_eq!(
                OwnedExternalOffer::try_new(ExternalOfferData::Mime {
                    name: String::from(name),
                    bytes: vec![],
                }),
                Err(ExternalOfferValidationError::InvalidMimeName),
                "{name}"
            );
        }
        assert!(
            OwnedExternalOffer::try_new(ExternalOfferData::Mime {
                name: "a".repeat(MAX_EXTERNAL_OFFER_MIME_NAME_BYTES - 2) + "/b",
                bytes: vec![],
            })
            .is_ok()
        );
        assert_eq!(
            OwnedExternalOffer::try_new(ExternalOfferData::Mime {
                name: "a".repeat(MAX_EXTERNAL_OFFER_MIME_NAME_BYTES - 1) + "/b",
                bytes: vec![],
            }),
            Err(ExternalOfferValidationError::MimeNameTooLarge)
        );
    }
}

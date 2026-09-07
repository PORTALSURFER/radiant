//! Backend-neutral external drag-and-drop requests.

use super::{
    MAX_EXTERNAL_OFFER_ITEM_BYTES, MAX_EXTERNAL_OFFER_MIME_BYTES, MAX_EXTERNAL_OFFER_TEXT_BYTES,
};
use std::path::PathBuf;

/// External drag payload that a native backend can offer to other applications.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExternalDragPayload {
    /// One or more filesystem paths, offered as a platform file drop.
    Files(Vec<PathBuf>),
    /// UTF-8 text offered through the platform's standard text drag format.
    Text(String),
    /// One deliberately exported absolute URL offered through the platform's
    /// standard URL drag format.
    ///
    /// This variant represents exactly one URL. Callers that need to export
    /// several URLs must choose an explicit application representation rather
    /// than relying on platform-specific URL-list flattening.
    Url(String),
    /// Arbitrary bytes offered under one deliberately exported MIME type.
    ///
    /// The MIME name is syntactically bounded at launch, but its bytes remain
    /// opaque: exporting them does not parse or semantically validate content,
    /// or guarantee that a receiver supports the representation.
    Mime {
        /// MIME `type/subtype` name selected by the caller.
        name: String,
        /// Exact bytes selected by the caller, including an empty payload.
        bytes: Vec<u8>,
    },
}

/// Native drag image metadata.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExternalDragPreview {
    /// Human-readable label to show in the native drag preview.
    pub label: String,
}

impl ExternalDragPreview {
    /// Build a drag preview from a label.
    pub fn label(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
        }
    }
}

/// Request to begin a native external drag session.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExternalDragRequest {
    /// Payload made available to external drop targets.
    pub payload: ExternalDragPayload,
    /// Drag preview metadata used by native backends that support drag images.
    pub preview: ExternalDragPreview,
}

impl ExternalDragRequest {
    /// Build a file-drag request with a preview label.
    pub fn files(paths: impl IntoIterator<Item = PathBuf>, label: impl Into<String>) -> Self {
        Self {
            payload: ExternalDragPayload::Files(paths.into_iter().collect()),
            preview: ExternalDragPreview::label(label),
        }
    }

    /// Build a text-drag request with a preview label.
    pub fn text(text: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            payload: ExternalDragPayload::Text(text.into()),
            preview: ExternalDragPreview::label(label),
        }
    }

    /// Build a single-URL drag request with a preview label.
    ///
    /// The caller deliberately chooses this exported URL. It is validated when
    /// the native drag launches, including for direct [`ExternalDragPayload`]
    /// construction.
    pub fn url(url: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            payload: ExternalDragPayload::Url(url.into()),
            preview: ExternalDragPreview::label(label),
        }
    }

    /// Build an opaque MIME-drag request with a preview label.
    ///
    /// The caller deliberately authorizes export of these bytes. Their MIME
    /// name and size are validated when the native drag launches, including
    /// for direct [`ExternalDragPayload`] construction; that validation does
    /// not parse the content or guarantee receiver support.
    pub fn mime(
        name: impl Into<String>,
        bytes: impl Into<Vec<u8>>,
        label: impl Into<String>,
    ) -> Self {
        Self {
            payload: ExternalDragPayload::Mime {
                name: name.into(),
                bytes: bytes.into(),
            },
            preview: ExternalDragPreview::label(label),
        }
    }

    /// Validate a payload before a native backend starts an external drag.
    ///
    /// This is intentionally performed at launch, rather than only by the
    /// convenience constructors, because callers may construct the public
    /// payload enum directly.
    pub(crate) fn validate_for_native_launch(&self) -> Result<(), String> {
        match &self.payload {
            ExternalDragPayload::Files(_) => Ok(()),
            ExternalDragPayload::Text(text) => validate_external_drag_text(text),
            ExternalDragPayload::Url(url) => validate_external_drag_url(url),
            ExternalDragPayload::Mime { name, bytes } => validate_external_drag_mime(name, bytes),
        }
    }
}

fn validate_external_drag_text(text: &str) -> Result<(), String> {
    if text.len() > MAX_EXTERNAL_OFFER_TEXT_BYTES {
        return Err(format!(
            "External drag text exceeds the {MAX_EXTERNAL_OFFER_TEXT_BYTES}-byte limit"
        ));
    }
    if text.contains('\0') {
        return Err(String::from(
            "External drag text contains an embedded NUL byte",
        ));
    }
    Ok(())
}

fn validate_external_drag_url(url: &str) -> Result<(), String> {
    if url.len() > MAX_EXTERNAL_OFFER_ITEM_BYTES {
        return Err(format!(
            "External drag URL exceeds the {MAX_EXTERNAL_OFFER_ITEM_BYTES}-byte limit"
        ));
    }
    if url.is_empty() {
        return Err(String::from("External drag URL is empty"));
    }
    if url
        .chars()
        .any(|character| character.is_whitespace() || character.is_control())
    {
        return Err(String::from(
            "External drag URL contains whitespace or a control character",
        ));
    }

    let Some(colon) = url.find(':') else {
        return Err(String::from("External drag URL has no absolute URI scheme"));
    };
    let scheme = &url[..colon];
    let remainder = &url[colon + 1..];
    if scheme.is_empty()
        || !scheme.as_bytes()[0].is_ascii_alphabetic()
        || !scheme
            .bytes()
            .skip(1)
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'.' | b'-'))
        || remainder.is_empty()
    {
        return Err(String::from(
            "External drag URL must be an absolute URI with a nonempty remainder",
        ));
    }
    Ok(())
}

fn validate_external_drag_mime(name: &str, bytes: &[u8]) -> Result<(), String> {
    super::external_offer::validate_mime_name(name)
        .map_err(|error| format!("External drag MIME type is invalid: {error}"))?;
    if bytes.len() > MAX_EXTERNAL_OFFER_MIME_BYTES {
        return Err(format!(
            "External drag MIME bytes exceed the {MAX_EXTERNAL_OFFER_MIME_BYTES}-byte limit"
        ));
    }
    Ok(())
}

/// Native drop effect reported by the platform after an external drag.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ExternalDragEffect {
    /// The drag was cancelled or rejected by the target.
    #[default]
    None,
    /// The target copied the payload.
    Copy,
    /// The target moved the payload.
    Move,
    /// The target linked the payload.
    Link,
}

/// Result returned after the native external drag loop finishes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ExternalDragOutcome {
    /// Drop effect chosen by the external target.
    pub effect: ExternalDragEffect,
}

impl ExternalDragOutcome {
    /// Return whether an external target accepted the drag.
    pub const fn accepted(self) -> bool {
        !matches!(self.effect, ExternalDragEffect::None)
    }
}

pub(crate) type ExternalDragCompletion<Message> =
    Box<dyn FnOnce(Result<ExternalDragOutcome, String>) -> Message + 'static>;

/// Identity fencing an external drag completion to its originating session.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ExternalDragIdentity {
    pub(crate) id: u64,
    pub(crate) epoch: u64,
}

/// Active external drag session owned by the runtime until it is launched or cancelled.
pub(crate) struct ExternalDragSession<Message> {
    /// Request to launch when native drag-out begins.
    pub(crate) request: ExternalDragRequest,
    /// Optional mapper used to notify the host when the native drag loop finishes.
    pub(crate) on_completed: Option<ExternalDragCompletion<Message>>,
    /// Session identity used to reject late native completions.
    pub(crate) identity: ExternalDragIdentity,
}

/// Native-facing external drag launch data. UI-owned completion mappers never
/// cross this boundary.
pub(crate) struct ExternalDragLaunch {
    pub(crate) request: ExternalDragRequest,
    pub(crate) identity: ExternalDragIdentity,
}

impl<Message> ExternalDragSession<Message> {
    /// Build one active external drag session.
    pub(crate) fn new(
        request: ExternalDragRequest,
        on_completed: Option<ExternalDragCompletion<Message>>,
        identity: ExternalDragIdentity,
    ) -> Self {
        Self {
            request,
            on_completed,
            identity,
        }
    }
}

/// A native completion waiting for the next controller-owned drain pass.
pub(crate) struct PendingExternalDragCompletion<Message> {
    pub(crate) identity: ExternalDragIdentity,
    pub(crate) on_completed: ExternalDragCompletion<Message>,
    pub(crate) result: Result<ExternalDragOutcome, String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_request_preserves_its_text_and_preview_label() {
        let request = ExternalDragRequest::text("hello", "Greeting");

        assert_eq!(request.preview.label, "Greeting");
        assert_eq!(
            request.payload,
            ExternalDragPayload::Text(String::from("hello"))
        );
        assert!(request.validate_for_native_launch().is_ok());
    }

    #[test]
    fn direct_text_payloads_are_checked_at_native_launch() {
        let invalid_nul = ExternalDragRequest {
            payload: ExternalDragPayload::Text(String::from("before\0after")),
            preview: ExternalDragPreview::label("text"),
        };
        let oversized = ExternalDragRequest {
            payload: ExternalDragPayload::Text("x".repeat(MAX_EXTERNAL_OFFER_TEXT_BYTES + 1)),
            preview: ExternalDragPreview::label("text"),
        };

        assert!(invalid_nul.validate_for_native_launch().is_err());
        assert!(oversized.validate_for_native_launch().is_err());
        assert!(
            ExternalDragRequest::text("", "empty")
                .validate_for_native_launch()
                .is_ok()
        );
    }

    #[test]
    fn url_request_preserves_its_url_and_preview_label() {
        let request = ExternalDragRequest::url("https://example.test/drag", "Example");

        assert_eq!(request.preview.label, "Example");
        assert_eq!(
            request.payload,
            ExternalDragPayload::Url(String::from("https://example.test/drag"))
        );
        assert!(request.validate_for_native_launch().is_ok());
    }

    #[test]
    fn direct_url_payloads_are_checked_at_native_launch() {
        let invalid = [
            String::new(),
            String::from("relative/path"),
            String::from("1http://example.test"),
            String::from("ht*tp://example.test"),
            String::from("https:"),
            String::from("http://example.test/with space"),
            String::from("http://example.test/before\0after"),
            String::from("http://example.test/before\nafter"),
            format!("x:{}", "a".repeat(MAX_EXTERNAL_OFFER_ITEM_BYTES)),
        ];

        for url in invalid {
            let request = ExternalDragRequest {
                payload: ExternalDragPayload::Url(url),
                preview: ExternalDragPreview::label("URL"),
            };
            assert!(request.validate_for_native_launch().is_err());
        }

        let maximum_url = format!("x:{}", "a".repeat(MAX_EXTERNAL_OFFER_ITEM_BYTES - 2));
        for url in [
            "https://example.test/path?item=one",
            "custom+scheme.v1:opaque-value",
            "mailto:person@example.test",
            &maximum_url,
        ] {
            assert!(
                ExternalDragRequest::url(url, "URL")
                    .validate_for_native_launch()
                    .is_ok()
            );
        }
    }

    #[test]
    fn mime_request_preserves_exact_bytes_and_preview_label() {
        let request = ExternalDragRequest::mime("Application/X-Radiant", [0, 1, 255], "Preset");

        assert_eq!(request.preview.label, "Preset");
        assert_eq!(
            request.payload,
            ExternalDragPayload::Mime {
                name: String::from("Application/X-Radiant"),
                bytes: vec![0, 1, 255],
            }
        );
        assert!(request.validate_for_native_launch().is_ok());
    }

    #[test]
    fn direct_mime_payloads_are_checked_at_native_launch() {
        for (name, bytes) in [
            (String::from("text"), Vec::new()),
            (String::from("text/plain; charset=utf-8"), Vec::new()),
            (String::from("text/pla in"), Vec::new()),
            (
                String::from("text/plain"),
                vec![0; MAX_EXTERNAL_OFFER_MIME_BYTES + 1],
            ),
        ] {
            let request = ExternalDragRequest {
                payload: ExternalDragPayload::Mime { name, bytes },
                preview: ExternalDragPreview::label("MIME"),
            };
            assert!(request.validate_for_native_launch().is_err());
        }

        for bytes in [
            Vec::new(),
            vec![0, 1, 255],
            vec![0; MAX_EXTERNAL_OFFER_MIME_BYTES],
        ] {
            assert!(
                ExternalDragRequest::mime("application/x-radiant", bytes, "MIME")
                    .validate_for_native_launch()
                    .is_ok()
            );
        }
    }
}

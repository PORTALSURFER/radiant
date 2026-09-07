//! Backend-neutral external drag-and-drop requests.

use super::MAX_EXTERNAL_OFFER_TEXT_BYTES;
use std::path::PathBuf;

/// External drag payload that a native backend can offer to other applications.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExternalDragPayload {
    /// One or more filesystem paths, offered as a platform file drop.
    Files(Vec<PathBuf>),
    /// UTF-8 text offered through the platform's standard text drag format.
    Text(String),
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

    /// Validate a payload before a native backend starts an external drag.
    ///
    /// This is intentionally performed at launch, rather than only by the
    /// convenience constructors, because callers may construct the public
    /// payload enum directly.
    pub(crate) fn validate_for_native_launch(&self) -> Result<(), String> {
        match &self.payload {
            ExternalDragPayload::Files(_) => Ok(()),
            ExternalDragPayload::Text(text) => validate_external_drag_text(text),
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
}

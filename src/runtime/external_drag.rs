//! Backend-neutral external drag-and-drop requests.

use std::path::PathBuf;

/// External drag payload that a native backend can offer to other applications.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExternalDragPayload {
    /// One or more filesystem paths, offered as a platform file drop.
    Files(Vec<PathBuf>),
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
    Box<dyn FnOnce(Result<ExternalDragOutcome, String>) -> Message + Send + 'static>;

/// Active external drag session owned by the runtime until it is launched or cancelled.
pub(crate) struct ExternalDragSession<Message> {
    /// Request to launch when native drag-out begins.
    pub(crate) request: ExternalDragRequest,
    /// Optional mapper used to notify the host when the native drag loop finishes.
    pub(crate) on_completed: Option<ExternalDragCompletion<Message>>,
}

impl<Message> ExternalDragSession<Message> {
    /// Build one active external drag session.
    pub(crate) fn new(
        request: ExternalDragRequest,
        on_completed: Option<ExternalDragCompletion<Message>>,
    ) -> Self {
        Self {
            request,
            on_completed,
        }
    }
}

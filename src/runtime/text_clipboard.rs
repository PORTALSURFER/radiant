//! Typed, revocable clipboard admission for builtin text controls.

use super::{MAX_PLATFORM_TEXT_BYTES, PlatformRequest};
use crate::widgets::{
    TextEditAuthority, TextEditorSnapshot, TextInputState, TextPrivacy, WidgetId,
};

/// A clipboard operation admitted against the currently focused text control.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextClipboardOperation {
    /// Copy the current selection without changing the value.
    Copy,
    /// Copy the current selection, deleting it only after a successful write.
    Cut,
    /// Read text and replace the same still-current selection.
    Paste,
}

pub(crate) enum TextClipboardSnapshot {
    SingleLine(TextInputState),
    Multiline(TextEditorSnapshot),
}

/// Opaque exact-state receipt for deferred text clipboard work.
///
/// Builtin text controls construct these receipts. They retain bounded input
/// state for exact comparison, never keep a document owner alive, and exclude
/// text payloads from diagnostic output.
pub struct TextClipboardReceipt {
    pub(crate) widget: WidgetId,
    pub(crate) operation: TextClipboardOperation,
    pub(crate) privacy: TextPrivacy,
    pub(crate) authority: TextEditAuthority,
    pub(crate) snapshot: TextClipboardSnapshot,
    copied_text: Option<String>,
}

impl std::fmt::Debug for TextClipboardReceipt {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("TextClipboardReceipt")
            .field("widget", &self.widget)
            .field("operation", &self.operation)
            .field("privacy", &self.privacy)
            .field("authority", &self.authority)
            .finish_non_exhaustive()
    }
}

impl TextClipboardReceipt {
    pub(crate) fn new(
        widget: WidgetId,
        operation: TextClipboardOperation,
        privacy: TextPrivacy,
        authority: TextEditAuthority,
        snapshot: TextClipboardSnapshot,
        selection: Option<&str>,
    ) -> Option<Self> {
        let copied_text = match operation {
            TextClipboardOperation::Paste => None,
            TextClipboardOperation::Copy | TextClipboardOperation::Cut => {
                if matches!(privacy, TextPrivacy::Secret(policy) if !policy.copy_allowed()) {
                    return None;
                }
                let selected = selection
                    .filter(|text| !text.is_empty() && text.len() <= MAX_PLATFORM_TEXT_BYTES)?;
                Some(selected.to_owned())
            }
        };
        Some(Self {
            widget,
            operation,
            privacy,
            authority,
            snapshot,
            copied_text,
        })
    }

    pub(crate) fn cancellation_probe(&self) -> std::sync::Arc<dyn Fn() -> bool + Send + Sync> {
        let authority = self.authority.clone();
        let document = match &self.snapshot {
            TextClipboardSnapshot::SingleLine(_) => None,
            TextClipboardSnapshot::Multiline(snapshot) => {
                Some(snapshot.document_cancellation_probe())
            }
        };
        std::sync::Arc::new(move || {
            authority.is_cancelled() || document.as_ref().is_some_and(|probe| probe())
        })
    }

    pub(crate) fn request(&self) -> PlatformRequest {
        match self.operation {
            TextClipboardOperation::Paste => PlatformRequest::ReadText,
            TextClipboardOperation::Copy | TextClipboardOperation::Cut => {
                PlatformRequest::CopyText(self.copied_text.clone().unwrap_or_default())
            }
        }
    }
}

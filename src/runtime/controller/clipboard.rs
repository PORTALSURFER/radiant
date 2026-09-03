//! UI-local in-process clipboard coordination.

use crate::runtime::{
    ClipboardIdentity, ClipboardValue, PlatformFailure, PlatformRequest, PlatformResponse,
    PlatformResult,
};

/// One app-instance-owned typed clipboard slot.
///
/// This state is intentionally controller-local. It is never included in a
/// `RuntimePlatformResultSink` request sent to an adapter, and it is cleared
/// only when the owning runtime begins shutdown.
#[derive(Default)]
pub(super) struct InProcessClipboard {
    slot: Option<ClipboardSlot>,
    next_generation: u64,
}

struct ClipboardSlot {
    identity: ClipboardIdentity,
    value: ClipboardValue,
}

impl InProcessClipboard {
    pub(super) fn execute(&mut self, request: &PlatformRequest) -> PlatformResult {
        match request {
            PlatformRequest::WriteClipboard(value) => {
                value.validate()?;
                self.next_generation = self.next_generation.saturating_add(1).max(1);
                self.slot = Some(ClipboardSlot {
                    identity: ClipboardIdentity::new(self.next_generation),
                    value: value.clone(),
                });
                Ok(PlatformResponse::Completed)
            }
            PlatformRequest::ReadClipboard(format) => {
                let Some(slot) = self.slot.as_ref() else {
                    return Err(PlatformFailure::ClipboardEmpty);
                };
                if slot.value.format() != *format {
                    return Err(PlatformFailure::ClipboardTypeMismatch {
                        requested: *format,
                        available: slot.value.format(),
                    });
                }
                // Reading the identity keeps generation replacement explicit
                // in the coordinator without exposing it to adapter payloads.
                let _identity = slot.identity;
                Ok(PlatformResponse::Clipboard(slot.value.clone()))
            }
            _ => Err(PlatformFailure::InvalidRequest),
        }
    }

    pub(super) fn clear(&mut self) {
        self.slot = None;
    }

    #[cfg(test)]
    pub(super) fn has_value(&self) -> bool {
        self.slot.is_some()
    }

    #[cfg(test)]
    pub(super) fn current_identity(&self) -> Option<ClipboardIdentity> {
        self.slot.as_ref().map(|slot| slot.identity)
    }

    #[cfg(test)]
    pub(super) fn current_format(&self) -> Option<crate::runtime::ClipboardFormat> {
        self.slot.as_ref().map(|slot| slot.value.format())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn clipboard_replaces_by_generation_and_clears_only_on_shutdown() {
        let mut clipboard = InProcessClipboard::default();
        let text = ClipboardValue::text("hello").expect("bounded clipboard text");
        assert_eq!(
            clipboard.execute(&PlatformRequest::WriteClipboard(text.clone())),
            Ok(PlatformResponse::Completed)
        );
        let first_identity = clipboard.current_identity();
        assert!(clipboard.has_value());
        assert_eq!(
            clipboard.current_format(),
            Some(crate::runtime::ClipboardFormat::Text)
        );
        assert_eq!(
            clipboard.execute(&PlatformRequest::ReadClipboard(
                crate::runtime::ClipboardFormat::FilePaths,
            )),
            Err(PlatformFailure::ClipboardTypeMismatch {
                requested: crate::runtime::ClipboardFormat::FilePaths,
                available: crate::runtime::ClipboardFormat::Text,
            })
        );

        let files = ClipboardValue::file_paths(vec![PathBuf::from("/tmp/one")])
            .expect("bounded clipboard paths");
        assert_eq!(
            clipboard.execute(&PlatformRequest::WriteClipboard(files.clone())),
            Ok(PlatformResponse::Completed)
        );
        assert_ne!(clipboard.current_identity(), first_identity);
        assert_eq!(
            clipboard.current_format(),
            Some(crate::runtime::ClipboardFormat::FilePaths)
        );
        assert_eq!(
            clipboard.execute(&PlatformRequest::ReadClipboard(
                crate::runtime::ClipboardFormat::FilePaths,
            )),
            Ok(PlatformResponse::Clipboard(files))
        );

        clipboard.clear();
        assert!(!clipboard.has_value());
        assert_eq!(
            clipboard.execute(&PlatformRequest::ReadClipboard(
                crate::runtime::ClipboardFormat::FilePaths,
            )),
            Err(PlatformFailure::ClipboardEmpty)
        );
    }

    #[test]
    fn clipboard_value_bounds_are_rejected_before_slot_replacement() {
        let too_long = "x".repeat(crate::runtime::MAX_CLIPBOARD_TEXT_BYTES + 1);
        assert_eq!(
            ClipboardValue::text(too_long),
            Err(crate::runtime::ClipboardValueError::TextTooLarge)
        );
        assert_eq!(
            ClipboardValue::file_paths(Vec::new()),
            Err(crate::runtime::ClipboardValueError::EmptyPaths)
        );
    }
}

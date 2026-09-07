//! Declarative external-offer decoding on the existing owned worker lane.

use super::{
    Command, ExternalOfferKind, ExternalOfferMetadata, ExternalOfferValidationError,
    OwnedExternalOffer, TaskPriority,
};
use crate::application::{CancellationToken, DeclarativeEffectOwner};
use std::{rc::Rc, sync::Arc};

/// Immediate admission outcome for one owned external drop.
///
/// Acceptance reports worker admission, not successful content decoding. The
/// decoder's result is delivered on a later UI turn while the owner is live.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExternalOfferAdmission {
    /// The host accepted an independent, owner-qualified decoder worker.
    Accepted,
    /// A target rejected the format, owner qualification, or worker admission.
    Rejected,
    /// No eligible target exists at the supplied position.
    NoTarget,
}

/// One explicitly approved representation for an external drop target.
///
/// Approval only selects a decoder. Offer contents remain untrusted; semantic
/// validation and any file or network I/O belong inside that worker decoder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExternalOfferFormat {
    kind: ExternalOfferKind,
    mime: Option<String>,
}

impl ExternalOfferFormat {
    /// Approve bounded filesystem path offers.
    pub const fn files() -> Self {
        Self {
            kind: ExternalOfferKind::Files,
            mime: None,
        }
    }

    /// Approve bounded, still-unparsed URL offers.
    pub const fn urls() -> Self {
        Self {
            kind: ExternalOfferKind::Urls,
            mime: None,
        }
    }

    /// Approve bounded UTF-8 text offers.
    pub const fn text() -> Self {
        Self {
            kind: ExternalOfferKind::Text,
            mime: None,
        }
    }

    /// Explicitly approve one MIME type, without parameters or wildcards.
    /// Matching is ASCII case-insensitive. Approval does not validate contents.
    pub fn approved_mime(name: &str) -> Result<Self, ExternalOfferValidationError> {
        super::external_offer::validate_mime_name(name)?;
        if name.contains('*') {
            return Err(ExternalOfferValidationError::InvalidMimeName);
        }
        Ok(Self {
            kind: ExternalOfferKind::Mime,
            mime: Some(name.to_ascii_lowercase()),
        })
    }

    /// Return whether the bounded metadata matches this approved format.
    pub fn accepts(&self, metadata: &ExternalOfferMetadata) -> bool {
        self.kind == metadata.kind()
            && match self.mime.as_deref() {
                Some(expected) => metadata
                    .mime_type()
                    .is_some_and(|actual| actual.eq_ignore_ascii_case(expected)),
                None => true,
            }
    }
}

type OfferCommand<Message> = dyn Fn(OwnedExternalOffer) -> Command<Message>;

/// Immutable UI-local external drop target with a worker-only decoder.
///
/// Attach this descriptor with [`crate::application::ViewNode::external_drop_target`]
/// and key the resulting wrapper. Only that exact live keyed owner's generation
/// can admit work or deliver its decoded result. Decoder output must be `Send`;
/// application messages and their mapping closure remain on the UI owner.
///
/// Each accepted offer is independent. Rebuilding the view changes future
/// admissions, while previously accepted work retains its decoder and mapper.
/// Removing or incompatibly replacing the owner suppresses late results.
pub struct ExternalDropTarget<Message> {
    owner: DeclarativeEffectOwner,
    format: ExternalOfferFormat,
    command: Rc<OfferCommand<Message>>,
}

impl<Message> Clone for ExternalDropTarget<Message> {
    fn clone(&self) -> Self {
        Self {
            owner: self.owner,
            format: self.format.clone(),
            command: Rc::clone(&self.command),
        }
    }
}

impl<Message: 'static> ExternalDropTarget<Message> {
    /// Build an external drop decoder and its UI-local completion mapper.
    ///
    /// `decode` receives only bounded owned data and runs on the worker lane.
    /// Return a `Result` as its output when decoding can fail; `map` then handles
    /// both success and failure through the normal application update path.
    pub fn new<Output: Send + 'static>(
        owner: DeclarativeEffectOwner,
        format: ExternalOfferFormat,
        decode: impl Fn(OwnedExternalOffer) -> Output + Send + Sync + 'static,
        map: impl Fn(Output) -> Message + 'static,
    ) -> Self {
        let decode = Arc::new(decode);
        let map = Rc::new(map);
        let command = Rc::new(move |offer| {
            let decode = Arc::clone(&decode);
            let map = Rc::clone(&map);
            let cancellation = CancellationToken::new();
            // Independent one-shots use the established owner/cancellation lane.
            // A mapper-owned LatestTask would invalidate its post-mapping fence
            // when dropped, incorrectly suppressing its own application message.
            Command::perform_worker_effect_with_priority_and_receipt_for_owner_with_options(
                owner,
                "external-offer-decode",
                TaskPriority::BlockingIo,
                Some(Box::new(move || cancellation.is_cancelled())),
                None,
                move |_| decode(offer),
                move |output| map(output),
            )
        });
        Self {
            owner,
            format,
            command,
        }
    }
}

impl<Message> ExternalDropTarget<Message> {
    /// Return the explicit keyed owner required to accept an offer.
    pub const fn owner(&self) -> DeclarativeEffectOwner {
        self.owner
    }

    /// Decide format acceptance from bounded metadata without calling user code.
    pub fn accepts(&self, metadata: &ExternalOfferMetadata) -> bool {
        self.format.accepts(metadata)
    }

    pub(crate) fn command(&self, offer: OwnedExternalOffer) -> Command<Message> {
        (self.command)(offer)
    }

    // Immutable attachment reuse evidence only, never source/owner authority.
    pub(crate) fn same_attachment(&self, other: &Self) -> bool {
        self.owner == other.owner
            && self.format == other.format
            && Rc::ptr_eq(&self.command, &other.command)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::ExternalOfferData;

    #[test]
    fn format_admission_is_explicit_and_mime_matching_is_bounded() {
        let text = OwnedExternalOffer::try_new(ExternalOfferData::Text("text".into())).unwrap();
        assert!(ExternalOfferFormat::text().accepts(text.metadata()));
        assert!(!ExternalOfferFormat::urls().accepts(text.metadata()));
        assert!(!ExternalOfferFormat::files().accepts(text.metadata()));
        let mime = OwnedExternalOffer::try_new(ExternalOfferData::Mime {
            name: "Application/Example".into(),
            bytes: vec![0, 255],
        })
        .unwrap();
        assert!(
            ExternalOfferFormat::approved_mime("application/example")
                .unwrap()
                .accepts(mime.metadata())
        );
        assert!(
            !ExternalOfferFormat::approved_mime("application/other")
                .unwrap()
                .accepts(mime.metadata())
        );
        assert!(!ExternalOfferFormat::text().accepts(mime.metadata()));
        for invalid in [
            "*/*",
            "application/*",
            "text/plain; charset=utf-8",
            "not-a-mime",
        ] {
            assert!(ExternalOfferFormat::approved_mime(invalid).is_err());
        }
        assert!(ExternalOfferFormat::approved_mime(&"x".repeat(128)).is_err());
    }
}

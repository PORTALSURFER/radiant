//! One-shot admission for bounded external offers.
//!
//! This stays separate from native backend extraction. Backends pass an
//! already validated [`OwnedExternalOffer`] to this ingress; the controller
//! selects an exact current target and submits its existing owned worker
//! command only after all UI-side authority checks pass.

use super::SurfaceRuntime;
use crate::{
    application::runtime::update_context::business::admission::AdmissionReceiptGuard,
    application::runtime::{BusinessTaskAdmission, BusinessTaskAdmissionReceipt},
    gui::types::Point,
    runtime::{
        Command, ExternalOfferAdmission, OwnedExternalOffer, RuntimeBridge, surface::WidgetPath,
    },
};

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
    Message: 'static,
{
    /// Admit one bounded, owned external offer at the current surface position.
    ///
    /// This does not invoke a decoder or mapper itself. Those closures remain
    /// inside the accepted owned worker command. Empty container background is
    /// deliberately unsupported in this slice: a normal hit-testable child
    /// must anchor the target at the supplied point.
    pub fn dispatch_external_offer(
        &mut self,
        position: Point,
        offer: OwnedExternalOffer,
    ) -> ExternalOfferAdmission {
        if !self.lifecycle_accepts_work()
            || !position.is_finite()
            || !self.viewport.contains(position)
        {
            return ExternalOfferAdmission::NoTarget;
        }

        let Some(anchor) = self
            .widget_at_for_input(
                position,
                &crate::widgets::WidgetInput::pointer_move(position),
            )
            .and_then(|id| {
                self.overlay_focus_allows(id)
                    .then(|| self.traversal.widgets.paths.current.get(&id).cloned())
                    .flatten()
                    .map(|path| (id, path))
            })
        else {
            return ExternalOfferAdmission::NoTarget;
        };

        let mut visited = 0usize;
        let mut selected = None;
        for record in &self.traversal.containers.external_drop_targets {
            if !self.external_target_contains(record.id, &record.path, position, &anchor.1) {
                continue;
            }
            visited += 1;
            if visited > 64 {
                return ExternalOfferAdmission::Rejected;
            }
            if selected.is_none_or(
                |current: &crate::runtime::surface::SurfaceExternalDropTargetRecord<Message>| {
                    record.path.as_slice().len() > current.path.as_slice().len()
                },
            ) {
                selected = Some(record);
            }
        }
        let Some(record) = selected else {
            return ExternalOfferAdmission::NoTarget;
        };

        if !record.target.accepts(offer.metadata())
            || !self.external_target_owner_is_current(record)
        {
            return ExternalOfferAdmission::Rejected;
        }

        let receipt = BusinessTaskAdmissionReceipt::new();
        let mut command = record.target.command(offer);
        let Command::PerformWorker(effect) = &mut command else {
            return ExternalOfferAdmission::Rejected;
        };
        effect.admission_receipt = Some(AdmissionReceiptGuard(receipt.weak()));
        let outcome = self.execute_command(command);
        self.pending_input_command_outcome.merge(outcome);
        match receipt.poll() {
            BusinessTaskAdmission::Accepted => ExternalOfferAdmission::Accepted,
            BusinessTaskAdmission::Pending
            | BusinessTaskAdmission::Rejected
            | BusinessTaskAdmission::Closed => ExternalOfferAdmission::Rejected,
        }
    }

    fn external_target_contains(
        &self,
        id: crate::layout::NodeId,
        path: &WidgetPath,
        position: Point,
        anchor_path: &WidgetPath,
    ) -> bool {
        anchor_path.as_slice().starts_with(path.as_slice())
            && self
                .surface
                .find_container_at_path(path)
                .is_some_and(|container| {
                    container.node_id() == id
                        && container.revision().layout_policy.is_none()
                        && self.container_contains_point(id, position)
                })
    }

    fn external_target_owner_is_current(
        &self,
        record: &crate::runtime::surface::SurfaceExternalDropTargetRecord<Message>,
    ) -> bool {
        let Some(container) = self.surface.find_container_at_path(&record.path) else {
            return false;
        };
        let Some(source) = container.source_metadata_handle() else {
            return false;
        };
        container.node_id() == record.id
            && container
                .external_drop_target()
                .is_some_and(|current| current.same_attachment(&record.target))
            && matches!(
                self.declarative_owner.resolve_handle_candidates(record.target.owner()),
                super::declarative_owner::DeclarativeOwnerCandidateOutcome::KeyedNode(candidate)
                    if candidate.identity == source.identity && source.identity.origin.is_keyed()
            )
            && self
                .declarative_owner_origin_for_handle(record.target.owner())
                .is_some()
    }
}

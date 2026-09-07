//! Deadline-owned foreign scrolling. Timer advancement only marks a receipt
//! due; this collector owns native re-hit, scroll reduction, and requalification.
use super::*;

impl<Bridge, Message> GenericNativeVelloRunner<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(in crate::gui_runtime::native_vello::generic_runtime) fn route_drag_autoscroll_for_owner(
        &mut self,
        owner: Option<&AuxiliaryWindowOwner>,
        now: Instant,
    ) -> NativeDragRoute {
        let receiver = match owner {
            Some(owner) => self
                .auxiliary_windows
                .iter()
                .find(|window| window.owner.is_same_generation(owner))
                .and_then(|window| window.window_id())
                .and_then(|id| self.drag_endpoint(id)),
            None => self.window.id.and_then(|id| self.drag_endpoint(id)),
        };
        let Some(receiver) = receiver else {
            return NativeDragRoute::default();
        };
        self.route_drag_autoscroll_with_resolver(&receiver, now, &mut |host, location| {
            host.resolve_drag_receiver(location)
        })
    }

    pub(super) fn route_drag_autoscroll_with_resolver<F>(
        &mut self,
        receiver: &NativeDragEndpoint,
        now: Instant,
        receiver_at: &mut F,
    ) -> NativeDragRoute
    where
        F: FnMut(
            &Self,
            cross_window_hit::NativeDragLocation,
        ) -> Option<(NativeDragEndpoint, Point)>,
    {
        let mut outcome = NativeDragRoute::default();
        if !with_drag_runtime!(
            self,
            receiver,
            runtime,
            runtime.has_pending_cross_window_autoscroll()
        )
        .unwrap_or(false)
        {
            return outcome;
        }
        // Only the window whose timed drain completed is eligible here. A
        // different child's marker may still belong to a deferred Deadline.
        let candidates: Vec<_> = self
            .cross_window_transfers
            .iter()
            .filter(|transfer| transfer.receiver.same(receiver))
            .map(|transfer| {
                (
                    transfer.source.clone(),
                    transfer.key,
                    transfer.parent_projection,
                    transfer.location,
                    transfer.position,
                )
            })
            .collect();
        for (source, key, projection, location, position) in candidates {
            let Some(export) = self
                .drag_source_export(&source)
                .filter(|export| export.key() == key && export.is_live())
            else {
                self.drag_prune_expired_transfers(None, &mut outcome);
                continue;
            };
            if projection != self.drag_parent_projection() {
                if source.owner.is_some()
                    && !self.drag_refresh_endpoint_and_drain(&source, &mut outcome)
                {
                    self.drag_remove_transfer(&source, key);
                    self.drag_cancel_receiver_after_source_refresh(
                        receiver,
                        key,
                        projection,
                        &mut outcome,
                    );
                    continue;
                }
                let projection_before_receiver_refresh = self.drag_parent_projection();
                if !self.drag_refresh_endpoint_and_drain(receiver, &mut outcome) {
                    self.drag_remove_transfer(&source, key);
                    self.drag_discard_foreign(receiver, key);
                    continue;
                }
                let projection_before_source_refresh = self.drag_parent_projection();
                if !self.drag_refresh_source_after_receiver_reduction(
                    &source,
                    &export.source_proof(),
                    projection_before_receiver_refresh,
                    &mut outcome,
                ) {
                    self.drag_remove_transfer(&source, key);
                    self.drag_cancel_receiver_after_source_refresh(
                        receiver,
                        key,
                        projection_before_source_refresh,
                        &mut outcome,
                    );
                    continue;
                }
                // A compensating receiver refresh may itself reduce messages.
                // If it changes the parent again, source evidence is no longer
                // qualified; stop this tick instead of chasing callback loops.
                let projection_after_source_refresh = self.drag_parent_projection();
                if projection_before_source_refresh != projection_after_source_refresh
                    && (!self.drag_refresh_endpoint_and_drain(receiver, &mut outcome)
                        || projection_after_source_refresh != self.drag_parent_projection())
                {
                    self.drag_remove_transfer(&source, key);
                    self.drag_discard_foreign(receiver, key);
                    continue;
                }
            }
            if !self.drag_source_proof_is_current(&source, &export.source_proof()) {
                self.drag_prune_expired_transfers(None, &mut outcome);
                continue;
            }
            if !receiver_at(self, location)
                .is_some_and(|(current, point)| current.same(receiver) && point == position)
            {
                if let Some(transfer) = self.drag_remove_transfer(&source, key) {
                    self.drag_clear_previous_receiver(transfer, &mut outcome);
                }
                continue;
            }
            let projection_before_scroll = self.drag_parent_projection();
            let Some(attempt) = with_drag_runtime!(
                self,
                receiver,
                runtime,
                runtime.take_pending_cross_window_autoscroll(key, now)
            ) else {
                self.drag_remove_transfer(&source, key);
                continue;
            };
            // Scroll callbacks from an auxiliary bridge are queued. They must
            // reach the owning application reducer before refreshing either
            // endpoint or asking the target to map another phase.
            let Some(messages) = self.take_drag_endpoint_messages(receiver) else {
                self.drag_remove_transfer(&source, key);
                continue;
            };
            if !attempt.accepted && !attempt.moved && messages.is_empty() {
                continue;
            }
            if !self.drag_reduce_messages(receiver, messages, &mut outcome)
                || !self.drag_refresh_endpoint_and_drain(receiver, &mut outcome)
            {
                self.drag_remove_transfer(&source, key);
                self.drag_discard_foreign(receiver, key);
                continue;
            }
            let projection_before_source_refresh = self.drag_parent_projection();
            if !self.drag_refresh_source_after_receiver_reduction(
                &source,
                &export.source_proof(),
                projection_before_scroll,
                &mut outcome,
            ) {
                self.drag_remove_transfer(&source, key);
                self.drag_cancel_receiver_after_source_refresh(
                    receiver,
                    key,
                    projection_before_source_refresh,
                    &mut outcome,
                );
                continue;
            }
            let projection_after_source_refresh = self.drag_parent_projection();
            if projection_before_source_refresh != projection_after_source_refresh
                && (!self.drag_refresh_endpoint_and_drain(receiver, &mut outcome)
                    || projection_after_source_refresh != self.drag_parent_projection())
            {
                self.drag_remove_transfer(&source, key);
                self.drag_discard_foreign(receiver, key);
                continue;
            }
            if !self.drag_source_proof_is_current(&source, &export.source_proof()) {
                self.drag_prune_expired_transfers(None, &mut outcome);
                continue;
            }
            if !attempt.moved {
                // Boundary/no-op ticks consume their marker without arming a
                // busy deadline. Another admitted pointer sample may rearm.
                continue;
            }
            outcome.mark_rebuild(receiver);
            let projection_before_drive = self.drag_parent_projection();
            if !receiver_at(self, location)
                .is_some_and(|(current, point)| current.same(receiver) && point == position)
                || !self.drag_drive_foreign(
                    receiver,
                    ForeignDrive {
                        source: source.clone(),
                        source_proof: export.source_proof(),
                        key,
                        location,
                        position,
                        input: Self::drag_export_input(&export, position),
                    },
                    &mut outcome,
                    receiver_at,
                )
                || !self.drag_source_proof_is_current(&source, &export.source_proof())
            {
                self.drag_remove_transfer(&source, key);
                self.drag_cancel_receiver_after_source_refresh(
                    receiver,
                    key,
                    projection_before_drive,
                    &mut outcome,
                );
                continue;
            }
            let _ = with_drag_runtime!(
                self,
                receiver,
                runtime,
                runtime.rearm_cross_window_autoscroll(key, now)
            );
            self.drag_store_transfer(source, key, receiver.clone(), location, position);
        }
        outcome
    }
}

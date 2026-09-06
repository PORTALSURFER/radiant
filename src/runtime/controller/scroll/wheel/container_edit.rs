//! Container edit evidence retained under the shared managed wheel authority.
use super::{SurfaceRuntime, WheelOrScrollRoute};
use crate::{
    gui::types::{Point, Vector2},
    layout::NodeId,
    runtime::controller::interaction_state::{
        RuntimeManagedWheelSequenceState, ScrollWheelEditOwner, ScrollWheelEditSequence,
    },
    runtime::{RuntimeBridge, ScrollEditBatch, ScrollUpdate},
    widgets::{EditEvent, InteractionProvenance, WheelPhase, WheelSample},
};

fn provenance(sample: Option<WheelSample>) -> InteractionProvenance {
    InteractionProvenance::Pointer {
        modifiers: sample.map(|sample| sample.modifiers()).unwrap_or_default(),
        timestamp: sample.and_then(|sample| sample.timestamp()),
        sequence_range: sample.and_then(|sample| sample.sequence_range()),
    }
}

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(in crate::runtime::controller) fn scroll_wheel_edit_is_live(&self) -> bool {
        let Some(sequence) = &self.interaction.wheel.scroll_edit else {
            return false;
        };
        self.interaction.wheel.managed_sequence
            == RuntimeManagedWheelSequenceState::Scroll {
                transaction: sequence.transaction,
            }
            && sequence
                .owners
                .iter()
                .all(|owner| self.scroll_wheel_owner_is_live(owner))
    }

    fn scroll_wheel_owner_is_live(&self, owner: &ScrollWheelEditOwner) -> bool {
        self.traversal
            .containers
            .scroll_content_by_container
            .get(&owner.node_id)
            == Some(&owner.content_id)
            && self.layout.rects.get(&owner.node_id).is_some_and(|rect| {
                Vector2::new(rect.width(), rect.height()) == owner.viewport_size
            })
            && self
                .layout
                .rects
                .get(&owner.content_id)
                .is_some_and(|rect| Vector2::new(rect.width(), rect.height()) == owner.content_size)
            && self
                .scroll_policy_for_node(owner.node_id)
                .is_some_and(|policy| policy.scroll_policy == owner.policy)
            && self.layout_state.scroll_offset(owner.node_id) == owner.edit.value
    }

    pub(super) fn begin_container_wheel_edit(&mut self, point: Point, sample: WheelSample) -> bool {
        let Some(primary) = self.scroll_container_at(point) else {
            return false;
        };
        let mut candidates = vec![primary];
        if let Some(path) = self.traversal.containers.clip_ancestors.get(&primary) {
            candidates.extend(path.as_slice().iter().rev().copied());
        }
        let mut owners = Vec::new();
        for node_id in candidates {
            if owners
                .iter()
                .any(|owner: &ScrollWheelEditOwner| owner.node_id == node_id)
            {
                continue;
            }
            if node_id != primary && !self.scroll_container_accepts_point(node_id, point) {
                continue;
            }
            let Some(content_id) = self
                .traversal
                .containers
                .scroll_content_by_container
                .get(&node_id)
                .copied()
            else {
                continue;
            };
            let (Some(viewport), Some(content), Some(policy)) = (
                self.layout.rects.get(&node_id),
                self.layout.rects.get(&content_id),
                self.scroll_policy_for_node(node_id),
            ) else {
                continue;
            };
            let edit = EditEvent::begin(
                self.layout_state.scroll_offset(node_id),
                provenance(Some(sample)),
            );
            owners.push(ScrollWheelEditOwner {
                node_id,
                content_id,
                viewport_size: Vector2::new(viewport.width(), viewport.height()),
                content_size: Vector2::new(content.width(), content.height()),
                policy: policy.scroll_policy,
                edit,
                begun: false,
                published: false,
            });
            if !policy.scroll_policy.chaining {
                break;
            }
        }
        let Some(first) = owners.first() else {
            return false;
        };
        let transaction = first.edit.transaction;
        self.interaction.wheel.scroll_edit = Some(ScrollWheelEditSequence {
            transaction,
            owners,
        });
        self.interaction.wheel.managed_sequence =
            RuntimeManagedWheelSequenceState::Scroll { transaction };
        true
    }

    pub(super) fn route_container_wheel_edit(
        &mut self,
        point: Point,
        sample: WheelSample,
        refresh_after_message: bool,
    ) -> WheelOrScrollRoute {
        if !self.scroll_wheel_edit_is_live() {
            self.cancel_scroll_wheel_edit(false, Some(sample), refresh_after_message);
            self.interaction.wheel.managed_sequence =
                RuntimeManagedWheelSequenceState::ScrollClosed;
            return WheelOrScrollRoute::NotRouted;
        }
        if sample.phase() == Some(WheelPhase::Cancelled) {
            self.cancel_scroll_wheel_edit(true, Some(sample), refresh_after_message);
            return WheelOrScrollRoute::ScrollContainer;
        }
        let Some(mut remaining) = self.wheel_delta_for_scroll(sample, true) else {
            return WheelOrScrollRoute::NotRouted;
        };
        let Some(mut sequence) = self.interaction.wheel.scroll_edit.clone() else {
            return WheelOrScrollRoute::NotRouted;
        };
        let terminal = sample.phase() == Some(WheelPhase::Ended);
        let mut batches = Vec::new();
        let mut changed_offsets = Vec::new();
        let mut reached_boundary = false;
        for (index, owner) in sequence.owners.iter_mut().enumerate() {
            let previous = owner.edit.value;
            let mut effective = if reached_boundary {
                Vector2::new(0.0, 0.0)
            } else {
                remaining
            };
            if !owner.policy.allows_horizontal() {
                effective.x = 0.0;
            }
            if !owner.policy.configured_axes().includes_vertical() {
                effective.y = 0.0;
            }
            match owner.policy.axis_lock {
                crate::layout::ScrollAxisLock::Horizontal => effective.y = 0.0,
                crate::layout::ScrollAxisLock::Vertical => effective.x = 0.0,
                crate::layout::ScrollAxisLock::None => {}
            }
            let requested = Vector2::new(
                (previous.x + effective.x).max(0.0),
                (previous.y + effective.y).max(0.0),
            );
            if requested != previous {
                self.layout_state
                    .scroll_offsets
                    .insert(owner.node_id, requested);
                self.note_layout_state_mutation();
                self.relayout_current_surface();
            }
            let offset = self.layout_state.scroll_offset(owner.node_id);
            let mut events = Vec::with_capacity(3);
            if !owner.begun && (index == 0 || offset != previous) {
                // Ancestors that first consume later begin at that sample's provenance.
                owner.edit = EditEvent::begin(previous, provenance(Some(sample)));
                owner.begun = true;
                events.push(owner.edit);
            }
            let update = (offset != previous).then(|| ScrollUpdate {
                node_id: owner.node_id,
                position: point,
                delta: effective,
                previous_offset: previous,
                offset,
                viewport: owner.viewport_size,
                metadata: sample.into(),
            });
            if let Some(update) = update {
                if let Some(edit) = owner.edit.update(offset, provenance(Some(sample))) {
                    owner.edit = edit;
                    events.push(edit);
                }
                if owner.policy.allows_horizontal() {
                    remaining.x -= update.offset.x - previous.x;
                }
                if owner.policy.configured_axes().includes_vertical() {
                    remaining.y -= update.offset.y - previous.y;
                }
                changed_offsets.push((owner.node_id, offset));
            }
            if terminal
                && owner.begun
                && let Some(commit) = owner.edit.commit(offset, provenance(Some(sample)))
            {
                owner.edit = commit;
                events.push(commit);
            }
            if let Some(batch) = ScrollEditBatch::new(owner.node_id, &events, update) {
                batches.push(batch);
            }
            reached_boundary |= !owner.policy.chaining
                || (remaining.x.abs() <= f32::EPSILON && remaining.y.abs() <= f32::EPSILON);
        }
        // Publish all retained values before calling any application mapper.
        if terminal {
            self.interaction.wheel.scroll_edit = None;
            self.interaction.wheel.managed_sequence =
                RuntimeManagedWheelSequenceState::ScrollClosed;
        } else {
            self.interaction.wheel.scroll_edit = Some(sequence.clone());
        }
        self.mark_scroll_activity(&changed_offsets, sample.phase());
        self.queue_scroll_settlements(&changed_offsets);
        // Detach this terminal's settlement cohort before callbacks can enqueue successors.
        let settlements = if terminal {
            self.take_pending_scroll_settlements()
        } else {
            Vec::new()
        };
        for batch in batches {
            if !terminal
                && self.interaction.wheel.managed_sequence
                    != (RuntimeManagedWheelSequenceState::Scroll {
                        transaction: sequence.transaction,
                    })
            {
                break;
            }
            let Some(owner) = sequence
                .owners
                .iter()
                .find(|owner| owner.node_id == batch.node_id())
            else {
                continue;
            };
            if !self.scroll_wheel_owner_is_live(owner) {
                if !terminal {
                    self.cancel_scroll_wheel_edit(false, None, refresh_after_message);
                }
                break;
            }
            if !terminal
                && let Some(retained) = self.interaction.wheel.scroll_edit.as_mut()
                && retained.transaction == sequence.transaction
                && let Some(owner) = retained
                    .owners
                    .iter_mut()
                    .find(|owner| owner.node_id == batch.node_id())
            {
                owner.published = true;
            }
            self.report_scroll_edit(batch, refresh_after_message);
        }
        if terminal {
            for (node_id, offset) in settlements {
                if self.layout_state.scroll_offset(node_id) == offset
                    && self
                        .traversal
                        .containers
                        .scroll_content_by_container
                        .contains_key(&node_id)
                {
                    self.emit_scroll_offset_settled(node_id, offset, refresh_after_message);
                }
            }
        } else if !self.scroll_wheel_edit_is_live() && self.interaction.wheel.scroll_edit.is_some()
        {
            self.cancel_scroll_wheel_edit(false, None, refresh_after_message);
            self.interaction.wheel.managed_sequence =
                RuntimeManagedWheelSequenceState::ScrollClosed;
        }
        WheelOrScrollRoute::ScrollContainer
    }

    pub(in crate::runtime::controller) fn scroll_wheel_edit_contains(
        &self,
        node_id: NodeId,
    ) -> bool {
        self.interaction
            .wheel
            .scroll_edit
            .as_ref()
            .is_some_and(|sequence| sequence.owners.iter().any(|owner| owner.node_id == node_id))
    }

    pub(in crate::runtime::controller) fn cancel_scroll_wheel_edit(
        &mut self,
        restore: bool,
        sample: Option<WheelSample>,
        refresh_after_message: bool,
    ) {
        let Some(sequence) = self.interaction.wheel.scroll_edit.take() else {
            return;
        };
        self.interaction.wheel.managed_sequence = RuntimeManagedWheelSequenceState::ScrollClosed;
        self.clear_pending_scroll_settlements();
        self.clear_phaseful_scroll_activity();
        let mut batches = Vec::new();
        for owner in &sequence.owners {
            if !owner.published {
                continue;
            }
            let current = self.layout_state.scroll_offset(owner.node_id);
            let mut update = None;
            if restore
                && self.scroll_wheel_owner_is_live(owner)
                && current != owner.edit.start_value
            {
                self.layout_state
                    .scroll_offsets
                    .insert(owner.node_id, owner.edit.start_value);
                self.note_layout_state_mutation();
                self.relayout_current_surface();
                let offset = self.layout_state.scroll_offset(owner.node_id);
                update = Some(ScrollUpdate {
                    node_id: owner.node_id,
                    position: Point::new(0.0, 0.0),
                    delta: Vector2::new(offset.x - current.x, offset.y - current.y),
                    previous_offset: current,
                    offset,
                    viewport: owner.viewport_size,
                    metadata: sample.map(Into::into).unwrap_or_default(),
                });
            }
            if let Some(cancel) = owner.edit.cancel(provenance(sample))
                && let Some(batch) = ScrollEditBatch::new(owner.node_id, &[cancel], update)
            {
                batches.push(batch);
            }
        }
        for batch in batches {
            self.report_scroll_edit(batch, refresh_after_message);
        }
    }
}

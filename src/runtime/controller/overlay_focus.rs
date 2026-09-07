//! Publication-owned overlay focus continuity and admission.

use super::{
    FocusBookmark, FocusTransferOutcome, SurfaceRuntime, focus::FocusTransition,
    interaction_state::RuntimeFocusOwner, traversal_state::RuntimeFocusOrderEntry,
};
use crate::runtime::surface::{OverlayFocusKey, OverlayFocusProjection};
use crate::{
    runtime::{OverlayFocusPolicy, RuntimeBridge, UiSurface},
    widgets::{FocusLossDecision, WidgetId},
};

struct OverlayEntry {
    key: OverlayFocusKey,
    policy: OverlayFocusPolicy,
    prior: Option<FocusBookmark>,
}

#[cfg(test)]
mod tests;

struct FocusLossPermit {
    bookmark: FocusBookmark,
    request: u64,
    generation: u64,
}

#[derive(Default)]
pub(super) struct OverlayFocusState {
    projection: OverlayFocusProjection,
    entries: Vec<OverlayEntry>,
    permit: Option<FocusLossPermit>,
}

pub(super) struct OverlayFocusTransition {
    projection: OverlayFocusProjection,
    prior: Option<FocusBookmark>,
    approved: bool,
    request: u64,
    focused_before: Option<WidgetId>,
    input_owners: Vec<OverlayInputOwner>,
    deferred_candidate: bool,
}

struct OverlayInputOwner {
    id: WidgetId,
    bounds: crate::layout::Rect,
    focused: bool,
    captured: bool,
    wheel: bool,
    hovered: bool,
}

fn owner_node(owner: RuntimeFocusOwner) -> WidgetId {
    match owner {
        RuntimeFocusOwner::Widget(id) => id,
        RuntimeFocusOwner::SplitPaneSeparator(owner) => owner.target.container_id,
    }
}

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    /// This decision precedes path swaps, layout-state changes, and retirement.
    /// A veto retains the last complete surface; the proposed application view
    /// may be retried by a later ordinary update, never replayed in this attempt.
    pub(super) fn prepare_overlay_focus_transition(
        &mut self,
        surface: &UiSurface<Message>,
    ) -> Result<OverlayFocusTransition, ()> {
        let mut projection = OverlayFocusProjection::collect(surface);
        if projection.is_invalid() {
            return Err(());
        }
        let deferred_candidate = projection.has_deferred_records()
            || projection.has_virtual_content()
            || !self.virtual_layout.is_empty();
        // Anchored declarations depend on final geometry (and, for virtual
        // content, final materialization), so their admission is deferred to
        // `finish_overlay_relayout`. Ordinary modal veto remains a strict
        // pre-publication boundary.
        projection.suppress_deferred();
        let new_owner = projection.records().iter().any(|record| {
            record.active()
                && record.policy() != OverlayFocusPolicy::None
                && !self
                    .interaction
                    .overlay_focus
                    .entries
                    .iter()
                    .any(|entry| &entry.key == record.key() && entry.policy == record.policy())
        });
        let moves_live_owner = projection.top_modal().is_some()
            && self.interaction.focus.owner.is_some_and(|owner| {
                !projection.contains_top_modal(owner_node(owner))
                    && owner
                        .widget_id()
                        .is_none_or(|id| surface.find_widget(id).is_some())
            });
        let prior = if new_owner || moves_live_owner || deferred_candidate {
            match self.capture_focus() {
                Ok(bookmark) => Some(bookmark),
                Err(_) if moves_live_owner => return Err(()),
                Err(_) => None,
            }
        } else {
            None
        };
        if moves_live_owner
            && let Some(widget) = self.interaction.focus.focused_widget()
            && self.prepare_focus_loss(widget) == FocusLossDecision::Veto
        {
            self.repaint_requested = true;
            return Err(());
        }
        Ok(OverlayFocusTransition {
            projection,
            prior,
            approved: moves_live_owner,
            request: self.fresh_surface_request_revision,
            focused_before: self.interaction.focus.owner.map(owner_node),
            input_owners: self.capture_overlay_input_owners(),
            deferred_candidate,
        })
    }

    /// Publish one final overlay projection. The ordinary transition retains
    /// strict pre-publication approval for unanchored modals; when a deferred
    /// candidate exists, its accepted-layout projection replaces the
    /// suppressed ordinary projection without an intermediate focus/state
    /// publication.
    pub(super) fn finish_surface_overlay_transition(
        &mut self,
        transition: OverlayFocusTransition,
    ) -> bool {
        if transition.deferred_candidate {
            self.finish_overlay_relayout(Some(transition))
        } else {
            self.publish_overlay_focus_transition(transition)
        }
    }

    fn capture_overlay_input_owners(&self) -> Vec<OverlayInputOwner> {
        let focused = self.interaction.focus.focused_widget();
        let captured = self.interaction.pointer.capture;
        let wheel = match self.interaction.wheel.managed_sequence {
            super::interaction_state::RuntimeManagedWheelSequenceState::Active { widget_id } => {
                Some(widget_id)
            }
            _ => None,
        };
        let hovered = self.interaction.hover.widget;
        let mut owners: Vec<OverlayInputOwner> = Vec::new();
        for id in [focused, captured, wheel, hovered].into_iter().flatten() {
            if let Some(index) = owners.iter().position(|owner| owner.id == id) {
                let owner = &mut owners[index];
                owner.focused |= focused == Some(id);
                owner.captured |= captured == Some(id);
                owner.wheel |= wheel == Some(id);
                owner.hovered |= hovered == Some(id);
                continue;
            }
            let projection = &self.interaction.overlay_focus.projection;
            if !projection
                .records()
                .iter()
                .enumerate()
                .any(|(scope, _)| projection.contains(scope, id))
            {
                continue;
            }
            let Some(bounds) = self.layout.rects.get(&id).copied() else {
                continue;
            };
            owners.push(OverlayInputOwner {
                id,
                bounds,
                focused: focused == Some(id),
                captured: captured == Some(id),
                wheel: wheel == Some(id),
                hovered: hovered == Some(id),
            });
        }
        owners
    }

    // Source remains mounted when an anchor disappears. Explicitly end its
    // input ownership so a later layout cannot revive a key/composition/drag.
    fn retire_omitted_overlay_input(&mut self, owners: Vec<OverlayInputOwner>) -> Vec<Message> {
        let mut messages = Vec::new();
        for owner in owners {
            if self.layout.rects.contains_key(&owner.id) {
                continue;
            }
            // Full refresh may already have retired this incarnation. A saved
            // numeric id must never deliver a terminal to its replacement.
            let focused =
                owner.focused && self.interaction.focus.focused_widget() == Some(owner.id);
            let captured = owner.captured && self.interaction.pointer.capture == Some(owner.id);
            let wheel = owner.wheel
                && matches!(
                    self.interaction.wheel.managed_sequence,
                    super::interaction_state::RuntimeManagedWheelSequenceState::Active { widget_id }
                        if widget_id == owner.id
                );
            let hovered = owner.hovered && self.interaction.hover.widget == Some(owner.id);
            if !focused && !captured && !wheel && !hovered {
                continue;
            }
            if wheel {
                // Do not let a reentrant terminal callback retain or replay a
                // sequence whose owner this layout pass just omitted.
                self.block_managed_wheel_sequence();
                let cancellation = crate::widgets::WheelSample::from_parts(
                    crate::widgets::WheelDelta::Pixels(crate::gui::types::Vector2::default()),
                    Some(crate::widgets::WheelPhase::Cancelled),
                    crate::widgets::PointerModifiers::default(),
                    None,
                    None,
                );
                if let Some((result, _)) = self.dispatch_surface_wheel_sample(
                    owner.id,
                    owner.bounds,
                    owner.bounds.center(),
                    cancellation,
                ) && let crate::runtime::ResolvedWidgetDispatchResult::Message(message) =
                    self.resolve_widget_dispatch(result)
                {
                    messages.push(message);
                }
            }
            if hovered {
                self.clear_pointer_hover();
            }
            self.discard_widget_ownership(owner.id);
            if captured
                && let Some(result) =
                    self.dispatch_surface_pointer_capture_cancelled(owner.id, owner.bounds)
                && let crate::runtime::ResolvedWidgetDispatchResult::Message(message) =
                    self.resolve_widget_dispatch(result)
            {
                messages.push(message);
            }
            if focused
                && let Some(result) = self.dispatch_surface_input(
                    owner.id,
                    owner.bounds,
                    crate::widgets::WidgetInput::FocusChanged(false),
                )
                && let crate::runtime::ResolvedWidgetDispatchResult::Message(message) =
                    self.resolve_widget_dispatch(result)
            {
                messages.push(message);
            }
        }
        self.validate_managed_composition_authority();
        self.validate_managed_pointer_capture_authority();
        self.validate_managed_wheel_sequence_authority();
        self.validate_gesture_capture();
        messages
    }

    pub(super) fn capture_overlay_relayout(&mut self) -> Option<OverlayFocusTransition> {
        if !self.lifecycle_accepts_work()
            || (self
                .interaction
                .overlay_focus
                .projection
                .records()
                .is_empty()
                && self.virtual_layout.is_empty())
        {
            return None;
        }
        Some(OverlayFocusTransition {
            projection: OverlayFocusProjection::default(),
            prior: self.capture_focus().ok(),
            approved: false,
            request: self.fresh_surface_request_revision,
            focused_before: self.interaction.focus.owner.map(owner_node),
            input_owners: self.capture_overlay_input_owners(),
            deferred_candidate: true,
        })
    }

    pub(super) fn finish_overlay_relayout(
        &mut self,
        transition: Option<OverlayFocusTransition>,
    ) -> bool {
        let Some(mut transition) = transition else {
            return false;
        };
        // A settled-scroll callback may already have published another source.
        if transition.request != self.fresh_surface_request_revision {
            return false;
        }
        // Virtual geometry relayout may have materialized a different item
        // set without requesting a new application surface. Membership must
        // describe that accepted set, while bookmarks/terminals describe the
        // owners captured before layout.
        transition.projection = OverlayFocusProjection::collect(&self.surface);
        transition.projection.qualify(&self.layout);
        let top_modal_is_new = transition.projection.top_modal().is_some_and(|index| {
            let record = &transition.projection.records()[index];
            !self
                .interaction
                .overlay_focus
                .entries
                .iter()
                .any(|entry| &entry.key == record.key() && entry.policy == record.policy())
        });
        let moves_live_owner = top_modal_is_new
            && self.interaction.focus.owner.is_some_and(|owner| {
                !transition.projection.contains_top_modal(owner_node(owner))
                    && self.layout.rects.contains_key(&owner_node(owner))
            });
        if moves_live_owner {
            let projection_before = self.refresh_counters.application_projection;
            let generation_before = self.fresh_surface_active_generation;
            // A replacement may have installed another incarnation with the
            // same numeric widget id. It owns its normal replacement outcome;
            // never send a loss probe through this deferred anchor path unless
            // the bookmark still identifies the retained incumbent.
            if transition.approved {
                return self.publish_overlay_focus_transition(transition);
            }
            if !transition
                .prior
                .as_ref()
                .is_some_and(|prior| self.bookmark_matches_current_focus_owner(prior))
            {
                self.omit_new_deferred_modal_groups(&mut transition);
                self.publish_overlay_focus_transition(transition);
                return true;
            }
            let veto = self
                .interaction
                .focus
                .focused_widget()
                .is_some_and(|id| self.prepare_focus_loss(id) == FocusLossDecision::Veto);
            if transition.request != self.fresh_surface_request_revision
                || projection_before != self.refresh_counters.application_projection
                || generation_before != self.fresh_surface_active_generation
            {
                return true;
            }
            if veto {
                // Reject newly activated modal groups, retaining current base
                // geometry rather than retaining a stale viewport after resize.
                self.omit_new_deferred_modal_groups(&mut transition);
                self.publish_overlay_focus_transition(transition);
                return true;
            } else {
                transition.approved = true;
            }
        }
        self.publish_overlay_focus_transition(transition)
    }

    fn omit_new_deferred_modal_groups(&mut self, transition: &mut OverlayFocusTransition) {
        let roots: Vec<_> =
            transition
                .projection
                .records()
                .iter()
                .enumerate()
                .filter(|(_, record)| record.active())
                .filter(|(index, _)| {
                    transition.projection.records().iter().enumerate().any(
                        |(ancestor, candidate)| {
                            candidate.policy() == OverlayFocusPolicy::Modal
                                && !self.interaction.overlay_focus.entries.iter().any(|entry| {
                                    &entry.key == candidate.key()
                                        && entry.policy == candidate.policy()
                                })
                                && transition.projection.contains_scope(ancestor, *index)
                        },
                    )
                })
                .map(|(_, record)| record.root())
                .collect();
        for root in roots {
            if let Some(node) = find_layout_node(&self.layout_root, root) {
                self.layout.omit_resolved_subtree(node);
            }
        }
        self.completed_layout = None;
        self.refresh_visible_traversal_orders();
        transition.projection.qualify(&self.layout);
    }

    pub(super) fn overlay_focus_allows(&self, node: WidgetId) -> bool {
        let projection = &self.interaction.overlay_focus.projection;
        projection.is_valid()
            && (projection.top_modal().is_none() || projection.contains_top_modal(node))
    }

    pub(super) fn has_modal_focus_scope(&self) -> bool {
        self.interaction
            .overlay_focus
            .projection
            .top_modal()
            .is_some()
    }

    /// Consume the one-shot approval only for the same retained incarnation and
    /// publication. It is gone before a FocusChanged callback can reenter.
    pub(super) fn consume_overlay_focus_loss_permit(&mut self, widget: WidgetId) -> bool {
        let Some(permit) = self.interaction.overlay_focus.permit.take() else {
            return false;
        };
        self.interaction.focus.focused_widget() == Some(widget)
            && permit.request == self.fresh_surface_request_revision
            && permit.generation == self.fresh_surface_active_generation
            && self.bookmark_matches_current_focus_owner(&permit.bookmark)
    }

    /// Install membership before any focus callbacks. Returns whether this
    /// transaction already delivered the focus state and the caller must avoid
    /// an additional retained FocusChanged(true) notification.
    pub(super) fn publish_overlay_focus_transition(
        &mut self,
        mut transition: OverlayFocusTransition,
    ) -> bool {
        transition.projection.qualify(&self.layout);
        let focused = transition.focused_before;
        let state = &mut self.interaction.overlay_focus;
        let mut old_entries = std::mem::take(&mut state.entries);
        // Outermost removed scope wins when several nested scopes close in one
        // publication: an inner bookmark can name a simultaneously retired body.
        let survives = |entry: &OverlayEntry| {
            transition.projection.records().iter().any(|record| {
                record.active() && record.key() == &entry.key && record.policy() == entry.policy
            })
        };
        let mut restore_index = old_entries
            .iter()
            .enumerate()
            .find_map(|(entry_index, entry)| {
                let owned_focus = state
                    .projection
                    .records()
                    .iter()
                    .enumerate()
                    .find(|(_, record)| record.key() == &entry.key)
                    .is_some_and(|(index, _)| {
                        focused.map_or_else(
                            || {
                                entry.policy == OverlayFocusPolicy::Modal
                                    && state.projection.top_modal().is_some_and(|top| {
                                        state.projection.contains_scope(index, top)
                                    })
                            },
                            |node| state.projection.contains(index, node),
                        )
                    });
                (!survives(entry) && owned_focus).then_some(entry_index)
            });
        // Raw modal siblings also form a focus-restoration stack. Follow an
        // observed prior node into an earlier removed scope before validating
        // its bookmark; stop at a surviving scope or the base. Indices strictly
        // decrease, and no node id itself grants focus authority.
        while let Some(index) = restore_index {
            let Some(prior) = old_entries[index].prior.as_ref() else {
                break;
            };
            let earlier = old_entries[..index].iter().position(|entry| {
                !survives(entry)
                    && state
                        .projection
                        .records()
                        .iter()
                        .enumerate()
                        .find(|(_, record)| record.key() == &entry.key)
                        .is_some_and(|(scope, _)| {
                            state.projection.contains(scope, prior.observed_node())
                        })
            });
            let Some(earlier) = earlier else {
                break;
            };
            restore_index = Some(earlier);
        }
        let restore = restore_index.map(|index| old_entries[index].prior.clone());
        let mut entries = Vec::new();
        for record in transition.projection.records() {
            if !record.active() || record.policy() == OverlayFocusPolicy::None {
                continue;
            }
            let prior = if let Some(index) = old_entries
                .iter()
                .position(|entry| &entry.key == record.key() && entry.policy == record.policy())
            {
                old_entries.remove(index).prior
            } else {
                transition.prior.clone()
            };
            entries.push(OverlayEntry {
                key: record.key().clone(),
                policy: record.policy(),
                prior,
            });
        }
        state.entries = entries;
        state.projection = transition.projection;
        if transition.approved
            && transition.request == self.fresh_surface_request_revision
            && let Some(bookmark) = transition.prior
            && self.bookmark_matches_current_focus_owner(&bookmark)
        {
            self.interaction.overlay_focus.permit = Some(FocusLossPermit {
                bookmark,
                request: transition.request,
                generation: self.fresh_surface_active_generation,
            });
        }
        let projection_before = self.refresh_counters.application_projection;
        let generation_before = self.fresh_surface_active_generation;
        let terminal_messages = self.retire_omitted_overlay_input(transition.input_owners);
        if projection_before != self.refresh_counters.application_projection
            || generation_before != self.fresh_surface_active_generation
        {
            self.interaction.overlay_focus.permit = None;
            for message in terminal_messages {
                let outcome = self.dispatch_message(message);
                self.pending_input_command_outcome.merge(outcome);
            }
            return true;
        }
        let changed = self.apply_overlay_focus_destination(
            restore.as_ref().and_then(Option::as_ref),
            restore.is_some(),
        );
        self.interaction.overlay_focus.permit = None;
        for message in terminal_messages {
            let outcome = self.dispatch_message(message);
            self.pending_input_command_outcome.merge(outcome);
        }
        changed
            || projection_before != self.refresh_counters.application_projection
            || generation_before != self.fresh_surface_active_generation
    }

    fn apply_overlay_focus_destination(
        &mut self,
        restore: Option<&FocusBookmark>,
        restore_requested: bool,
    ) -> bool {
        if let Some(bookmark) = restore {
            match self.restore_focus(bookmark) {
                FocusTransferOutcome::Admitted(_)
                | FocusTransferOutcome::AdmittedRuntimeOwned
                | FocusTransferOutcome::Invalidated => return true,
                FocusTransferOutcome::Vetoed => return false,
                _ => {}
            }
        }
        let needs_modal_focus = self.has_modal_focus_scope()
            && self
                .interaction
                .focus
                .owner
                .is_none_or(|owner| !self.overlay_focus_allows(owner_node(owner)));
        if !needs_modal_focus && !restore_requested {
            return false;
        }
        let target = if self.traversal.widgets.mixed_focus_order.is_empty() {
            self.traversal
                .widgets
                .keyboard_focus
                .order()
                .iter()
                .copied()
                .find(|id| self.overlay_focus_allows(*id) && self.is_live_focus_target(*id))
                .map(RuntimeFocusOrderEntry::Widget)
        } else {
            self.traversal
                .widgets
                .mixed_focus_order
                .iter()
                .copied()
                .find(|entry| match entry {
                    RuntimeFocusOrderEntry::Widget(id) => {
                        self.overlay_focus_allows(*id) && self.is_live_focus_target(*id)
                    }
                    RuntimeFocusOrderEntry::SplitPaneSeparator(projection) => {
                        self.overlay_focus_allows(projection.target.container_id)
                    }
                })
        };
        let result = match target {
            Some(RuntimeFocusOrderEntry::Widget(id)) => self.request_focus(id),
            Some(RuntimeFocusOrderEntry::SplitPaneSeparator(projection)) => {
                self.request_split_pane_separator_focus(projection)
            }
            None => self.clear_focus_with_transition(),
        };
        matches!(result, FocusTransition::Changed)
    }

    /// Offer Escape to its current widget once, then dismiss only the exact
    /// topmost overlay captured before that delivery. Repeats never descend
    /// through the overlay stack and reentrant updates invalidate the attempt.
    pub(super) fn route_overlay_escape(
        &mut self,
        key: crate::widgets::WidgetKey,
        modifiers: crate::widgets::KeyboardModifiers,
        repeat: bool,
        timestamp: Option<crate::gui::input::InputTimestamp>,
    ) -> Option<bool> {
        use crate::widgets::{FocusedKeyDisposition, WidgetInput, WidgetKey};
        if !self.lifecycle_accepts_work()
            || key != WidgetKey::Escape
            || modifiers.command
            || modifiers.control
            || modifiers.shift
            || modifiers.alt
            || self.interaction.focus.focused_key_capture.is_some()
            || self.interaction.composition.managed_composition
                != super::interaction_state::RuntimeManagedCompositionState::Idle
        {
            return None;
        }
        let record = self.interaction.overlay_focus.projection.top_active()?;
        let callback = self.surface.root().overlay_escape_callback(record.key())?;
        if repeat {
            return Some(true);
        }
        let revision = self.fresh_surface_request_revision;
        let projection = self.refresh_counters.application_projection;
        if let Some(delivery) = self.dispatch_focused_key_input(
            WidgetInput::key_press_with_metadata(key, modifiers, false, timestamp),
        ) {
            if delivery.disposition == FocusedKeyDisposition::Consumed {
                if delivery.fallback_eligible {
                    self.establish_focused_key_capture(delivery.widget_id, key);
                }
                return Some(true);
            }
            if !delivery.fallback_eligible {
                return Some(true);
            }
        }
        if revision != self.fresh_surface_request_revision
            || projection != self.refresh_counters.application_projection
        {
            return Some(true);
        }
        let outcome = self.dispatch_message(callback());
        self.pending_input_command_outcome.merge(outcome);
        Some(true)
    }

    pub(in crate::runtime) fn qualify_overlay_semantics(
        &self,
        node: &mut crate::gui::automation::AutomationNodeSnapshot,
    ) {
        if self
            .interaction
            .overlay_focus
            .projection
            .records()
            .is_empty()
        {
            return;
        }
        self.remove_omitted_overlay_semantics(node);
        if self.has_modal_focus_scope() {
            self.qualify_overlay_semantic_node(node);
        }
    }

    fn remove_omitted_overlay_semantics(
        &self,
        node: &mut crate::gui::automation::AutomationNodeSnapshot,
    ) {
        node.children.retain(|child| {
            !child
                .id
                .0
                .parse::<WidgetId>()
                .is_ok_and(|id| self.layout.is_omitted(id))
        });
        for child in &mut node.children {
            self.remove_omitted_overlay_semantics(child);
        }
    }

    fn qualify_overlay_semantic_node(
        &self,
        node: &mut crate::gui::automation::AutomationNodeSnapshot,
    ) -> bool {
        let mut contains_active = node
            .id
            .0
            .parse::<WidgetId>()
            .is_ok_and(|id| self.overlay_focus_allows(id));
        for child in &mut node.children {
            contains_active |= self.qualify_overlay_semantic_node(child);
        }
        if !contains_active {
            node.enabled = false;
            node.available_actions.clear();
            node.semantics.disabled = true;
            node.semantics.focusable = false;
            node.semantics.focused = false;
            node.semantics.focus_hints = Default::default();
            node.semantics
                .metadata
                .insert("overlay.inert".into(), "true".into());
            node.metadata.insert("overlay.inert".into(), "true".into());
        }
        contains_active
    }

    pub(super) fn initialize_overlay_focus(&mut self) {
        let transition = OverlayFocusTransition {
            projection: OverlayFocusProjection::collect(&self.surface),
            prior: None,
            approved: false,
            request: self.fresh_surface_request_revision,
            focused_before: self.interaction.focus.owner.map(owner_node),
            input_owners: self.capture_overlay_input_owners(),
            deferred_candidate: false,
        };
        self.publish_overlay_focus_transition(transition);
    }
}

fn find_layout_node(
    node: &crate::layout::LayoutNode,
    id: WidgetId,
) -> Option<&crate::layout::LayoutNode> {
    if node.id() == id {
        return Some(node);
    }
    if let crate::layout::LayoutNode::Container(container) = node {
        return container
            .children
            .iter()
            .find_map(|child| find_layout_node(&child.child, id));
    }
    None
}

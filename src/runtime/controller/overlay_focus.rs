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
        let projection = OverlayFocusProjection::collect(surface);
        if projection.is_invalid() {
            return Err(());
        }
        let new_owner = projection.records().iter().any(|record| {
            record.policy() != OverlayFocusPolicy::None
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
        let prior = if new_owner || moves_live_owner {
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
        })
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
        let focused = self.interaction.focus.owner.map(owner_node);
        let state = &mut self.interaction.overlay_focus;
        let mut old_entries = std::mem::take(&mut state.entries);
        // Outermost removed scope wins when several nested scopes close in one
        // publication: an inner bookmark can name a simultaneously retired body.
        let restore = old_entries.iter().find_map(|entry| {
            let survives = transition.projection.records().iter().any(|record| {
                record.active() && record.key() == &entry.key && record.policy() == entry.policy
            });
            let owned_focus = state
                .projection
                .records()
                .iter()
                .enumerate()
                .find(|(_, record)| record.key() == &entry.key)
                .is_some_and(|(index, _)| {
                    focused.is_none_or(|node| state.projection.contains(index, node))
                });
            (!survives && owned_focus)
                .then(|| entry.prior.clone())
                .flatten()
        });
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
        let changed = self.apply_overlay_focus_destination(restore.as_ref());
        self.interaction.overlay_focus.permit = None;
        changed
            || projection_before != self.refresh_counters.application_projection
            || generation_before != self.fresh_surface_active_generation
    }

    fn apply_overlay_focus_destination(&mut self, restore: Option<&FocusBookmark>) -> bool {
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
        if !needs_modal_focus && restore.is_none() {
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
        if key != WidgetKey::Escape
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
        if !self.has_modal_focus_scope() {
            return;
        }
        self.qualify_overlay_semantic_node(node);
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
        };
        self.publish_overlay_focus_transition(transition);
    }
}

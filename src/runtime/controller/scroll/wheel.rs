//! Wheel routing for scrollable and wheel-aware runtime surfaces.
//!
//! Deltas reaching this module use logical scroll-offset direction. Native
//! adapters perform platform sign and unit conversion once before routing;
//! generic widget, controller, and layout paths do not flip the axes again.
//! Exact unit and phase evidence is retained only when a qualified widget or
//! policy route consumes a `WheelSample`; the native coalesced scroll-container
//! fallback projects to one selected logical-pixel axis before controller
//! routing. Direct exact-sample routing can retain both projected logical-pixel
//! components.

use super::super::CommandOutcome;
use super::{ScrollUpdateMetadata, SurfaceRuntime};
use crate::widgets::{WheelDelta, WheelPhase, WheelSample};
use crate::{
    gui::types::{Point, Vector2},
    runtime::{RuntimeBridge, WheelHitTarget, WidgetDispatchResult},
    widgets::{PointerModifiers, WidgetId, WidgetInput},
};

use super::super::interaction_state::RuntimeManagedWheelSequenceState;

#[path = "wheel/container_edit.rs"]
mod container_edit;

#[cfg(test)]
#[path = "wheel/tests.rs"]
mod tests;

/// Route taken by a wheel event after widget-first routing and scroll fallback.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WheelOrScrollRoute {
    /// No widget or scroll container accepted the wheel event.
    NotRouted,
    /// A wheel-aware widget handled the event.
    Widget,
    /// The event fell back to a scroll container.
    ScrollContainer,
}

/// Result of routing one sample to the widget selected by the current phase.
///
/// Retained no-output is deliberately distinct from an unhandled widget
/// sample: the former owns the sample and must not fall through to scrolling.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WheelWidgetDispatch {
    Handled { retained: bool },
    RetainedNoOutput,
    Unhandled,
}

impl WheelWidgetDispatch {
    fn retained(self) -> bool {
        matches!(
            self,
            Self::RetainedNoOutput | Self::Handled { retained: true }
        )
    }
}

impl From<WheelSample> for ScrollUpdateMetadata {
    fn from(sample: WheelSample) -> Self {
        Self {
            modifiers: sample.modifiers(),
            timestamp: sample.timestamp(),
            sequence_range: sample.sequence_range(),
        }
    }
}

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    /// Route a legacy phase-less logical-pixel offset delta to a widget, then
    /// a scroll container.
    ///
    /// Positive `x`/`y` increases the corresponding logical scroll offset.
    pub fn wheel_or_scroll_at(&mut self, point: Point, delta: Vector2) -> bool {
        self.wheel_or_scroll_at_with_metadata(point, delta, PointerModifiers::default(), None, None)
    }

    /// Route a legacy modified phase-less logical-pixel offset delta.
    /// Positive `x`/`y` increases the corresponding logical scroll offset.
    pub fn wheel_or_scroll_at_with_modifiers(
        &mut self,
        point: Point,
        delta: Vector2,
        modifiers: PointerModifiers,
    ) -> bool {
        self.wheel_or_scroll_at_with_metadata(point, delta, modifiers, None, None)
    }

    /// Route one exact unit- and phase-qualified wheel sample whose delta is
    /// already in logical scroll-offset direction. Exact unit and phase
    /// evidence is retained for a qualified widget or policy consumer. The
    /// native coalesced scroll-container fallback receives one selected
    /// logical-pixel axis without phase or unit evidence; direct exact-sample
    /// routing can retain both projected logical-pixel components.
    pub fn wheel_or_scroll_at_with_sample(&mut self, point: Point, sample: WheelSample) -> bool {
        self.wheel_or_scroll_route_with_sample(point, sample, true, true)
            != WheelOrScrollRoute::NotRouted
    }

    pub(crate) fn wheel_or_scroll_at_with_metadata(
        &mut self,
        point: Point,
        delta: Vector2,
        modifiers: PointerModifiers,
        timestamp: Option<crate::gui::input::InputTimestamp>,
        sequence_range: Option<crate::gui::input::InputSequenceRange>,
    ) -> bool {
        let sample = WheelSample::from_parts(
            WheelDelta::legacy_pixels(delta),
            None,
            modifiers,
            timestamp,
            sequence_range,
        );
        self.wheel_or_scroll_route_with_sample(point, sample, true, false)
            != WheelOrScrollRoute::NotRouted
    }

    /// Route a legacy phase-less wheel input while deferring host-surface
    /// refresh until the caller chooses to refresh.
    pub fn wheel_or_scroll_at_deferred_refresh(&mut self, point: Point, delta: Vector2) -> bool {
        self.wheel_or_scroll_at_deferred_refresh_with_metadata(
            point,
            delta,
            PointerModifiers::default(),
            None,
            None,
        )
    }

    /// Route a legacy modified wheel input while deferring host-surface refresh.
    pub fn wheel_or_scroll_at_deferred_refresh_with_modifiers(
        &mut self,
        point: Point,
        delta: Vector2,
        modifiers: PointerModifiers,
    ) -> bool {
        self.wheel_or_scroll_at_deferred_refresh_with_metadata(point, delta, modifiers, None, None)
    }

    /// Route an exact wheel sample while deferring host-surface refresh.
    /// Qualified widget or policy consumers retain its unit and phase. The
    /// native coalesced scroll-container fallback receives one selected
    /// logical-pixel axis without phase or unit evidence; direct exact-sample
    /// routing can retain both projected logical-pixel components.
    pub fn wheel_or_scroll_at_deferred_refresh_with_sample(
        &mut self,
        point: Point,
        sample: WheelSample,
    ) -> bool {
        self.wheel_or_scroll_route_with_sample(point, sample, false, true)
            != WheelOrScrollRoute::NotRouted
    }

    pub(crate) fn wheel_or_scroll_route_deferred_refresh_with_sample(
        &mut self,
        point: Point,
        sample: WheelSample,
    ) -> WheelOrScrollRoute {
        self.wheel_or_scroll_route_with_sample(point, sample, false, true)
    }

    pub(crate) fn wheel_or_scroll_at_deferred_refresh_with_metadata(
        &mut self,
        point: Point,
        delta: Vector2,
        modifiers: PointerModifiers,
        timestamp: Option<crate::gui::input::InputTimestamp>,
        sequence_range: Option<crate::gui::input::InputSequenceRange>,
    ) -> bool {
        self.wheel_or_scroll_route_deferred_refresh_with_metadata(
            point,
            delta,
            modifiers,
            timestamp,
            sequence_range,
        ) != WheelOrScrollRoute::NotRouted
    }

    /// Route a legacy modified wheel input while reporting widget or scroll
    /// fallback acceptance and deferring refresh.
    #[cfg(test)]
    pub(crate) fn wheel_or_scroll_route_deferred_refresh_with_modifiers(
        &mut self,
        point: Point,
        delta: Vector2,
        modifiers: PointerModifiers,
    ) -> WheelOrScrollRoute {
        self.wheel_or_scroll_route_deferred_refresh_with_metadata(
            point, delta, modifiers, None, None,
        )
    }

    pub(crate) fn wheel_or_scroll_route_deferred_refresh_with_metadata(
        &mut self,
        point: Point,
        delta: Vector2,
        modifiers: PointerModifiers,
        timestamp: Option<crate::gui::input::InputTimestamp>,
        sequence_range: Option<crate::gui::input::InputSequenceRange>,
    ) -> WheelOrScrollRoute {
        let sample = WheelSample::from_parts(
            WheelDelta::legacy_pixels(delta),
            None,
            modifiers,
            timestamp,
            sequence_range,
        );
        self.wheel_or_scroll_route_with_sample(point, sample, false, false)
    }

    fn wheel_or_scroll_route_with_sample(
        &mut self,
        point: Point,
        sample: WheelSample,
        refresh_after_message: bool,
        exact_sample: bool,
    ) -> WheelOrScrollRoute {
        if self.gesture_owns_pointer_capture() || self.scrollbar_drag_active() {
            return WheelOrScrollRoute::NotRouted;
        }
        let phase = sample.phase();
        if self.interaction.wheel.scroll_edit.is_some() && !self.scroll_wheel_edit_is_live() {
            self.cancel_scroll_wheel_edit(false, None, refresh_after_message);
        }
        if self.interaction.wheel.scroll_edit.is_some() {
            if phase == Some(WheelPhase::Started) {
                self.cancel_scroll_wheel_edit(true, None, refresh_after_message);
            } else if matches!(
                phase,
                Some(WheelPhase::Changed | WheelPhase::Ended | WheelPhase::Cancelled)
            ) {
                if !sample.is_valid() {
                    return WheelOrScrollRoute::NotRouted;
                }
                return self.route_container_wheel_edit(point, sample, refresh_after_message);
            } else {
                return WheelOrScrollRoute::NotRouted;
            }
        }
        if phase == Some(WheelPhase::Started) {
            self.clear_phaseful_scroll_activity();
            if exact_sample && !sample.is_valid() {
                // Invalid explicit starts do not settle an admitted prior
                // phase-less owner. They remain a non-routing boundary.
                self.clear_managed_wheel_sequence();
                return WheelOrScrollRoute::NotRouted;
            }
            self.emit_pending_scroll_settlements(refresh_after_message);
            if exact_sample {
                self.cancel_live_managed_wheel_sequence_for_start(point, refresh_after_message);
            }
            // Every valid explicit start is a fresh managed-widget boundary.
            self.clear_managed_wheel_sequence();
        }
        if matches!(phase, Some(WheelPhase::Ended | WheelPhase::Cancelled))
            && !matches!(
                self.interaction.wheel.managed_sequence,
                RuntimeManagedWheelSequenceState::Active { .. }
            )
        {
            self.clear_phaseful_scroll_activity();
        }
        if exact_sample && !sample.is_valid() {
            return WheelOrScrollRoute::NotRouted;
        }
        match (self.interaction.wheel.managed_sequence, phase) {
            (
                RuntimeManagedWheelSequenceState::ScrollClosed,
                Some(WheelPhase::Changed | WheelPhase::Ended | WheelPhase::Cancelled),
            ) => {
                return WheelOrScrollRoute::NotRouted;
            }
            (RuntimeManagedWheelSequenceState::Blocked, Some(WheelPhase::Changed)) => {
                // A known orphan cannot be rebound by a continuation that
                // happens to land over another widget.
                return WheelOrScrollRoute::NotRouted;
            }
            (
                RuntimeManagedWheelSequenceState::Blocked,
                Some(WheelPhase::Ended | WheelPhase::Cancelled),
            ) => {
                // The first terminal after an orphan closes the blocked slot,
                // but is not delivered through the compatibility path.
                self.clear_managed_wheel_sequence();
                self.clear_phaseful_scroll_activity();
                return WheelOrScrollRoute::NotRouted;
            }
            (RuntimeManagedWheelSequenceState::Active { widget_id }, Some(WheelPhase::Changed)) => {
                if !self.managed_wheel_sequence_is_live(widget_id) {
                    self.block_managed_wheel_sequence();
                    return WheelOrScrollRoute::NotRouted;
                }
                return self.dispatch_managed_wheel_sequence(
                    widget_id,
                    point,
                    sample,
                    refresh_after_message,
                );
            }
            (
                RuntimeManagedWheelSequenceState::Active { widget_id },
                Some(WheelPhase::Ended | WheelPhase::Cancelled),
            ) => {
                if !self.managed_wheel_sequence_is_live(widget_id) {
                    // A stale terminal closes the slot without guessing a
                    // replacement target or delivering a fallback sample.
                    self.clear_managed_wheel_sequence();
                    self.clear_phaseful_scroll_activity();
                    return WheelOrScrollRoute::NotRouted;
                }
                let route = self.dispatch_managed_wheel_sequence(
                    widget_id,
                    point,
                    sample,
                    refresh_after_message,
                );
                self.clear_phaseful_scroll_activity();
                return route;
            }
            _ => {}
        }

        if phase != Some(WheelPhase::Started) {
            self.validate_managed_wheel_sequence_authority();
        }

        let Some(input) = self.wheel_input_for_hit_test(point, sample, exact_sample) else {
            return WheelOrScrollRoute::NotRouted;
        };
        let route = match self.wheel_target_at(point, &input) {
            Some(WheelHitTarget::Widget(widget_id)) => {
                let Some(dispatch) = self.dispatch_wheel_to_widget_with_refresh(
                    widget_id,
                    point,
                    sample,
                    refresh_after_message,
                    exact_sample,
                ) else {
                    return self
                        .finish_scroll_terminal_after_routing(sample, refresh_after_message);
                };
                let may_install = exact_sample
                    && sample.phase() == Some(WheelPhase::Started)
                    && self.interaction.wheel.managed_sequence
                        == RuntimeManagedWheelSequenceState::Idle
                    && dispatch.retained()
                    && self.wheel_widget_is_unique(widget_id)
                    && self.managed_wheel_sequence_is_live(widget_id);
                if may_install {
                    self.interaction.wheel.managed_sequence =
                        RuntimeManagedWheelSequenceState::Active { widget_id };
                }
                match dispatch {
                    WheelWidgetDispatch::Handled { .. } | WheelWidgetDispatch::RetainedNoOutput => {
                        WheelOrScrollRoute::Widget
                    }
                    WheelWidgetDispatch::Unhandled => self.scroll_fallback_for_sample(
                        point,
                        sample,
                        exact_sample,
                        refresh_after_message,
                    ),
                }
            }
            Some(WheelHitTarget::ScrollContainer(_)) => {
                self.scroll_fallback_for_sample(point, sample, exact_sample, refresh_after_message)
            }
            None => WheelOrScrollRoute::NotRouted,
        };
        if matches!(
            sample.phase(),
            Some(WheelPhase::Ended | WheelPhase::Cancelled)
        ) {
            self.finish_scroll_terminal_after_routing(sample, refresh_after_message);
        }
        route
    }

    fn scroll_fallback_for_sample(
        &mut self,
        point: Point,
        sample: WheelSample,
        exact_sample: bool,
        refresh_after_message: bool,
    ) -> WheelOrScrollRoute {
        if matches!(
            sample.phase(),
            Some(WheelPhase::Ended | WheelPhase::Cancelled)
        ) {
            // A terminal without retained container authority cannot produce a late delta.
            return self.finish_scroll_terminal_after_routing(sample, refresh_after_message);
        }
        if sample.phase() == Some(WheelPhase::Started)
            && self.begin_container_wheel_edit(point, sample)
        {
            return self.route_container_wheel_edit(point, sample, refresh_after_message);
        }
        let Some(delta) = self.wheel_delta_for_scroll(sample, exact_sample) else {
            return WheelOrScrollRoute::NotRouted;
        };
        let before = self
            .traversal
            .containers
            .scroll
            .visible()
            .iter()
            .copied()
            .map(|node_id| (node_id, self.layout_state.scroll_offset(node_id)))
            .collect::<Vec<_>>();
        if self.scroll_at_with_refresh_and_metadata(
            point,
            delta,
            sample.into(),
            refresh_after_message,
            crate::widgets::InteractionProvenance::Pointer {
                modifiers: sample.modifiers(),
                timestamp: sample.timestamp(),
                sequence_range: sample.sequence_range(),
            },
        ) {
            let changed = before
                .into_iter()
                .filter_map(|(node_id, previous)| {
                    let offset = self.layout_state.scroll_offset(node_id);
                    (offset != previous).then_some((node_id, offset))
                })
                .collect::<Vec<_>>();
            match sample.phase() {
                Some(WheelPhase::Started | WheelPhase::Changed) => {
                    self.mark_scroll_activity(&changed, sample.phase());
                    self.queue_scroll_settlements(&changed);
                }
                Some(WheelPhase::Ended) => {
                    self.mark_scroll_activity(&changed, sample.phase());
                    self.queue_scroll_settlements(&changed);
                    self.emit_pending_scroll_settlements(refresh_after_message);
                }
                Some(WheelPhase::Cancelled) => {
                    self.mark_scroll_activity(&changed, sample.phase());
                    self.clear_pending_scroll_settlements();
                }
                Some(WheelPhase::Discrete) | None => {
                    self.mark_scroll_activity(&changed, sample.phase());
                    if sample.phase().is_none() {
                        self.queue_scroll_settlements(&changed);
                        if !changed.is_empty() {
                            self.interaction.wheel.scroll_settlement_deadline = Some(
                                self.timed_repaint_now() + std::time::Duration::from_millis(100),
                            );
                        }
                    } else {
                        for (node_id, offset) in changed {
                            self.emit_scroll_offset_settled(node_id, offset, refresh_after_message);
                        }
                    }
                }
            }
            WheelOrScrollRoute::ScrollContainer
        } else {
            WheelOrScrollRoute::NotRouted
        }
    }

    fn finish_scroll_terminal_after_routing(
        &mut self,
        sample: WheelSample,
        refresh_after_message: bool,
    ) -> WheelOrScrollRoute {
        match sample.phase() {
            Some(WheelPhase::Ended) => {
                self.emit_pending_scroll_settlements(refresh_after_message);
            }
            Some(WheelPhase::Cancelled) => {
                self.clear_pending_scroll_settlements();
            }
            _ => {}
        }
        WheelOrScrollRoute::NotRouted
    }

    /// Keep Auto scrollbar activity independent from callback settlement. The
    /// latter is an input contract; this state only controls transient paint.
    fn mark_scroll_activity(
        &mut self,
        changed: &[(crate::layout::NodeId, Vector2)],
        phase: Option<WheelPhase>,
    ) {
        let now = self.timed_repaint_now();
        let idle_deadline = now.checked_add(std::time::Duration::from_millis(100));
        let phaseful = matches!(phase, Some(WheelPhase::Started | WheelPhase::Changed));
        let terminal = matches!(phase, Some(WheelPhase::Ended | WheelPhase::Cancelled));
        if terminal {
            // A phaseful owner remains active while its terminal sample is
            // dispatched, then leaves the visual state at that boundary.
            let had_phaseful = self
                .interaction
                .wheel
                .scroll_activity
                .values()
                .any(Option::is_none);
            self.interaction
                .wheel
                .scroll_activity
                .retain(|_, deadline| deadline.is_some());
            if had_phaseful {
                self.note_scroll_visibility_mutation();
                self.repaint_requested = true;
            }
        }
        if !terminal {
            for &(node_id, _) in changed {
                let deadline = if phaseful { None } else { idle_deadline };
                self.interaction
                    .wheel
                    .scroll_activity
                    .insert(node_id, deadline);
                self.note_scroll_visibility_mutation();
            }
        }
        if !changed.is_empty() {
            self.repaint_requested = true;
        }
    }

    fn clear_phaseful_scroll_activity(&mut self) {
        let had_phaseful = self
            .interaction
            .wheel
            .scroll_activity
            .values()
            .any(Option::is_none);
        if !had_phaseful {
            return;
        }
        self.interaction
            .wheel
            .scroll_activity
            .retain(|_, deadline| deadline.is_some());
        self.note_scroll_visibility_mutation();
        self.repaint_requested = true;
    }

    pub(in crate::runtime::controller) fn scroll_auto_visibility(
        &self,
    ) -> Vec<crate::layout::NodeId> {
        let mut visible = self
            .interaction
            .wheel
            .scroll_activity
            .keys()
            .copied()
            .collect::<Vec<_>>();
        if let Some(node_id) = self.interaction.hover.scroll_viewport {
            visible.push(node_id);
        }
        if let Some(node_id) = self.interaction.hover.scroll_affordance {
            visible.push(node_id);
        }
        if let Some(capture) = self.interaction.pointer.scroll_drag_capture {
            visible.push(capture.node_id);
        }
        visible.sort_unstable();
        visible.dedup();
        visible
    }

    pub(in crate::runtime::controller) fn note_scroll_visibility_mutation(&mut self) {
        if self.interaction.wheel.scroll_visibility_revision_exhausted {
            return;
        }
        let Some(next) = self
            .interaction
            .wheel
            .scroll_visibility_revision
            .checked_add(1)
        else {
            self.interaction.wheel.scroll_visibility_revision_exhausted = true;
            return;
        };
        self.interaction.wheel.scroll_visibility_revision = next;
    }

    fn wheel_input_for_hit_test(
        &self,
        point: Point,
        sample: WheelSample,
        exact_sample: bool,
    ) -> Option<WidgetInput> {
        let delta = if exact_sample {
            sample.delta().to_logical_pixels()?
        } else {
            sample.delta().vector()
        };
        Some(WidgetInput::wheel(point, delta, sample.modifiers()))
    }

    fn wheel_delta_for_scroll(&self, sample: WheelSample, exact_sample: bool) -> Option<Vector2> {
        sample
            .delta()
            .to_logical_pixels()
            .or_else(|| (!exact_sample).then(|| sample.delta().vector()))
    }

    fn dispatch_managed_wheel_sequence(
        &mut self,
        widget_id: WidgetId,
        point: Point,
        sample: WheelSample,
        refresh_after_message: bool,
    ) -> WheelOrScrollRoute {
        let terminal = matches!(
            sample.phase(),
            Some(WheelPhase::Ended | WheelPhase::Cancelled)
        );
        if terminal {
            // Clear controller authority before widget dispatch can project a
            // re-entrant host update that observes the terminal boundary.
            self.clear_managed_wheel_sequence();
        }
        let dispatched = self.dispatch_wheel_to_widget_with_refresh(
            widget_id,
            point,
            sample,
            refresh_after_message,
            true,
        );
        if !terminal
            && let RuntimeManagedWheelSequenceState::Active {
                widget_id: active_widget_id,
            } = self.interaction.wheel.managed_sequence
            && active_widget_id == widget_id
            && !self.managed_wheel_sequence_is_live(widget_id)
        {
            // A synchronous refresh or widget callback may have removed
            // capability or authority. Only an explicit soft transition
            // to Idle is allowed to avoid this hard orphan boundary.
            self.block_managed_wheel_sequence();
        }
        if dispatched.is_some() {
            WheelOrScrollRoute::Widget
        } else {
            WheelOrScrollRoute::NotRouted
        }
    }

    fn cancel_live_managed_wheel_sequence_for_start(
        &mut self,
        point: Point,
        refresh_after_message: bool,
    ) {
        let RuntimeManagedWheelSequenceState::Active { widget_id } =
            self.interaction.wheel.managed_sequence
        else {
            return;
        };
        if !self.managed_wheel_sequence_is_live(widget_id) {
            return;
        }

        // The superseding start must not observe the old authority while its
        // owner processes teardown. The synthetic terminal is owner-only:
        // unlike ordinary routing, it must never fall through to scrolling.
        self.clear_managed_wheel_sequence();
        let cancellation = WheelSample::from_parts(
            WheelDelta::Pixels(Vector2::new(0.0, 0.0)),
            Some(WheelPhase::Cancelled),
            PointerModifiers::default(),
            None,
            None,
        );
        debug_assert!(cancellation.is_valid());
        let _ = self.dispatch_wheel_to_widget_with_refresh(
            widget_id,
            point,
            cancellation,
            refresh_after_message,
            true,
        );
    }

    fn dispatch_wheel_to_widget_with_refresh(
        &mut self,
        widget_id: WidgetId,
        point: Point,
        sample: WheelSample,
        refresh_after_message: bool,
        exact_sample: bool,
    ) -> Option<WheelWidgetDispatch> {
        let bounds = self.layout.rects.get(&widget_id).copied()?;
        let result = if exact_sample {
            self.dispatch_surface_wheel_sample(widget_id, bounds, point, sample)?
        } else {
            let input = WidgetInput::wheel_with_metadata(
                point,
                sample.delta().vector(),
                sample.modifiers(),
                sample.timestamp(),
                sample.sequence_range(),
            );
            (
                self.dispatch_surface_input(widget_id, bounds, input)?,
                false,
            )
        };
        let retained = result.1;
        let dispatch = match self.resolve_widget_dispatch(result.0) {
            crate::runtime::ResolvedWidgetDispatchResult::Message(message) => {
                if refresh_after_message {
                    let outcome = self.dispatch_message(message);
                    self.pending_input_command_outcome.merge(outcome);
                } else {
                    let mut outcome = CommandOutcome::default();
                    self.dispatch_message_inner_deferred_refresh(message, &mut outcome);
                    self.pending_input_command_outcome.merge(outcome);
                }
                WheelWidgetDispatch::Handled { retained }
            }
            crate::runtime::ResolvedWidgetDispatchResult::UnmappedOutput => {
                self.relayout();
                WheelWidgetDispatch::Handled { retained }
            }
            crate::runtime::ResolvedWidgetDispatchResult::NoOutput if retained => {
                WheelWidgetDispatch::RetainedNoOutput
            }
            crate::runtime::ResolvedWidgetDispatchResult::NoOutput => {
                WheelWidgetDispatch::Unhandled
            }
        };
        Some(dispatch)
    }

    fn dispatch_surface_wheel_sample(
        &mut self,
        widget_id: WidgetId,
        bounds: crate::gui::types::Rect,
        position: Point,
        sample: WheelSample,
    ) -> Option<(WidgetDispatchResult<Message>, bool)> {
        if let Some(child_path) = self.traversal.widgets.paths.current.get(&widget_id) {
            self.surface
                .find_widget_mut_at_path(widget_id, child_path)
                .map(|widget| widget.dispatch_wheel_sample(widget_id, bounds, position, sample))
        } else {
            self.surface
                .find_widget_mut(widget_id)
                .map(|widget| widget.dispatch_wheel_sample(widget_id, bounds, position, sample))
        }
    }

    fn wheel_widget_is_unique(&self, widget_id: WidgetId) -> bool {
        let mut found = false;
        for candidate in self.traversal.widgets.wheel.visible() {
            if *candidate != widget_id {
                continue;
            }
            if found {
                return false;
            }
            found = true;
        }
        found
    }

    fn managed_wheel_sequence_is_live(&self, widget_id: WidgetId) -> bool {
        self.wheel_widget_is_unique(widget_id)
            && self.surface_widget(widget_id).is_some_and(|widget| {
                let common = widget.widget_object().common();
                widget.id() == widget_id
                    && !common.state.disabled
                    && !common.state.read_only
                    && (!widget.is_focusable()
                        || self.interaction.focus.focused_widget() == Some(widget_id))
                    && widget.receives_wheel_input()
                    && widget.retains_managed_wheel_sequence()
            })
    }

    pub(in crate::runtime::controller) fn validate_managed_wheel_sequence_authority(
        &mut self,
    ) -> bool {
        match self.interaction.wheel.managed_sequence {
            RuntimeManagedWheelSequenceState::Idle => true,
            RuntimeManagedWheelSequenceState::Blocked
            | RuntimeManagedWheelSequenceState::ScrollClosed => false,
            RuntimeManagedWheelSequenceState::Scroll { .. } => {
                let live = self.scroll_wheel_edit_is_live();
                if !live {
                    self.block_managed_wheel_sequence();
                }
                live
            }
            RuntimeManagedWheelSequenceState::Active { widget_id }
                if self.managed_wheel_sequence_is_live(widget_id) =>
            {
                true
            }
            RuntimeManagedWheelSequenceState::Active { .. } => {
                self.block_managed_wheel_sequence();
                false
            }
        }
    }

    pub(in crate::runtime::controller) fn clear_managed_wheel_sequence_for_widget(
        &mut self,
        widget_id: WidgetId,
    ) {
        if matches!(
            self.interaction.wheel.managed_sequence,
            RuntimeManagedWheelSequenceState::Active {
                widget_id: active_widget_id
            } if active_widget_id == widget_id
        ) {
            self.clear_managed_wheel_sequence();
        }
    }

    fn clear_managed_wheel_sequence(&mut self) {
        self.interaction.wheel.managed_sequence = RuntimeManagedWheelSequenceState::Idle;
    }

    fn take_pending_scroll_settlements(
        &mut self,
    ) -> Vec<(crate::layout::NodeId, crate::gui::types::Vector2)> {
        self.interaction.wheel.scroll_settlement_deadline = None;
        std::mem::take(&mut self.interaction.wheel.pending_scroll_settlement)
    }

    pub(in crate::runtime::controller) fn emit_pending_scroll_settlements(
        &mut self,
        refresh_after_message: bool,
    ) {
        let settlements = self.take_pending_scroll_settlements();
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
    }

    fn clear_pending_scroll_settlements(&mut self) {
        self.interaction.wheel.pending_scroll_settlement.clear();
        self.interaction.wheel.scroll_settlement_deadline = None;
    }

    fn queue_scroll_settlements(&mut self, changed: &[(crate::layout::NodeId, Vector2)]) {
        for &(node_id, offset) in changed {
            if let Some(existing) = self
                .interaction
                .wheel
                .pending_scroll_settlement
                .iter_mut()
                .find(|(id, _)| *id == node_id)
            {
                existing.1 = offset;
            } else {
                self.interaction
                    .wheel
                    .pending_scroll_settlement
                    .push((node_id, offset));
            }
        }
    }

    pub(in crate::runtime::controller) fn block_managed_wheel_sequence(&mut self) {
        self.interaction.wheel.managed_sequence = RuntimeManagedWheelSequenceState::Blocked;
    }

    #[cfg(test)]
    pub(crate) fn wheel_widget_accepts_at(
        &self,
        point: Point,
        delta: Vector2,
        modifiers: PointerModifiers,
    ) -> bool {
        self.wheel_widget_accepts_at_with_metadata(point, delta, modifiers, None)
    }

    pub(crate) fn wheel_widget_accepts_at_with_metadata(
        &self,
        point: Point,
        delta: Vector2,
        modifiers: PointerModifiers,
        _timestamp: Option<crate::gui::input::InputTimestamp>,
    ) -> bool {
        self.wheel_widget_at(point, delta, modifiers).is_some()
    }

    pub(crate) fn can_coalesce_scroll_container_wheel_with_sample(
        &self,
        point: Point,
        sample: WheelSample,
    ) -> bool {
        if self.gesture_owns_pointer_capture()
            || sample.phase() != Some(WheelPhase::Changed)
            || !sample.is_valid()
        {
            return false;
        }

        // An explicit continuation must remain ordered through the controller
        // while any managed owner or orphan boundary is live. The native
        // adapter cannot resolve that authority from a hit test alone.
        if !matches!(
            self.interaction.wheel.managed_sequence,
            RuntimeManagedWheelSequenceState::Idle
        ) {
            return false;
        }

        let Some(input) = self.wheel_input_for_hit_test(point, sample, true) else {
            return false;
        };
        matches!(
            self.wheel_target_at(point, &input),
            Some(WheelHitTarget::ScrollContainer(_))
        )
    }

    fn wheel_widget_at(
        &self,
        point: Point,
        delta: Vector2,
        modifiers: PointerModifiers,
    ) -> Option<WidgetId> {
        let input = WidgetInput::wheel(point, delta, modifiers);
        self.traversal
            .widgets
            .wheel
            .visible()
            .iter()
            .rev()
            .copied()
            .find(|widget_id| {
                self.layout
                    .rects
                    .get(widget_id)
                    .is_some_and(|rect| rect.contains(point))
                    && self.widget_clip_contains_point(*widget_id, point)
                    && self.widget_accepts_pointer_input(*widget_id, &input)
            })
    }

    fn wheel_target_at(&self, point: Point, input: &WidgetInput) -> Option<WheelHitTarget> {
        self.traversal
            .widgets
            .wheel_targets
            .visible()
            .iter()
            .rev()
            .copied()
            .find(|target| match *target {
                WheelHitTarget::Widget(widget_id) => {
                    self.layout
                        .rects
                        .get(&widget_id)
                        .is_some_and(|rect| rect.contains(point))
                        && self.widget_clip_contains_point(widget_id, point)
                        && self.widget_accepts_pointer_input(widget_id, input)
                }
                WheelHitTarget::ScrollContainer(node_id) => {
                    self.scroll_container_accepts_point(node_id, point)
                }
            })
    }
}

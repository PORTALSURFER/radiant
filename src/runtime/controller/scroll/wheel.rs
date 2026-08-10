//! Wheel routing for scrollable and wheel-aware runtime surfaces.

use super::super::CommandOutcome;
use super::{ScrollUpdateMetadata, SurfaceRuntime};
use crate::widgets::{WheelDelta, WheelPhase, WheelSample};
use crate::{
    gui::types::{Point, Vector2},
    runtime::{RuntimeBridge, WheelHitTarget, WidgetDispatchResult},
    widgets::{PointerModifiers, WidgetId, WidgetInput},
};

use super::super::interaction_state::{
    RuntimeManagedWheelSequence, RuntimeManagedWheelSequenceState,
};

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
    /// Route a legacy phase-less logical-pixel wheel input to a widget, then a
    /// scroll container.
    pub fn wheel_or_scroll_at(&mut self, point: Point, delta: Vector2) -> bool {
        self.wheel_or_scroll_at_with_metadata(point, delta, PointerModifiers::default(), None, None)
    }

    /// Route a legacy modified phase-less logical-pixel wheel input.
    pub fn wheel_or_scroll_at_with_modifiers(
        &mut self,
        point: Point,
        delta: Vector2,
        modifiers: PointerModifiers,
    ) -> bool {
        self.wheel_or_scroll_at_with_metadata(point, delta, modifiers, None, None)
    }

    /// Route one exact unit- and phase-qualified wheel sample.
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
    pub fn wheel_or_scroll_at_deferred_refresh_with_sample(
        &mut self,
        point: Point,
        sample: WheelSample,
    ) -> bool {
        self.wheel_or_scroll_route_with_sample(point, sample, false, true)
            != WheelOrScrollRoute::NotRouted
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
        if exact_sample && !sample.is_valid() {
            return WheelOrScrollRoute::NotRouted;
        }

        let continuation = matches!(
            sample.phase(),
            Some(WheelPhase::Changed | WheelPhase::Ended | WheelPhase::Cancelled)
        );
        if continuation {
            if let Some(capture) = self.interaction.wheel.managed_sequence {
                if capture.state != RuntimeManagedWheelSequenceState::Active
                    || !self.managed_wheel_sequence_is_live(capture.widget_id)
                {
                    // A known stale record is ignored and never rebound to the
                    // widget currently under the pointer.
                    self.clear_managed_wheel_sequence();
                    return WheelOrScrollRoute::NotRouted;
                }
                return self.dispatch_managed_wheel_sequence(
                    capture.widget_id,
                    point,
                    sample,
                    refresh_after_message,
                );
            }
        } else {
            self.validate_managed_wheel_sequence_authority();
        }

        let Some(input) = self.wheel_input_for_hit_test(point, sample, exact_sample) else {
            return WheelOrScrollRoute::NotRouted;
        };
        match self.wheel_target_at(point, &input) {
            Some(WheelHitTarget::Widget(widget_id)) => {
                let Some(dispatch) = self.dispatch_wheel_to_widget_with_refresh(
                    widget_id,
                    point,
                    sample,
                    refresh_after_message,
                ) else {
                    return WheelOrScrollRoute::NotRouted;
                };
                let may_install = exact_sample
                    && sample.phase() == Some(WheelPhase::Started)
                    && self.interaction.wheel.managed_sequence.is_none()
                    && dispatch.retained()
                    && self.wheel_widget_is_unique(widget_id)
                    && self.managed_wheel_sequence_is_live(widget_id);
                if may_install {
                    self.interaction.wheel.managed_sequence = Some(RuntimeManagedWheelSequence {
                        widget_id,
                        state: RuntimeManagedWheelSequenceState::Active,
                    });
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
        }
    }

    fn scroll_fallback_for_sample(
        &mut self,
        point: Point,
        sample: WheelSample,
        exact_sample: bool,
        refresh_after_message: bool,
    ) -> WheelOrScrollRoute {
        let Some(delta) = self.wheel_delta_for_scroll(sample, exact_sample) else {
            return WheelOrScrollRoute::NotRouted;
        };
        if self.scroll_at_with_refresh_and_metadata(
            point,
            delta,
            sample.into(),
            refresh_after_message,
        ) {
            WheelOrScrollRoute::ScrollContainer
        } else {
            WheelOrScrollRoute::NotRouted
        }
    }

    fn wheel_input_for_hit_test(
        &self,
        point: Point,
        sample: WheelSample,
        exact_sample: bool,
    ) -> Option<WidgetInput> {
        sample.to_widget_input(point).or_else(|| {
            (!exact_sample).then(|| {
                WidgetInput::wheel_with_metadata(
                    point,
                    sample.delta().vector(),
                    sample.modifiers(),
                    sample.timestamp(),
                    sample.sequence_range(),
                )
            })
        })
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
        );
        if dispatched.is_some() {
            if !terminal && !self.managed_wheel_sequence_is_live(widget_id) {
                self.clear_managed_wheel_sequence();
            }
            WheelOrScrollRoute::Widget
        } else {
            WheelOrScrollRoute::NotRouted
        }
    }

    fn dispatch_wheel_to_widget_with_refresh(
        &mut self,
        widget_id: WidgetId,
        point: Point,
        sample: WheelSample,
        refresh_after_message: bool,
    ) -> Option<WheelWidgetDispatch> {
        let bounds = self.layout.rects.get(&widget_id).copied()?;
        let result = self.dispatch_surface_wheel_sample(widget_id, bounds, point, sample)?;
        let retained = result.1;
        let dispatch = match result.0 {
            WidgetDispatchResult::Message(message) => {
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
            WidgetDispatchResult::UnmappedOutput => {
                self.relayout();
                WheelWidgetDispatch::Handled { retained }
            }
            WidgetDispatchResult::NoOutput if retained => WheelWidgetDispatch::RetainedNoOutput,
            WidgetDispatchResult::NoOutput => WheelWidgetDispatch::Unhandled,
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
                        || self.interaction.focus.focused_widget == Some(widget_id))
                    && widget.receives_wheel_input()
                    && widget.retains_managed_wheel_sequence()
            })
    }

    pub(in crate::runtime::controller) fn validate_managed_wheel_sequence_authority(
        &mut self,
    ) -> bool {
        let Some(capture) = self.interaction.wheel.managed_sequence else {
            return true;
        };
        if capture.state == RuntimeManagedWheelSequenceState::Active
            && self.managed_wheel_sequence_is_live(capture.widget_id)
        {
            true
        } else {
            self.clear_managed_wheel_sequence();
            false
        }
    }

    pub(in crate::runtime::controller) fn clear_managed_wheel_sequence_for_widget(
        &mut self,
        widget_id: WidgetId,
    ) {
        if self
            .interaction
            .wheel
            .managed_sequence
            .is_some_and(|capture| capture.widget_id == widget_id)
        {
            self.clear_managed_wheel_sequence();
        }
    }

    fn clear_managed_wheel_sequence(&mut self) {
        self.interaction.wheel.managed_sequence = None;
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

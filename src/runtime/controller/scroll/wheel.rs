//! Wheel routing for scrollable and wheel-aware runtime surfaces.

use super::super::CommandOutcome;
use super::{ScrollUpdateMetadata, SurfaceRuntime};
use crate::{
    gui::input::{InputSequenceRange, InputTimestamp},
    gui::types::{Point, Vector2},
    runtime::{RuntimeBridge, WheelHitTarget, WidgetDispatchResult},
    widgets::{PointerModifiers, WidgetId, WidgetInput},
};

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

#[derive(Clone, Copy)]
struct WheelInputMetadata {
    modifiers: PointerModifiers,
    timestamp: Option<InputTimestamp>,
    sequence_range: Option<InputSequenceRange>,
}

impl From<WheelInputMetadata> for ScrollUpdateMetadata {
    fn from(metadata: WheelInputMetadata) -> Self {
        Self {
            modifiers: metadata.modifiers,
            timestamp: metadata.timestamp,
            sequence_range: metadata.sequence_range,
        }
    }
}

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    /// Route wheel input to the topmost widget under `point`, then fall back to
    /// scrolling the topmost scroll container under the pointer.
    pub fn wheel_or_scroll_at(&mut self, point: Point, delta: Vector2) -> bool {
        self.wheel_or_scroll_at_with_metadata(point, delta, PointerModifiers::default(), None, None)
    }

    /// Route modified wheel input to the topmost widget under `point`, then
    /// fall back to scrolling the topmost scroll container under the pointer.
    pub fn wheel_or_scroll_at_with_modifiers(
        &mut self,
        point: Point,
        delta: Vector2,
        modifiers: PointerModifiers,
    ) -> bool {
        self.wheel_or_scroll_at_with_metadata(point, delta, modifiers, None, None)
    }

    pub(crate) fn wheel_or_scroll_at_with_metadata(
        &mut self,
        point: Point,
        delta: Vector2,
        modifiers: PointerModifiers,
        timestamp: Option<InputTimestamp>,
        sequence_range: Option<InputSequenceRange>,
    ) -> bool {
        self.wheel_or_scroll_route_with_metadata(
            point,
            delta,
            modifiers,
            timestamp,
            sequence_range,
            true,
        ) != WheelOrScrollRoute::NotRouted
    }

    /// Route wheel input but defer host-surface refresh until the caller chooses
    /// to refresh. This is intended for GPU-backed surfaces whose bounds do not
    /// change during rapid wheel updates.
    pub fn wheel_or_scroll_at_deferred_refresh(&mut self, point: Point, delta: Vector2) -> bool {
        self.wheel_or_scroll_at_deferred_refresh_with_metadata(
            point,
            delta,
            PointerModifiers::default(),
            None,
            None,
        )
    }

    /// Route modified wheel input while deferring host-surface refresh.
    pub fn wheel_or_scroll_at_deferred_refresh_with_modifiers(
        &mut self,
        point: Point,
        delta: Vector2,
        modifiers: PointerModifiers,
    ) -> bool {
        self.wheel_or_scroll_at_deferred_refresh_with_metadata(point, delta, modifiers, None, None)
    }

    pub(crate) fn wheel_or_scroll_at_deferred_refresh_with_metadata(
        &mut self,
        point: Point,
        delta: Vector2,
        modifiers: PointerModifiers,
        timestamp: Option<InputTimestamp>,
        sequence_range: Option<InputSequenceRange>,
    ) -> bool {
        self.wheel_or_scroll_route_deferred_refresh_with_metadata(
            point,
            delta,
            modifiers,
            timestamp,
            sequence_range,
        ) != WheelOrScrollRoute::NotRouted
    }

    /// Route modified wheel input while reporting whether widget handling or
    /// scroll-container fallback accepted it.
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
        timestamp: Option<InputTimestamp>,
        sequence_range: Option<InputSequenceRange>,
    ) -> WheelOrScrollRoute {
        self.wheel_or_scroll_route_with_metadata(
            point,
            delta,
            modifiers,
            timestamp,
            sequence_range,
            false,
        )
    }

    fn wheel_or_scroll_route_with_metadata(
        &mut self,
        point: Point,
        delta: Vector2,
        modifiers: PointerModifiers,
        timestamp: Option<InputTimestamp>,
        sequence_range: Option<InputSequenceRange>,
        refresh_after_message: bool,
    ) -> WheelOrScrollRoute {
        let metadata = WheelInputMetadata {
            modifiers,
            timestamp,
            sequence_range,
        };
        let input = WidgetInput::wheel(point, delta, metadata.modifiers);
        match self.wheel_target_at(point, &input) {
            Some(WheelHitTarget::Widget(widget_id)) => {
                if self.dispatch_wheel_to_widget_with_refresh(
                    widget_id,
                    point,
                    delta,
                    metadata,
                    refresh_after_message,
                ) {
                    WheelOrScrollRoute::Widget
                } else if self.scroll_at_with_refresh_and_metadata(
                    point,
                    delta,
                    metadata.into(),
                    refresh_after_message,
                ) {
                    WheelOrScrollRoute::ScrollContainer
                } else {
                    WheelOrScrollRoute::NotRouted
                }
            }
            Some(WheelHitTarget::ScrollContainer(_)) => {
                if self.scroll_at_with_refresh_and_metadata(
                    point,
                    delta,
                    metadata.into(),
                    refresh_after_message,
                ) {
                    WheelOrScrollRoute::ScrollContainer
                } else {
                    WheelOrScrollRoute::NotRouted
                }
            }
            None => WheelOrScrollRoute::NotRouted,
        }
    }

    fn dispatch_wheel_to_widget_with_refresh(
        &mut self,
        widget_id: WidgetId,
        point: Point,
        delta: Vector2,
        metadata: WheelInputMetadata,
        refresh_after_message: bool,
    ) -> bool {
        let Some(bounds) = self.layout.rects.get(&widget_id).copied() else {
            return false;
        };
        let Some(result) = self.dispatch_surface_input(
            widget_id,
            bounds,
            WidgetInput::wheel_with_metadata(
                point,
                delta,
                metadata.modifiers,
                metadata.timestamp,
                metadata.sequence_range,
            ),
        ) else {
            return false;
        };
        self.capture_pointer_capture_state(widget_id);
        match result {
            WidgetDispatchResult::Message(message) => {
                if refresh_after_message {
                    let outcome = self.dispatch_message(message);
                    self.pending_input_command_outcome.merge(outcome);
                } else {
                    let mut outcome = CommandOutcome::default();
                    self.dispatch_message_inner_deferred_refresh(message, &mut outcome);
                    self.pending_input_command_outcome.merge(outcome);
                }
            }
            WidgetDispatchResult::UnmappedOutput => self.relayout(),
            WidgetDispatchResult::NoOutput => return false,
        }
        true
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
        _timestamp: Option<InputTimestamp>,
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

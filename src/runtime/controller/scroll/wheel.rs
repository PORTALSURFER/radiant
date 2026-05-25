//! Wheel routing for scrollable and wheel-aware runtime surfaces.

use super::super::{CommandOutcome, SurfaceRuntime};
use crate::{
    gui::types::{Point, Vector2},
    runtime::{RuntimeBridge, WidgetDispatchResult},
    widgets::{PointerModifiers, WidgetId, WidgetInput},
};

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    /// Route wheel input to the topmost widget under `point`, then fall back to
    /// scrolling the topmost scroll container under the pointer.
    pub fn wheel_or_scroll_at(&mut self, point: Point, delta: Vector2) -> bool {
        self.wheel_or_scroll_at_with_modifiers(point, delta, PointerModifiers::default())
    }

    /// Route modified wheel input to the topmost widget under `point`, then
    /// fall back to scrolling the topmost scroll container under the pointer.
    pub fn wheel_or_scroll_at_with_modifiers(
        &mut self,
        point: Point,
        delta: Vector2,
        modifiers: PointerModifiers,
    ) -> bool {
        if self.dispatch_wheel_at(point, delta, modifiers) {
            return true;
        }
        self.scroll_at(point, delta)
    }

    /// Route wheel input but defer host-surface refresh until the caller chooses
    /// to refresh. This is intended for GPU-backed surfaces whose bounds do not
    /// change during rapid wheel updates.
    pub fn wheel_or_scroll_at_deferred_refresh(&mut self, point: Point, delta: Vector2) -> bool {
        self.wheel_or_scroll_at_deferred_refresh_with_modifiers(
            point,
            delta,
            PointerModifiers::default(),
        )
    }

    /// Route modified wheel input while deferring host-surface refresh.
    pub fn wheel_or_scroll_at_deferred_refresh_with_modifiers(
        &mut self,
        point: Point,
        delta: Vector2,
        modifiers: PointerModifiers,
    ) -> bool {
        if self.dispatch_wheel_at_with_refresh(point, delta, modifiers, false) {
            return true;
        }
        self.scroll_at(point, delta)
    }

    fn dispatch_wheel_at(
        &mut self,
        point: Point,
        delta: Vector2,
        modifiers: PointerModifiers,
    ) -> bool {
        self.dispatch_wheel_at_with_refresh(point, delta, modifiers, true)
    }

    fn dispatch_wheel_at_with_refresh(
        &mut self,
        point: Point,
        delta: Vector2,
        modifiers: PointerModifiers,
        refresh_after_message: bool,
    ) -> bool {
        let Some(widget_id) = self.wheel_widget_at(point) else {
            return false;
        };
        let Some(bounds) = self.layout.rects.get(&widget_id).copied() else {
            return false;
        };
        let Some(result) = self.dispatch_surface_input(
            widget_id,
            bounds,
            WidgetInput::Wheel {
                position: point,
                delta,
                modifiers,
            },
        ) else {
            return false;
        };
        self.capture_pointer_capture_state(widget_id);
        match result {
            WidgetDispatchResult::Message(message) => {
                if refresh_after_message {
                    self.dispatch_message(message);
                } else {
                    let mut outcome = CommandOutcome::default();
                    self.dispatch_message_inner(message, &mut outcome);
                }
            }
            WidgetDispatchResult::UnmappedOutput => self.relayout(),
            WidgetDispatchResult::NoOutput => return false,
        }
        true
    }

    fn wheel_widget_at(&self, point: Point) -> Option<WidgetId> {
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
            })
    }
}

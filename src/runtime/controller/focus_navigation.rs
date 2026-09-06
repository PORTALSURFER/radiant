//! Explicit traversal through committed focus order and current visible geometry.
use super::{
    FocusDirection, FocusTransferOutcome, FocusTraversal, SurfaceRuntime,
    focus::SequentialFocusTraversalDisposition,
};
use crate::runtime::RuntimeBridge;

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    /// Traverse the current sequence with terminal veto/invalidation outcomes.
    /// Runtime-owned separator stops retain their ordinary position in the order.
    pub fn traverse_focus_explicit(&mut self, direction: FocusTraversal) -> FocusTransferOutcome {
        if !self.lifecycle_accepts_work() {
            return FocusTransferOutcome::Unavailable;
        }
        let generation = self.refresh_counters().runtime_projection;
        let result = self.traverse_focus_with_disposition(direction);
        match result {
            SequentialFocusTraversalDisposition::NoDestination => {
                FocusTransferOutcome::NoDestination
            }
            SequentialFocusTraversalDisposition::Vetoed => FocusTransferOutcome::Vetoed,
            SequentialFocusTraversalDisposition::Invalidated => FocusTransferOutcome::Invalidated,
            _ if generation != self.refresh_counters().runtime_projection => {
                FocusTransferOutcome::Invalidated
            }
            SequentialFocusTraversalDisposition::AdmittedWidget(id) => {
                FocusTransferOutcome::Admitted(id)
            }
            SequentialFocusTraversalDisposition::AdmittedPrivateSplitPaneSeparator => {
                FocusTransferOutcome::AdmittedRuntimeOwned
            }
        }
    }

    /// Move to the nearest visible keyboard target in the requested half-plane.
    /// Ties use committed traversal order. No current widget or candidate returns
    /// `NoDestination`; this does not materialize offscreen virtual content.
    pub fn traverse_focus_spatial(&mut self, direction: FocusDirection) -> FocusTransferOutcome {
        if !self.lifecycle_accepts_work() {
            return FocusTransferOutcome::Unavailable;
        }
        let Some(current) = self.focused_widget() else {
            return FocusTransferOutcome::NoDestination;
        };
        let Some(origin) = self.layout.rects.get(&current).map(|rect| rect.center()) else {
            return FocusTransferOutcome::Invalidated;
        };
        if !origin.is_finite() {
            return FocusTransferOutcome::Invalidated;
        }
        let mut best: Option<(f64, u64)> = None;
        for &id in self.traversal.widgets.keyboard_focus.order() {
            if id == current || !self.is_live_focus_target(id) {
                continue;
            }
            let Some(rect) = self.layout.rects.get(&id) else {
                continue;
            };
            let center = rect.center();
            let viewport = crate::gui::types::Rect::from_size(
                self.context().viewport.width(),
                self.context().viewport.height(),
            );
            let mut visible = rect.intersection(viewport);
            if let Some(clips) = self.traversal.widgets.paths.clip_ancestors.get(&id) {
                for clip in clips.as_slice() {
                    visible = visible.and_then(|bounds| {
                        self.layout
                            .rects
                            .get(clip)
                            .and_then(|clip| bounds.intersection(*clip))
                    });
                }
            }
            if !center.is_finite()
                || !visible.is_some_and(|bounds| bounds.width() > 0.0 && bounds.height() > 0.0)
            {
                continue;
            }
            let dx = f64::from(center.x) - f64::from(origin.x);
            let dy = f64::from(center.y) - f64::from(origin.y);
            let eligible = match direction {
                FocusDirection::Left => dx < 0.0,
                FocusDirection::Right => dx > 0.0,
                FocusDirection::Up => dy < 0.0,
                FocusDirection::Down => dy > 0.0,
            };
            let distance = dx * dx + dy * dy;
            if eligible && best.is_none_or(|(score, _)| distance < score) {
                best = Some((distance, id));
            }
        }
        let Some((_, id)) = best else {
            return FocusTransferOutcome::NoDestination;
        };
        let Some(target) = self.focus_target(id) else {
            return FocusTransferOutcome::Invalidated;
        };
        self.transfer_focus(&target)
    }
}

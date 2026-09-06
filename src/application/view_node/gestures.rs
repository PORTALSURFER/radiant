//! Declarative container recognition without synthetic pointer hit regions.
use super::ViewNode;
use crate::{
    layout::{
        LayoutGestures, LayoutInteraction, LayoutInteractionCapabilities, LayoutInteractionRevision,
    },
    widgets::{GestureEvent, GesturePolicy},
};
use std::rc::Rc;

struct GestureBinding<Message> {
    policy: GesturePolicy,
    revision: LayoutInteractionRevision,
    map: Box<dyn Fn(GestureEvent) -> Option<Message>>,
}
impl<Message> LayoutInteraction<Message> for GestureBinding<Message> {
    fn revision(&self) -> LayoutInteractionRevision {
        LayoutInteractionRevision::exact(())
    }
    fn capabilities_v2(&self) -> LayoutInteractionCapabilities<'_, Message> {
        LayoutInteractionCapabilities::none().with_gestures(self)
    }
}
impl<Message> LayoutGestures<Message> for GestureBinding<Message> {
    fn revision(&self) -> LayoutInteractionRevision {
        self.revision.clone()
    }
    fn policy(&self) -> GesturePolicy {
        self.policy
    }
    fn dispatch(&self, event: GestureEvent) -> Option<Message> {
        (self.map)(event)
    }
}
impl<Message: 'static> ViewNode<Message> {
    /// Wrap this subtree in a gesture region with conservative mapper evidence.
    /// A rebuild retires its sequence. Ordinary pointer hits still reach children.
    /// Recognition currently requires a hit-testable descendant at the anchor.
    pub fn on_gesture(
        self,
        policy: GesturePolicy,
        map: impl Fn(GestureEvent) -> Option<Message> + 'static,
    ) -> Self {
        self.with_gesture_binding(policy, LayoutInteractionRevision::conservative(), map)
    }
    /// Wrap this subtree in a gesture region with exact mapper evidence.
    /// Equal revisions promise equivalent callback behavior across rebuilds;
    /// include every captured value affecting the callback in `revision`.
    /// The deepest candidate crossing its threshold on a sample wins once.
    pub fn on_gesture_with_revision<Revision: Eq + 'static>(
        self,
        policy: GesturePolicy,
        revision: Revision,
        map: impl Fn(GestureEvent) -> Option<Message> + 'static,
    ) -> Self {
        self.with_gesture_binding(policy, LayoutInteractionRevision::exact(revision), map)
    }
    fn with_gesture_binding(
        self,
        policy: GesturePolicy,
        revision: LayoutInteractionRevision,
        map: impl Fn(GestureEvent) -> Option<Message> + 'static,
    ) -> Self {
        let mut region = crate::application::column([self]).spacing(0.0);
        region.gesture_interaction = Some(Rc::new(GestureBinding {
            policy,
            revision,
            map: Box::new(map),
        }));
        region
    }
}

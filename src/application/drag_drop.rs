//! Typed declarative contracts lowered into the existing interaction descriptor.
use crate::{
    gui::drag_drop::*,
    layout::{
        LayoutDragSource, LayoutDropTarget, LayoutGestures, LayoutInteraction,
        LayoutInteractionCapabilities, LayoutInteractionRevision,
    },
    widgets::{GestureEvent, GesturePolicy},
};
use std::{any::TypeId, rc::Rc};
type SourceMapper<T, Message> = Rc<dyn Fn(DragSourceEvent<T>) -> Option<Message>>;
type TargetMapper<T, Message> = Rc<dyn Fn(DropEvent<T>) -> Option<Message>>;
type TargetNegotiator<T> = Rc<dyn Fn(&T, DragEventContext) -> DropDecision>;

/// Immutable typed drag source attached to an ordinary view.
/// Equal payload and callback revisions must describe equivalent behavior.
pub struct DragSource<T, Message> {
    payload: Rc<T>,
    operations: DragOperations,
    preview: DragPreviewInfo,
    threshold: f32,
    map: Option<SourceMapper<T, Message>>,
    map_revision: LayoutInteractionRevision,
}
#[derive(PartialEq, Eq)]
struct SourceRevision<T: Eq> {
    payload: Rc<T>,
    operations: DragOperations,
    preview: DragPreviewInfo,
    threshold: u32,
    map: LayoutInteractionRevision,
}
impl<T: Eq + 'static, Message: 'static> DragSource<T, Message> {
    /// Declare an immutable payload with copy semantics and a six-pixel threshold.
    pub fn new(payload: T) -> Self {
        Self {
            payload: Rc::new(payload),
            operations: DragOperations::default(),
            preview: DragPreviewInfo::default(),
            threshold: 6.0,
            map: None,
            map_revision: LayoutInteractionRevision::exact(()),
        }
    }
    /// Replace the allowed operation set.
    pub fn operations(mut self, operations: DragOperations) -> Self {
        self.operations = operations;
        self
    }
    /// Replace the checked transient preview metadata.
    pub fn preview(mut self, preview: DragPreviewInfo) -> Self {
        self.preview = preview;
        self
    }
    /// Configure the finite logical recognition threshold.
    pub fn recognize_after(mut self, threshold: f32) -> Result<Self, DragDescriptorError> {
        if !threshold.is_finite() || threshold < 0.0 {
            return Err(DragDescriptorError::InvalidThreshold);
        }
        self.threshold = threshold;
        Ok(self)
    }
    /// Map lifecycle events conservatively. Reprojection retires the source.
    pub fn on_event(
        mut self,
        map: impl Fn(DragSourceEvent<T>) -> Option<Message> + 'static,
    ) -> Self {
        self.map = Some(Rc::new(map));
        self.map_revision = LayoutInteractionRevision::conservative();
        self
    }
    /// Map lifecycle events with exact equality evidence for captured behavior.
    pub fn on_event_with_revision<Revision: Eq + 'static>(
        mut self,
        revision: Revision,
        map: impl Fn(DragSourceEvent<T>) -> Option<Message> + 'static,
    ) -> Self {
        self.map = Some(Rc::new(map));
        self.map_revision = LayoutInteractionRevision::exact(revision);
        self
    }
    fn evidence(&self) -> LayoutInteractionRevision {
        if !self.map_revision.is_exact() {
            return LayoutInteractionRevision::conservative();
        }
        LayoutInteractionRevision::exact(SourceRevision {
            payload: self.payload.clone(),
            operations: self.operations,
            preview: self.preview.clone(),
            threshold: self.threshold.to_bits(),
            map: self.map_revision.clone(),
        })
    }
}
impl<T: Eq + 'static, Message: 'static> LayoutInteraction<Message> for DragSource<T, Message> {
    fn revision(&self) -> LayoutInteractionRevision {
        LayoutInteractionRevision::exact(())
    }
    fn capabilities_v2(&self) -> LayoutInteractionCapabilities<'_, Message> {
        LayoutInteractionCapabilities::none()
            .with_gestures(self)
            .with_drag_source(self)
    }
}
impl<T: Eq + 'static, Message: 'static> LayoutGestures<Message> for DragSource<T, Message> {
    fn revision(&self) -> LayoutInteractionRevision {
        self.evidence()
    }
    fn policy(&self) -> GesturePolicy {
        GesturePolicy::none()
            .recognize(
                crate::gui::pointer_ingress::GestureKind::Pan,
                self.threshold,
            )
            .unwrap_or_default()
    }
    // The controller consumes the source facet, preserving one mapper per event.
    fn dispatch(&self, _: GestureEvent) -> Option<Message> {
        None
    }
}
impl<T: Eq + 'static, Message: 'static> LayoutDragSource<Message> for DragSource<T, Message> {
    fn revision(&self) -> LayoutInteractionRevision {
        self.evidence()
    }
    fn offer(&self) -> DragOffer {
        DragOffer::new(self.payload.clone(), self.operations, self.preview.clone())
    }
    fn dispatch(
        &self,
        offer: &DragOffer,
        context: DragEventContext,
        phase: DragSourcePhase,
    ) -> Option<Message> {
        let payload = offer.payload::<T>()?;
        self.map.as_ref()?(DragSourceEvent {
            payload,
            context,
            phase,
        })
    }
}

/// Typed drop negotiation and lifecycle mapping attached to an ordinary view.
pub struct DropTarget<T, Message> {
    operations: DragOperations,
    negotiate: Option<TargetNegotiator<T>>,
    policy_revision: LayoutInteractionRevision,
    map: Option<TargetMapper<T, Message>>,
    map_revision: LayoutInteractionRevision,
}
#[derive(PartialEq, Eq)]
struct TargetRevision {
    payload: TypeId,
    operations: DragOperations,
    policy: LayoutInteractionRevision,
    map: LayoutInteractionRevision,
}
impl<T: 'static, Message: 'static> DropTarget<T, Message> {
    /// Accept this concrete payload type, preferring copy, then move, then link.
    pub fn new() -> Self {
        Self {
            operations: DragOperations::all(),
            negotiate: None,
            map: None,
            policy_revision: LayoutInteractionRevision::exact(()),
            map_revision: LayoutInteractionRevision::exact(()),
        }
    }
    /// Restrict the target's allowed operations.
    pub fn operations(mut self, operations: DragOperations) -> Self {
        self.operations = operations;
        self
    }
    /// Observe current payload/context through a pure, exactly revisioned predicate.
    pub fn negotiate_with_revision<Revision: Eq + 'static>(
        mut self,
        revision: Revision,
        negotiate: impl Fn(&T, DragEventContext) -> DropDecision + 'static,
    ) -> Self {
        self.negotiate = Some(Rc::new(negotiate));
        self.policy_revision = LayoutInteractionRevision::exact(revision);
        self
    }
    /// Map target events conservatively; changed projections retire hover authority.
    pub fn on_event(mut self, map: impl Fn(DropEvent<T>) -> Option<Message> + 'static) -> Self {
        self.map = Some(Rc::new(map));
        self.map_revision = LayoutInteractionRevision::conservative();
        self
    }
    /// Map target events with exact equality evidence for every captured behavior value.
    pub fn on_event_with_revision<Revision: Eq + 'static>(
        mut self,
        revision: Revision,
        map: impl Fn(DropEvent<T>) -> Option<Message> + 'static,
    ) -> Self {
        self.map = Some(Rc::new(map));
        self.map_revision = LayoutInteractionRevision::exact(revision);
        self
    }
}
impl<T: 'static, Message: 'static> Default for DropTarget<T, Message> {
    fn default() -> Self {
        Self::new()
    }
}
impl<T: 'static, Message: 'static> LayoutInteraction<Message> for DropTarget<T, Message> {
    fn revision(&self) -> LayoutInteractionRevision {
        LayoutInteractionRevision::exact(())
    }
    fn capabilities_v2(&self) -> LayoutInteractionCapabilities<'_, Message> {
        LayoutInteractionCapabilities::none().with_drop_target(self)
    }
}
impl<T: 'static, Message: 'static> LayoutDropTarget<Message> for DropTarget<T, Message> {
    fn revision(&self) -> LayoutInteractionRevision {
        if !self.map_revision.is_exact() || !self.policy_revision.is_exact() {
            return LayoutInteractionRevision::conservative();
        }
        LayoutInteractionRevision::exact(TargetRevision {
            payload: TypeId::of::<T>(),
            operations: self.operations,
            policy: self.policy_revision.clone(),
            map: self.map_revision.clone(),
        })
    }
    fn accepts_payload(&self, offer: &DragOffer) -> bool {
        offer.is::<T>()
    }
    fn negotiate(&self, offer: &DragOffer, context: DragEventContext) -> DropDecision {
        let Some(payload) = offer.payload::<T>() else {
            return DropDecision::Rejected;
        };
        let decision = self.negotiate.as_ref().map_or_else(
            || {
                [
                    DragOperation::Copy,
                    DragOperation::Move,
                    DragOperation::Link,
                ]
                .into_iter()
                .find(|operation| {
                    self.operations.contains(*operation) && offer.operations().contains(*operation)
                })
                .map_or(DropDecision::Rejected, DropDecision::Accepted)
            },
            |negotiate| negotiate(&payload, context),
        );
        if let DropDecision::Accepted(operation) = decision
            && (!self.operations.contains(operation) || !offer.operations().contains(operation))
        {
            DropDecision::Rejected
        } else {
            decision
        }
    }
    fn dispatch(
        &self,
        offer: &DragOffer,
        context: DragEventContext,
        phase: DropPhase,
        decision: DropDecision,
    ) -> Option<Message> {
        let payload = offer.payload::<T>()?;
        self.map.as_ref()?(DropEvent {
            payload,
            context,
            phase,
            decision,
        })
    }
}
impl<Message: 'static> super::ViewNode<Message> {
    /// Attach a typed source to this subtree using the shared gesture/capture lifecycle.
    pub fn drag_source<T: Eq + 'static>(self, source: DragSource<T, Message>) -> Self {
        self.interaction_region(Rc::new(source))
    }
    /// Attach a typed drop target without intercepting ordinary child pointer input.
    pub fn drop_target<T: 'static>(self, target: DropTarget<T, Message>) -> Self {
        self.interaction_region(Rc::new(target))
    }
}

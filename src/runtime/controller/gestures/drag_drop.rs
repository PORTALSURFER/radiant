//! Typed offer metadata piggybacks on the existing gesture capture owner.
use super::*;
use crate::{
    gui::drag_drop::*,
    layout::{LayoutInteraction, LayoutInteractionRevision},
    runtime::{DragPreview, DragRequest, drag::DragSession},
};
use std::rc::Rc;

pub(in crate::runtime::controller) struct TypedDragSession<Message> {
    token: GestureSequenceToken,
    source: GestureTarget,
    handler: Rc<dyn LayoutInteraction<Message>>,
    offer: DragOffer,
    position: Point,
    modifiers: crate::widgets::PointerModifiers,
    target: Option<DropBinding<Message>>,
}
struct DropBinding<Message> {
    id: WidgetId,
    path: crate::runtime::WidgetPath,
    handler: Rc<dyn LayoutInteraction<Message>>,
    revision: LayoutInteractionRevision,
    root_revision: LayoutInteractionRevision,
    policy: crate::layout::ContainerPolicy,
    contract_version: u16,
    generation: u64,
    decision: DropDecision,
}
impl<Message> TypedDragSession<Message> {
    fn context(&self, target: Option<WidgetId>) -> DragEventContext {
        DragEventContext {
            token: DragSessionToken::new(self.token.0),
            source: self.source.id,
            target,
            position: self.position,
            modifiers: self.modifiers,
        }
    }
    fn source_message(&self, phase: DragSourcePhase) -> Option<Message> {
        self.handler.capabilities_v2().drag_source()?.dispatch(
            &self.offer,
            self.context(self.target.as_ref().map(|target| target.id)),
            phase,
        )
    }
    fn target_message(&self, target: &DropBinding<Message>, phase: DropPhase) -> Option<Message> {
        target.handler.capabilities_v2().drop_target()?.dispatch(
            &self.offer,
            self.context(Some(target.id)),
            phase,
            target.decision,
        )
    }
}
impl<Message, Bridge> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(super) fn is_drag_source(&self, target: &GestureTarget) -> bool {
        !target.is_widget()
            && self
                .surface
                .find_container_at_path(&target.path)
                .and_then(|container| container.revision().layout_capabilities)
                .and_then(|capabilities| capabilities.interaction.as_ref())
                .is_some_and(|interaction| interaction.capabilities_v2().drag_source().is_some())
    }
    fn typed_drag_live(&self, token: GestureSequenceToken) -> bool {
        self.interaction
            .drag
            .typed
            .as_ref()
            .is_some_and(|session| session.token == token)
            && self
                .interaction
                .gesture
                .as_ref()
                .is_some_and(|capture| capture.token == token)
    }
    fn typed_drag_message(&mut self, message: Option<Message>) {
        if let Some(message) = message {
            let outcome = self.dispatch_message(message);
            self.pending_input_command_outcome.merge(outcome);
        }
    }
    pub(super) fn deliver_typed_drag(
        &mut self,
        target: &GestureTarget,
        event: GestureEvent,
    ) -> bool {
        let Some(token) = self
            .interaction
            .gesture
            .as_ref()
            .map(|capture| capture.token)
        else {
            return false;
        };
        let position = Point::new(
            event.anchor.x + event.accumulated.x,
            event.anchor.y + event.accumulated.y,
        );
        if !position.is_finite() {
            self.cancel_gesture_capture(GestureCancellation::InvalidSample);
            return true;
        }
        if event.phase == GesturePhase::Started {
            let Some(handler) = self
                .surface
                .find_container_at_path(&target.path)
                .and_then(|container| container.revision().layout_capabilities)
                .and_then(|capabilities| capabilities.interaction.clone())
            else {
                return false;
            };
            let Some(source) = handler.capabilities_v2().drag_source() else {
                return false;
            };
            let offer = source.offer();
            let preview = DragPreview::sized(offer.preview().label(), offer.preview().size());
            self.interaction.drag.session =
                Some(DragSession::new(DragRequest::new(preview, position)));
            self.interaction.drag.typed = Some(TypedDragSession {
                token,
                source: target.clone(),
                handler,
                offer,
                position,
                modifiers: event.sample.modifiers(),
                target: None,
            });
            self.repaint_requested = true;
            let message = self
                .interaction
                .drag
                .typed
                .as_ref()
                .and_then(|session| session.source_message(DragSourcePhase::Started));
            self.typed_drag_message(message);
        } else if let Some(session) = self
            .interaction
            .drag
            .typed
            .as_mut()
            .filter(|session| session.token == token)
        {
            session.position = position;
            session.modifiers = event.sample.modifiers();
            if let Some(preview) = self.interaction.drag.session.as_mut() {
                preview.pointer = position;
                preview.visible = true;
            }
            self.repaint_requested = true;
        } else {
            return false;
        }
        if !self.typed_drag_live(token) {
            return true;
        }
        self.refresh_drop_target(token);
        if !self.typed_drag_live(token) {
            return true;
        }
        if event.phase == GesturePhase::Ended {
            // Detach both capture and payload before either terminal mapper runs.
            self.interaction.gesture = None;
            let messages = self.take_typed_drag_terminal(None);
            for message in messages {
                self.typed_drag_message(Some(message));
            }
        } else if event.phase != GesturePhase::Started {
            let message = self
                .interaction
                .drag
                .typed
                .as_ref()
                .and_then(|session| session.source_message(DragSourcePhase::Moved));
            self.typed_drag_message(message);
        }
        true
    }
    fn drop_binding_matches(
        &self,
        binding: &DropBinding<Message>,
        surface: &crate::runtime::UiSurface<Message>,
        same_projection: bool,
    ) -> bool {
        surface
            .find_container_at_path(&binding.path)
            .is_some_and(|container| {
                let revision = container.revision();
                container.node_id() == binding.id
                    && revision.layout_policy.is_none()
                    && *revision.policy == binding.policy
                    && revision.layout_capabilities.is_some_and(|capabilities| {
                        capabilities.contract_version == binding.contract_version
                            && capabilities
                                .interaction
                                .as_ref()
                                .is_some_and(|interaction| {
                                    let facets = interaction.capabilities_v2();
                                    facets.drop_target().is_some()
                                        && ((binding.root_revision.is_exact()
                                            && binding.root_revision == interaction.revision())
                                            || (!binding.root_revision.is_exact()
                                                && same_projection))
                                        && ((binding.revision.is_exact()
                                            && binding.revision == facets.revision_evidence())
                                            || (!binding.revision.is_exact() && same_projection))
                                })
                    })
            })
    }
    fn current_drop_target(
        &self,
        session: &TypedDragSession<Message>,
    ) -> Option<DropBinding<Message>> {
        let position = session.position;
        if !self.viewport.contains(position)
            || self.layout_target_at(position).is_some()
            || self.scroll_affordance_at(position).is_some()
        {
            return None;
        }
        let hit = self
            .widget_at_for_input(position, &WidgetInput::pointer_move(position))
            .and_then(|id| self.traversal.widgets.paths.current.get(&id));
        let mut candidates = Vec::new();
        for record in &self.traversal.containers.layout_interactions {
            let facets = record.interaction.capabilities_v2();
            let Some(target) = facets.drop_target() else {
                continue;
            };
            if !record.gesture_qualified {
                return None;
            }
            let Some(bounds) = self.layout.rects.get(&record.id) else {
                continue;
            };
            if !bounds.has_finite_positive_area() || !bounds.contains(position) {
                continue;
            }
            if !self
                .traversal
                .containers
                .layout_clip_for_container(record.id, &self.layout)
                .all(|clip| clip.contains(position))
            {
                continue;
            }
            // An unrelated, later painted child occludes this declared region.
            if hit.is_some_and(|path| {
                !path.as_slice().starts_with(record.path.as_slice())
                    && path.as_slice() > record.path.as_slice()
            }) {
                continue;
            }
            if !target.accepts_payload(&session.offer) {
                continue;
            }
            if candidates.len() == 64 {
                return None;
            }
            let Some(container) = self.surface.find_container_at_path(&record.path) else {
                continue;
            };
            if container.revision().layout_policy.is_some() {
                continue;
            }
            candidates.push((record, container));
        }
        let (record, container) = candidates
            .into_iter()
            .max_by(|(left, _), (right, _)| left.path.as_slice().cmp(right.path.as_slice()))?;
        let facets = record.interaction.capabilities_v2();
        let decision = facets
            .drop_target()?
            .negotiate(&session.offer, session.context(Some(record.id)));
        let decision = match decision {
            DropDecision::Accepted(operation)
                if !session.offer.operations().contains(operation) =>
            {
                DropDecision::Rejected
            }
            other => other,
        };
        Some(DropBinding {
            id: record.id,
            path: record.path.clone(),
            handler: record.interaction.clone(),
            revision: facets.revision_evidence(),
            root_revision: record.revision.clone(),
            policy: container.revision().policy.clone(),
            contract_version: record.contract_version,
            generation: self.refresh_counters().runtime_projection,
            decision,
        })
    }
    fn refresh_drop_target(&mut self, token: GestureSequenceToken) {
        let candidate = self
            .interaction
            .drag
            .typed
            .as_ref()
            .and_then(|session| self.current_drop_target(session));
        let same = self
            .interaction
            .drag
            .typed
            .as_ref()
            .and_then(|session| session.target.as_ref())
            .zip(candidate.as_ref())
            .is_some_and(|(old, new)| {
                old.id == new.id
                    && old.path == new.path
                    && self.drop_binding_matches(
                        old,
                        &self.surface,
                        old.generation == self.refresh_counters().runtime_projection,
                    )
            });
        if same {
            let message = self.interaction.drag.typed.as_mut().and_then(|session| {
                session.target = candidate;
                session
                    .target
                    .as_ref()
                    .and_then(|target| session.target_message(target, DropPhase::Over))
            });
            self.typed_drag_message(message);
            return;
        }
        let message = self.interaction.drag.typed.as_mut().and_then(|session| {
            let old = session.target.take()?;
            session.target_message(&old, DropPhase::Left)
        });
        self.typed_drag_message(message);
        if !self.typed_drag_live(token) {
            return;
        }
        // Leaving may rebuild the surface. Obtain the next target afresh.
        let candidate = self
            .interaction
            .drag
            .typed
            .as_ref()
            .and_then(|session| self.current_drop_target(session));
        let message = self.interaction.drag.typed.as_mut().and_then(|session| {
            session.target = candidate;
            session
                .target
                .as_ref()
                .and_then(|target| session.target_message(target, DropPhase::Entered))
        });
        self.typed_drag_message(message);
    }
    pub(super) fn take_typed_drag_terminal(
        &mut self,
        reason: Option<DragCancelReason>,
    ) -> Vec<Message> {
        let Some(session) = self.interaction.drag.typed.take() else {
            return Vec::new();
        };
        self.interaction.drag.session = None;
        self.repaint_requested = true;
        let current_target = reason
            .is_none()
            .then(|| self.current_drop_target(&session))
            .flatten();
        let accepted = reason
            .is_none()
            .then_some(session.target.as_ref())
            .flatten()
            .filter(|target| {
                current_target.as_ref().is_some_and(|current| {
                    current.id == target.id
                        && current.path == target.path
                        && current.decision == target.decision
                }) && self.drop_binding_matches(
                    target,
                    &self.surface,
                    target.generation == self.refresh_counters().runtime_projection,
                )
            })
            .and_then(|target| match target.decision {
                DropDecision::Accepted(operation) => Some(operation),
                _ => None,
            });
        let mut messages = Vec::with_capacity(2);
        if let Some(target) = &session.target {
            let phase = if accepted.is_some() {
                DropPhase::Dropped
            } else {
                DropPhase::Cancelled
            };
            if let Some(message) = session.target_message(target, phase) {
                messages.push(message);
            }
        }
        let phase = accepted.map_or_else(
            || DragSourcePhase::Cancelled(reason.unwrap_or(DragCancelReason::NoTarget)),
            DragSourcePhase::Completed,
        );
        if let Some(message) = session.source_message(phase) {
            messages.push(message);
        }
        messages
    }
    pub(super) fn reconcile_drop_before_surface_replace(
        &mut self,
        next: &crate::runtime::UiSurface<Message>,
        messages: &mut Vec<Message>,
    ) {
        let retire = self
            .interaction
            .drag
            .typed
            .as_ref()
            .and_then(|session| session.target.as_ref())
            .is_some_and(|target| {
                !next.gesture_source_is_unambiguous()
                    || !self.drop_binding_matches(target, next, false)
            });
        if retire
            && let Some(session) = self.interaction.drag.typed.as_mut()
            && let Some(target) = session.target.take()
            && let Some(message) = session.target_message(&target, DropPhase::Left)
        {
            messages.push(message);
        }
    }
}

//! Typed offer metadata piggybacks on the existing gesture capture owner.
use super::*;
use crate::{
    gui::drag_drop::*,
    layout::{LayoutInteraction, LayoutInteractionRevision},
    runtime::{DragPreview, DragRequest, ScrollUpdateMetadata, drag::DragSession},
};
use std::{
    rc::{Rc, Weak},
    time::{Duration, Instant},
};

const DRAG_AUTOSCROLL_TICK: Duration = Duration::from_millis(16);
const DRAG_AUTOSCROLL_MAX_ELAPSED: Duration = Duration::from_millis(50);

pub(in crate::runtime::controller) struct TypedDragSession<Message> {
    token: GestureSequenceToken,
    source: GestureTarget,
    handler: Rc<dyn LayoutInteraction<Message>>,
    offer: DragOffer,
    position: Point,
    modifiers: crate::widgets::PointerModifiers,
    metadata: ScrollUpdateMetadata,
    target: Option<DropBinding<Message>>,
    autoscroll: Option<DragAutoscrollPolicy>,
    autoscroll_deadline: Option<Instant>,
    autoscroll_last_tick: Option<Instant>,
    local_target_suppressed: bool,
    cross_window_lease: Rc<CrossWindowDragLease>,
}

/// Private identity for one exported in-process typed drag.  The gesture
/// sequence token already includes the runtime identity, so a native host must
/// not manufacture or supplement it with a device-local identifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CrossWindowDragKey(GestureSequenceToken);

/// Per-native-sample routing evidence supplied by the same-app host.  It has
/// no lifetime beyond one admitted input route and never grants capture.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct CrossWindowInputHint {
    suppress_local_target: bool,
}

/// Mutable terminal slots carried only through one pointer ingress route.
pub(in crate::runtime::controller) type CrossWindowPointerIngress<'a, Message> = (
    CrossWindowInputHint,
    &'a mut Option<CrossWindowTerminalRequest<Message>>,
    &'a mut Option<CrossWindowDragKey>,
);

impl CrossWindowInputHint {
    pub(crate) const fn local() -> Self {
        Self {
            suppress_local_target: false,
        }
    }

    pub(crate) const fn foreign_or_none() -> Self {
        Self {
            suppress_local_target: true,
        }
    }

    pub(crate) const fn suppresses_local_target(self) -> bool {
        self.suppress_local_target
    }
}

/// A source-held lifetime witness.  Receivers retain only a weak reference;
/// this is deliberately neither Send nor a capability to route input.
pub(crate) struct CrossWindowDragLease;
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
    feedback: Option<DropTargetFeedback>,
    feedback_layout: Option<crate::gui::layout_core::LayoutInputEvidence>,
    insertion: Option<DropInsertion>,
}

/// Data-only export for the native same-app coordinator.  It deliberately
/// carries no capture, receiver coordinates, or mutable routing authority.
pub(crate) struct CrossWindowDragExport {
    key: CrossWindowDragKey,
    offer: DragOffer,
    source: WidgetId,
    modifiers: crate::widgets::PointerModifiers,
    position: Point,
    lease: Weak<CrossWindowDragLease>,
    proof: CrossWindowSourceProof,
}

impl CrossWindowDragExport {
    pub(crate) const fn key(&self) -> CrossWindowDragKey {
        self.key
    }

    pub(crate) fn offer(&self) -> DragOffer {
        self.offer.clone()
    }

    pub(crate) const fn source(&self) -> WidgetId {
        self.source
    }

    /// Modifiers captured on the same admitted source sample as this export.
    pub(crate) const fn modifiers(&self) -> crate::widgets::PointerModifiers {
        self.modifiers
    }

    /// Logical source-surface position captured by the admitted sample.
    pub(crate) const fn position(&self) -> Point {
        self.position
    }

    pub(crate) fn lease(&self) -> Weak<CrossWindowDragLease> {
        self.lease.clone()
    }

    pub(crate) fn source_proof(&self) -> CrossWindowSourceProof {
        self.proof.clone()
    }

    pub(crate) fn is_live(&self) -> bool {
        self.lease.upgrade().is_some()
    }
}

/// Frozen source compatibility evidence.  A terminal request owns its lease,
/// so post-release freshness is established from this proof and the allocator
/// snapshot rather than from a retained gesture capture.
#[derive(Clone)]
pub(crate) struct CrossWindowSourceProof {
    key: CrossWindowDragKey,
    target: GestureTarget,
    projection: u64,
    allocator: crate::gui::pointer_ingress::PointerSequenceAllocator,
}

/// Input supplied by the native coordinator after it has resolved a fresh
/// receiving window-local point.  A source position is never reused here.
pub(crate) struct CrossWindowForeignInput {
    key: CrossWindowDragKey,
    offer: DragOffer,
    source: WidgetId,
    lease: Weak<CrossWindowDragLease>,
    position: Point,
    modifiers: crate::widgets::PointerModifiers,
}

impl CrossWindowForeignInput {
    pub(crate) fn new(
        key: CrossWindowDragKey,
        offer: DragOffer,
        source: WidgetId,
        lease: Weak<CrossWindowDragLease>,
        position: Point,
        modifiers: crate::widgets::PointerModifiers,
    ) -> Self {
        Self {
            key,
            offer,
            source,
            lease,
            position,
            modifiers,
        }
    }
}

/// Receiver-local transient state.  It intentionally has no GestureCapture.
pub(in crate::runtime::controller) struct CrossWindowForeignDrag<Message> {
    key: CrossWindowDragKey,
    offer: DragOffer,
    source: WidgetId,
    lease: Weak<CrossWindowDragLease>,
    position: Point,
    modifiers: crate::widgets::PointerModifiers,
    target: Option<DropBinding<Message>>,
    preview: DragSession,
}

/// Messages mapped by a receiver-local foreign update.  The host, not this
/// controller, reduces them under the receiving window's owner.
pub(crate) struct CrossWindowForeignRoute<Message> {
    messages: Vec<Message>,
    repaint: bool,
    needs_transition: bool,
}

impl<Message> CrossWindowForeignRoute<Message> {
    fn empty() -> Self {
        Self {
            messages: Vec::new(),
            repaint: false,
            needs_transition: false,
        }
    }

    pub(crate) fn into_messages(self) -> Vec<Message> {
        self.messages
    }

    pub(crate) const fn needs_transition(&self) -> bool {
        self.needs_transition
    }
}

/// One-shot terminal authority extracted from a currently admitted source
/// release.  It is not stored in controller state and cannot be cloned.
pub(crate) struct CrossWindowTerminalRequest<Message> {
    proof: CrossWindowSourceProof,
    session: TypedDragSession<Message>,
}

impl<Message> CrossWindowTerminalRequest<Message> {
    pub(crate) fn key(&self) -> CrossWindowDragKey {
        self.proof.key
    }

    pub(crate) fn source_proof(&self) -> &CrossWindowSourceProof {
        &self.proof
    }

    pub(crate) fn offer(&self) -> DragOffer {
        self.session.offer.clone()
    }

    pub(crate) const fn source(&self) -> WidgetId {
        self.session.source.id
    }

    /// Modifiers captured on the exact terminal sample before the source was
    /// detached. The coordinator must not read a later native modifier state.
    pub(crate) const fn modifiers(&self) -> crate::widgets::PointerModifiers {
        self.session.modifiers
    }

    /// Logical source-surface position captured by the terminal sample.
    pub(crate) const fn position(&self) -> Point {
        self.session.position
    }

    pub(crate) fn lease(&self) -> Weak<CrossWindowDragLease> {
        Rc::downgrade(&self.session.cross_window_lease)
    }
}

/// Detached receiver evidence consumed exactly once with a terminal request.
pub(crate) struct CrossWindowForeignTerminal<Message> {
    key: CrossWindowDragKey,
    input: CrossWindowForeignInput,
    target: Option<DropBinding<Message>>,
    accepted: bool,
}

impl<Message> CrossWindowForeignTerminal<Message> {
    pub(crate) const fn accepted(&self) -> bool {
        self.accepted
    }

    pub(crate) fn target_id(&self) -> Option<WidgetId> {
        self.target.as_ref().map(|target| target.id)
    }
}

/// Ordered, already-mapped terminal callbacks.  The host reduces target then
/// source before it next synchronizes auxiliary projections.
pub(crate) struct CrossWindowTerminalMessages<Message> {
    pub(crate) target: Option<Message>,
    pub(crate) source: Option<Message>,
}

/// Result of one checked pointer ingress when the native host supplied
/// cross-window routing evidence.  `terminal` is stack-scoped by the caller;
/// the controller never stores an Ended capture.
pub(crate) struct CrossWindowPointerRoute<Message> {
    pub(crate) disposition: crate::gui::pointer_ingress::PointerIngressDisposition,
    pub(crate) terminal: Option<CrossWindowTerminalRequest<Message>>,
    pub(crate) source_moved: Option<CrossWindowDragKey>,
}
impl<Message> TypedDragSession<Message> {
    fn context(
        &self,
        target: Option<WidgetId>,
        insertion: Option<DropInsertion>,
    ) -> DragEventContext {
        DragEventContext {
            token: DragSessionToken::new(self.token.0),
            source: self.source.id,
            target,
            position: self.position,
            modifiers: self.modifiers,
            insertion,
        }
    }
    fn source_message(&self, phase: DragSourcePhase) -> Option<Message> {
        self.handler.capabilities_v2().drag_source()?.dispatch(
            &self.offer,
            self.context(self.target.as_ref().map(|target| target.id), None),
            phase,
        )
    }
    fn target_message(&self, target: &DropBinding<Message>, phase: DropPhase) -> Option<Message> {
        target.handler.capabilities_v2().drop_target()?.dispatch(
            &self.offer,
            self.context(Some(target.id), target.insertion),
            phase,
            target.decision,
        )
    }
}

#[derive(Clone, Copy)]
struct DragDispatchInput<'a> {
    token: GestureSequenceToken,
    source: WidgetId,
    offer: &'a DragOffer,
    position: Point,
    modifiers: crate::widgets::PointerModifiers,
}

impl DragDispatchInput<'_> {
    fn context(
        self,
        target: Option<WidgetId>,
        insertion: Option<DropInsertion>,
    ) -> DragEventContext {
        DragEventContext {
            token: DragSessionToken::new(self.token.0),
            source: self.source,
            target,
            position: self.position,
            modifiers: self.modifiers,
            insertion,
        }
    }
}

impl<Message> TypedDragSession<Message> {
    fn input(&self) -> DragDispatchInput<'_> {
        DragDispatchInput {
            token: self.token,
            source: self.source.id,
            offer: &self.offer,
            position: self.position,
            modifiers: self.modifiers,
        }
    }

    fn source_message_for(
        &self,
        target: Option<WidgetId>,
        phase: DragSourcePhase,
    ) -> Option<Message> {
        self.handler.capabilities_v2().drag_source()?.dispatch(
            &self.offer,
            self.input().context(target, None),
            phase,
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

    pub(crate) fn has_cross_window_drag_candidate(&self) -> bool {
        self.interaction.drag.typed.is_some()
            || self.interaction.gesture.as_ref().is_some_and(|capture| {
                capture
                    .candidates
                    .iter()
                    .any(|target| self.is_drag_source(target))
                    || self.is_drag_source(&capture.target)
            })
    }
    pub(in crate::runtime::controller) fn typed_drag_live(
        &self,
        token: GestureSequenceToken,
    ) -> bool {
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
        hint: CrossWindowInputHint,
        terminal: Option<&mut Option<CrossWindowTerminalRequest<Message>>>,
        source_moved: Option<&mut Option<CrossWindowDragKey>>,
    ) -> bool {
        let defer_source_moved = source_moved.is_some();
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
            let autoscroll = source.autoscroll_policy();
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
                metadata: ScrollUpdateMetadata {
                    modifiers: event.sample.modifiers(),
                    timestamp: event.sample.timestamp(),
                    sequence_range: event.sample.sequence_range(),
                },
                target: None,
                autoscroll,
                autoscroll_deadline: None,
                autoscroll_last_tick: None,
                local_target_suppressed: hint.suppresses_local_target(),
                cross_window_lease: Rc::new(CrossWindowDragLease),
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
            session.metadata = ScrollUpdateMetadata {
                modifiers: event.sample.modifiers(),
                timestamp: event.sample.timestamp(),
                sequence_range: event.sample.sequence_range(),
            };
            session.local_target_suppressed = hint.suppresses_local_target();
            if session.local_target_suppressed {
                session.autoscroll_deadline = None;
                session.autoscroll_last_tick = None;
            }
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
        if !defer_source_moved || hint.suppresses_local_target() {
            self.refresh_drop_target(token);
        }
        if !self.typed_drag_live(token) {
            return true;
        }
        self.update_typed_drag_autoscroll(token, self.timed_repaint_now());
        if event.phase == GesturePhase::Ended {
            // Detach both capture and payload before either terminal mapper runs.
            if let Some(terminal) = terminal {
                *terminal = self.take_cross_window_terminal_request(token);
            } else {
                self.interaction.gesture = None;
                let messages = self.take_typed_drag_terminal(None);
                for message in messages {
                    self.typed_drag_message(Some(message));
                }
            }
        } else if event.phase != GesturePhase::Started {
            if let Some(source_moved) = source_moved {
                *source_moved = Some(CrossWindowDragKey(token));
            } else {
                let message = self
                    .interaction
                    .drag
                    .typed
                    .as_ref()
                    .and_then(|session| session.source_message(DragSourcePhase::Moved));
                self.typed_drag_message(message);
            }
        }
        true
    }

    fn autoscroll_delta(
        &self,
        session: &TypedDragSession<Message>,
        elapsed: Duration,
    ) -> Option<Vector2> {
        let policy = session.autoscroll?;
        let node_id = self.scroll_container_at(session.position)?;
        let viewport = self
            .layout
            .viewport_bounds
            .get(&node_id)
            .or_else(|| self.layout.rects.get(&node_id))
            .copied()?;
        if !viewport.has_finite_positive_area() || !viewport.contains(session.position) {
            return None;
        }
        let horizontal_zone = policy.edge_zone().min(viewport.width() * 0.5);
        let vertical_zone = policy.edge_zone().min(viewport.height() * 0.5);
        let strength = |zone: f32, near: f32, far: f32, value: f32| {
            if value - near < zone {
                -((zone - (value - near)) / zone).clamp(0.0, 1.0)
            } else if far - value < zone {
                ((zone - (far - value)) / zone).clamp(0.0, 1.0)
            } else {
                0.0
            }
        };
        let seconds = elapsed.as_secs_f32();
        let delta = Vector2::new(
            strength(
                horizontal_zone,
                viewport.min.x,
                viewport.max.x,
                session.position.x,
            ) * policy.max_speed()
                * seconds,
            strength(
                vertical_zone,
                viewport.min.y,
                viewport.max.y,
                session.position.y,
            ) * policy.max_speed()
                * seconds,
        );
        (delta.x.abs() > f32::EPSILON || delta.y.abs() > f32::EPSILON).then_some(delta)
    }

    fn update_typed_drag_autoscroll(&mut self, token: GestureSequenceToken, now: Instant) {
        let armed = self
            .interaction
            .drag
            .typed
            .as_ref()
            .filter(|session| session.token == token && !session.local_target_suppressed)
            .and_then(|session| self.autoscroll_delta(session, DRAG_AUTOSCROLL_TICK));
        let Some(session) = self
            .interaction
            .drag
            .typed
            .as_mut()
            .filter(|session| session.token == token)
        else {
            return;
        };
        if armed.is_some() {
            if session.autoscroll_deadline.is_none() {
                session.autoscroll_last_tick = Some(now);
                session.autoscroll_deadline = now.checked_add(DRAG_AUTOSCROLL_TICK);
            }
        } else {
            session.autoscroll_deadline = None;
            session.autoscroll_last_tick = None;
        }
    }

    pub(in crate::runtime::controller) fn typed_drag_autoscroll_deadline(&self) -> Option<Instant> {
        self.interaction
            .drag
            .typed
            .as_ref()
            .and_then(|session| session.autoscroll_deadline)
    }

    pub(in crate::runtime::controller) fn advance_typed_drag_autoscroll(
        &mut self,
        now: Instant,
    ) -> bool {
        let Some((token, deadline, last_tick)) =
            self.interaction.drag.typed.as_ref().and_then(|session| {
                session.autoscroll_deadline.map(|deadline| {
                    (
                        session.token,
                        deadline,
                        session.autoscroll_last_tick.unwrap_or(deadline),
                    )
                })
            })
        else {
            return false;
        };
        if now < deadline || !self.typed_drag_live(token) {
            return false;
        }
        let elapsed = now
            .saturating_duration_since(last_tick)
            .min(DRAG_AUTOSCROLL_MAX_ELAPSED);
        let delta = self
            .interaction
            .drag
            .typed
            .as_ref()
            .and_then(|session| self.autoscroll_delta(session, elapsed));
        let Some(delta) = delta else {
            self.update_typed_drag_autoscroll(token, now);
            return false;
        };
        if let Some(session) = self
            .interaction
            .drag
            .typed
            .as_mut()
            .filter(|session| session.token == token)
        {
            session.autoscroll_last_tick = Some(now);
            session.autoscroll_deadline = None;
        }
        let point = self
            .interaction
            .drag
            .typed
            .as_ref()
            .map(|session| session.position)
            .unwrap_or_default();
        let metadata = self
            .interaction
            .drag
            .typed
            .as_ref()
            .map(|session| session.metadata)
            .unwrap_or_default();
        let attempt = self.scroll_at_with_refresh_and_metadata_guarded(
            point,
            delta,
            metadata,
            true,
            crate::widgets::InteractionProvenance::Pointer {
                modifiers: metadata.modifiers,
                timestamp: metadata.timestamp,
                sequence_range: metadata.sequence_range,
            },
            Some(token),
        );
        let moved = attempt.moved;
        if moved && self.typed_drag_live(token) {
            self.refresh_drop_target(token);
            if self.typed_drag_live(token) {
                self.update_typed_drag_autoscroll(token, now);
            }
        }
        moved || attempt.accepted && !self.typed_drag_live(token)
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
        self.current_drop_target_for(session.input())
    }

    fn drop_target_message_for(
        input: DragDispatchInput<'_>,
        target: &DropBinding<Message>,
        phase: DropPhase,
    ) -> Option<Message> {
        target.handler.capabilities_v2().drop_target()?.dispatch(
            input.offer,
            input.context(Some(target.id), target.insertion),
            phase,
            target.decision,
        )
    }

    fn current_drop_target_for(
        &self,
        input: DragDispatchInput<'_>,
    ) -> Option<DropBinding<Message>> {
        let position = input.position;
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
            if !self.overlay_focus_allows(record.id) {
                continue;
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
            if !target.accepts_payload(input.offer) {
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
        let target = facets.drop_target()?;
        let bounds = self.layout.rects.get(&record.id)?;
        let insertion = target.insertion_axis().map(|axis| {
            let side = match axis {
                DropInsertionAxis::Horizontal if position.x < bounds.center().x => {
                    DropInsertionSide::Before
                }
                DropInsertionAxis::Vertical if position.y < bounds.center().y => {
                    DropInsertionSide::Before
                }
                _ => DropInsertionSide::After,
            };
            DropInsertion::new(axis, side)
        });
        // Insertion is part of the input snapshot used by negotiation and every
        // lifecycle event. Equality at the midpoint deliberately chooses After.
        let decision = facets
            .drop_target()?
            .negotiate(input.offer, input.context(Some(record.id), insertion));
        let decision = match decision {
            DropDecision::Accepted(operation) if !input.offer.operations().contains(operation) => {
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
            feedback: facets.drop_target()?.feedback(),
            feedback_layout: self.runtime_layout_input_evidence(self.mounted_layout_source_present),
            insertion,
        })
    }
    fn refresh_drop_target(&mut self, token: GestureSequenceToken) {
        self.repaint_requested = true;
        let candidate = self.interaction.drag.typed.as_ref().and_then(|session| {
            (!session.local_target_suppressed)
                .then(|| self.current_drop_target(session))
                .flatten()
        });
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
            self.requalify_drop_feedback_after_message(token);
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
        let candidate = self.interaction.drag.typed.as_ref().and_then(|session| {
            (!session.local_target_suppressed)
                .then(|| self.current_drop_target(session))
                .flatten()
        });
        let message = self.interaction.drag.typed.as_mut().and_then(|session| {
            session.target = candidate;
            session
                .target
                .as_ref()
                .and_then(|target| session.target_message(target, DropPhase::Entered))
        });
        self.typed_drag_message(message);
        self.requalify_drop_feedback_after_message(token);
    }
    fn requalify_drop_feedback_after_message(&mut self, token: GestureSequenceToken) {
        if !self.typed_drag_live(token) {
            return;
        }
        let candidate = self.interaction.drag.typed.as_ref().and_then(|session| {
            if session.local_target_suppressed {
                return None;
            }
            let target = session.target.as_ref()?;
            if target.generation == self.refresh_counters().runtime_projection
                || !self.drop_binding_matches(target, &self.surface, false)
            {
                return None;
            }
            let candidate = self.current_drop_target(session)?;
            (candidate.id == target.id
                && candidate.path == target.path
                && candidate.decision == target.decision)
                .then_some(candidate)
        });
        if let Some(candidate) = candidate
            && let Some(session) = self.interaction.drag.typed.as_mut()
            && session.token == token
        {
            session.target = Some(candidate);
        }
    }

    pub(crate) fn cross_window_drag_export(&self) -> Option<CrossWindowDragExport> {
        let session = self.interaction.drag.typed.as_ref()?;
        if !self.typed_drag_live(session.token) {
            return None;
        }
        let capture = self.interaction.gesture.as_ref()?;
        Some(CrossWindowDragExport {
            key: CrossWindowDragKey(session.token),
            offer: session.offer.clone(),
            source: session.source.id,
            modifiers: session.modifiers,
            position: session.position,
            lease: Rc::downgrade(&session.cross_window_lease),
            proof: CrossWindowSourceProof {
                key: CrossWindowDragKey(session.token),
                target: capture.target.clone(),
                projection: capture.generation,
                allocator: self.interaction.pointer.ingress.allocator,
            },
        })
    }

    pub(crate) fn cross_window_source_proof_is_current(
        &self,
        proof: &CrossWindowSourceProof,
    ) -> bool {
        let target = &proof.target;
        let same_projection = proof.projection == self.refresh_counters().runtime_projection;
        self.lifecycle_accepts_work()
            && self.interaction.pointer.ingress.allocator == proof.allocator
            && self.layout.rects.get(&target.id).is_some_and(|bounds| {
                bounds.min.is_finite()
                    && bounds.max.is_finite()
                    && bounds.width() > 0.0
                    && bounds.height() > 0.0
            })
            && self.overlay_focus_allows(target.id)
            && !self
                .traversal
                .widgets
                .duplicate_widget_ids
                .contains(&target.id)
            && (!matches!(target.owner, GestureOwner::Container { .. })
                || self
                    .traversal
                    .containers
                    .layout_interactions
                    .iter()
                    .filter(|record| record.id == target.id && record.gesture_qualified)
                    .count()
                    == 1)
            && Self::gesture_target_matches_surface(
                target,
                &self.surface,
                &self.traversal.widgets.paths.current,
                same_projection,
            )
    }

    /// Resume source-window target qualification after the coordinator has
    /// reduced a foreign receiver's `Left`. This never replays raw input.
    pub(crate) fn advance_cross_window_local_target(
        &mut self,
        key: CrossWindowDragKey,
    ) -> CrossWindowForeignRoute<Message> {
        if !self.typed_drag_live(key.0) {
            return CrossWindowForeignRoute::empty();
        }
        let Some(session) = self
            .interaction
            .drag
            .typed
            .as_mut()
            .filter(|session| session.token == key.0)
        else {
            return CrossWindowForeignRoute::empty();
        };
        session.local_target_suppressed = false;
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
        let Some(session) = self.interaction.drag.typed.as_mut() else {
            return CrossWindowForeignRoute::empty();
        };
        self.repaint_requested = true;
        if same {
            session.target = candidate;
            return CrossWindowForeignRoute {
                messages: session
                    .target
                    .as_ref()
                    .and_then(|target| session.target_message(target, DropPhase::Over))
                    .into_iter()
                    .collect(),
                repaint: true,
                needs_transition: false,
            };
        }
        if let Some(old) = session.target.take() {
            let message = session.target_message(&old, DropPhase::Left);
            return CrossWindowForeignRoute {
                messages: message.into_iter().collect(),
                repaint: true,
                needs_transition: candidate.is_some(),
            };
        }
        session.target = candidate;
        CrossWindowForeignRoute {
            messages: session
                .target
                .as_ref()
                .and_then(|target| session.target_message(target, DropPhase::Entered))
                .into_iter()
                .collect(),
            repaint: true,
            needs_transition: false,
        }
    }

    /// Map a source move only after the coordinator completed target phases.
    pub(crate) fn map_cross_window_source_moved(
        &mut self,
        key: CrossWindowDragKey,
        target: Option<WidgetId>,
    ) -> Option<Message> {
        if !self.typed_drag_live(key.0) {
            return None;
        }
        self.update_typed_drag_autoscroll(key.0, self.timed_repaint_now());
        self.interaction
            .drag
            .typed
            .as_ref()
            .filter(|session| session.token == key.0)
            .and_then(|session| session.source_message_for(target, DragSourcePhase::Moved))
    }

    /// Read the current receiver-local target identity for a live foreign
    /// receipt. This is data only; qualification remains with the receiving
    /// runtime and mapping remains with the coordinator.
    pub(crate) fn cross_window_foreign_target_id(
        &self,
        key: CrossWindowDragKey,
    ) -> Option<WidgetId> {
        self.interaction
            .drag
            .cross_window_foreign
            .as_ref()
            .filter(|foreign| {
                foreign.key == key
                    && foreign.lease.upgrade().is_some()
                    && self.lifecycle_accepts_work()
            })
            .and_then(|foreign| foreign.target.as_ref().map(|target| target.id))
    }

    /// Read the source-surface target restored after a foreign receipt has
    /// left. This is data only and is used to preserve `SourceMoved` context
    /// ordering without replaying raw ingress.
    pub(crate) fn cross_window_local_target_id(&self, key: CrossWindowDragKey) -> Option<WidgetId> {
        self.typed_drag_live(key.0)
            .then(|| {
                self.interaction
                    .drag
                    .typed
                    .as_ref()
                    .filter(|session| session.token == key.0)
                    .and_then(|session| session.target.as_ref().map(|target| target.id))
            })
            .flatten()
    }

    pub(crate) fn cross_window_foreign_preview(&self) -> Option<&DragSession> {
        let foreign = self.interaction.drag.cross_window_foreign.as_ref()?;
        (self.lifecycle_accepts_work()
            && foreign.lease.upgrade().is_some()
            && foreign.position.is_finite()
            && foreign.preview.visible
            && foreign.preview.pointer == foreign.position)
            .then_some(&foreign.preview)
    }

    pub(crate) fn route_cross_window_foreign(
        &mut self,
        input: CrossWindowForeignInput,
    ) -> CrossWindowForeignRoute<Message> {
        if !self.lifecycle_accepts_work()
            || !input.position.is_finite()
            || input.lease.upgrade().is_none()
        {
            return self.discard_cross_window_foreign(input.key);
        }
        let mut route = CrossWindowForeignRoute::empty();
        if let Some(previous) = self.interaction.drag.cross_window_foreign.as_ref()
            && previous.key != input.key
        {
            // A receiver can hold one foreign receipt at a time. Do not let a
            // second live source remove it: its `Left` must be reduced before
            // a replacement can be admitted, and that sequencing belongs to
            // the coordinator's phased route.
            if previous.lease.upgrade().is_some() {
                return route;
            }
            self.interaction.drag.cross_window_foreign = None;
            self.repaint_requested = true;
        }
        if let Some(foreign) = self.interaction.drag.cross_window_foreign.as_mut() {
            foreign.offer = input.offer;
            foreign.source = input.source;
            foreign.lease = input.lease;
            foreign.position = input.position;
            foreign.modifiers = input.modifiers;
            foreign.preview.pointer = input.position;
            foreign.preview.visible = true;
        } else {
            let preview =
                DragPreview::sized(input.offer.preview().label(), input.offer.preview().size());
            self.interaction.drag.cross_window_foreign = Some(CrossWindowForeignDrag {
                key: input.key,
                offer: input.offer,
                source: input.source,
                lease: input.lease,
                position: input.position,
                modifiers: input.modifiers,
                target: None,
                preview: DragSession::new(DragRequest::new(preview, input.position)),
            });
        }
        let refreshed = self.refresh_cross_window_foreign();
        route.messages.extend(refreshed.messages);
        route.repaint |= refreshed.repaint;
        route.needs_transition |= refreshed.needs_transition;
        route
    }

    pub(crate) fn requalify_cross_window_foreign(
        &mut self,
        key: CrossWindowDragKey,
    ) -> CrossWindowForeignRoute<Message> {
        if !self.lifecycle_accepts_work() {
            return self.discard_cross_window_foreign(key);
        }
        if !self
            .interaction
            .drag
            .cross_window_foreign
            .as_ref()
            .is_some_and(|foreign| foreign.key == key && foreign.lease.upgrade().is_some())
        {
            return self.discard_cross_window_foreign(key);
        }
        self.refresh_cross_window_foreign()
    }

    /// Refresh only retained feedback after a receiver mapper has been
    /// reduced. Unlike input routing, this never maps `Over`, `Left`, or
    /// `Entered`; the coordinator uses it after its bounded transition steps.
    /// A changed target or decision is hidden until a later checked sample
    /// performs the next ordinary transition.
    pub(crate) fn finish_cross_window_foreign_feedback(&mut self, key: CrossWindowDragKey) -> bool {
        if !self.lifecycle_accepts_work() {
            let _ = self.discard_cross_window_foreign(key);
            return false;
        }
        let candidate = self
            .interaction
            .drag
            .cross_window_foreign
            .as_ref()
            .filter(|foreign| foreign.key == key && foreign.lease.upgrade().is_some())
            .and_then(|foreign| self.current_drop_target_for(Self::foreign_input(foreign)));
        let same = self
            .interaction
            .drag
            .cross_window_foreign
            .as_ref()
            .and_then(|foreign| foreign.target.as_ref())
            .zip(candidate.as_ref())
            .is_some_and(|(old, new)| {
                old.id == new.id
                    && old.path == new.path
                    && old.decision == new.decision
                    && old.insertion == new.insertion
                    && self.drop_binding_matches(
                        old,
                        &self.surface,
                        old.generation == self.refresh_counters().runtime_projection,
                    )
            });
        let Some(foreign) = self.interaction.drag.cross_window_foreign.as_mut() else {
            return false;
        };
        if foreign.key != key || foreign.lease.upgrade().is_none() {
            return false;
        }
        foreign.target = same.then_some(candidate).flatten();
        self.repaint_requested = true;
        same
    }

    pub(crate) fn clear_cross_window_foreign(
        &mut self,
        key: CrossWindowDragKey,
    ) -> CrossWindowForeignRoute<Message> {
        if !self
            .interaction
            .drag
            .cross_window_foreign
            .as_ref()
            .is_some_and(|foreign| foreign.key == key)
        {
            return CrossWindowForeignRoute::empty();
        }
        if !self.lifecycle_accepts_work()
            || self
                .interaction
                .drag
                .cross_window_foreign
                .as_ref()
                .is_some_and(|foreign| foreign.lease.upgrade().is_none())
        {
            return self.discard_cross_window_foreign(key);
        }
        self.clear_cross_window_foreign_internal()
    }

    /// Retire a foreign receipt after its source has already ended. This is
    /// the sole receiver-side cleanup callback for that receipt: source lease
    /// liveness is deliberately not consulted, while receiver lifecycle and
    /// exact binding evidence remain mandatory.
    pub(crate) fn cancel_cross_window_foreign(
        &mut self,
        key: CrossWindowDragKey,
    ) -> CrossWindowForeignRoute<Message> {
        if !self.lifecycle_accepts_work() {
            return self.discard_cross_window_foreign(key);
        }
        let Some(mut foreign) = self.interaction.drag.cross_window_foreign.take() else {
            return CrossWindowForeignRoute::empty();
        };
        if foreign.key != key {
            self.interaction.drag.cross_window_foreign = Some(foreign);
            return CrossWindowForeignRoute::empty();
        }
        let same_projection = foreign
            .target
            .as_ref()
            .is_some_and(|target| target.generation == self.refresh_counters().runtime_projection);
        let message = foreign.target.take().and_then(|target| {
            self.drop_binding_matches(&target, &self.surface, same_projection)
                .then(|| {
                    Self::drop_target_message_for(
                        Self::foreign_input(&foreign),
                        &target,
                        DropPhase::Cancelled,
                    )
                })
                .flatten()
        });
        self.repaint_requested = true;
        CrossWindowForeignRoute {
            messages: message.into_iter().collect(),
            repaint: true,
            needs_transition: false,
        }
    }

    /// Remove a stale foreign receipt without entering a target mapper. A
    /// source that has already retired cannot authorize a later `Left` in a
    /// receiver window, and a closing receiver likewise accepts no callbacks.
    fn discard_cross_window_foreign(
        &mut self,
        key: CrossWindowDragKey,
    ) -> CrossWindowForeignRoute<Message> {
        if !self
            .interaction
            .drag
            .cross_window_foreign
            .as_ref()
            .is_some_and(|foreign| foreign.key == key)
        {
            return CrossWindowForeignRoute::empty();
        }
        self.interaction.drag.cross_window_foreign = None;
        self.repaint_requested = true;
        CrossWindowForeignRoute {
            messages: Vec::new(),
            repaint: true,
            needs_transition: false,
        }
    }

    fn foreign_input(foreign: &CrossWindowForeignDrag<Message>) -> DragDispatchInput<'_> {
        DragDispatchInput {
            token: foreign.key.0,
            source: foreign.source,
            offer: &foreign.offer,
            position: foreign.position,
            modifiers: foreign.modifiers,
        }
    }

    fn clear_cross_window_foreign_internal(&mut self) -> CrossWindowForeignRoute<Message> {
        let Some(mut foreign) = self.interaction.drag.cross_window_foreign.take() else {
            return CrossWindowForeignRoute::empty();
        };
        let message = foreign.target.take().and_then(|target| {
            Self::drop_target_message_for(Self::foreign_input(&foreign), &target, DropPhase::Left)
        });
        self.repaint_requested = true;
        CrossWindowForeignRoute {
            messages: message.into_iter().collect(),
            repaint: true,
            needs_transition: false,
        }
    }

    fn refresh_cross_window_foreign(&mut self) -> CrossWindowForeignRoute<Message> {
        let candidate = self
            .interaction
            .drag
            .cross_window_foreign
            .as_ref()
            .and_then(|foreign| {
                (foreign.lease.upgrade().is_some())
                    .then(|| self.current_drop_target_for(Self::foreign_input(foreign)))
                    .flatten()
            });
        let same = self
            .interaction
            .drag
            .cross_window_foreign
            .as_ref()
            .and_then(|foreign| foreign.target.as_ref())
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
        let Some(foreign) = self.interaction.drag.cross_window_foreign.as_mut() else {
            return CrossWindowForeignRoute::empty();
        };
        self.repaint_requested = true;
        if same {
            foreign.target = candidate;
            let message = foreign.target.as_ref().and_then(|target| {
                Self::drop_target_message_for(Self::foreign_input(foreign), target, DropPhase::Over)
            });
            return CrossWindowForeignRoute {
                messages: message.into_iter().collect(),
                repaint: true,
                needs_transition: false,
            };
        }
        let old = foreign.target.take().and_then(|target| {
            Self::drop_target_message_for(Self::foreign_input(foreign), &target, DropPhase::Left)
        });
        if old.is_some() {
            return CrossWindowForeignRoute {
                messages: old.into_iter().collect(),
                repaint: true,
                needs_transition: candidate.is_some(),
            };
        }
        foreign.target = candidate;
        let entered = foreign.target.as_ref().and_then(|target| {
            Self::drop_target_message_for(Self::foreign_input(foreign), target, DropPhase::Entered)
        });
        CrossWindowForeignRoute {
            messages: entered.into_iter().collect(),
            repaint: true,
            needs_transition: false,
        }
    }

    pub(crate) fn take_cross_window_foreign_terminal(
        &mut self,
        key: CrossWindowDragKey,
    ) -> Option<CrossWindowForeignTerminal<Message>> {
        if !self.lifecycle_accepts_work() {
            let _ = self.discard_cross_window_foreign(key);
            return None;
        }
        let current = self.interaction.drag.cross_window_foreign.as_ref()?;
        if current.key != key {
            return None;
        }
        if current.lease.upgrade().is_none() {
            let _ = self.discard_cross_window_foreign(key);
            return None;
        }
        let foreign = self.interaction.drag.cross_window_foreign.take()?;
        let input = CrossWindowForeignInput {
            key: foreign.key,
            offer: foreign.offer.clone(),
            source: foreign.source,
            lease: foreign.lease.clone(),
            position: foreign.position,
            modifiers: foreign.modifiers,
        };
        let current = self.current_drop_target_for(Self::foreign_input(&foreign));
        let accepted =
            foreign
                .target
                .as_ref()
                .zip(current.as_ref())
                .is_some_and(|(old, current)| {
                    old.id == current.id
                        && old.path == current.path
                        && old.decision == current.decision
                        && matches!(current.decision, DropDecision::Accepted(_))
                        && self.drop_binding_matches(
                            old,
                            &self.surface,
                            old.generation == self.refresh_counters().runtime_projection,
                        )
                });
        let target = if accepted { current } else { foreign.target };
        self.repaint_requested = true;
        Some(CrossWindowForeignTerminal {
            key,
            input,
            target,
            accepted,
        })
    }

    pub(crate) fn map_cross_window_terminal(
        &self,
        request: &CrossWindowTerminalRequest<Message>,
        terminal: Option<&CrossWindowForeignTerminal<Message>>,
        cancel_reason: DragCancelReason,
    ) -> CrossWindowTerminalMessages<Message> {
        if !self.cross_window_source_proof_is_current(request.source_proof()) {
            return CrossWindowTerminalMessages {
                target: None,
                source: None,
            };
        }
        let operation = terminal
            .filter(|terminal| terminal.accepted())
            .and_then(|terminal| {
                terminal
                    .target
                    .as_ref()
                    .and_then(|target| match target.decision {
                        DropDecision::Accepted(operation) => Some(operation),
                        DropDecision::Pending | DropDecision::Rejected => None,
                    })
            });
        let accepted = operation.is_some();
        let target_id = terminal.and_then(CrossWindowForeignTerminal::target_id);
        let target = terminal.and_then(|terminal| {
            terminal.target.as_ref().and_then(|target| {
                let phase = if accepted {
                    DropPhase::Dropped
                } else {
                    DropPhase::Cancelled
                };
                Self::drop_target_message_for(
                    DragDispatchInput {
                        token: terminal.key.0,
                        source: terminal.input.source,
                        offer: &terminal.input.offer,
                        position: terminal.input.position,
                        modifiers: terminal.input.modifiers,
                    },
                    target,
                    phase,
                )
            })
        });
        let source = request.session.source_message_for(
            target_id,
            match operation {
                Some(operation) => DragSourcePhase::Completed(operation),
                None => DragSourcePhase::Cancelled(cancel_reason),
            },
        );
        CrossWindowTerminalMessages { target, source }
    }

    pub(crate) fn take_cross_window_terminal_request(
        &mut self,
        token: GestureSequenceToken,
    ) -> Option<CrossWindowTerminalRequest<Message>> {
        if !self.typed_drag_live(token) {
            return None;
        }
        let capture = self.interaction.gesture.take()?;
        let session = self.interaction.drag.typed.take()?;
        if capture.token != token || session.token != token {
            self.interaction.gesture = Some(capture);
            self.interaction.drag.typed = Some(session);
            return None;
        }
        // This follows the local terminal path: source capture and its
        // pointer/touch ownership end before the coordinator can map either
        // terminal callback. The enclosing pointer ingress still retires its
        // exact transport record after this stack-scoped request is returned.
        self.retire_gesture_touch(&capture);
        self.clear_gesture_pointer_capture(&capture.target);
        self.interaction.drag.session = None;
        self.repaint_requested = true;
        Some(CrossWindowTerminalRequest {
            proof: CrossWindowSourceProof {
                key: CrossWindowDragKey(token),
                target: capture.target,
                projection: capture.generation,
                allocator: self.interaction.pointer.ingress.allocator,
            },
            session,
        })
    }
    pub(in crate::runtime::controller) fn append_drop_target_feedback(
        &self,
        theme: &crate::theme::ThemeTokens,
        primitives: &mut Vec<crate::runtime::PaintPrimitive>,
    ) {
        if let Some(foreign) = self.interaction.drag.cross_window_foreign.as_ref()
            && foreign.lease.upgrade().is_some()
            && foreign.preview.visible
            && let Some(target) = foreign.target.as_ref()
        {
            self.append_foreign_drop_target_feedback(theme, primitives, target);
        }
        let Some(session) = self.interaction.drag.typed.as_ref() else {
            return;
        };
        let Some(target) = session.target.as_ref() else {
            return;
        };
        let Some(feedback) = target.feedback else {
            return;
        };
        // Painting never negotiates or emits messages. Geometry/source changes
        // hide the old feedback until input qualifies the target again.
        if !self.typed_drag_live(session.token)
            || session.local_target_suppressed
            || !self
                .interaction
                .drag
                .session
                .as_ref()
                .is_some_and(|preview| preview.visible)
            || target.generation != self.refresh_counters().runtime_projection
            || target.feedback_layout.is_none()
            || target.feedback_layout
                != self.runtime_layout_input_evidence(self.mounted_layout_source_present)
        {
            return;
        }
        let Some(bounds) = self
            .layout
            .rects
            .get(&target.id)
            .copied()
            .filter(|r| r.has_finite_positive_area())
        else {
            return;
        };
        let Some(mut clip) = bounds.intersection(self.viewport) else {
            return;
        };
        for ancestor in self
            .traversal
            .containers
            .layout_clip_for_container(target.id, &self.layout)
        {
            let Some(intersection) = clip.intersection(ancestor) else {
                return;
            };
            clip = intersection;
        }
        if !clip.has_finite_positive_area() {
            return;
        }
        let tokens = crate::widgets::resolve_widget_visual_tokens(
            theme,
            feedback.style(target.decision),
            crate::widgets::WidgetState {
                active: true,
                selected: true,
                ..Default::default()
            },
        );
        primitives.push(crate::runtime::PaintPrimitive::ClipStart(
            crate::runtime::PaintClipStart {
                node_id: target.id,
                rect: clip,
            },
        ));
        let marker = target
            .insertion
            .map(|insertion| match (insertion.axis(), insertion.side()) {
                (DropInsertionAxis::Horizontal, DropInsertionSide::Before) => {
                    crate::gui::types::Rect::from_min_max(
                        bounds.min,
                        crate::gui::types::Point::new(
                            (bounds.min.x + 2.0).min(bounds.max.x),
                            bounds.max.y,
                        ),
                    )
                }
                (DropInsertionAxis::Horizontal, DropInsertionSide::After) => {
                    crate::gui::types::Rect::from_min_max(
                        crate::gui::types::Point::new(
                            (bounds.max.x - 2.0).max(bounds.min.x),
                            bounds.min.y,
                        ),
                        bounds.max,
                    )
                }
                (DropInsertionAxis::Vertical, DropInsertionSide::Before) => {
                    crate::gui::types::Rect::from_min_max(
                        bounds.min,
                        crate::gui::types::Point::new(
                            bounds.max.x,
                            (bounds.min.y + 2.0).min(bounds.max.y),
                        ),
                    )
                }
                (DropInsertionAxis::Vertical, DropInsertionSide::After) => {
                    crate::gui::types::Rect::from_min_max(
                        crate::gui::types::Point::new(
                            bounds.min.x,
                            (bounds.max.y - 2.0).max(bounds.min.y),
                        ),
                        bounds.max,
                    )
                }
            });
        let primitive = if let Some(marker) = marker {
            crate::runtime::PaintPrimitive::FillRect(crate::runtime::PaintFillRect {
                widget_id: target.id,
                rect: marker,
                color: tokens.emphasis,
            })
        } else {
            crate::runtime::PaintPrimitive::StrokeRect(crate::runtime::PaintStrokeRect {
                widget_id: target.id,
                rect: bounds,
                color: tokens.emphasis,
                width: 2.0,
            })
        };
        primitives.push(primitive);
        primitives.push(crate::runtime::PaintPrimitive::ClipEnd(
            crate::runtime::PaintClipEnd { node_id: target.id },
        ));
    }

    fn append_foreign_drop_target_feedback(
        &self,
        theme: &crate::theme::ThemeTokens,
        primitives: &mut Vec<crate::runtime::PaintPrimitive>,
        target: &DropBinding<Message>,
    ) {
        let Some(feedback) = target.feedback else {
            return;
        };
        if target.generation != self.refresh_counters().runtime_projection
            || target.feedback_layout.is_none()
            || target.feedback_layout
                != self.runtime_layout_input_evidence(self.mounted_layout_source_present)
        {
            return;
        }
        let Some(bounds) = self
            .layout
            .rects
            .get(&target.id)
            .copied()
            .filter(|rect| rect.has_finite_positive_area())
        else {
            return;
        };
        let Some(mut clip) = bounds.intersection(self.viewport) else {
            return;
        };
        for ancestor in self
            .traversal
            .containers
            .layout_clip_for_container(target.id, &self.layout)
        {
            let Some(intersection) = clip.intersection(ancestor) else {
                return;
            };
            clip = intersection;
        }
        if !clip.has_finite_positive_area() {
            return;
        }
        let tokens = crate::widgets::resolve_widget_visual_tokens(
            theme,
            feedback.style(target.decision),
            crate::widgets::WidgetState {
                active: true,
                selected: true,
                ..Default::default()
            },
        );
        primitives.push(crate::runtime::PaintPrimitive::ClipStart(
            crate::runtime::PaintClipStart {
                node_id: target.id,
                rect: clip,
            },
        ));
        let marker = target
            .insertion
            .map(|insertion| match (insertion.axis(), insertion.side()) {
                (DropInsertionAxis::Horizontal, DropInsertionSide::Before) => {
                    crate::gui::types::Rect::from_min_max(
                        bounds.min,
                        crate::gui::types::Point::new(
                            (bounds.min.x + 2.0).min(bounds.max.x),
                            bounds.max.y,
                        ),
                    )
                }
                (DropInsertionAxis::Horizontal, DropInsertionSide::After) => {
                    crate::gui::types::Rect::from_min_max(
                        crate::gui::types::Point::new(
                            (bounds.max.x - 2.0).max(bounds.min.x),
                            bounds.min.y,
                        ),
                        bounds.max,
                    )
                }
                (DropInsertionAxis::Vertical, DropInsertionSide::Before) => {
                    crate::gui::types::Rect::from_min_max(
                        bounds.min,
                        crate::gui::types::Point::new(
                            bounds.max.x,
                            (bounds.min.y + 2.0).min(bounds.max.y),
                        ),
                    )
                }
                (DropInsertionAxis::Vertical, DropInsertionSide::After) => {
                    crate::gui::types::Rect::from_min_max(
                        crate::gui::types::Point::new(
                            bounds.min.x,
                            (bounds.max.y - 2.0).max(bounds.min.y),
                        ),
                        bounds.max,
                    )
                }
            });
        let primitive = if let Some(marker) = marker {
            crate::runtime::PaintPrimitive::FillRect(crate::runtime::PaintFillRect {
                widget_id: target.id,
                rect: marker,
                color: tokens.emphasis,
            })
        } else {
            crate::runtime::PaintPrimitive::StrokeRect(crate::runtime::PaintStrokeRect {
                widget_id: target.id,
                rect: bounds,
                color: tokens.emphasis,
                width: 2.0,
            })
        };
        primitives.push(primitive);
        primitives.push(crate::runtime::PaintPrimitive::ClipEnd(
            crate::runtime::PaintClipEnd { node_id: target.id },
        ));
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
        let terminal_target = accepted
            .is_some()
            .then_some(current_target.as_ref())
            .flatten()
            .or(session.target.as_ref());
        if let Some(target) = terminal_target {
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

#[cfg(test)]
mod autoscroll_tests {
    use super::*;
    use crate::{
        application::{DragSource, button, scroll},
        gui::pointer_ingress::{
            GestureIngress, GestureKind, GesturePhase, GestureUnit, InputDeviceId,
        },
        layout::Vector2,
        runtime::{GestureRequest, SurfaceRuntime},
    };
    use std::{cell::RefCell, rc::Rc};

    fn pan(phase: GesturePhase, y: f32) -> GestureIngress {
        GestureIngress::new(
            GestureKind::Pan,
            phase,
            GestureUnit::LogicalPixels,
            Vector2::new(0.0, y),
            InputDeviceId::from_host(1).unwrap(),
            Some(Point::new(20.0, 88.0)),
            Default::default(),
            None,
            None,
        )
        .unwrap()
    }

    #[test]
    fn cross_window_terminal_detaches_source_and_fences_post_release_input() {
        let phases = Rc::new(RefCell::new(Vec::new()));
        let observed = Rc::clone(&phases);
        let bridge = crate::app(())
            .view(|_| {
                button("source")
                    .filter_mapped(|_| None::<DragSourcePhase>)
                    .width(100.0)
                    .height(100.0)
                    .id(1)
                    .drag_source(
                        DragSource::new(1_u8)
                            .on_event_with_revision((), |event| Some(event.phase())),
                    )
                    .id(10)
            })
            .update(move |_, phase: DragSourcePhase| observed.borrow_mut().push(phase))
            .into_bridge();
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(100.0, 100.0));
        let token = runtime
            .dispatch_gesture_request(GestureRequest::new(pan(GesturePhase::Started, 0.0)))
            .token()
            .expect("the source admits an initial pan");
        runtime.dispatch_gesture_request_with_cross_window(
            GestureRequest::new(pan(GesturePhase::Changed, 10.0)).with_token(token),
            CrossWindowInputHint::foreign_or_none(),
            None,
            None,
        );
        let export = runtime
            .cross_window_drag_export()
            .expect("the recognized typed drag exports data-only evidence");
        assert!(export.is_live());
        assert!(runtime.interaction.drag.typed.is_some());
        assert!(
            runtime
                .interaction
                .drag
                .typed
                .as_ref()
                .is_some_and(|session| session.local_target_suppressed)
        );

        let mut terminal = None;
        let admission = runtime.dispatch_gesture_request_with_cross_window(
            GestureRequest::new(pan(GesturePhase::Ended, 0.0)).with_token(token),
            CrossWindowInputHint::foreign_or_none(),
            Some(&mut terminal),
            None,
        );
        assert_eq!(admission.token(), None);
        let request = terminal.expect("release extracts one stack-scoped terminal request");
        assert!(runtime.interaction.gesture.is_none());
        assert!(runtime.interaction.drag.typed.is_none());
        assert!(!runtime.drag_session_active());
        assert!(runtime.cross_window_source_proof_is_current(request.source_proof()));

        let next =
            runtime.dispatch_gesture_request(GestureRequest::new(pan(GesturePhase::Started, 0.0)));
        assert!(next.token().is_some());
        assert!(!runtime.cross_window_source_proof_is_current(request.source_proof()));
        drop(request);
        assert!(!export.is_live());
        assert_eq!(phases.borrow().as_slice(), &[DragSourcePhase::Started]);
    }

    #[test]
    fn timed_typed_drag_autoscroll_arms_once_clamps_catch_up_and_stops_at_boundary() {
        let bridge = crate::app(())
            .view(|_| {
                scroll(
                    button("source")
                        .filter_mapped(|_| None::<()>)
                        .width(100.0)
                        .height(240.0)
                        .id(1)
                        .drag_source(
                            DragSource::new(1_u8)
                                .autoscroll(DragAutoscrollPolicy::new(24.0, 1_000.0).unwrap()),
                        )
                        .id(10),
                )
                .width(100.0)
                .height(100.0)
                .id(20)
            })
            .update(|_, _: ()| {})
            .into_bridge();
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(100.0, 100.0));
        assert_eq!(runtime.layout.rects[&20].height(), 100.0);
        let origin = Instant::now();
        runtime.set_timed_repaint_clock(Some(origin));
        let token = runtime
            .dispatch_gesture_request(GestureRequest::new(pan(GesturePhase::Started, 0.0)))
            .token()
            .unwrap();
        runtime.dispatch_gesture_request(
            GestureRequest::new(pan(GesturePhase::Changed, 10.0)).with_token(token),
        );
        let first_deadline = runtime.typed_drag_autoscroll_deadline().unwrap();
        // Pointer traffic in the same edge zone must not postpone a stationary tick.
        runtime.dispatch_gesture_request(
            GestureRequest::new(pan(GesturePhase::Changed, 0.0)).with_token(token),
        );
        assert_eq!(
            runtime.interaction.drag.typed.as_ref().unwrap().position,
            Point::new(20.0, 98.0)
        );
        assert_eq!(
            runtime.typed_drag_autoscroll_deadline(),
            Some(first_deadline)
        );
        assert!(runtime.advance_typed_drag_autoscroll(first_deadline));
        let first = runtime
            .layout_state
            .scroll_offsets
            .values()
            .next()
            .unwrap()
            .y;
        let delayed = first_deadline + Duration::from_secs(1);
        assert!(runtime.advance_typed_drag_autoscroll(delayed));
        let second = runtime
            .layout_state
            .scroll_offsets
            .values()
            .next()
            .unwrap()
            .y;
        assert!(second - first <= 50.1);
        for _ in 0..32 {
            let Some(deadline) = runtime.typed_drag_autoscroll_deadline() else {
                break;
            };
            runtime.advance_typed_drag_autoscroll(deadline);
        }
        assert_eq!(runtime.layout_state.scroll_offset(20).y, 140.0);
        assert_eq!(runtime.typed_drag_autoscroll_deadline(), None);
    }

    #[test]
    fn focus_loss_makes_an_armed_typed_drag_timer_stale() {
        let bridge = crate::app(())
            .view(|_| {
                scroll(
                    button("source")
                        .filter_mapped(|_| None::<()>)
                        .width(100.0)
                        .height(240.0)
                        .id(1)
                        .drag_source(
                            DragSource::new(1_u8).autoscroll(DragAutoscrollPolicy::default()),
                        )
                        .id(10),
                )
                .width(100.0)
                .height(100.0)
                .id(20)
            })
            .update(|_, _: ()| {})
            .into_bridge();
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(160.0, 120.0));
        let origin = Instant::now();
        runtime.set_timed_repaint_clock(Some(origin));
        let token = runtime
            .dispatch_gesture_request(GestureRequest::new(pan(GesturePhase::Started, 0.0)))
            .token()
            .unwrap();
        runtime.dispatch_gesture_request(
            GestureRequest::new(pan(GesturePhase::Changed, 10.0)).with_token(token),
        );
        let deadline = runtime.typed_drag_autoscroll_deadline().unwrap();
        runtime.clear_focus();
        assert_eq!(runtime.typed_drag_autoscroll_deadline(), None);
        assert!(!runtime.advance_typed_drag_autoscroll(deadline));
    }

    #[test]
    fn autoscroll_edges_exclude_padding_and_reserved_scrollbars() {
        use crate::layout::{ScrollAxis, ScrollPolicy, ScrollbarPlacement};
        for reserved in [false, true] {
            let bridge = crate::app(())
                .view(move |_| {
                    scroll(
                        button("source")
                            .filter_mapped(|_| None::<()>)
                            .width(240.0)
                            .height(240.0)
                            .id(1)
                            .drag_source(
                                DragSource::new(1_u8).autoscroll(DragAutoscrollPolicy::default()),
                            )
                            .id(10),
                    )
                    .padding(if reserved { 0.0 } else { 10.0 })
                    .scroll_policy(
                        ScrollPolicy::default()
                            .axes(ScrollAxis::Both)
                            .scrollbar_placement(if reserved {
                                ScrollbarPlacement::Reserved
                            } else {
                                ScrollbarPlacement::Overlay
                            }),
                    )
                    .id(20)
                })
                .update(|_, _: ()| {})
                .into_bridge();
            let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(100.0, 100.0));
            runtime.set_timed_repaint_clock(Some(Instant::now()));
            let anchor = if reserved {
                Point::new(78.0, 20.0)
            } else {
                Point::new(20.0, 78.0)
            };
            let sample = |phase, amount| {
                GestureIngress::new(
                    GestureKind::Pan,
                    phase,
                    GestureUnit::LogicalPixels,
                    if reserved {
                        Vector2::new(amount, 0.0)
                    } else {
                        Vector2::new(0.0, amount)
                    },
                    InputDeviceId::from_host(1).unwrap(),
                    Some(anchor),
                    Default::default(),
                    None,
                    None,
                )
                .unwrap()
            };
            let token = runtime
                .dispatch_gesture_request(GestureRequest::new(sample(GesturePhase::Started, 0.0)))
                .token()
                .unwrap();
            runtime.dispatch_gesture_request(
                GestureRequest::new(sample(GesturePhase::Changed, 20.0)).with_token(token),
            );
            let outside = runtime.interaction.drag.typed.as_ref().unwrap().position;
            assert!(runtime.layout.rects[&20].contains(outside));
            assert!(!runtime.layout.viewport_bounds[&20].contains(outside));
            assert_eq!(runtime.typed_drag_autoscroll_deadline(), None);
            runtime.dispatch_gesture_request(
                GestureRequest::new(sample(
                    GesturePhase::Changed,
                    if reserved { -3.0 } else { -10.0 },
                ))
                .with_token(token),
            );
            let inside = runtime.interaction.drag.typed.as_ref().unwrap().position;
            assert!(runtime.layout.viewport_bounds[&20].contains(inside));
            let deadline = runtime.typed_drag_autoscroll_deadline().unwrap();
            assert!(runtime.advance_typed_drag_autoscroll(deadline));
            let offset = runtime.layout_state.scroll_offset(20);
            assert!(if reserved {
                offset.x > 0.0
            } else {
                offset.y > 0.0
            });
        }
    }

    #[test]
    fn modal_publication_retires_an_armed_autoscroll_timer() {
        use crate::application::{Layer, scene};
        let bridge = crate::app(false)
            .view(|open: &bool| {
                let base = scroll(
                    button("source")
                        .filter_mapped(|_| None::<()>)
                        .width(100.0)
                        .height(240.0)
                        .id(1)
                        .drag_source(
                            DragSource::new(1_u8).autoscroll(DragAutoscrollPolicy::default()),
                        )
                        .id(10),
                )
                .id(20);
                let mut root = scene(base);
                if *open {
                    root = root.layer(Layer::modal(
                        button("modal").filter_mapped(|_| None::<()>).id(2),
                    ));
                }
                root.into_view()
            })
            .update(|open, ()| *open = true)
            .into_bridge();
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(100.0, 100.0));
        runtime.set_timed_repaint_clock(Some(Instant::now()));
        let token = runtime
            .dispatch_gesture_request(GestureRequest::new(pan(GesturePhase::Started, 0.0)))
            .token()
            .unwrap();
        runtime.dispatch_gesture_request(
            GestureRequest::new(pan(GesturePhase::Changed, 10.0)).with_token(token),
        );
        let deadline = runtime.typed_drag_autoscroll_deadline().unwrap();
        runtime.dispatch_message(());
        assert!(!runtime.drag_session_active());
        assert_eq!(runtime.typed_drag_autoscroll_deadline(), None);
        let offset = runtime.layout_state.scroll_offset(20);
        assert!(!runtime.advance_typed_drag_autoscroll(deadline));
        assert_eq!(runtime.layout_state.scroll_offset(20), offset);
        assert_eq!(
            runtime
                .dispatch_gesture_request(
                    GestureRequest::new(pan(GesturePhase::Changed, 1.0)).with_token(token),
                )
                .outcome(),
            &crate::runtime::GestureOutcome::Stale
        );
    }

    #[test]
    fn nested_autoscroll_refresh_reselects_ancestors_and_source_removal_retires_timer() {
        use crate::application::{column, spacer};
        use crate::gui::input::{InputSequence, InputSequenceRange, InputTimestamp};
        use std::cell::RefCell;
        // Exercise both a compatible callback projection and source retirement.
        for remove_source in [false, true] {
            let updates = Rc::new(RefCell::new(Vec::new()));
            let observed = Rc::clone(&updates);
            let bridge = crate::app(false)
                .view(move |updated: &bool| {
                    let source = button("source")
                        .filter_mapped(|_| None::<crate::runtime::ScrollUpdate>)
                        .width(100.0)
                        .height(102.0)
                        .id(1);
                    let source = if remove_source && *updated {
                        source
                    } else {
                        source.drag_source(
                            DragSource::new(1_u8)
                                .autoscroll(DragAutoscrollPolicy::new(24.0, 1_000.0).unwrap()),
                        )
                    };
                    scroll(column([
                        scroll(source.id(10))
                            .width(100.0)
                            .height(100.0)
                            .id(20)
                            .on_scroll_update(|update| update),
                        spacer().height(140.0),
                    ]))
                    .width(100.0)
                    .height(100.0)
                    .id(30)
                    .on_scroll_update(|update| update)
                })
                .update(move |updated, update: crate::runtime::ScrollUpdate| {
                    *updated = true;
                    observed.borrow_mut().push(update);
                })
                .into_bridge();
            let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(160.0, 120.0));
            runtime.set_timed_repaint_clock(Some(Instant::now()));
            let token = runtime
                .dispatch_gesture_request(GestureRequest::new(pan(GesturePhase::Started, 0.0)))
                .token()
                .unwrap();
            let metadata = ScrollUpdateMetadata {
                modifiers: crate::widgets::PointerModifiers {
                    shift: true,
                    ..Default::default()
                },
                timestamp: Some(InputTimestamp::capture()),
                sequence_range: Some(InputSequenceRange::singleton(
                    InputSequence::from_runtime_value(7),
                )),
            };
            let changed = GestureIngress::new(
                GestureKind::Pan,
                GesturePhase::Changed,
                GestureUnit::LogicalPixels,
                Vector2::new(0.0, 10.0),
                InputDeviceId::from_host(1).unwrap(),
                Some(Point::new(20.0, 88.0)),
                metadata.modifiers,
                metadata.timestamp,
                metadata.sequence_range,
            )
            .unwrap();
            runtime.dispatch_gesture_request(GestureRequest::new(changed).with_token(token));
            let deadline = runtime.typed_drag_autoscroll_deadline().unwrap();
            assert!(runtime.advance_typed_drag_autoscroll(deadline));
            assert_eq!(updates.borrow().len(), 1);
            assert_eq!(updates.borrow()[0].node_id, 20);
            assert_eq!(updates.borrow()[0].metadata, metadata);
            assert_eq!(
                runtime.layout_state.scroll_offset(30).y,
                0.0,
                "a callback projection must stop the saved ancestor chain"
            );
            if remove_source {
                assert_eq!(runtime.typed_drag_autoscroll_deadline(), None);
                assert!(!runtime.advance_typed_drag_autoscroll(deadline + Duration::from_secs(1)));
                assert_eq!(updates.borrow().len(), 1);
            } else {
                assert!(runtime.typed_drag_live(token));
                let next = runtime.typed_drag_autoscroll_deadline().unwrap();
                assert!(runtime.advance_typed_drag_autoscroll(next));
                assert!(
                    runtime.layout_state.scroll_offset(30).y > 0.0,
                    "a fresh tick must chain beyond the clamped inner viewport"
                );
                assert_eq!(updates.borrow().last().unwrap().node_id, 30);
            }
        }
    }
}

#[cfg(test)]
mod cross_window_tests;

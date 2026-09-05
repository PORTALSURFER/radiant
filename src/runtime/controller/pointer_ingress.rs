//! Runtime admission for typed pointer and gesture ingress.

use super::SurfaceRuntime;
use crate::runtime::WidgetDispatchResult;
use crate::{
    gui::layout_core::LayoutTargetIdentity,
    gui::pointer_ingress::{
        DeviceKind, GestureIngress, GestureIngressDisposition, PointerEvent, PointerIngress,
        PointerIngressAdmission, PointerIngressDisposition, PointerPhase,
        PointerSequenceAllocationError, PointerSequenceAllocator, PointerSequenceToken,
    },
    gui::types::Point,
    layout::NodeId,
    runtime::RuntimeBridge,
    runtime::controller::interaction_state::ScrollbarAxis,
    widgets::{PointerButton, PointerModifiers, WidgetId},
};

const MAX_POINTER_SEQUENCES: usize = 16;

/// Private evidence threaded through the existing pointer router.  It carries
/// no routing authority; the existing press/move/release paths still decide
/// focus, layout, scrollbar, managed capture, and hit testing.
#[derive(Clone, Copy, Debug)]
pub(super) struct TypedPointerDeliveryContext {
    pub(super) ingress: PointerIngress,
    pub(super) event: Option<PointerEvent>,
    pub(super) record_index: Option<usize>,
    pub(super) route: Option<TypedPointerRoute>,
}

/// The existing input route that actually admitted a press.  This is evidence
/// for the bounded ingress table; it never replaces the existing capture
/// owner or its continuation router.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum TypedPointerRoute {
    Widget(WidgetId),
    Layout {
        identity: LayoutTargetIdentity,
        contract_version: u16,
        button: PointerButton,
    },
    Scrollbar {
        node_id: NodeId,
        axis: ScrollbarAxis,
        button: PointerButton,
    },
}

impl TypedPointerDeliveryContext {
    pub(super) fn new(ingress: PointerIngress) -> Self {
        Self {
            ingress,
            event: None,
            record_index: None,
            route: None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct PointerSequenceRecord {
    pub(super) token: PointerSequenceToken,
    pub(super) device: crate::gui::pointer_ingress::InputDeviceId,
    pub(super) contact: crate::gui::pointer_ingress::PointerContactId,
    pub(super) kind: DeviceKind,
    pub(super) button: PointerButton,
    pub(super) owner: Option<PointerOwnerWitness>,
    pub(super) last_position: Point,
    pub(super) last_buttons: crate::gui::pointer_ingress::PointerButtons,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PointerOwnerWitness {
    Widget {
        id: WidgetId,
        button: PointerButton,
        managed: bool,
        compatibility_kind: Option<&'static str>,
    },
    Layout {
        identity: LayoutTargetIdentity,
        contract_version: u16,
        button: PointerButton,
    },
    Scrollbar {
        node_id: NodeId,
        axis: ScrollbarAxis,
        button: PointerButton,
    },
    Unsupported,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct PointerIngressState {
    pub(super) allocator: PointerSequenceAllocator,
    pub(super) records: [Option<PointerSequenceRecord>; MAX_POINTER_SEQUENCES],
}

impl PointerIngressState {
    pub(super) fn new(runtime_identity: u64) -> Self {
        let allocator = PointerSequenceAllocator::new(runtime_identity)
            .unwrap_or_else(|_| PointerSequenceAllocator::invalid());
        Self {
            allocator,
            records: [None; MAX_POINTER_SEQUENCES],
        }
    }

    fn find(&self, ingress: PointerIngress) -> Option<(usize, PointerSequenceRecord)> {
        self.records.iter().enumerate().find_map(|(index, record)| {
            let record = (*record)?;
            (record.device == ingress.device()
                && record.contact == ingress.contact()
                && record.kind == ingress.kind()
                && !matches!(ingress.phase(), PointerPhase::Ended { button } if button != record.button)
                && ingress.token().is_some_and(|token| token == record.token))
            .then_some((index, record))
        })
    }

    fn issue(
        &mut self,
        ingress: PointerIngress,
    ) -> Result<(usize, PointerEvent), PointerIngressDisposition> {
        if self
            .records
            .iter()
            .flatten()
            .any(|record| record.device == ingress.device() && record.contact == ingress.contact())
        {
            return Err(PointerIngressDisposition::Blocked);
        }
        let Some(index) = self.records.iter().position(Option::is_none) else {
            return Err(PointerIngressDisposition::CapacityExhausted);
        };
        let token = self.allocator.issue().map_err(|error| match error {
            PointerSequenceAllocationError::Exhausted => {
                PointerIngressDisposition::IdentityExhausted
            }
            PointerSequenceAllocationError::InvalidRuntimeIdentity => {
                PointerIngressDisposition::IdentityExhausted
            }
        })?;
        let button = match ingress.phase() {
            PointerPhase::Started { button } => button,
            _ => return Err(PointerIngressDisposition::Invalid),
        };
        let event = PointerEvent::from_ingress(ingress.with_token(token), Some(token));
        self.records[index] = Some(PointerSequenceRecord {
            token,
            device: ingress.device(),
            contact: ingress.contact(),
            kind: ingress.kind(),
            button,
            owner: None,
            last_position: ingress.logical_position(),
            last_buttons: ingress.buttons(),
        });
        Ok((index, event))
    }

    fn can_issue(&self, ingress: PointerIngress) -> Result<(), PointerIngressDisposition> {
        if self
            .records
            .iter()
            .flatten()
            .any(|record| record.device == ingress.device() && record.contact == ingress.contact())
        {
            return Err(PointerIngressDisposition::Blocked);
        }
        if self.records.iter().all(Option::is_some) {
            return Err(PointerIngressDisposition::CapacityExhausted);
        }
        let mut allocator = self.allocator;
        allocator.issue().map(|_| ()).map_err(|error| match error {
            PointerSequenceAllocationError::Exhausted
            | PointerSequenceAllocationError::InvalidRuntimeIdentity => {
                PointerIngressDisposition::IdentityExhausted
            }
        })
    }
}

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    /// Route one checked pointer sample through the bounded runtime admission
    /// table. Consumers opt in by mapping `WidgetInput::Pointer`.
    pub fn dispatch_pointer_ingress(
        &mut self,
        ingress: PointerIngress,
    ) -> PointerIngressDisposition {
        if !self.lifecycle_accepts_work() {
            return PointerIngressDisposition::Blocked;
        }
        // Only mouse samples are allowed to enter the existing capture and
        // hit-test router. Nonmouse samples remain explicit unsupported
        // sequences so their token can be carried by a host without creating
        // synthetic mouse ownership or disturbing an active mouse sequence.
        if ingress.kind() != DeviceKind::Mouse {
            return self.dispatch_unsupported_pointer_ingress(ingress);
        }
        match ingress.phase() {
            PointerPhase::Hover => {
                let event = PointerEvent::from_ingress(ingress, None);
                let dispatch = self.dispatch_pointer_move_target_with_delivery(
                    event.logical_position(),
                    true,
                    event.modifiers(),
                    event.timestamp(),
                    None,
                    Some(event),
                );
                dispatch
                    .target
                    .map_or(PointerIngressDisposition::Blocked, |owner| {
                        if self.surface.widget_has_pointer_mapper(owner) {
                            PointerIngressDisposition::RoutedWidget(owner)
                        } else {
                            PointerIngressDisposition::AdmittedUnsupportedConsumer
                        }
                    })
            }
            PointerPhase::Started { .. } => {
                if let Err(disposition) = self.interaction.pointer.ingress.can_issue(ingress) {
                    return disposition;
                }
                if self
                    .interaction
                    .pointer
                    .ingress
                    .records
                    .iter()
                    .flatten()
                    .any(|record| {
                        record.device == ingress.device() && record.contact == ingress.contact()
                    })
                {
                    return PointerIngressDisposition::Blocked;
                }
                let mut delivery = TypedPointerDeliveryContext::new(ingress);
                let routed = self.dispatch_pointer_press_event_with_delivery(
                    ingress.logical_position(),
                    match ingress.phase() {
                        PointerPhase::Started { button } => button,
                        _ => return PointerIngressDisposition::Invalid,
                    },
                    ingress.modifiers(),
                    ingress.timestamp(),
                    Some(&mut delivery),
                );
                let route = delivery
                    .route
                    .or_else(|| routed.map(TypedPointerRoute::Widget));
                let Some(route) = route else {
                    if let Some(index) = delivery.record_index {
                        self.interaction.pointer.ingress.records[index] = None;
                    }
                    return PointerIngressDisposition::Blocked;
                };
                let (index, event) = match delivery.event {
                    Some(event) => (
                        delivery.record_index.expect("typed record installed"),
                        event,
                    ),
                    None => match self.interaction.pointer.ingress.issue(ingress) {
                        Ok(value) => value,
                        Err(disposition) => return disposition,
                    },
                };
                let witness = self.pointer_owner_witness(route, ingress);
                if let Some(record) = self.interaction.pointer.ingress.records[index].as_mut() {
                    record.owner = Some(witness);
                }
                let _ = event;
                match route {
                    TypedPointerRoute::Widget(owner) => {
                        PointerIngressDisposition::RoutedWidget(owner)
                    }
                    TypedPointerRoute::Layout { .. } => PointerIngressDisposition::HandledLayout,
                    TypedPointerRoute::Scrollbar { .. } => {
                        PointerIngressDisposition::HandledScrollbar
                    }
                }
            }
            PointerPhase::Moved | PointerPhase::Ended { .. } | PointerPhase::Cancelled => {
                let Some((index, record)) = self.interaction.pointer.ingress.find(ingress) else {
                    return PointerIngressDisposition::Stale;
                };
                let token = record.token;
                let event = PointerEvent::from_ingress(ingress, Some(token));
                let disposition = if matches!(ingress.phase(), PointerPhase::Cancelled) {
                    match record.owner {
                        Some(PointerOwnerWitness::Widget { id, .. })
                            if self.pointer_widget_witness_is_current(record.owner) =>
                        {
                            let delivered = self.cancel_pointer_capture_with_delivery(
                                self.surface.widget_has_pointer_mapper(id).then_some(event),
                            );
                            if delivered {
                                PointerIngressDisposition::RoutedWidget(id)
                            } else {
                                PointerIngressDisposition::Blocked
                            }
                        }
                        Some(PointerOwnerWitness::Layout { .. }) => {
                            self.cancel_pointer_capture_with_delivery(None);
                            PointerIngressDisposition::HandledLayout
                        }
                        Some(PointerOwnerWitness::Scrollbar { .. }) => {
                            self.cancel_pointer_capture_with_delivery(None);
                            PointerIngressDisposition::HandledScrollbar
                        }
                        Some(PointerOwnerWitness::Unsupported) => {
                            self.cancel_pointer_capture_with_delivery(None);
                            PointerIngressDisposition::AdmittedUnsupportedConsumer
                        }
                        _ => PointerIngressDisposition::Stale,
                    }
                } else if matches!(ingress.phase(), PointerPhase::Ended { .. }) {
                    let button = match ingress.phase() {
                        PointerPhase::Ended { button } => button,
                        _ => unreachable!(),
                    };
                    match record.owner {
                        Some(PointerOwnerWitness::Widget { id, .. })
                            if self.pointer_widget_witness_is_current(record.owner) =>
                        {
                            let typed = self.surface.widget_has_pointer_mapper(id).then_some(event);
                            self.dispatch_pointer_release_event_with_delivery(
                                event.logical_position(),
                                button,
                                event.modifiers(),
                                event.timestamp(),
                                typed,
                            )
                            .map_or(PointerIngressDisposition::Blocked, |owner| {
                                PointerIngressDisposition::RoutedWidget(owner)
                            })
                        }
                        Some(PointerOwnerWitness::Layout { .. })
                            if self.interaction.layout_capture.is_some() =>
                        {
                            let dispatch = self.dispatch_captured_layout_input(
                                crate::layout::LayoutInput::PointerRelease {
                                    position: event.logical_position(),
                                    button,
                                    modifiers: event.modifiers(),
                                    timestamp: event.timestamp(),
                                },
                                true,
                            );
                            if dispatch.handled {
                                PointerIngressDisposition::HandledLayout
                            } else {
                                PointerIngressDisposition::Blocked
                            }
                        }
                        Some(PointerOwnerWitness::Scrollbar { .. })
                            if self.interaction.pointer.scroll_drag_capture.is_some() =>
                        {
                            self.dispatch_pointer_release_event_with_delivery(
                                event.logical_position(),
                                button,
                                event.modifiers(),
                                event.timestamp(),
                                None,
                            );
                            PointerIngressDisposition::HandledScrollbar
                        }
                        Some(PointerOwnerWitness::Unsupported) => {
                            PointerIngressDisposition::AdmittedUnsupportedConsumer
                        }
                        _ => PointerIngressDisposition::Stale,
                    }
                } else {
                    match record.owner {
                        Some(PointerOwnerWitness::Widget { id, .. })
                            if self.pointer_widget_witness_is_current(record.owner) =>
                        {
                            let typed = self.surface.widget_has_pointer_mapper(id).then_some(event);
                            let moved = self.dispatch_pointer_move_to_exact_target(
                                event.logical_position(),
                                id,
                                true,
                                event.modifiers(),
                                event.timestamp(),
                                event.sequence_range(),
                                typed,
                            );
                            moved
                                .target
                                .map_or(PointerIngressDisposition::Blocked, |target| {
                                    PointerIngressDisposition::RoutedWidget(target)
                                })
                        }
                        Some(PointerOwnerWitness::Layout {
                            identity,
                            contract_version,
                            ..
                        }) if self
                            .interaction
                            .layout_capture
                            .as_ref()
                            .is_some_and(|capture| {
                                capture.identity == identity
                                    && capture.contract_version == contract_version
                            }) =>
                        {
                            let dispatch = self.dispatch_captured_layout_input(
                                crate::layout::LayoutInput::PointerMove {
                                    position: event.logical_position(),
                                    modifiers: event.modifiers(),
                                    timestamp: event.timestamp(),
                                    sequence_range: event.sequence_range(),
                                },
                                true,
                            );
                            if dispatch.handled {
                                PointerIngressDisposition::HandledLayout
                            } else {
                                PointerIngressDisposition::Blocked
                            }
                        }
                        Some(PointerOwnerWitness::Scrollbar { node_id, axis, .. })
                            if self.interaction.pointer.scroll_drag_capture.is_some_and(
                                |capture| capture.node_id == node_id && capture.axis == axis,
                            ) =>
                        {
                            if self.drag_scrollbar_to(
                                event.logical_position(),
                                true,
                                crate::runtime::ScrollUpdateMetadata {
                                    modifiers: event.modifiers(),
                                    timestamp: event.timestamp(),
                                    sequence_range: event.sequence_range(),
                                },
                            ) {
                                PointerIngressDisposition::HandledScrollbar
                            } else {
                                PointerIngressDisposition::Blocked
                            }
                        }
                        Some(PointerOwnerWitness::Unsupported) => {
                            PointerIngressDisposition::AdmittedUnsupportedConsumer
                        }
                        _ => PointerIngressDisposition::Stale,
                    }
                };
                if matches!(
                    ingress.phase(),
                    PointerPhase::Ended { .. } | PointerPhase::Cancelled
                ) {
                    self.interaction.pointer.ingress.records[index] = None;
                } else if !matches!(disposition, PointerIngressDisposition::Stale)
                    && let Some(record) = self.interaction.pointer.ingress.records[index].as_mut()
                {
                    record.last_position = ingress.logical_position();
                    record.last_buttons = ingress.buttons();
                }
                disposition
            }
        }
    }

    fn dispatch_unsupported_pointer_ingress(
        &mut self,
        ingress: PointerIngress,
    ) -> PointerIngressDisposition {
        if matches!(ingress.phase(), PointerPhase::Hover) {
            return PointerIngressDisposition::AdmittedUnsupportedConsumer;
        }
        if matches!(ingress.phase(), PointerPhase::Started { .. }) {
            if let Err(disposition) = self.interaction.pointer.ingress.can_issue(ingress) {
                return disposition;
            }
            let (index, _) = match self.interaction.pointer.ingress.issue(ingress) {
                Ok(value) => value,
                Err(disposition) => return disposition,
            };
            if let Some(record) = self.interaction.pointer.ingress.records[index].as_mut() {
                record.owner = Some(PointerOwnerWitness::Unsupported);
            }
            return PointerIngressDisposition::AdmittedUnsupportedConsumer;
        }

        let Some((index, record)) = self.interaction.pointer.ingress.find(ingress) else {
            return PointerIngressDisposition::Stale;
        };
        if !matches!(record.owner, Some(PointerOwnerWitness::Unsupported)) {
            return PointerIngressDisposition::Stale;
        }
        if matches!(
            ingress.phase(),
            PointerPhase::Ended { .. } | PointerPhase::Cancelled
        ) {
            self.interaction.pointer.ingress.records[index] = None;
        } else if let Some(record) = self.interaction.pointer.ingress.records[index].as_mut() {
            record.last_position = ingress.logical_position();
            record.last_buttons = ingress.buttons();
        }
        PointerIngressDisposition::AdmittedUnsupportedConsumer
    }

    /// Admit one checked sample and return the opaque runtime token for a
    /// started sequence. This is the host-facing continuation handoff for
    /// layout, scrollbar, and unsupported consumers without a widget callback.
    pub fn dispatch_pointer_ingress_with_admission(
        &mut self,
        ingress: PointerIngress,
    ) -> PointerIngressAdmission {
        let identity = (ingress.device(), ingress.contact());
        let phase = ingress.phase();
        let disposition = self.dispatch_pointer_ingress(ingress);
        let token = if matches!(phase, PointerPhase::Started { .. }) {
            self.interaction
                .pointer
                .ingress
                .records
                .iter()
                .flatten()
                .find(|record| record.device == identity.0 && record.contact == identity.1)
                .map(|record| record.token)
        } else {
            ingress.token()
        };
        PointerIngressAdmission::new(disposition, token)
    }

    /// Normalize a gesture at the controller boundary. Gesture arena
    /// recognition is intentionally deferred; valid samples are reported as an
    /// explicitly admitted unsupported consumer in this phase.
    pub fn dispatch_gesture_ingress(
        &mut self,
        ingress: GestureIngress,
    ) -> GestureIngressDisposition {
        if !self.lifecycle_accepts_work() {
            GestureIngressDisposition::Blocked
        } else {
            let _ = ingress;
            GestureIngressDisposition::AdmittedUnsupportedConsumer
        }
    }

    /// Native adapters use the controller-issued token already retained for
    /// this device/contact.  The helper stays crate-private so a host cannot
    /// look up a sequence by identity and bypass the public checked ingress.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn dispatch_native_pointer_continuation(
        &mut self,
        kind: DeviceKind,
        device: crate::gui::pointer_ingress::InputDeviceId,
        contact: crate::gui::pointer_ingress::PointerContactId,
        token: PointerSequenceToken,
        phase: PointerPhase,
        position: Point,
        buttons: crate::gui::pointer_ingress::PointerButtons,
        modifiers: PointerModifiers,
        pressure: Option<crate::gui::pointer_ingress::PointerPressure>,
        tilt: Option<crate::gui::pointer_ingress::PointerTilt>,
        timestamp: Option<crate::gui::input::InputTimestamp>,
        sequence_range: Option<crate::gui::input::InputSequenceRange>,
    ) -> PointerIngressDisposition {
        let Ok(ingress) = PointerIngress::from_runtime(
            kind,
            device,
            contact,
            phase,
            position,
            buttons,
            modifiers,
            pressure,
            tilt,
            timestamp,
            sequence_range,
            token,
        ) else {
            return PointerIngressDisposition::Invalid;
        };
        self.dispatch_pointer_ingress(ingress)
    }

    /// Drain admitted pointer records through the same owner-aware terminal
    /// path used by native cancellation before the lifecycle fences work.
    pub(crate) fn cancel_pointer_ingress_sequences(&mut self) {
        let records = self.interaction.pointer.ingress.records;
        for record in records.into_iter().flatten() {
            let Ok(ingress) = PointerIngress::from_runtime(
                record.kind,
                record.device,
                record.contact,
                PointerPhase::Cancelled,
                record.last_position,
                record.last_buttons,
                PointerModifiers::default(),
                None,
                None,
                None,
                None,
                record.token,
            ) else {
                continue;
            };
            let _ = self.dispatch_pointer_ingress(ingress);
        }
        self.interaction.pointer.ingress.records = [None; MAX_POINTER_SEQUENCES];
    }

    /// Reconcile fixed records after the existing capture/layout reconciliation
    /// boundary. Compatible owners remain attached; retired witnesses receive
    /// one cancellation through the ordinary owner-aware path.
    pub(crate) fn reconcile_pointer_ingress_sequences(&mut self) {
        let records = self.interaction.pointer.ingress.records;
        for record in records.into_iter().flatten() {
            let compatible = match record.owner {
                Some(PointerOwnerWitness::Widget { .. }) => {
                    self.pointer_widget_witness_is_current(record.owner)
                }
                Some(PointerOwnerWitness::Layout {
                    identity,
                    contract_version,
                    ..
                }) => self
                    .interaction
                    .layout_capture
                    .as_ref()
                    .is_some_and(|capture| {
                        capture.identity == identity && capture.contract_version == contract_version
                    }),
                Some(PointerOwnerWitness::Scrollbar { node_id, axis, .. }) => self
                    .interaction
                    .pointer
                    .scroll_drag_capture
                    .is_some_and(|capture| capture.node_id == node_id && capture.axis == axis),
                Some(PointerOwnerWitness::Unsupported) | None => true,
            };
            if compatible {
                continue;
            }
            let Ok(ingress) = PointerIngress::from_runtime(
                record.kind,
                record.device,
                record.contact,
                PointerPhase::Cancelled,
                record.last_position,
                record.last_buttons,
                PointerModifiers::default(),
                None,
                None,
                None,
                None,
                record.token,
            ) else {
                continue;
            };
            let _ = self.dispatch_pointer_ingress(ingress);
        }
    }

    /// Retire incompatible widget ingress while the old surface and its
    /// mapper are still live. The replacement surface is inspected only as
    /// evidence; it never receives the old terminal callback.
    pub(in crate::runtime::controller) fn reconcile_pointer_ingress_before_surface_replace(
        &mut self,
        next_surface: &crate::runtime::UiSurface<Message>,
        previous_paths: &std::collections::HashMap<WidgetId, crate::runtime::WidgetPath>,
        current_paths: &std::collections::HashMap<WidgetId, crate::runtime::WidgetPath>,
        retired_widget_ids: &[WidgetId],
        terminal_messages: &mut Vec<Message>,
    ) {
        let records = self.interaction.pointer.ingress.records;
        for record in records.into_iter().flatten() {
            let Some(PointerOwnerWitness::Widget { id, .. }) = record.owner else {
                continue;
            };
            let compatible = !retired_widget_ids.contains(&id)
                && previous_paths.get(&id).is_some_and(|previous_path| {
                    current_paths.get(&id).is_some_and(|current_path| {
                        self.surface
                            .widget_compatibility_at_path(previous_path.as_slice())
                            .zip(next_surface.widget_compatibility_at_path(current_path.as_slice()))
                            .is_some_and(|((old_kind, old_valid), (new_kind, new_valid))| {
                                old_valid && new_valid && old_kind == new_kind
                            })
                    })
                });
            if compatible {
                continue;
            }
            let active = self.pointer_widget_witness_is_current(record.owner);
            let old_mapper = active && self.surface.widget_has_pointer_mapper(id);
            let old_event = old_mapper.then(|| {
                PointerIngress::from_runtime(
                    record.kind,
                    record.device,
                    record.contact,
                    PointerPhase::Cancelled,
                    record.last_position,
                    record.last_buttons,
                    PointerModifiers::default(),
                    None,
                    None,
                    None,
                    None,
                    record.token,
                )
                .ok()
                .map(|ingress| PointerEvent::from_ingress(ingress, Some(record.token)))
            });
            let old_event = old_event.flatten();
            // Remove the ingress authority and existing capture before invoking
            // the old mapper. A callback may synchronously request another
            // refresh; the retired sequence must already be inert then.
            if let Some(index) =
                self.interaction
                    .pointer
                    .ingress
                    .records
                    .iter()
                    .position(|candidate| {
                        candidate.is_some_and(|candidate| candidate.token == record.token)
                    })
            {
                self.interaction.pointer.ingress.records[index] = None;
            }
            if active {
                self.cancel_pointer_capture_with_delivery(None);
            }
            if let Some(event) = old_event {
                let Some(bounds) = self.layout.rects.get(&id).copied() else {
                    continue;
                };
                let result = if let Some(child_path) = self.traversal.widgets.paths.current.get(&id)
                {
                    self.surface.dispatch_widget_pointer_event_message_at_path(
                        id, child_path, bounds, event,
                    )
                } else {
                    self.surface
                        .dispatch_widget_pointer_event_message(id, bounds, event)
                };
                if let Some(WidgetDispatchResult::Message(message)) = result {
                    terminal_messages.push(message);
                }
            }
        }
    }

    /// Deterministic hover wrapper for hosts that do not need to construct the
    /// full checked sample manually.
    pub fn dispatch_pointer_hover(
        &mut self,
        kind: DeviceKind,
        device: crate::gui::pointer_ingress::InputDeviceId,
        contact: crate::gui::pointer_ingress::PointerContactId,
        position: Point,
        buttons: crate::gui::pointer_ingress::PointerButtons,
        modifiers: PointerModifiers,
    ) -> PointerIngressDisposition {
        let ingress = PointerIngress::new(
            kind,
            device,
            contact,
            PointerPhase::Hover,
            position,
            buttons,
            modifiers,
            None,
            None,
            None,
            None,
        );
        ingress.map_or(PointerIngressDisposition::Invalid, |ingress| {
            self.dispatch_pointer_ingress(ingress)
        })
    }

    /// Admit a deterministic new pointer sequence for one device/contact.
    #[allow(clippy::too_many_arguments)]
    pub fn dispatch_pointer_start(
        &mut self,
        kind: DeviceKind,
        device: crate::gui::pointer_ingress::InputDeviceId,
        contact: crate::gui::pointer_ingress::PointerContactId,
        position: Point,
        button: PointerButton,
        buttons: crate::gui::pointer_ingress::PointerButtons,
        modifiers: PointerModifiers,
    ) -> PointerIngressDisposition {
        let ingress = PointerIngress::new(
            kind,
            device,
            contact,
            PointerPhase::Started { button },
            position,
            buttons,
            modifiers,
            None,
            None,
            None,
            None,
        );
        ingress.map_or(PointerIngressDisposition::Invalid, |ingress| {
            self.dispatch_pointer_ingress(ingress)
        })
    }

    /// Route a deterministic continuation for the current device/contact.
    #[allow(dead_code)]
    pub(crate) fn dispatch_pointer_move(
        &mut self,
        kind: DeviceKind,
        device: crate::gui::pointer_ingress::InputDeviceId,
        contact: crate::gui::pointer_ingress::PointerContactId,
        position: Point,
        buttons: crate::gui::pointer_ingress::PointerButtons,
        modifiers: PointerModifiers,
    ) -> PointerIngressDisposition {
        self.dispatch_pointer_continuation(
            kind,
            device,
            contact,
            PointerPhase::Moved,
            position,
            buttons,
            modifiers,
        )
    }

    /// Route a deterministic terminal release for the current device/contact.
    #[allow(clippy::too_many_arguments)]
    #[allow(dead_code)]
    pub(crate) fn dispatch_pointer_end(
        &mut self,
        kind: DeviceKind,
        device: crate::gui::pointer_ingress::InputDeviceId,
        contact: crate::gui::pointer_ingress::PointerContactId,
        position: Point,
        button: PointerButton,
        buttons: crate::gui::pointer_ingress::PointerButtons,
        modifiers: PointerModifiers,
    ) -> PointerIngressDisposition {
        self.dispatch_pointer_continuation(
            kind,
            device,
            contact,
            PointerPhase::Ended { button },
            position,
            buttons,
            modifiers,
        )
    }

    /// Cancel the current device/contact sequence exactly once.
    #[allow(dead_code)]
    pub(crate) fn dispatch_pointer_cancel(
        &mut self,
        kind: DeviceKind,
        device: crate::gui::pointer_ingress::InputDeviceId,
        contact: crate::gui::pointer_ingress::PointerContactId,
        position: Point,
        buttons: crate::gui::pointer_ingress::PointerButtons,
        modifiers: PointerModifiers,
    ) -> PointerIngressDisposition {
        self.dispatch_pointer_continuation(
            kind,
            device,
            contact,
            PointerPhase::Cancelled,
            position,
            buttons,
            modifiers,
        )
    }

    #[allow(clippy::too_many_arguments)]
    #[allow(dead_code)]
    fn dispatch_pointer_continuation(
        &mut self,
        kind: DeviceKind,
        device: crate::gui::pointer_ingress::InputDeviceId,
        contact: crate::gui::pointer_ingress::PointerContactId,
        phase: PointerPhase,
        position: Point,
        buttons: crate::gui::pointer_ingress::PointerButtons,
        modifiers: PointerModifiers,
    ) -> PointerIngressDisposition {
        let Some(token) = self
            .interaction
            .pointer
            .ingress
            .records
            .iter()
            .flatten()
            .find(|record| {
                record.kind == kind && record.device == device && record.contact == contact
            })
            .map(|record| record.token)
        else {
            return PointerIngressDisposition::Stale;
        };
        let Ok(ingress) = PointerIngress::from_runtime(
            kind, device, contact, phase, position, buttons, modifiers, None, None, None, None,
            token,
        ) else {
            return PointerIngressDisposition::Invalid;
        };
        self.dispatch_pointer_ingress(ingress)
    }

    pub(super) fn issue_pointer_delivery(
        &mut self,
        context: &mut TypedPointerDeliveryContext,
    ) -> Result<PointerEvent, PointerIngressDisposition> {
        let (index, event) = self.interaction.pointer.ingress.issue(context.ingress)?;
        context.record_index = Some(index);
        context.event = Some(event);
        Ok(event)
    }

    fn pointer_owner_witness(
        &self,
        route: TypedPointerRoute,
        ingress: PointerIngress,
    ) -> PointerOwnerWitness {
        let PointerPhase::Started { button } = ingress.phase() else {
            return PointerOwnerWitness::Unsupported;
        };
        match route {
            TypedPointerRoute::Widget(widget_id) => PointerOwnerWitness::Widget {
                id: widget_id,
                button,
                managed: self.interaction.pointer.managed_capture.is_some(),
                compatibility_kind: self.pointer_press_target_compatibility_kind(widget_id),
            },
            TypedPointerRoute::Layout {
                identity,
                contract_version,
                button,
            } => PointerOwnerWitness::Layout {
                identity,
                contract_version,
                button,
            },
            TypedPointerRoute::Scrollbar {
                node_id,
                axis,
                button,
            } => PointerOwnerWitness::Scrollbar {
                node_id,
                axis,
                button,
            },
        }
    }

    fn pointer_widget_witness_is_current(&self, witness: Option<PointerOwnerWitness>) -> bool {
        let Some(PointerOwnerWitness::Widget {
            id,
            button,
            managed,
            compatibility_kind,
        }) = witness
        else {
            return false;
        };
        if managed {
            self.interaction
                .pointer
                .managed_capture
                .is_some_and(|capture| {
                    capture.widget_id == id
                        && capture.button == button
                        && self.pointer_press_target_compatibility_kind(id) == compatibility_kind
                })
        } else {
            self.interaction.pointer.capture == Some(id)
                && self.interaction.pointer.capture_button == Some(button)
                && self.pointer_press_target_compatibility_kind(id) == compatibility_kind
        }
    }

    pub(super) fn dispatch_pointer_output(
        &mut self,
        widget_id: WidgetId,
        event: PointerEvent,
    ) -> bool {
        if !self.lifecycle_accepts_work() {
            return false;
        }
        let Some(bounds) = self.layout.rects.get(&widget_id).copied() else {
            return false;
        };
        let result = if let Some(child_path) = self.traversal.widgets.paths.current.get(&widget_id)
        {
            self.surface
                .dispatch_widget_pointer_event_message_at_path(widget_id, child_path, bounds, event)
        } else {
            self.surface
                .dispatch_widget_pointer_event_message(widget_id, bounds, event)
        };
        let Some(result) = result else {
            return false;
        };
        match result {
            WidgetDispatchResult::Message(message) => {
                let outcome = self.dispatch_message(message);
                self.pending_input_command_outcome.merge(outcome);
                true
            }
            WidgetDispatchResult::UnmappedOutput => {
                self.relayout();
                true
            }
            WidgetDispatchResult::NoOutput => false,
        }
    }
}

#[cfg(test)]
mod production_route_tests {
    use super::*;
    use crate::{
        gui::types::Vector2,
        runtime::GpuSurfaceContent,
        runtime::{SurfaceNode, UiSurface, WidgetMessageMapper, test_arc_surface},
        widgets::{
            ButtonWidget, CanvasMessage, CanvasWidget, GpuSurfaceParts, GpuSurfaceWidget,
            PointerButton, TextWidget, Widget, WidgetInput, WidgetOutput, WidgetSizing,
        },
    };
    use std::{cell::RefCell, rc::Rc, sync::Arc};

    #[derive(Clone, Default)]
    struct PointerTraceBridge {
        events: Rc<RefCell<Vec<PointerEvent>>>,
    }

    #[derive(Clone, Default)]
    struct LegacyCanvasBridge {
        messages: Rc<RefCell<Vec<CanvasMessage>>>,
    }

    impl RuntimeBridge<CanvasMessage> for LegacyCanvasBridge {
        fn project_surface(&mut self) -> Arc<UiSurface<CanvasMessage>> {
            test_arc_surface(UiSurface::new(SurfaceNode::widget(
                CanvasWidget::new(1, WidgetSizing::fixed(Vector2::new(120.0, 80.0))),
                WidgetMessageMapper::canvas(|message| message),
            )))
        }

        fn reduce_message(&mut self, message: CanvasMessage) {
            self.messages.borrow_mut().push(message);
        }
    }

    impl RuntimeBridge<PointerEvent> for PointerTraceBridge {
        fn project_surface(&mut self) -> Arc<UiSurface<PointerEvent>> {
            test_arc_surface(UiSurface::new(SurfaceNode::widget(
                CanvasWidget::new(1, WidgetSizing::fixed(Vector2::new(120.0, 80.0))),
                WidgetMessageMapper::canvas_pointer(|event| event),
            )))
        }

        fn reduce_message(&mut self, message: PointerEvent) {
            self.events.borrow_mut().push(message);
        }
    }

    #[derive(Clone, Default)]
    struct GpuPointerTraceBridge {
        events: Rc<RefCell<Vec<PointerEvent>>>,
    }

    #[derive(Clone)]
    struct SurfaceReplacementPointerBridge {
        replacement: Rc<RefCell<bool>>,
        old_events: Rc<RefCell<Vec<PointerEvent>>>,
        new_events: Rc<RefCell<Vec<PointerEvent>>>,
    }

    impl RuntimeBridge<PointerEvent> for GpuPointerTraceBridge {
        fn project_surface(&mut self) -> Arc<UiSurface<PointerEvent>> {
            let content = GpuSurfaceContent::SignalBands {
                frames: 1,
                band_count: 1,
                frame_range: [0.0, 1.0],
                samples: Arc::from([0.0_f32]),
            };
            test_arc_surface(UiSurface::new(SurfaceNode::widget(
                GpuSurfaceWidget::from_parts(GpuSurfaceParts {
                    id: 1,
                    sizing: WidgetSizing::fixed(Vector2::new(120.0, 80.0)),
                    key: 1,
                    revision: 1,
                    content,
                })
                .with_input_events(true),
                WidgetMessageMapper::dynamic_pointer(|output| {
                    output.typed_cloned::<PointerEvent>()
                }),
            )))
        }

        fn reduce_message(&mut self, message: PointerEvent) {
            self.events.borrow_mut().push(message);
        }
    }

    impl RuntimeBridge<PointerEvent> for SurfaceReplacementPointerBridge {
        fn project_surface(&mut self) -> Arc<UiSurface<PointerEvent>> {
            if *self.replacement.borrow() {
                let new_events = Rc::clone(&self.new_events);
                test_arc_surface(UiSurface::new(SurfaceNode::widget(
                    TextWidget::new(
                        1,
                        "replacement",
                        WidgetSizing::fixed(Vector2::new(120.0, 80.0)),
                    ),
                    WidgetMessageMapper::canvas_pointer(move |event| {
                        new_events.borrow_mut().push(event);
                        event
                    }),
                )))
            } else {
                test_arc_surface(UiSurface::new(SurfaceNode::widget(
                    CanvasWidget::new(1, WidgetSizing::fixed(Vector2::new(120.0, 80.0))),
                    WidgetMessageMapper::canvas_pointer(|event| event),
                )))
            }
        }

        fn reduce_message(&mut self, message: PointerEvent) {
            self.old_events.borrow_mut().push(message);
        }
    }

    #[derive(Clone)]
    struct CompatiblePointerBridge {
        replacement: Rc<RefCell<bool>>,
        old_events: Rc<RefCell<Vec<PointerEvent>>>,
        new_events: Rc<RefCell<Vec<PointerEvent>>>,
    }

    impl RuntimeBridge<PointerEvent> for CompatiblePointerBridge {
        fn project_surface(&mut self) -> Arc<UiSurface<PointerEvent>> {
            if *self.replacement.borrow() {
                let new_events = Rc::clone(&self.new_events);
                test_arc_surface(UiSurface::new(SurfaceNode::widget(
                    CanvasWidget::new(1, WidgetSizing::fixed(Vector2::new(120.0, 80.0))),
                    WidgetMessageMapper::canvas_pointer(move |event| {
                        new_events.borrow_mut().push(event);
                        event
                    }),
                )))
            } else {
                test_arc_surface(UiSurface::new(SurfaceNode::widget(
                    CanvasWidget::new(1, WidgetSizing::fixed(Vector2::new(120.0, 80.0))),
                    WidgetMessageMapper::canvas_pointer(|event| event),
                )))
            }
        }

        fn reduce_message(&mut self, message: PointerEvent) {
            self.old_events.borrow_mut().push(message);
        }
    }

    #[derive(Clone)]
    struct ManagedCompatiblePointerBridge {
        replacement: Rc<RefCell<bool>>,
        old_events: Rc<RefCell<Vec<PointerEvent>>>,
        new_events: Rc<RefCell<Vec<PointerEvent>>>,
    }

    impl RuntimeBridge<PointerEvent> for ManagedCompatiblePointerBridge {
        fn project_surface(&mut self) -> Arc<UiSurface<PointerEvent>> {
            if *self.replacement.borrow() {
                let new_events = Rc::clone(&self.new_events);
                test_arc_surface(UiSurface::new(SurfaceNode::widget(
                    ManagedPointerWidget {
                        inner: ButtonWidget::new(
                            1,
                            "managed",
                            WidgetSizing::fixed(Vector2::new(120.0, 80.0)),
                        ),
                    },
                    WidgetMessageMapper::canvas_pointer(move |event| {
                        new_events.borrow_mut().push(event);
                        event
                    }),
                )))
            } else {
                test_arc_surface(UiSurface::new(SurfaceNode::widget(
                    ManagedPointerWidget {
                        inner: ButtonWidget::new(
                            1,
                            "managed",
                            WidgetSizing::fixed(Vector2::new(120.0, 80.0)),
                        ),
                    },
                    WidgetMessageMapper::canvas_pointer(|event| event),
                )))
            }
        }

        fn reduce_message(&mut self, message: PointerEvent) {
            self.old_events.borrow_mut().push(message);
        }
    }

    #[derive(Clone)]
    struct ManagedPointerWidget {
        inner: ButtonWidget,
    }

    impl Widget for ManagedPointerWidget {
        fn common(&self) -> &crate::widgets::WidgetCommon {
            self.inner.common()
        }

        fn common_mut(&mut self) -> &mut crate::widgets::WidgetCommon {
            self.inner.common_mut()
        }

        fn append_paint(
            &self,
            primitives: &mut Vec<crate::runtime::PaintPrimitive>,
            bounds: crate::gui::types::Rect,
            layout: &crate::layout::LayoutOutput,
            theme: &crate::theme::ThemeTokens,
        ) {
            self.inner.append_paint(primitives, bounds, layout, theme);
        }

        fn handle_input(
            &mut self,
            bounds: crate::gui::types::Rect,
            input: WidgetInput,
        ) -> Option<WidgetOutput> {
            self.inner
                .handle_input(bounds, input)
                .map(WidgetOutput::typed)
        }

        fn handle_pointer_event(
            &mut self,
            _bounds: crate::gui::types::Rect,
            event: PointerEvent,
        ) -> Option<WidgetOutput> {
            Some(WidgetOutput::typed(event))
        }

        fn preflight_pointer_press(
            &self,
            _bounds: crate::gui::types::Rect,
            _input: &WidgetInput,
        ) -> crate::widgets::PointerPressAdmission {
            crate::widgets::PointerPressAdmission::ManagedCapture
        }

        fn retains_managed_pointer_capture(&self) -> bool {
            true
        }

        fn handle_pointer_capture_cancelled(
            &mut self,
            bounds: crate::gui::types::Rect,
        ) -> Option<WidgetOutput> {
            Widget::handle_pointer_capture_cancelled(&mut self.inner, bounds)
        }
    }

    fn ids() -> (
        crate::gui::pointer_ingress::InputDeviceId,
        crate::gui::pointer_ingress::PointerContactId,
    ) {
        (
            crate::gui::pointer_ingress::InputDeviceId::from_host(1).unwrap(),
            crate::gui::pointer_ingress::PointerContactId::from_host(1).unwrap(),
        )
    }

    fn canvas_runtime() -> (
        SurfaceRuntime<PointerTraceBridge, PointerEvent>,
        Rc<RefCell<Vec<PointerEvent>>>,
    ) {
        let events = Rc::new(RefCell::new(Vec::new()));
        let runtime = SurfaceRuntime::new(
            PointerTraceBridge {
                events: Rc::clone(&events),
            },
            Vector2::new(120.0, 80.0),
        );
        (runtime, events)
    }

    fn assert_nonmouse_phases_are_isolated<Bridge>(
        mut runtime: SurfaceRuntime<Bridge, PointerEvent>,
        events: Rc<RefCell<Vec<PointerEvent>>>,
        with_mouse_capture: bool,
    ) where
        Bridge: RuntimeBridge<PointerEvent>,
    {
        let mouse_device = crate::gui::pointer_ingress::InputDeviceId::from_host(1).unwrap();
        let mouse_contact = crate::gui::pointer_ingress::PointerContactId::from_host(1).unwrap();
        let mouse_token = if with_mouse_capture {
            let admission = runtime.dispatch_pointer_ingress_with_admission(
                PointerIngress::new(
                    DeviceKind::Mouse,
                    mouse_device,
                    mouse_contact,
                    PointerPhase::Started {
                        button: PointerButton::Primary,
                    },
                    Point::new(20.0, 20.0),
                    crate::gui::pointer_ingress::PointerButtons::PRIMARY,
                    PointerModifiers::default(),
                    None,
                    None,
                    None,
                    None,
                )
                .expect("checked mouse start"),
            );
            assert_eq!(
                admission.disposition(),
                PointerIngressDisposition::RoutedWidget(1)
            );
            assert_eq!(runtime.interaction.pointer.capture, Some(1));
            Some(admission.sequence_token().expect("mouse token"))
        } else {
            None
        };
        let initial_event_count = events.borrow().len();
        let kinds = [
            DeviceKind::Touch,
            DeviceKind::Pen,
            DeviceKind::Trackpad,
            DeviceKind::Unknown,
        ];
        for (offset, kind) in kinds.into_iter().enumerate() {
            let device =
                crate::gui::pointer_ingress::InputDeviceId::from_host(100 + offset as u64).unwrap();
            let contact =
                crate::gui::pointer_ingress::PointerContactId::from_host(100 + offset as u64)
                    .unwrap();
            let hover = PointerIngress::new(
                kind,
                device,
                contact,
                PointerPhase::Hover,
                Point::new(20.0, 20.0),
                crate::gui::pointer_ingress::PointerButtons::empty(),
                PointerModifiers::default(),
                None,
                None,
                None,
                None,
            )
            .expect("checked nonmouse hover");
            assert_eq!(
                runtime.dispatch_pointer_ingress(hover),
                PointerIngressDisposition::AdmittedUnsupportedConsumer
            );
            assert!(
                runtime
                    .interaction
                    .pointer
                    .ingress
                    .records
                    .iter()
                    .all(|record| record.is_none_or(|record| record.device != device))
            );

            let started = PointerIngress::new(
                kind,
                device,
                contact,
                PointerPhase::Started {
                    button: PointerButton::Primary,
                },
                Point::new(20.0, 20.0),
                crate::gui::pointer_ingress::PointerButtons::PRIMARY,
                PointerModifiers::default(),
                None,
                None,
                None,
                None,
            )
            .expect("checked nonmouse start");
            let admission = runtime.dispatch_pointer_ingress_with_admission(started);
            assert_eq!(
                admission.disposition(),
                PointerIngressDisposition::AdmittedUnsupportedConsumer
            );
            let token = admission.sequence_token().expect("nonmouse token");
            assert_eq!(runtime.interaction.pointer.capture, mouse_token.map(|_| 1));
            assert_eq!(events.borrow().len(), initial_event_count);

            let moved = PointerIngress::from_runtime(
                kind,
                device,
                contact,
                PointerPhase::Moved,
                Point::new(24.0, 20.0),
                crate::gui::pointer_ingress::PointerButtons::PRIMARY,
                PointerModifiers::default(),
                None,
                None,
                None,
                None,
                token,
            )
            .expect("checked nonmouse move");
            assert_eq!(
                runtime.dispatch_pointer_ingress(moved),
                PointerIngressDisposition::AdmittedUnsupportedConsumer
            );
            let ended = PointerIngress::from_runtime(
                kind,
                device,
                contact,
                PointerPhase::Ended {
                    button: PointerButton::Primary,
                },
                Point::new(24.0, 20.0),
                crate::gui::pointer_ingress::PointerButtons::empty(),
                PointerModifiers::default(),
                None,
                None,
                None,
                None,
                token,
            )
            .expect("checked nonmouse end");
            assert_eq!(
                runtime.dispatch_pointer_ingress(ended),
                PointerIngressDisposition::AdmittedUnsupportedConsumer
            );
            assert_eq!(runtime.interaction.pointer.capture, mouse_token.map(|_| 1));

            let cancelled = PointerIngress::new(
                kind,
                device,
                contact,
                PointerPhase::Started {
                    button: PointerButton::Primary,
                },
                Point::new(20.0, 20.0),
                crate::gui::pointer_ingress::PointerButtons::PRIMARY,
                PointerModifiers::default(),
                None,
                None,
                None,
                None,
            )
            .expect("checked second nonmouse start");
            let cancelled_token = runtime
                .dispatch_pointer_ingress_with_admission(cancelled)
                .sequence_token()
                .expect("second nonmouse token");
            let cancelled = PointerIngress::from_runtime(
                kind,
                device,
                contact,
                PointerPhase::Cancelled,
                Point::new(20.0, 20.0),
                crate::gui::pointer_ingress::PointerButtons::empty(),
                PointerModifiers::default(),
                None,
                None,
                None,
                None,
                cancelled_token,
            )
            .expect("checked nonmouse cancel");
            assert_eq!(
                runtime.dispatch_pointer_ingress(cancelled),
                PointerIngressDisposition::AdmittedUnsupportedConsumer
            );
            assert_eq!(runtime.interaction.pointer.capture, mouse_token.map(|_| 1));
            assert_eq!(events.borrow().len(), initial_event_count);
        }

        if let Some(mouse_token) = mouse_token {
            let moved = PointerIngress::from_runtime(
                DeviceKind::Mouse,
                mouse_device,
                mouse_contact,
                PointerPhase::Moved,
                Point::new(28.0, 20.0),
                crate::gui::pointer_ingress::PointerButtons::PRIMARY,
                PointerModifiers::default(),
                None,
                None,
                None,
                None,
                mouse_token,
            )
            .expect("checked mouse continuation after unsupported phases");
            assert_eq!(
                runtime.dispatch_pointer_ingress(moved),
                PointerIngressDisposition::RoutedWidget(1)
            );
            let ended = PointerIngress::from_runtime(
                DeviceKind::Mouse,
                mouse_device,
                mouse_contact,
                PointerPhase::Ended {
                    button: PointerButton::Primary,
                },
                Point::new(28.0, 20.0),
                crate::gui::pointer_ingress::PointerButtons::empty(),
                PointerModifiers::default(),
                None,
                None,
                None,
                None,
                mouse_token,
            )
            .expect("checked mouse terminal after unsupported phases");
            assert_eq!(
                runtime.dispatch_pointer_ingress(ended),
                PointerIngressDisposition::RoutedWidget(1)
            );
            assert_eq!(events.borrow().len(), initial_event_count + 2);
            assert_eq!(runtime.interaction.pointer.capture, None);
        }
    }

    #[test]
    fn production_nonmouse_phases_are_isolated_alone_and_during_canvas_capture() {
        let (runtime, events) = canvas_runtime();
        assert_nonmouse_phases_are_isolated(runtime, Rc::clone(&events), false);
        let (runtime, events) = canvas_runtime();
        assert_nonmouse_phases_are_isolated(runtime, Rc::clone(&events), true);
    }

    #[test]
    fn production_nonmouse_phases_are_isolated_during_gpu_capture() {
        let events = Rc::new(RefCell::new(Vec::new()));
        let runtime = SurfaceRuntime::new(
            GpuPointerTraceBridge {
                events: Rc::clone(&events),
            },
            Vector2::new(120.0, 80.0),
        );
        assert_nonmouse_phases_are_isolated(runtime, events, true);
    }

    #[test]
    fn production_stop_cancels_mouse_once_regardless_of_unsupported_record_order() {
        for unsupported_first in [false, true] {
            let (mut runtime, events) = canvas_runtime();
            let mouse_device = crate::gui::pointer_ingress::InputDeviceId::from_host(1).unwrap();
            let mouse_contact =
                crate::gui::pointer_ingress::PointerContactId::from_host(1).unwrap();
            let touch_device = crate::gui::pointer_ingress::InputDeviceId::from_host(2).unwrap();
            let touch_contact =
                crate::gui::pointer_ingress::PointerContactId::from_host(2).unwrap();
            if unsupported_first {
                assert_eq!(
                    runtime.dispatch_pointer_start(
                        DeviceKind::Touch,
                        touch_device,
                        touch_contact,
                        Point::new(20.0, 20.0),
                        PointerButton::Primary,
                        crate::gui::pointer_ingress::PointerButtons::PRIMARY,
                        PointerModifiers::default(),
                    ),
                    PointerIngressDisposition::AdmittedUnsupportedConsumer
                );
                assert_eq!(
                    runtime.dispatch_pointer_start(
                        DeviceKind::Mouse,
                        mouse_device,
                        mouse_contact,
                        Point::new(20.0, 20.0),
                        PointerButton::Primary,
                        crate::gui::pointer_ingress::PointerButtons::PRIMARY,
                        PointerModifiers::default(),
                    ),
                    PointerIngressDisposition::RoutedWidget(1)
                );
            } else {
                assert_eq!(
                    runtime.dispatch_pointer_start(
                        DeviceKind::Mouse,
                        mouse_device,
                        mouse_contact,
                        Point::new(20.0, 20.0),
                        PointerButton::Primary,
                        crate::gui::pointer_ingress::PointerButtons::PRIMARY,
                        PointerModifiers::default(),
                    ),
                    PointerIngressDisposition::RoutedWidget(1)
                );
                assert_eq!(
                    runtime.dispatch_pointer_start(
                        DeviceKind::Touch,
                        touch_device,
                        touch_contact,
                        Point::new(20.0, 20.0),
                        PointerButton::Primary,
                        crate::gui::pointer_ingress::PointerButtons::PRIMARY,
                        PointerModifiers::default(),
                    ),
                    PointerIngressDisposition::AdmittedUnsupportedConsumer
                );
            }
            events.borrow_mut().clear();
            runtime.cancel_pointer_ingress_sequences();
            assert_eq!(
                events
                    .borrow()
                    .iter()
                    .map(|event| event.phase())
                    .collect::<Vec<_>>(),
                vec![PointerPhase::Cancelled]
            );
            assert!(
                runtime
                    .interaction
                    .pointer
                    .ingress
                    .records
                    .iter()
                    .all(Option::is_none)
            );
            assert_eq!(runtime.interaction.pointer.capture, None);
            runtime.cancel_pointer_ingress_sequences();
            assert_eq!(events.borrow().len(), 1);
        }
    }

    #[test]
    fn production_gpu_pointer_mapper_delivers_exact_phases_once() {
        let events = Rc::new(RefCell::new(Vec::new()));
        let mut runtime = SurfaceRuntime::new(
            GpuPointerTraceBridge {
                events: Rc::clone(&events),
            },
            Vector2::new(120.0, 80.0),
        );
        let (device, contact) = ids();
        let buttons = crate::gui::pointer_ingress::PointerButtons::PRIMARY;
        assert!(matches!(
            runtime.dispatch_pointer_start(
                DeviceKind::Mouse,
                device,
                contact,
                Point::new(20.0, 20.0),
                PointerButton::Primary,
                buttons,
                PointerModifiers::default(),
            ),
            PointerIngressDisposition::RoutedWidget(1)
        ));
        assert!(matches!(
            runtime.dispatch_pointer_move(
                DeviceKind::Mouse,
                device,
                contact,
                Point::new(24.0, 20.0),
                buttons,
                PointerModifiers::default(),
            ),
            PointerIngressDisposition::RoutedWidget(1)
        ));
        assert!(matches!(
            runtime.dispatch_pointer_end(
                DeviceKind::Mouse,
                device,
                contact,
                Point::new(24.0, 20.0),
                PointerButton::Primary,
                crate::gui::pointer_ingress::PointerButtons::empty(),
                PointerModifiers::default(),
            ),
            PointerIngressDisposition::RoutedWidget(1)
        ));
        assert_eq!(
            events
                .borrow()
                .iter()
                .map(|event| event.phase())
                .collect::<Vec<_>>(),
            vec![
                PointerPhase::Started {
                    button: PointerButton::Primary
                },
                PointerPhase::Moved,
                PointerPhase::Ended {
                    button: PointerButton::Primary
                }
            ]
        );
    }

    #[test]
    fn production_refresh_cancels_through_old_pointer_mapper_before_replacement() {
        let (mut runtime, events) = canvas_runtime();
        let (device, contact) = ids();
        assert!(matches!(
            runtime.dispatch_pointer_start(
                DeviceKind::Mouse,
                device,
                contact,
                Point::new(20.0, 20.0),
                PointerButton::Primary,
                crate::gui::pointer_ingress::PointerButtons::PRIMARY,
                PointerModifiers::default(),
            ),
            PointerIngressDisposition::RoutedWidget(1)
        ));
        events.borrow_mut().clear();
        let next_surface = test_arc_surface(UiSurface::new(SurfaceNode::widget(
            CanvasWidget::new(1, WidgetSizing::fixed(Vector2::new(120.0, 80.0))),
            WidgetMessageMapper::none(),
        )));
        let current_paths = runtime.traversal.widgets.paths.current.clone();
        let mut terminal_messages = Vec::new();
        runtime.reconcile_pointer_ingress_before_surface_replace(
            &next_surface,
            &current_paths,
            &current_paths,
            &[1],
            &mut terminal_messages,
        );
        runtime.dispatch_deferred_surface_messages(terminal_messages);
        let events = events.borrow();
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(event.phase(), PointerPhase::Cancelled))
                .count(),
            1
        );
        assert!(
            events
                .iter()
                .all(|event| matches!(event.phase(), PointerPhase::Cancelled))
        );
    }

    #[test]
    fn production_refresh_entrypoint_uses_old_mapper_for_incompatible_successor() {
        let replacement = Rc::new(RefCell::new(false));
        let old_events = Rc::new(RefCell::new(Vec::new()));
        let new_events = Rc::new(RefCell::new(Vec::new()));
        let mut runtime = SurfaceRuntime::new(
            SurfaceReplacementPointerBridge {
                replacement: Rc::clone(&replacement),
                old_events: Rc::clone(&old_events),
                new_events: Rc::clone(&new_events),
            },
            Vector2::new(120.0, 80.0),
        );
        let (device, contact) = ids();
        assert_eq!(
            runtime.dispatch_pointer_start(
                DeviceKind::Mouse,
                device,
                contact,
                Point::new(20.0, 20.0),
                PointerButton::Primary,
                crate::gui::pointer_ingress::PointerButtons::PRIMARY,
                PointerModifiers::default(),
            ),
            PointerIngressDisposition::RoutedWidget(1)
        );
        old_events.borrow_mut().clear();
        new_events.borrow_mut().clear();
        *replacement.borrow_mut() = true;
        runtime.refresh();
        let old_events = old_events.borrow();
        assert_eq!(
            old_events
                .iter()
                .filter(|event| matches!(event.phase(), PointerPhase::Cancelled))
                .count(),
            1
        );
        assert!(
            old_events
                .iter()
                .all(|event| matches!(event.phase(), PointerPhase::Cancelled))
        );
        assert!(new_events.borrow().is_empty());
    }

    #[test]
    fn production_refresh_entrypoint_preserves_compatible_legacy_owner() {
        let replacement = Rc::new(RefCell::new(false));
        let old_events = Rc::new(RefCell::new(Vec::new()));
        let new_events = Rc::new(RefCell::new(Vec::new()));
        let mut runtime = SurfaceRuntime::new(
            CompatiblePointerBridge {
                replacement: Rc::clone(&replacement),
                old_events: Rc::clone(&old_events),
                new_events: Rc::clone(&new_events),
            },
            Vector2::new(120.0, 80.0),
        );
        let (device, contact) = ids();
        assert_eq!(
            runtime.dispatch_pointer_start(
                DeviceKind::Mouse,
                device,
                contact,
                Point::new(20.0, 20.0),
                PointerButton::Primary,
                crate::gui::pointer_ingress::PointerButtons::PRIMARY,
                PointerModifiers::default(),
            ),
            PointerIngressDisposition::RoutedWidget(1)
        );
        old_events.borrow_mut().clear();
        new_events.borrow_mut().clear();
        *replacement.borrow_mut() = true;
        runtime.refresh();
        assert!(old_events.borrow().is_empty());
        assert!(new_events.borrow().is_empty());
        let record = runtime
            .interaction
            .pointer
            .ingress
            .records
            .iter()
            .flatten()
            .next()
            .copied()
            .expect("compatible owner retained");
        let moved = PointerIngress::from_runtime(
            DeviceKind::Mouse,
            device,
            contact,
            PointerPhase::Moved,
            Point::new(22.0, 22.0),
            crate::gui::pointer_ingress::PointerButtons::PRIMARY,
            PointerModifiers::default(),
            None,
            None,
            None,
            None,
            record.token,
        )
        .expect("checked continuation");
        assert_eq!(
            runtime.dispatch_pointer_ingress(moved),
            PointerIngressDisposition::RoutedWidget(1)
        );
        assert_eq!(
            new_events
                .borrow()
                .iter()
                .filter(|event| matches!(event.phase(), PointerPhase::Moved))
                .count(),
            1
        );
    }

    #[test]
    fn production_refresh_entrypoint_preserves_compatible_managed_owner() {
        let replacement = Rc::new(RefCell::new(false));
        let old_events = Rc::new(RefCell::new(Vec::new()));
        let new_events = Rc::new(RefCell::new(Vec::new()));
        let mut runtime = SurfaceRuntime::new(
            ManagedCompatiblePointerBridge {
                replacement: Rc::clone(&replacement),
                old_events: Rc::clone(&old_events),
                new_events: Rc::clone(&new_events),
            },
            Vector2::new(120.0, 80.0),
        );
        let (device, contact) = ids();
        assert_eq!(
            runtime.dispatch_pointer_start(
                DeviceKind::Mouse,
                device,
                contact,
                Point::new(20.0, 20.0),
                PointerButton::Primary,
                crate::gui::pointer_ingress::PointerButtons::PRIMARY,
                PointerModifiers::default(),
            ),
            PointerIngressDisposition::RoutedWidget(1)
        );
        assert!(runtime.interaction.pointer.managed_capture.is_some());
        old_events.borrow_mut().clear();
        new_events.borrow_mut().clear();
        *replacement.borrow_mut() = true;
        runtime.refresh();
        assert!(runtime.interaction.pointer.managed_capture.is_some());
        assert!(old_events.borrow().is_empty());
        assert!(new_events.borrow().is_empty());
        let record = runtime
            .interaction
            .pointer
            .ingress
            .records
            .iter()
            .flatten()
            .next()
            .copied()
            .expect("managed owner retained");
        let moved = PointerIngress::from_runtime(
            DeviceKind::Mouse,
            device,
            contact,
            PointerPhase::Moved,
            Point::new(22.0, 22.0),
            crate::gui::pointer_ingress::PointerButtons::PRIMARY,
            PointerModifiers::default(),
            None,
            None,
            None,
            None,
            record.token,
        )
        .expect("checked continuation");
        assert_eq!(
            runtime.dispatch_pointer_ingress(moved),
            PointerIngressDisposition::RoutedWidget(1)
        );
        assert_eq!(
            new_events
                .borrow()
                .iter()
                .filter(|event| matches!(event.phase(), PointerPhase::Moved))
                .count(),
            1
        );
    }

    #[test]
    fn public_admission_returns_opaque_token_for_explicit_continuation() {
        let (mut runtime, events) = canvas_runtime();
        let (device, contact) = ids();
        let started = PointerIngress::new(
            DeviceKind::Touch,
            device,
            contact,
            PointerPhase::Started {
                button: PointerButton::Primary,
            },
            Point::new(20.0, 20.0),
            crate::gui::pointer_ingress::PointerButtons::PRIMARY,
            PointerModifiers::default(),
            None,
            None,
            None,
            None,
        )
        .expect("checked started ingress");
        let admission = runtime.dispatch_pointer_ingress_with_admission(started);
        let token = admission.sequence_token().expect("runtime-issued token");
        assert_eq!(
            admission.disposition(),
            PointerIngressDisposition::AdmittedUnsupportedConsumer
        );
        let moved = PointerIngress::from_runtime(
            DeviceKind::Touch,
            device,
            contact,
            PointerPhase::Moved,
            Point::new(24.0, 20.0),
            crate::gui::pointer_ingress::PointerButtons::PRIMARY,
            PointerModifiers::default(),
            None,
            None,
            None,
            None,
            token,
        )
        .expect("explicit token continuation");
        assert_eq!(
            runtime.dispatch_pointer_ingress(moved),
            PointerIngressDisposition::AdmittedUnsupportedConsumer
        );
        assert!(events.borrow().is_empty());
    }

    #[test]
    fn production_legacy_canvas_press_and_release_use_one_existing_input_route() {
        let messages = Rc::new(RefCell::new(Vec::new()));
        let mut runtime = SurfaceRuntime::new(
            LegacyCanvasBridge {
                messages: Rc::clone(&messages),
            },
            Vector2::new(120.0, 80.0),
        );
        let (device, contact) = ids();
        let buttons = crate::gui::pointer_ingress::PointerButtons::PRIMARY;
        assert!(matches!(
            runtime.dispatch_pointer_start(
                DeviceKind::Mouse,
                device,
                contact,
                Point::new(20.0, 20.0),
                PointerButton::Primary,
                buttons,
                PointerModifiers::default()
            ),
            PointerIngressDisposition::RoutedWidget(1)
        ));
        assert!(matches!(
            runtime.dispatch_pointer_end(
                DeviceKind::Mouse,
                device,
                contact,
                Point::new(20.0, 20.0),
                PointerButton::Primary,
                crate::gui::pointer_ingress::PointerButtons::empty(),
                PointerModifiers::default()
            ),
            PointerIngressDisposition::RoutedWidget(1)
        ));
        let messages = messages.borrow();
        let inputs: Vec<_> = messages
            .iter()
            .map(|message| match message {
                CanvasMessage::Input { input } => input,
            })
            .collect();
        assert_eq!(
            inputs
                .iter()
                .filter(|input| matches!(input, WidgetInput::PointerPress { .. }))
                .count(),
            1
        );
        assert_eq!(
            inputs
                .iter()
                .filter(|input| matches!(input, WidgetInput::PointerRelease { .. }))
                .count(),
            1
        );
    }

    #[test]
    fn production_canvas_pointer_route_delivers_exact_phases_and_token_evidence() {
        let (mut runtime, events) = canvas_runtime();
        let (device, contact) = ids();
        assert_eq!(
            runtime.dispatch_pointer_start(
                DeviceKind::Mouse,
                device,
                contact,
                Point::new(20.0, 20.0),
                PointerButton::Primary,
                crate::gui::pointer_ingress::PointerButtons::PRIMARY,
                PointerModifiers::default(),
            ),
            PointerIngressDisposition::RoutedWidget(1)
        );
        assert_eq!(
            runtime.dispatch_pointer_move(
                DeviceKind::Mouse,
                device,
                contact,
                Point::new(24.0, 20.0),
                crate::gui::pointer_ingress::PointerButtons::PRIMARY,
                PointerModifiers::default(),
            ),
            PointerIngressDisposition::RoutedWidget(1)
        );
        assert_eq!(
            runtime.dispatch_pointer_end(
                DeviceKind::Mouse,
                device,
                contact,
                Point::new(24.0, 20.0),
                PointerButton::Primary,
                crate::gui::pointer_ingress::PointerButtons::empty(),
                PointerModifiers::default(),
            ),
            PointerIngressDisposition::RoutedWidget(1)
        );
        assert_eq!(
            events
                .borrow()
                .iter()
                .map(|event| event.phase())
                .collect::<Vec<_>>(),
            vec![
                PointerPhase::Started {
                    button: PointerButton::Primary
                },
                PointerPhase::Moved,
                PointerPhase::Ended {
                    button: PointerButton::Primary
                }
            ]
        );
        assert!(events.borrow()[0].sequence_token().is_some());
        assert_eq!(
            events.borrow()[1].sequence_token(),
            events.borrow()[0].sequence_token()
        );
        assert_eq!(
            events.borrow()[2].sequence_token(),
            events.borrow()[0].sequence_token()
        );
    }

    #[test]
    fn production_hover_is_tokenless_and_unregistered() {
        let (mut runtime, events) = canvas_runtime();
        let (device, contact) = ids();
        assert_eq!(
            runtime.dispatch_pointer_hover(
                DeviceKind::Mouse,
                device,
                contact,
                Point::new(20.0, 20.0),
                crate::gui::pointer_ingress::PointerButtons::empty(),
                PointerModifiers::default(),
            ),
            PointerIngressDisposition::RoutedWidget(1)
        );
        assert_eq!(events.borrow().len(), 1);
        assert_eq!(events.borrow()[0].phase(), PointerPhase::Hover);
        assert!(events.borrow()[0].sequence_token().is_none());
        assert!(
            runtime
                .interaction
                .pointer
                .ingress
                .records
                .iter()
                .all(Option::is_none)
        );
    }

    #[test]
    fn production_competing_button_and_wrong_release_preserve_winner() {
        let (mut runtime, events) = canvas_runtime();
        let (device, contact) = ids();
        let primary = crate::gui::pointer_ingress::PointerButtons::PRIMARY;
        assert!(matches!(
            runtime.dispatch_pointer_start(
                DeviceKind::Mouse,
                device,
                contact,
                Point::new(20.0, 20.0),
                PointerButton::Primary,
                primary,
                PointerModifiers::default()
            ),
            PointerIngressDisposition::RoutedWidget(1)
        ));
        assert_eq!(
            runtime.dispatch_pointer_start(
                DeviceKind::Mouse,
                device,
                contact,
                Point::new(20.0, 20.0),
                PointerButton::Secondary,
                primary.with(crate::gui::pointer_ingress::PointerButtons::SECONDARY),
                PointerModifiers::default()
            ),
            PointerIngressDisposition::Blocked
        );
        assert_eq!(
            runtime.dispatch_pointer_end(
                DeviceKind::Mouse,
                device,
                contact,
                Point::new(20.0, 20.0),
                PointerButton::Secondary,
                primary,
                PointerModifiers::default()
            ),
            PointerIngressDisposition::Stale
        );
        assert_eq!(
            runtime.dispatch_pointer_move(
                DeviceKind::Mouse,
                device,
                contact,
                Point::new(24.0, 20.0),
                primary,
                PointerModifiers::default()
            ),
            PointerIngressDisposition::RoutedWidget(1)
        );
        assert_eq!(events.borrow().len(), 2);
    }

    #[test]
    fn production_cancel_is_one_typed_terminal_delivery() {
        let (mut runtime, events) = canvas_runtime();
        let (device, contact) = ids();
        let buttons = crate::gui::pointer_ingress::PointerButtons::PRIMARY;
        assert!(matches!(
            runtime.dispatch_pointer_start(
                DeviceKind::Mouse,
                device,
                contact,
                Point::new(20.0, 20.0),
                PointerButton::Primary,
                buttons,
                PointerModifiers::default()
            ),
            PointerIngressDisposition::RoutedWidget(1)
        ));
        assert_eq!(
            runtime.dispatch_pointer_cancel(
                DeviceKind::Mouse,
                device,
                contact,
                Point::new(20.0, 20.0),
                buttons,
                PointerModifiers::default()
            ),
            PointerIngressDisposition::RoutedWidget(1)
        );
        assert_eq!(
            runtime.dispatch_pointer_cancel(
                DeviceKind::Mouse,
                device,
                contact,
                Point::new(20.0, 20.0),
                buttons,
                PointerModifiers::default()
            ),
            PointerIngressDisposition::Stale
        );
        assert_eq!(events.borrow().len(), 2);
        assert!(matches!(
            events.borrow()[1].phase(),
            PointerPhase::Cancelled
        ));
    }

    #[test]
    fn production_hit_miss_does_not_issue_token_or_record() {
        let phases = Rc::new(RefCell::new(Vec::new()));
        let mut runtime = SurfaceRuntime::new(
            PointerTraceBridge {
                events: Rc::clone(&phases),
            },
            Vector2::new(0.0, 0.0),
        );
        let (device, contact) = ids();
        assert_eq!(
            runtime.dispatch_pointer_start(
                DeviceKind::Mouse,
                device,
                contact,
                Point::new(20.0, 20.0),
                PointerButton::Primary,
                crate::gui::pointer_ingress::PointerButtons::PRIMARY,
                PointerModifiers::default()
            ),
            PointerIngressDisposition::Blocked
        );
        assert!(phases.borrow().is_empty());
        assert!(
            runtime
                .interaction
                .pointer
                .ingress
                .records
                .iter()
                .all(Option::is_none)
        );
    }

    #[test]
    fn production_capacity_is_sixteen_and_terminal_frees_the_slot() {
        let (mut runtime, events) = canvas_runtime();
        let device = crate::gui::pointer_ingress::InputDeviceId::from_host(1).unwrap();
        for raw in 1..=16 {
            let contact = crate::gui::pointer_ingress::PointerContactId::from_host(raw).unwrap();
            assert!(matches!(
                runtime.dispatch_pointer_start(
                    DeviceKind::Touch,
                    device,
                    contact,
                    Point::new(20.0, 20.0),
                    PointerButton::Primary,
                    crate::gui::pointer_ingress::PointerButtons::PRIMARY,
                    PointerModifiers::default()
                ),
                PointerIngressDisposition::AdmittedUnsupportedConsumer
            ));
        }
        let seventeenth = crate::gui::pointer_ingress::PointerContactId::from_host(17).unwrap();
        assert_eq!(
            runtime.dispatch_pointer_start(
                DeviceKind::Touch,
                device,
                seventeenth,
                Point::new(20.0, 20.0),
                PointerButton::Primary,
                crate::gui::pointer_ingress::PointerButtons::PRIMARY,
                PointerModifiers::default()
            ),
            PointerIngressDisposition::CapacityExhausted
        );
        let first = crate::gui::pointer_ingress::PointerContactId::from_host(1).unwrap();
        assert!(matches!(
            runtime.dispatch_pointer_end(
                DeviceKind::Touch,
                device,
                first,
                Point::new(20.0, 20.0),
                PointerButton::Primary,
                crate::gui::pointer_ingress::PointerButtons::empty(),
                PointerModifiers::default()
            ),
            PointerIngressDisposition::AdmittedUnsupportedConsumer
        ));
        assert!(matches!(
            runtime.dispatch_pointer_start(
                DeviceKind::Touch,
                device,
                seventeenth,
                Point::new(20.0, 20.0),
                PointerButton::Primary,
                crate::gui::pointer_ingress::PointerButtons::PRIMARY,
                PointerModifiers::default()
            ),
            PointerIngressDisposition::AdmittedUnsupportedConsumer
        ));
        assert!(events.borrow().is_empty());
    }

    #[test]
    fn production_stale_continuation_after_terminal_has_no_callback() {
        let (mut runtime, events) = canvas_runtime();
        let (device, contact) = ids();
        let buttons = crate::gui::pointer_ingress::PointerButtons::PRIMARY;
        assert!(matches!(
            runtime.dispatch_pointer_start(
                DeviceKind::Mouse,
                device,
                contact,
                Point::new(20.0, 20.0),
                PointerButton::Primary,
                buttons,
                PointerModifiers::default()
            ),
            PointerIngressDisposition::RoutedWidget(1)
        ));
        assert!(matches!(
            runtime.dispatch_pointer_end(
                DeviceKind::Mouse,
                device,
                contact,
                Point::new(20.0, 20.0),
                PointerButton::Primary,
                crate::gui::pointer_ingress::PointerButtons::empty(),
                PointerModifiers::default()
            ),
            PointerIngressDisposition::RoutedWidget(1)
        ));
        assert_eq!(
            runtime.dispatch_pointer_move(
                DeviceKind::Mouse,
                device,
                contact,
                Point::new(24.0, 20.0),
                buttons,
                PointerModifiers::default()
            ),
            PointerIngressDisposition::Stale
        );
        assert_eq!(events.borrow().len(), 2);
    }

    #[test]
    fn production_explicit_old_native_token_cannot_follow_contact_reuse() {
        let (mut runtime, events) = canvas_runtime();
        let (device, contact) = ids();
        let start = |runtime: &mut SurfaceRuntime<PointerTraceBridge, PointerEvent>| {
            runtime.dispatch_pointer_ingress_with_admission(
                PointerIngress::new(
                    DeviceKind::Mouse,
                    device,
                    contact,
                    PointerPhase::Started {
                        button: PointerButton::Primary,
                    },
                    Point::new(20.0, 20.0),
                    crate::gui::pointer_ingress::PointerButtons::PRIMARY,
                    PointerModifiers::default(),
                    None,
                    None,
                    None,
                    None,
                )
                .expect("checked native start"),
            )
        };
        let first = start(&mut runtime);
        let first_token = first.sequence_token().expect("first native token");
        let first_end = PointerIngress::from_runtime(
            DeviceKind::Mouse,
            device,
            contact,
            PointerPhase::Ended {
                button: PointerButton::Primary,
            },
            Point::new(20.0, 20.0),
            crate::gui::pointer_ingress::PointerButtons::empty(),
            PointerModifiers::default(),
            None,
            None,
            None,
            None,
            first_token,
        )
        .expect("checked first terminal");
        assert_eq!(
            runtime.dispatch_pointer_ingress(first_end),
            PointerIngressDisposition::RoutedWidget(1)
        );

        let second = start(&mut runtime);
        let second_token = second.sequence_token().expect("second native token");
        assert_ne!(first_token, second_token);
        let stale_move = PointerIngress::from_runtime(
            DeviceKind::Mouse,
            device,
            contact,
            PointerPhase::Moved,
            Point::new(24.0, 20.0),
            crate::gui::pointer_ingress::PointerButtons::PRIMARY,
            PointerModifiers::default(),
            None,
            None,
            None,
            None,
            first_token,
        )
        .expect("checked stale queued sample");
        assert_eq!(
            runtime.dispatch_pointer_ingress(stale_move),
            PointerIngressDisposition::Stale
        );
        assert_eq!(events.borrow().len(), 3);
    }
}

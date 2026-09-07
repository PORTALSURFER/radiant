//! Stack-scoped controller handoff for same-app cross-window drag samples.
//!
//! The native coordinator supplies only the already-resolved routing hint. This
//! module neither selects a receiver nor retains a terminal request beyond the
//! active native input call.
use super::GenericNativeVelloRunner;
use crate::{
    gui::{
        input::{InputSequenceRange, InputTimestamp},
        pointer_ingress::{
            DeviceKind, InputDeviceId, PointerButtons, PointerContactId, PointerIngress,
            PointerIngressDisposition, PointerPhase, PointerPressure, PointerSequenceToken,
            PointerTilt,
        },
        types::Point,
    },
    runtime::{
        CrossWindowDragKey, CrossWindowInputHint, CrossWindowTerminalRequest, RuntimeBridge,
    },
    widgets::PointerModifiers,
};

/// One native sample's cross-window evidence and, for an admitted release, its
/// detached source terminal authority. The coordinator consumes `terminal`
/// before this stack frame returns; no runner state owns an Ended capture.
pub(super) struct NativeCrossWindowInput<Message> {
    pub(super) hint: CrossWindowInputHint,
    pub(super) terminal: Option<CrossWindowTerminalRequest<Message>>,
    pub(super) source_moved: Option<CrossWindowDragKey>,
    pub(super) last_disposition: Option<PointerIngressDisposition>,
}

impl<Message> NativeCrossWindowInput<Message> {
    pub(super) const fn new(hint: CrossWindowInputHint) -> Self {
        Self {
            hint,
            terminal: None,
            source_moved: None,
            last_disposition: None,
        }
    }
}

impl<Bridge, Message> GenericNativeVelloRunner<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    /// Dispatch a fully checked ingress through the special controller path
    /// only while the caller holds a stack-local cross-window context.
    pub(super) fn dispatch_checked_pointer_ingress(
        &mut self,
        ingress: PointerIngress,
        cross_window: Option<&mut NativeCrossWindowInput<Message>>,
    ) -> PointerIngressDisposition {
        if let Some(cross_window) = cross_window {
            let route = self
                .core
                .runtime
                .dispatch_pointer_ingress_for_cross_window(ingress, cross_window.hint);
            cross_window.terminal = route.terminal;
            cross_window.source_moved = route.source_moved;
            cross_window.last_disposition = Some(route.disposition);
            route.disposition
        } else {
            self.core.runtime.dispatch_pointer_ingress(ingress)
        }
    }

    /// Build a continuation from the native adapter's exact controller-issued
    /// token, preserving the normal invalid-sample disposition and terminal
    /// cleanup ownership.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn dispatch_native_pointer_continuation_with_cross_window(
        &mut self,
        kind: DeviceKind,
        device: InputDeviceId,
        contact: PointerContactId,
        token: PointerSequenceToken,
        phase: PointerPhase,
        position: Point,
        buttons: PointerButtons,
        modifiers: PointerModifiers,
        pressure: Option<PointerPressure>,
        tilt: Option<PointerTilt>,
        timestamp: Option<InputTimestamp>,
        sequence_range: Option<InputSequenceRange>,
        cross_window: Option<&mut NativeCrossWindowInput<Message>>,
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
        self.dispatch_checked_pointer_ingress(ingress, cross_window)
    }
}

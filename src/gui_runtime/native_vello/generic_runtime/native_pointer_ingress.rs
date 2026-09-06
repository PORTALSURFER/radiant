//! Bounded native pointer identity and Winit normalization.
//!
//! This module owns the native side of the pointer boundary.  It intentionally
//! contains no widget or capture routing: the controller remains the only
//! owner of pointer sequences.  Native device/contact identities are monotonic
//! and are never recycled, while the two small tables bound retained native
//! state.

use super::input::logical_point_from_winit;
use crate::gui::{
    input::InputTimestamp,
    pointer_ingress::{
        DeviceKind, InputDeviceId, InvalidPointerIdentity, PointerContactId, PointerPressure,
        PointerSequenceToken, PointerTilt,
    },
    types::Point,
};
use crate::theme::DpiScale;
use crate::widgets::PointerModifiers;
use std::num::NonZeroU64;
use winit::event::{DeviceId, Force, Touch, TouchPhase};

pub(super) const MAX_NATIVE_POINTER_DEVICES: usize = 16;
pub(super) const MAX_NATIVE_POINTER_CONTACTS: usize = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum NativePointerIdentityError {
    Invalid(InvalidPointerIdentity),
    Capacity,
    Exhausted,
    DuplicateContact,
    MissingContact,
    WrongDevice,
    InvalidSample,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct NativeIdentityAllocator {
    next: Option<NonZeroU64>,
}

impl Default for NativeIdentityAllocator {
    fn default() -> Self {
        Self {
            next: NonZeroU64::new(1),
        }
    }
}

impl NativeIdentityAllocator {
    fn allocate_device(&mut self) -> Result<InputDeviceId, NativePointerIdentityError> {
        let value = self
            .next
            .take()
            .ok_or(NativePointerIdentityError::Exhausted)?;
        self.next = NonZeroU64::new(value.get().checked_add(1).unwrap_or(0));
        InputDeviceId::from_host(value.get()).map_err(NativePointerIdentityError::Invalid)
    }

    fn allocate_contact(&mut self) -> Result<PointerContactId, NativePointerIdentityError> {
        let value = self
            .next
            .take()
            .ok_or(NativePointerIdentityError::Exhausted)?;
        self.next = NonZeroU64::new(value.get().checked_add(1).unwrap_or(0));
        PointerContactId::from_host(value.get()).map_err(NativePointerIdentityError::Invalid)
    }

    #[cfg(test)]
    fn exhaust(&mut self) {
        self.next = NonZeroU64::new(u64::MAX);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct NativePointerDeviceRecord {
    native: DeviceId,
    normalized: InputDeviceId,
    kind: DeviceKind,
    active_contacts: u8,
    gesture_retained: bool,
    hover: bool,
}

impl NativePointerDeviceRecord {
    fn is_reclaimable(self) -> bool {
        self.active_contacts == 0 && !self.hover && !self.gesture_retained
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct NativePointerContactRecord {
    native_device: DeviceId,
    device: InputDeviceId,
    raw_contact: u64,
    normalized: PointerContactId,
    sequence_token: Option<PointerSequenceToken>,
    last_position: Option<Point>,
}

/// Native pointer state retained per window.
#[derive(Debug)]
pub(super) struct NativePointerIngressState {
    devices: [Option<NativePointerDeviceRecord>; MAX_NATIVE_POINTER_DEVICES],
    contacts: [Option<NativePointerContactRecord>; MAX_NATIVE_POINTER_CONTACTS],
    device_allocator: NativeIdentityAllocator,
    contact_allocator: NativeIdentityAllocator,
}

impl Default for NativePointerIngressState {
    fn default() -> Self {
        Self {
            devices: [None; MAX_NATIVE_POINTER_DEVICES],
            contacts: [None; MAX_NATIVE_POINTER_CONTACTS],
            device_allocator: NativeIdentityAllocator::default(),
            contact_allocator: NativeIdentityAllocator::default(),
        }
    }
}

impl NativePointerIngressState {
    fn find_device(&self, native: DeviceId) -> Option<usize> {
        self.devices
            .iter()
            .position(|record| record.is_some_and(|record| record.native == native))
    }

    /// Keep the exact pending/active runtime gesture device out of idle eviction.
    /// There is at most one retained gesture per window; teardown reconciles it
    /// before the next gesture admission and window destruction drops the table.
    pub(super) fn retain_gesture_device(&mut self, device: Option<InputDeviceId>) {
        for record in self.devices.iter_mut().flatten() {
            record.gesture_retained = Some(record.normalized) == device;
        }
    }

    /// Retain a native device, reclaiming the lowest idle non-hover slot.
    pub(super) fn retain_device(
        &mut self,
        native: DeviceId,
        kind: DeviceKind,
    ) -> Result<InputDeviceId, NativePointerIdentityError> {
        if let Some(index) = self.find_device(native) {
            let record = self.devices[index]
                .as_mut()
                .ok_or(NativePointerIdentityError::WrongDevice)?;
            record.kind = kind;
            return Ok(record.normalized);
        }
        let index = self
            .devices
            .iter()
            .position(Option::is_none)
            .or_else(|| {
                self.devices.iter().position(|record| {
                    record.is_some_and(NativePointerDeviceRecord::is_reclaimable)
                })
            })
            .ok_or(NativePointerIdentityError::Capacity)?;
        let normalized = self.device_allocator.allocate_device()?;
        self.devices[index] = Some(NativePointerDeviceRecord {
            native,
            normalized,
            kind,
            active_contacts: 0,
            gesture_retained: false,
            hover: false,
        });
        Ok(normalized)
    }

    pub(super) fn set_hover(
        &mut self,
        native: DeviceId,
        entered: bool,
    ) -> Result<InputDeviceId, NativePointerIdentityError> {
        let normalized = self.retain_device(native, DeviceKind::Mouse)?;
        let index = self
            .find_device(native)
            .ok_or(NativePointerIdentityError::WrongDevice)?;
        self.devices[index]
            .as_mut()
            .ok_or(NativePointerIdentityError::WrongDevice)?
            .hover = entered;
        Ok(normalized)
    }

    fn begin_contact_with_kind(
        &mut self,
        native: DeviceId,
        raw_contact: u64,
        kind: DeviceKind,
    ) -> Result<(InputDeviceId, PointerContactId), NativePointerIdentityError> {
        if self.contacts.iter().any(|record| {
            record.is_some_and(|record| {
                record.native_device == native && record.raw_contact == raw_contact
            })
        }) {
            return Err(NativePointerIdentityError::DuplicateContact);
        }
        let device = self.retain_device(native, kind)?;
        let index = self
            .contacts
            .iter()
            .position(Option::is_none)
            .ok_or(NativePointerIdentityError::Capacity)?;
        let normalized = self.contact_allocator.allocate_contact()?;
        self.contacts[index] = Some(NativePointerContactRecord {
            native_device: native,
            device,
            raw_contact,
            normalized,
            sequence_token: None,
            last_position: None,
        });
        let device_index = self
            .find_device(native)
            .ok_or(NativePointerIdentityError::WrongDevice)?;
        self.devices[device_index]
            .as_mut()
            .ok_or(NativePointerIdentityError::WrongDevice)?
            .active_contacts += 1;
        Ok((device, normalized))
    }

    fn begin_contact(
        &mut self,
        native: DeviceId,
        raw_contact: u64,
    ) -> Result<(InputDeviceId, PointerContactId), NativePointerIdentityError> {
        self.begin_contact_with_kind(native, raw_contact, DeviceKind::Touch)
    }

    /// Retain one stable normalized contact for a native mouse device.  The
    /// sentinel raw id is private to this adapter and is never exposed to the
    /// controller.
    pub(super) fn retain_mouse_contact(
        &mut self,
        native: DeviceId,
    ) -> Result<(InputDeviceId, PointerContactId), NativePointerIdentityError> {
        if let Some(record) = self
            .contacts
            .iter()
            .flatten()
            .find(|record| record.native_device == native && record.raw_contact == u64::MAX)
        {
            return Ok((record.device, record.normalized));
        }
        self.begin_contact_with_kind(native, u64::MAX, DeviceKind::Mouse)
    }

    fn continue_contact(
        &self,
        native: DeviceId,
        raw_contact: u64,
    ) -> Result<
        (
            InputDeviceId,
            PointerContactId,
            Option<PointerSequenceToken>,
        ),
        NativePointerIdentityError,
    > {
        self.contacts
            .iter()
            .flatten()
            .find(|record| record.native_device == native && record.raw_contact == raw_contact)
            .map(|record| (record.device, record.normalized, record.sequence_token))
            .ok_or(NativePointerIdentityError::MissingContact)
    }

    pub(super) fn contact_token(
        &self,
        native: DeviceId,
        raw_contact: u64,
    ) -> Option<PointerSequenceToken> {
        self.contacts.iter().flatten().find_map(|record| {
            (record.native_device == native && record.raw_contact == raw_contact)
                .then_some(record.sequence_token)
                .flatten()
        })
    }

    pub(super) fn contact_token_for_identity(
        &self,
        device: InputDeviceId,
        contact: PointerContactId,
    ) -> Option<PointerSequenceToken> {
        self.contacts.iter().flatten().find_map(|record| {
            (record.device == device && record.normalized == contact)
                .then_some(record.sequence_token)
                .flatten()
        })
    }

    pub(super) fn set_token_for_identity(
        &mut self,
        device: InputDeviceId,
        contact: PointerContactId,
        token: PointerSequenceToken,
    ) -> Result<(), NativePointerIdentityError> {
        let Some(record) = self
            .contacts
            .iter_mut()
            .flatten()
            .find(|record| record.device == device && record.normalized == contact)
        else {
            return Err(NativePointerIdentityError::MissingContact);
        };
        record.sequence_token = Some(token);
        Ok(())
    }

    pub(super) fn clear_token_for_identity(
        &mut self,
        device: InputDeviceId,
        contact: PointerContactId,
    ) {
        if let Some(record) = self
            .contacts
            .iter_mut()
            .flatten()
            .find(|record| record.device == device && record.normalized == contact)
        {
            record.sequence_token = None;
        }
    }

    fn end_contact(
        &mut self,
        native: DeviceId,
        raw_contact: u64,
    ) -> Result<(InputDeviceId, PointerContactId), NativePointerIdentityError> {
        let index = self
            .contacts
            .iter()
            .position(|record| {
                record.is_some_and(|record| {
                    record.native_device == native && record.raw_contact == raw_contact
                })
            })
            .ok_or(NativePointerIdentityError::MissingContact)?;
        let record = self.contacts[index]
            .take()
            .ok_or(NativePointerIdentityError::MissingContact)?;
        if let Some(device_index) = self.find_device(native) {
            let device = self.devices[device_index]
                .as_mut()
                .ok_or(NativePointerIdentityError::WrongDevice)?;
            device.active_contacts = device.active_contacts.saturating_sub(1);
        }
        Ok((record.device, record.normalized))
    }

    #[cfg(test)]
    pub(super) fn active_device_count(&self) -> usize {
        self.devices.iter().flatten().count()
    }

    #[cfg(test)]
    pub(super) fn active_contact_count(&self) -> usize {
        self.contacts.iter().flatten().count()
    }

    #[cfg(test)]
    pub(super) fn exhaust_allocators(&mut self) {
        self.device_allocator.exhaust();
        self.contact_allocator.exhaust();
    }
}

/// A normalized touch sample.  The timestamp and DPI conversion are captured
/// exactly once by the caller before this value reaches the controller.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct NativeTouchSample {
    pub(super) device: InputDeviceId,
    pub(super) contact: PointerContactId,
    pub(super) phase: TouchPhase,
    pub(super) position: Point,
    pub(super) pressure: Option<PointerPressure>,
    pub(super) tilt: Option<PointerTilt>,
    pub(super) modifiers: PointerModifiers,
    pub(super) timestamp: InputTimestamp,
    pub(super) sequence_token: Option<PointerSequenceToken>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum NativeUnsupportedInput {
    DesktopPan,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct NativeGestureSample {
    pub(super) kind: crate::gui::pointer_ingress::GestureKind,
    pub(super) unit: crate::gui::pointer_ingress::GestureUnit,
    pub(super) value: f32,
    pub(super) phase: TouchPhase,
    pub(super) device: InputDeviceId,
    pub(super) modifiers: PointerModifiers,
    pub(super) timestamp: InputTimestamp,
}

pub(super) fn normalize_touch(
    state: &mut NativePointerIngressState,
    touch: Touch,
    dpi_scale: DpiScale,
    modifiers: PointerModifiers,
    timestamp: InputTimestamp,
) -> Result<NativeTouchSample, NativePointerIdentityError> {
    let terminal = matches!(touch.phase, TouchPhase::Ended | TouchPhase::Cancelled);
    let result = (|| {
        let position = logical_point_from_winit(touch.location, dpi_scale)
            .ok_or(NativePointerIdentityError::InvalidSample)?;
        let pressure = touch
            .force
            .map(normalize_force)
            .transpose()
            .map_err(|_| NativePointerIdentityError::InvalidSample)?;
        let (device, contact, sequence_token) = match touch.phase {
            TouchPhase::Started => {
                let (device, contact) = state.begin_contact(touch.device_id, touch.id)?;
                (device, contact, None)
            }
            _ => state.continue_contact(touch.device_id, touch.id)?,
        };
        let record = state
            .contacts
            .iter_mut()
            .flatten()
            .find(|record| record.device == device && record.normalized == contact)
            .ok_or(NativePointerIdentityError::MissingContact)?;
        record.last_position = Some(position);
        Ok(NativeTouchSample {
            device,
            contact,
            phase: touch.phase,
            position,
            pressure,
            tilt: None,
            modifiers,
            timestamp,
            sequence_token,
        })
    })();
    // A malformed terminal cannot become a motion/drop, but must retire its
    // exact admitted sequence. Preserve the last valid logical position and
    // token; never cancel another contact or the legacy mouse capture.
    let result = if terminal && result.is_err() {
        state
            .contacts
            .iter()
            .flatten()
            .find(|record| {
                record.native_device == touch.device_id && record.raw_contact == touch.id
            })
            .and_then(|record| {
                record.last_position.map(|position| NativeTouchSample {
                    device: record.device,
                    contact: record.normalized,
                    phase: TouchPhase::Cancelled,
                    position,
                    pressure: None,
                    tilt: None,
                    modifiers,
                    timestamp,
                    sequence_token: record.sequence_token,
                })
            })
            .ok_or(NativePointerIdentityError::MissingContact)
    } else {
        result
    };
    if terminal {
        let _ = state.end_contact(touch.device_id, touch.id);
    }
    result
}

fn normalize_force(force: Force) -> Result<PointerPressure, ()> {
    let normalized = match force {
        Force::Normalized(value) => value,
        Force::Calibrated {
            force,
            max_possible_force,
            ..
        } => {
            if !force.is_finite() || !max_possible_force.is_finite() || max_possible_force <= 0.0 {
                return Err(());
            }
            force / max_possible_force
        }
    };
    PointerPressure::new(normalized as f32).map_err(|_| ())
}

pub(super) fn normalize_gesture(
    state: &mut NativePointerIngressState,
    device: DeviceId,
    gesture: GestureInput,
    dpi_scale: DpiScale,
    modifiers: PointerModifiers,
    timestamp: InputTimestamp,
) -> Result<Result<NativeGestureSample, NativeUnsupportedInput>, NativePointerIdentityError> {
    let normalized_device = state.retain_device(device, DeviceKind::Trackpad)?;
    let sample = match gesture {
        GestureInput::Pan { delta, phase: _ } => {
            if !delta.x.is_finite() || !delta.y.is_finite() {
                return Err(NativePointerIdentityError::InvalidSample);
            }
            let _ = Point::new(
                dpi_scale.physical_to_logical(delta.x),
                dpi_scale.physical_to_logical(delta.y),
            );
            Err(NativeUnsupportedInput::DesktopPan)
        }
        GestureInput::Pinch { delta, phase } => {
            // GestureIngress checks the resulting scalar at the controller
            // boundary, where malformed continuations can retire their owner.
            Ok(NativeGestureSample {
                kind: crate::gui::pointer_ingress::GestureKind::Pinch,
                unit: crate::gui::pointer_ingress::GestureUnit::Scale,
                // Winit reports magnification delta; the public contract
                // carries the resulting positive scale factor.
                value: 1.0 + delta as f32,
                phase,
                device: normalized_device,
                modifiers,
                timestamp,
            })
        }
        GestureInput::Rotate {
            delta_degrees,
            phase,
        } => Ok(NativeGestureSample {
            kind: crate::gui::pointer_ingress::GestureKind::Rotate,
            unit: crate::gui::pointer_ingress::GestureUnit::Radians,
            value: delta_degrees.to_radians(),
            phase,
            device: normalized_device,
            modifiers,
            timestamp,
        }),
    };
    Ok(sample)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum GestureInput {
    Pan {
        delta: winit::dpi::PhysicalPosition<f32>,
        phase: TouchPhase,
    },
    Pinch {
        delta: f64,
        phase: TouchPhase,
    },
    Rotate {
        delta_degrees: f32,
        phase: TouchPhase,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::input::InputTimestamp;
    use winit::dpi::PhysicalPosition;

    fn touch(device: DeviceId, id: u64, phase: TouchPhase) -> Touch {
        Touch {
            device_id: device,
            phase,
            location: PhysicalPosition::new(20.0, 40.0),
            force: None,
            id,
        }
    }

    #[test]
    fn bounded_devices_reclaim_only_lowest_idle_slot() {
        let mut state = NativePointerIngressState::default();
        assert_eq!(state.active_device_count(), 0);
        state.set_hover(DeviceId::dummy(), true).unwrap();
        assert_eq!(state.active_device_count(), 1);
        // The native table has a fixed 16-slot bound; no eviction is allowed
        // while the sole retained device is marked as hovering.
        assert!(state.devices.iter().flatten().all(|record| record.hover));
    }

    #[test]
    fn allocator_boundary_issues_last_identity_once_without_reuse() {
        let mut state = NativePointerIngressState::default();
        state.exhaust_allocators();
        let native = DeviceId::dummy();
        let last = state.begin_contact(native, 7).unwrap();
        assert_eq!(state.active_contact_count(), 1);
        assert_eq!(state.end_contact(native, 7).unwrap(), last);
        assert_eq!(
            state.begin_contact(native, 8),
            Err(NativePointerIdentityError::Exhausted)
        );
        assert_eq!(state.active_contact_count(), 0);
        assert_eq!(
            state.device_allocator.allocate_device(),
            Err(NativePointerIdentityError::Exhausted)
        );
    }

    #[test]
    fn malformed_terminal_cancels_exact_token_at_last_valid_position() {
        let native = DeviceId::dummy();
        let mut tokens = crate::gui::pointer_ingress::PointerSequenceAllocator::new(7).unwrap();
        for invalid_force in [false, true] {
            let mut state = NativePointerIngressState::default();
            let first = normalize_touch(
                &mut state,
                touch(native, 7, TouchPhase::Started),
                DpiScale::ONE,
                PointerModifiers::default(),
                InputTimestamp::capture(),
            )
            .unwrap();
            let token = tokens.issue().unwrap();
            state
                .set_token_for_identity(first.device, first.contact, token)
                .unwrap();
            let mut terminal = touch(native, 7, TouchPhase::Ended);
            if invalid_force {
                terminal.force = Some(Force::Normalized(f64::NAN));
            } else {
                terminal.location.x = f64::NAN;
            }
            let cancelled = normalize_touch(
                &mut state,
                terminal,
                DpiScale::ONE,
                PointerModifiers::default(),
                InputTimestamp::capture(),
            )
            .unwrap();
            assert_eq!(cancelled.phase, TouchPhase::Cancelled);
            assert_eq!(cancelled.position, first.position);
            assert_eq!(cancelled.sequence_token, Some(token));
            assert_eq!(cancelled.contact, first.contact);
            assert_eq!(state.active_contact_count(), 0);
            assert!(
                normalize_touch(
                    &mut state,
                    terminal,
                    DpiScale::ONE,
                    PointerModifiers::default(),
                    InputTimestamp::capture()
                )
                .is_err()
            );
        }
    }

    #[test]
    fn mouse_contact_is_stable_while_touch_reuse_gets_a_fresh_identity() {
        let mut state = NativePointerIngressState::default();
        let device = DeviceId::dummy();
        let first = state.retain_mouse_contact(device).unwrap();
        let second = state.retain_mouse_contact(device).unwrap();
        assert_eq!(first, second);
        let pinch = normalize_gesture(
            &mut state,
            device,
            GestureInput::Pinch {
                delta: 0.25,
                phase: TouchPhase::Moved,
            },
            DpiScale::ONE,
            PointerModifiers::default(),
            InputTimestamp::capture(),
        )
        .unwrap()
        .unwrap();
        assert!((pinch.value - 1.25).abs() < 0.001);
    }

    #[test]
    fn contacts_are_fresh_after_terminal_reuse_and_malformed_terminal_cleans_up() {
        let device = DeviceId::dummy();
        let mut state = NativePointerIngressState::default();
        let first = normalize_touch(
            &mut state,
            touch(device, 7, TouchPhase::Started),
            DpiScale::ONE,
            PointerModifiers::default(),
            InputTimestamp::capture(),
        )
        .unwrap();
        let ended = normalize_touch(
            &mut state,
            touch(device, 7, TouchPhase::Ended),
            DpiScale::ONE,
            PointerModifiers::default(),
            InputTimestamp::capture(),
        )
        .unwrap();
        assert_eq!(first.device, ended.device);
        let second = normalize_touch(
            &mut state,
            touch(device, 7, TouchPhase::Started),
            DpiScale::ONE,
            PointerModifiers::default(),
            InputTimestamp::capture(),
        )
        .unwrap();
        assert_ne!(first.contact, second.contact);
        let mut malformed = touch(device, 9, TouchPhase::Started);
        malformed.location.x = f64::NAN;
        assert!(
            normalize_touch(
                &mut state,
                malformed,
                DpiScale::ONE,
                PointerModifiers::default(),
                InputTimestamp::capture()
            )
            .is_err()
        );
        let malformed_end = touch(device, 9, TouchPhase::Ended);
        assert!(
            normalize_touch(
                &mut state,
                malformed_end,
                DpiScale::ONE,
                PointerModifiers::default(),
                InputTimestamp::capture()
            )
            .is_err()
        );
        assert_eq!(state.active_contact_count(), 1);
    }

    #[test]
    fn force_is_checked_and_gestures_have_explicit_transport_outcomes() {
        let device = DeviceId::dummy();
        let mut state = NativePointerIngressState::default();
        let gesture = normalize_gesture(
            &mut state,
            device,
            GestureInput::Rotate {
                delta_degrees: 180.0,
                phase: TouchPhase::Moved,
            },
            DpiScale::ONE,
            PointerModifiers::default(),
            InputTimestamp::capture(),
        )
        .unwrap()
        .unwrap();
        assert!((gesture.value - std::f32::consts::PI).abs() < 0.001);
        assert_eq!(
            normalize_gesture(
                &mut state,
                device,
                GestureInput::Pan {
                    delta: PhysicalPosition::new(1.0, 2.0),
                    phase: TouchPhase::Moved
                },
                DpiScale::ONE,
                PointerModifiers::default(),
                InputTimestamp::capture()
            )
            .unwrap(),
            Err(NativeUnsupportedInput::DesktopPan)
        );
    }
    #[test]
    fn pending_and_active_gesture_devices_are_not_idle_eviction_candidates() {
        let mut state = NativePointerIngressState::default();
        let native = DeviceId::dummy();
        let device = state.retain_device(native, DeviceKind::Trackpad).unwrap();
        let index = state.find_device(native).unwrap();
        assert!(state.devices[index].unwrap().is_reclaimable());
        state.retain_gesture_device(Some(device));
        assert!(!state.devices[index].unwrap().is_reclaimable());
        assert_eq!(
            state.retain_device(native, DeviceKind::Trackpad),
            Ok(device)
        );
        state.retain_gesture_device(None);
        assert!(state.devices[index].unwrap().is_reclaimable());
        state.set_hover(native, true).unwrap();
        assert!(!state.devices[index].unwrap().is_reclaimable());
    }
}

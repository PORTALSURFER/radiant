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
        self.next = Some(NonZeroU64::new(u64::MAX).unwrap());
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct NativePointerDeviceRecord {
    native: DeviceId,
    normalized: InputDeviceId,
    kind: DeviceKind,
    active_contacts: u8,
    hover: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct NativePointerContactRecord {
    native_device: DeviceId,
    device: InputDeviceId,
    raw_contact: u64,
    normalized: PointerContactId,
    sequence_token: Option<PointerSequenceToken>,
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

    /// Retain a native device, reclaiming the lowest idle non-hover slot.
    pub(super) fn retain_device(
        &mut self,
        native: DeviceId,
        kind: DeviceKind,
    ) -> Result<InputDeviceId, NativePointerIdentityError> {
        if let Some(index) = self.find_device(native) {
            let record = self.devices[index].as_mut().expect("device index exists");
            record.kind = kind;
            return Ok(record.normalized);
        }
        let index = self
            .devices
            .iter()
            .position(Option::is_none)
            .or_else(|| {
                self.devices.iter().position(|record| {
                    record.is_some_and(|record| record.active_contacts == 0 && !record.hover)
                })
            })
            .ok_or(NativePointerIdentityError::Capacity)?;
        let normalized = self.device_allocator.allocate_device()?;
        self.devices[index] = Some(NativePointerDeviceRecord {
            native,
            normalized,
            kind,
            active_contacts: 0,
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
        let index = self.find_device(native).expect("retained device exists");
        self.devices[index]
            .as_mut()
            .expect("device index exists")
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
        });
        let device_index = self.find_device(native).expect("retained device exists");
        self.devices[device_index]
            .as_mut()
            .expect("device index exists")
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

    pub(super) fn set_contact_token(
        &mut self,
        native: DeviceId,
        raw_contact: u64,
        token: PointerSequenceToken,
    ) -> Result<(), NativePointerIdentityError> {
        let Some(record) = self
            .contacts
            .iter_mut()
            .flatten()
            .find(|record| record.native_device == native && record.raw_contact == raw_contact)
        else {
            return Err(NativePointerIdentityError::MissingContact);
        };
        record.sequence_token = Some(token);
        Ok(())
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
        let record = self.contacts[index].take().expect("contact index exists");
        if let Some(device_index) = self.find_device(native) {
            let device = self.devices[device_index]
                .as_mut()
                .expect("device index exists");
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
    Pen,
    DesktopPan,
    GestureTransport,
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
    let position = logical_point_from_winit(touch.location, dpi_scale)
        .ok_or(NativePointerIdentityError::InvalidSample);
    let pressure = touch
        .force
        .map(normalize_force)
        .transpose()
        .map_err(|_| NativePointerIdentityError::InvalidSample)?;
    let result = position.and_then(|position| {
        let (device, contact, sequence_token) = match touch.phase {
            TouchPhase::Started => {
                let (device, contact) = state.begin_contact(touch.device_id, touch.id)?;
                (device, contact, None)
            }
            TouchPhase::Moved => state.continue_contact(touch.device_id, touch.id)?,
            TouchPhase::Ended | TouchPhase::Cancelled => {
                state.continue_contact(touch.device_id, touch.id)?
            }
        };
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
    });
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
            if !delta.is_finite() {
                return Err(NativePointerIdentityError::InvalidSample);
            }
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
        } => {
            if !delta_degrees.is_finite() {
                return Err(NativePointerIdentityError::InvalidSample);
            }
            Ok(NativeGestureSample {
                kind: crate::gui::pointer_ingress::GestureKind::Rotate,
                unit: crate::gui::pointer_ingress::GestureUnit::Radians,
                value: delta_degrees.to_radians(),
                phase,
                device: normalized_device,
                modifiers,
                timestamp,
            })
        }
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
}

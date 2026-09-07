//! Bounded, side-effect-free geometry for two admitted touch contacts.

use crate::gui::{
    pointer_ingress::{InputDeviceId, PointerContactId, PointerPhase, PointerSequenceToken},
    types::{Point, Vector2},
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::runtime::controller) struct TouchContactSample {
    pub(in crate::runtime::controller) device: InputDeviceId,
    pub(in crate::runtime::controller) contact: PointerContactId,
    pub(in crate::runtime::controller) token: PointerSequenceToken,
    pub(in crate::runtime::controller) phase: PointerPhase,
    pub(in crate::runtime::controller) position: Point,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::runtime::controller) struct TouchPairGeometry {
    pub(in crate::runtime::controller) centroid: Point,
    pub(in crate::runtime::controller) pan: Vector2,
    pub(in crate::runtime::controller) scale: f32,
    pub(in crate::runtime::controller) scale_delta: f32,
    pub(in crate::runtime::controller) rotation: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::runtime::controller) enum TouchPairReset {
    ThirdContact,
    DeviceMismatch,
    Terminal,
    InvalidSample,
    StaleMember,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::runtime::controller) enum TouchPairUpdate {
    FirstContact,
    PairEstablished(TouchPairGeometry),
    Updated(TouchPairGeometry),
    PairEnded(TouchPairGeometry),
    Reset(TouchPairReset),
    Ignored,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Contact {
    contact: PointerContactId,
    token: PointerSequenceToken,
    baseline: Point,
    current: Point,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(in crate::runtime::controller) struct TouchPairState {
    device: Option<InputDeviceId>,
    contacts: [Option<Contact>; 2],
}

impl TouchPairState {
    pub(in crate::runtime::controller) fn is_empty(&self) -> bool {
        self.contacts.iter().all(Option::is_none)
    }

    pub(in crate::runtime::controller) fn contains_token(
        &self,
        token: PointerSequenceToken,
    ) -> bool {
        self.contacts
            .iter()
            .flatten()
            .any(|contact| contact.token == token)
    }

    /// Observe only samples whose pointer sequence has already been admitted.
    pub(in crate::runtime::controller) fn observe(
        &mut self,
        sample: TouchContactSample,
    ) -> TouchPairUpdate {
        if !sample.position.is_finite() {
            return self.reset(TouchPairReset::InvalidSample);
        }
        if self.device.is_some_and(|device| device != sample.device) {
            return self.reset(TouchPairReset::DeviceMismatch);
        }
        match sample.phase {
            PointerPhase::Started { .. } => self.start(sample),
            PointerPhase::Moved => self.move_contact(sample),
            PointerPhase::Ended { .. } => {
                let Some(index) = self.member(sample) else {
                    return if self.contact_index(sample.contact).is_some() {
                        self.reset(TouchPairReset::StaleMember)
                    } else {
                        TouchPairUpdate::Ignored
                    };
                };
                if self.contacts[1].is_none() {
                    return self.reset(TouchPairReset::Terminal);
                }
                let Some(contact) = self.contacts[index].as_mut() else {
                    return self.reset(TouchPairReset::StaleMember);
                };
                contact.current = sample.position;
                let geometry = self.geometry();
                self.clear();
                geometry.map_or(
                    TouchPairUpdate::Reset(TouchPairReset::InvalidSample),
                    TouchPairUpdate::PairEnded,
                )
            }
            PointerPhase::Cancelled => {
                if self.member(sample).is_none() {
                    return if self.contact_index(sample.contact).is_some() {
                        self.reset(TouchPairReset::StaleMember)
                    } else {
                        TouchPairUpdate::Ignored
                    };
                }
                self.reset(TouchPairReset::Terminal)
            }
            PointerPhase::Hover => TouchPairUpdate::Ignored,
        }
    }

    pub(in crate::runtime::controller) fn tokens(&self) -> Option<[PointerSequenceToken; 2]> {
        Some([self.contacts[0]?.token, self.contacts[1]?.token])
    }

    pub(in crate::runtime::controller) fn clear(&mut self) {
        *self = Self::default();
    }

    fn start(&mut self, sample: TouchContactSample) -> TouchPairUpdate {
        if self
            .contacts
            .iter()
            .flatten()
            .any(|contact| contact.contact == sample.contact)
        {
            return self.reset(TouchPairReset::StaleMember);
        }
        if let Some(device) = self.device
            && device != sample.device
        {
            return self.reset(TouchPairReset::DeviceMismatch);
        }
        let Some(slot) = self.contacts.iter().position(Option::is_none) else {
            return self.reset(TouchPairReset::ThirdContact);
        };
        self.device = Some(sample.device);
        self.contacts[slot] = Some(Contact {
            contact: sample.contact,
            token: sample.token,
            baseline: sample.position,
            current: sample.position,
        });
        if slot == 0 {
            return TouchPairUpdate::FirstContact;
        }
        for contact in self.contacts.iter_mut().flatten() {
            contact.baseline = contact.current;
        }
        match self.geometry() {
            Some(geometry) => TouchPairUpdate::PairEstablished(geometry),
            None => self.reset(TouchPairReset::InvalidSample),
        }
    }

    fn move_contact(&mut self, sample: TouchContactSample) -> TouchPairUpdate {
        let Some(index) = self.member(sample) else {
            return if self.contact_index(sample.contact).is_some() {
                self.reset(TouchPairReset::StaleMember)
            } else {
                TouchPairUpdate::Ignored
            };
        };
        let Some(contact) = self.contacts[index].as_mut() else {
            return self.reset(TouchPairReset::StaleMember);
        };
        contact.current = sample.position;
        if self.contacts[1].is_none() {
            return TouchPairUpdate::FirstContact;
        }
        match self.geometry() {
            Some(geometry) => TouchPairUpdate::Updated(geometry),
            None => self.reset(TouchPairReset::InvalidSample),
        }
    }

    fn member(&self, sample: TouchContactSample) -> Option<usize> {
        (self.device == Some(sample.device)).then_some(())?;
        self.contacts.iter().position(|contact| {
            contact.is_some_and(|contact| {
                contact.contact == sample.contact && contact.token == sample.token
            })
        })
    }

    fn contact_index(&self, contact: PointerContactId) -> Option<usize> {
        self.contacts
            .iter()
            .position(|entry| entry.is_some_and(|entry| entry.contact == contact))
    }

    fn geometry(&self) -> Option<TouchPairGeometry> {
        let [Some(first), Some(second)] = self.contacts else {
            return None;
        };
        let base = Vector2::new(
            second.baseline.x - first.baseline.x,
            second.baseline.y - first.baseline.y,
        );
        let current = Vector2::new(
            second.current.x - first.current.x,
            second.current.y - first.current.y,
        );
        let base_distance = base.x.hypot(base.y);
        let current_distance = current.x.hypot(current.y);
        if !base_distance.is_finite()
            || !current_distance.is_finite()
            || base_distance <= f32::EPSILON
            || current_distance <= f32::EPSILON
        {
            return None;
        }
        let normalized_base = Vector2::new(base.x / base_distance, base.y / base_distance);
        let normalized_current =
            Vector2::new(current.x / current_distance, current.y / current_distance);
        let scale = current_distance / base_distance;
        let rotation = (normalized_base.x * normalized_current.y
            - normalized_base.y * normalized_current.x)
            .atan2(
                normalized_base.x * normalized_current.x + normalized_base.y * normalized_current.y,
            );
        let centroid = Point::new(
            first.current.x * 0.5 + second.current.x * 0.5,
            first.current.y * 0.5 + second.current.y * 0.5,
        );
        let base_centroid = Point::new(
            first.baseline.x * 0.5 + second.baseline.x * 0.5,
            first.baseline.y * 0.5 + second.baseline.y * 0.5,
        );
        let pan = Vector2::new(centroid.x - base_centroid.x, centroid.y - base_centroid.y);
        (scale.is_finite()
            && rotation.is_finite()
            && centroid.is_finite()
            && base_centroid.is_finite()
            && pan.x.is_finite()
            && pan.y.is_finite())
        .then_some(TouchPairGeometry {
            centroid,
            pan,
            scale,
            scale_delta: scale - 1.0,
            rotation,
        })
    }

    fn reset(&mut self, reason: TouchPairReset) -> TouchPairUpdate {
        *self = Self::default();
        TouchPairUpdate::Reset(reason)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::pointer_ingress::PointerSequenceAllocator;
    use crate::widgets::PointerButton;

    fn sample(
        device: u64,
        contact: u64,
        token: PointerSequenceToken,
        phase: PointerPhase,
        x: f32,
        y: f32,
    ) -> TouchContactSample {
        TouchContactSample {
            device: InputDeviceId::new(device).unwrap(),
            contact: PointerContactId::new(contact).unwrap(),
            token,
            phase,
            position: Point::new(x, y),
        }
    }
    #[test]
    fn pair_derives_centroid_pan_scale_and_wrapped_rotation() {
        let mut state = TouchPairState::default();
        let mut tokens = PointerSequenceAllocator::new(1).unwrap();
        let a = tokens.issue().unwrap();
        let b = tokens.issue().unwrap();
        assert_eq!(
            state.observe(sample(
                1,
                1,
                a,
                PointerPhase::Started {
                    button: PointerButton::Primary
                },
                0.0,
                0.0
            )),
            TouchPairUpdate::FirstContact
        );
        assert!(matches!(
            state.observe(sample(
                1,
                2,
                b,
                PointerPhase::Started {
                    button: PointerButton::Primary
                },
                2.0,
                0.0
            )),
            TouchPairUpdate::PairEstablished(_)
        ));
        let TouchPairUpdate::Updated(geometry) =
            state.observe(sample(1, 2, b, PointerPhase::Moved, 0.0, 4.0))
        else {
            panic!("pair update")
        };
        assert_eq!(geometry.centroid, Point::new(0.0, 2.0));
        assert_eq!(geometry.pan, Vector2::new(-1.0, 2.0));
        assert_eq!(geometry.scale, 2.0);
        assert!((geometry.rotation - core::f32::consts::FRAC_PI_2).abs() < 0.0001);
    }
    #[test]
    fn moving_first_contact_rebases_when_second_contact_starts() {
        let mut state = TouchPairState::default();
        let mut tokens = PointerSequenceAllocator::new(1).unwrap();
        let a = tokens.issue().unwrap();
        let b = tokens.issue().unwrap();
        let start = PointerPhase::Started {
            button: PointerButton::Primary,
        };
        assert_eq!(
            state.observe(sample(1, 1, a, start, 0.0, 0.0)),
            TouchPairUpdate::FirstContact
        );
        assert_eq!(
            state.observe(sample(1, 1, a, PointerPhase::Moved, 4.0, 3.0)),
            TouchPairUpdate::FirstContact
        );
        let TouchPairUpdate::PairEstablished(geometry) =
            state.observe(sample(1, 2, b, start, 6.0, 3.0))
        else {
            panic!("pair")
        };
        assert_eq!(geometry.pan, Vector2::default());
        assert_eq!(geometry.scale, 1.0);
        assert_eq!(geometry.rotation, 0.0);
    }
    #[test]
    fn third_device_terminal_and_stale_contact_reset_or_ignore() {
        let mut state = TouchPairState::default();
        let mut tokens = PointerSequenceAllocator::new(1).unwrap();
        let a = tokens.issue().unwrap();
        let b = tokens.issue().unwrap();
        let c = tokens.issue().unwrap();
        let start = PointerPhase::Started {
            button: PointerButton::Primary,
        };
        let _ = state.observe(sample(1, 1, a, start, 0.0, 0.0));
        let _ = state.observe(sample(1, 2, b, start, 1.0, 0.0));
        assert_eq!(
            state.observe(sample(1, 3, c, start, 2.0, 0.0)),
            TouchPairUpdate::Reset(TouchPairReset::ThirdContact)
        );
        assert_eq!(
            state.observe(sample(1, 1, a, PointerPhase::Moved, 1.0, 0.0)),
            TouchPairUpdate::Ignored
        );
        let _ = state.observe(sample(1, 1, a, start, 0.0, 0.0));
        assert_eq!(
            state.observe(sample(2, 2, b, start, 1.0, 0.0)),
            TouchPairUpdate::Reset(TouchPairReset::DeviceMismatch)
        );
    }

    #[test]
    fn terminal_and_coincident_pair_fail_closed() {
        let mut state = TouchPairState::default();
        let mut tokens = PointerSequenceAllocator::new(1).unwrap();
        let a = tokens.issue().unwrap();
        let b = tokens.issue().unwrap();
        let start = PointerPhase::Started {
            button: PointerButton::Primary,
        };
        let _ = state.observe(sample(1, 1, a, start, 0.0, 0.0));
        assert_eq!(
            state.observe(sample(1, 2, b, start, 0.0, 0.0)),
            TouchPairUpdate::Reset(TouchPairReset::InvalidSample)
        );
        let _ = state.observe(sample(1, 1, a, start, 0.0, 0.0));
        let _ = state.observe(sample(1, 2, b, start, 2.0, 0.0));
        let TouchPairUpdate::PairEnded(geometry) = state.observe(sample(
            1,
            2,
            b,
            PointerPhase::Ended {
                button: PointerButton::Primary,
            },
            2.0,
            2.0,
        )) else {
            panic!("pair end")
        };
        assert_eq!(geometry.pan, Vector2::new(0.0, 1.0));
        assert_eq!(
            state.observe(sample(1, 1, a, PointerPhase::Moved, 1.0, 0.0)),
            TouchPairUpdate::Ignored
        );
    }

    #[test]
    fn extreme_finite_geometry_remains_finite() {
        let mut state = TouchPairState::default();
        let mut tokens = PointerSequenceAllocator::new(1).unwrap();
        let a = tokens.issue().unwrap();
        let b = tokens.issue().unwrap();
        let start = PointerPhase::Started {
            button: PointerButton::Primary,
        };
        let _ = state.observe(sample(1, 1, a, start, -1.0e30, 0.0));
        let TouchPairUpdate::PairEstablished(geometry) =
            state.observe(sample(1, 2, b, start, 1.0e30, 0.0))
        else {
            panic!("pair")
        };
        assert!(geometry.scale.is_finite() && geometry.rotation.is_finite());
    }
}

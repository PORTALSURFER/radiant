//! Optional stable pointer-motion capability for public widgets.
//!
//! This descriptor covers stable-motion admission, captured-pointer
//! pass-through, and the transient-overlay/full-scene policy. A widget-local
//! snapped timeline cursor, hover affordance, or resize-handle preview may
//! request repaint even when `handle_input` returns `None`; its overlay remains
//! local and must not emit host messages. The runtime uses the descriptor only
//! after culling and otherwise falls back to rebuilding the base scene.

use super::widget::PointerCapturePolicy;
use std::{any::Any, fmt, rc::Rc};

/// Typed revision evidence for one exported [`WidgetPointerMotion`] capability.
#[derive(Clone)]
pub struct WidgetPointerMotionRevision {
    representation: PointerMotionRevisionRepresentation,
}

#[derive(Clone, Default)]
enum PointerMotionRevisionRepresentation {
    #[default]
    Conservative,
    Exact(Rc<dyn PointerMotionRevisionValue>),
}

impl Default for WidgetPointerMotionRevision {
    fn default() -> Self {
        Self::conservative()
    }
}

impl fmt::Debug for WidgetPointerMotionRevision {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("WidgetPointerMotionRevision")
            .field(
                "representation",
                &match self.representation {
                    PointerMotionRevisionRepresentation::Conservative => "conservative",
                    PointerMotionRevisionRepresentation::Exact(_) => "exact",
                },
            )
            .finish()
    }
}

impl PartialEq for WidgetPointerMotionRevision {
    fn eq(&self, other: &Self) -> bool {
        match (&self.representation, &other.representation) {
            (
                PointerMotionRevisionRepresentation::Conservative,
                PointerMotionRevisionRepresentation::Conservative,
            ) => true,
            (
                PointerMotionRevisionRepresentation::Exact(previous),
                PointerMotionRevisionRepresentation::Exact(current),
            ) => previous.equals(&**current),
            _ => false,
        }
    }
}

impl Eq for WidgetPointerMotionRevision {}

trait PointerMotionRevisionValue: Any {
    fn equals(&self, other: &dyn PointerMotionRevisionValue) -> bool;
}

impl<T> PointerMotionRevisionValue for T
where
    T: Eq + 'static,
{
    fn equals(&self, other: &dyn PointerMotionRevisionValue) -> bool {
        other
            .as_any()
            .downcast_ref::<T>()
            .is_some_and(|candidate| self == candidate)
    }
}

impl dyn PointerMotionRevisionValue {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl WidgetPointerMotionRevision {
    /// Return the safe fallback when pointer-motion behavior cannot prove its changes.
    #[must_use]
    pub const fn conservative() -> Self {
        Self {
            representation: PointerMotionRevisionRepresentation::Conservative,
        }
    }

    /// Build exact typed evidence for pointer-motion behavior.
    #[must_use]
    pub fn exact<T>(value: T) -> Self
    where
        T: Eq + 'static,
    {
        Self {
            representation: PointerMotionRevisionRepresentation::Exact(Rc::new(value)),
        }
    }

    pub(crate) fn is_exact(&self) -> bool {
        matches!(
            self.representation,
            PointerMotionRevisionRepresentation::Exact(_)
        )
    }
}

/// Object-safe stable pointer-motion capability.
///
/// The runtime queries this after hit-test culling. The capability is a pure,
/// allocation-free observation: it cannot install capture, alter focus,
/// schedule work, or access host, renderer, or application authority.
pub trait WidgetPointerMotion {
    /// Return typed revision evidence for this capability's behavior.
    fn revision(&self) -> WidgetPointerMotionRevision {
        WidgetPointerMotionRevision::conservative()
    }

    /// Return whether stable pointer motion should be delivered.
    fn accepts_pointer_move(&self) -> bool {
        true
    }

    /// Return pointer routing behavior while this widget owns capture.
    fn pointer_capture_policy(&self) -> PointerCapturePolicy {
        PointerCapturePolicy::PassThrough
    }

    /// Return whether stable hover/capture changes may use a valid transient
    /// overlay instead of rebuilding the full scene.
    fn prefers_pointer_move_paint_only(&self) -> bool {
        false
    }

    /// Return whether the widget has a valid transient overlay for the
    /// stable pointer-motion path.
    ///
    /// This is a promise that [`crate::widgets::Widget::append_runtime_overlay_paint`]
    /// can represent every pointer-local visual change without rebuilding the
    /// base scene. Keep it `false` when the overlay is absent, ambiguous, or
    /// would need host output, full widget chrome, text, or uncapped work.
    /// Returning `true` is still only an admission hint: the runtime owns
    /// repaint and presentation authority and never asks this descriptor to
    /// paint or emit output.
    fn pointer_move_overlay_is_valid(&self) -> bool {
        false
    }
}

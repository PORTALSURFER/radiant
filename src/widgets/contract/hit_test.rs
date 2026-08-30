//! Optional event-aware hit-test and cursor capability for public widgets.

use crate::{
    gui::types::{Point, Rect},
    widgets::interaction::{WidgetCursor, WidgetInput},
};
use std::{any::Any, fmt, rc::Rc};

/// The result of one widget-local hit-test observation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum WidgetHitTestResult {
    /// Keep this widget as the front-most target for the input.
    #[default]
    Opaque,
    /// Continue hit testing with the next widget behind this one.
    PassThrough,
}

/// Typed revision evidence for one exported [`WidgetHitTest`] capability.
#[derive(Clone)]
pub struct WidgetHitTestRevision {
    representation: HitTestRevisionRepresentation,
}

#[derive(Clone, Default)]
enum HitTestRevisionRepresentation {
    #[default]
    Conservative,
    Exact(Rc<dyn HitTestRevisionValue>),
}

impl Default for WidgetHitTestRevision {
    fn default() -> Self {
        Self::conservative()
    }
}

impl fmt::Debug for WidgetHitTestRevision {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("WidgetHitTestRevision")
            .field(
                "representation",
                &match self.representation {
                    HitTestRevisionRepresentation::Conservative => "conservative",
                    HitTestRevisionRepresentation::Exact(_) => "exact",
                },
            )
            .finish()
    }
}

impl PartialEq for WidgetHitTestRevision {
    fn eq(&self, other: &Self) -> bool {
        match (&self.representation, &other.representation) {
            (
                HitTestRevisionRepresentation::Conservative,
                HitTestRevisionRepresentation::Conservative,
            ) => true,
            (
                HitTestRevisionRepresentation::Exact(previous),
                HitTestRevisionRepresentation::Exact(current),
            ) => previous.equals(&**current),
            _ => false,
        }
    }
}

impl Eq for WidgetHitTestRevision {}

trait HitTestRevisionValue: Any {
    fn equals(&self, other: &dyn HitTestRevisionValue) -> bool;
}

impl<T> HitTestRevisionValue for T
where
    T: Eq + 'static,
{
    fn equals(&self, other: &dyn HitTestRevisionValue) -> bool {
        other
            .as_any()
            .downcast_ref::<T>()
            .is_some_and(|candidate| self == candidate)
    }
}

impl dyn HitTestRevisionValue {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl WidgetHitTestRevision {
    /// Return the safe fallback when hit-test behavior cannot prove its changes.
    #[must_use]
    pub const fn conservative() -> Self {
        Self {
            representation: HitTestRevisionRepresentation::Conservative,
        }
    }

    /// Build exact typed evidence for hit-test and cursor behavior.
    #[must_use]
    pub fn exact<T>(value: T) -> Self
    where
        T: Eq + 'static,
    {
        Self {
            representation: HitTestRevisionRepresentation::Exact(Rc::new(value)),
        }
    }

    pub(crate) fn is_exact(&self) -> bool {
        matches!(self.representation, HitTestRevisionRepresentation::Exact(_))
    }
}

/// Object-safe event-aware hit-test and cursor capability.
///
/// The runtime calls this only after layout bounds and clip ancestry have
/// admitted a candidate. It supplies the current pointer event so a widget
/// can expose opaque or pass-through behavior without changing traversal,
/// capture, or router authority. Implementations should remain pure,
/// allocation-free observations of widget-local state.
pub trait WidgetHitTest {
    /// Return typed revision evidence for this capability's behavior.
    fn revision(&self) -> WidgetHitTestRevision {
        WidgetHitTestRevision::conservative()
    }

    /// Classify the candidate at `point` for this input.
    ///
    /// The default preserves the historical rectangular opaque hit.
    fn hit_test(&self, _bounds: Rect, _point: Point, _input: &WidgetInput) -> WidgetHitTestResult {
        WidgetHitTestResult::Opaque
    }

    /// Return the cursor for an admitted opaque target or capture owner.
    ///
    /// Returning `None` keeps the runtime's default cursor.
    fn cursor_for_point(&self, _bounds: Rect, _point: Point) -> Option<WidgetCursor> {
        None
    }
}

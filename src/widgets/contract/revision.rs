//! Conservative and typed widget revision descriptors.

use std::{any::Any, fmt, rc::Rc};

/// Declarative revision metadata for a [`Widget`](super::Widget).
///
/// `WidgetRevision::exact` accepts four independently typed values. Values are
/// compared by their `Eq` implementations and their concrete types must match;
/// no hashes or caller-provided integer fingerprints are involved. A mismatch
/// or unavailable value is intentionally treated as conservative by the
/// runtime classifier.
///
/// Exact revisions are UI-local and therefore use reference-counted erased
/// values. This preserves cloning and equality of revision snapshots, but the
/// older `Copy` implementation is intentionally not retained: copying an
/// exact revision would imply copying ownership of arbitrary user values.
#[derive(Clone)]
pub struct WidgetRevision {
    representation: Representation,
}

#[derive(Clone)]
enum Representation {
    Conservative,
    Exact(WidgetRevisionComponents),
}

impl Default for Representation {
    fn default() -> Self {
        Self::Conservative
    }
}

impl Default for WidgetRevision {
    fn default() -> Self {
        Self::conservative()
    }
}

impl fmt::Debug for WidgetRevision {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("WidgetRevision")
            .field(
                "representation",
                &match self.representation {
                    Representation::Conservative => "conservative",
                    Representation::Exact(_) => "exact",
                },
            )
            .finish()
    }
}

impl PartialEq for WidgetRevision {
    fn eq(&self, other: &Self) -> bool {
        match (&self.representation, &other.representation) {
            (Representation::Conservative, Representation::Conservative) => true,
            (Representation::Exact(previous), Representation::Exact(current)) => {
                previous == current
            }
            _ => false,
        }
    }
}

impl Eq for WidgetRevision {}

#[derive(Clone)]
pub(crate) struct WidgetRevisionComponents {
    structure: Rc<dyn RevisionValue>,
    geometry: Rc<dyn RevisionValue>,
    paint: Rc<dyn RevisionValue>,
    interaction: Rc<dyn RevisionValue>,
}

impl fmt::Debug for WidgetRevisionComponents {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("WidgetRevisionComponents(..)")
    }
}

impl PartialEq for WidgetRevisionComponents {
    fn eq(&self, other: &Self) -> bool {
        self.structure.equals(&*other.structure)
            && self.geometry.equals(&*other.geometry)
            && self.paint.equals(&*other.paint)
            && self.interaction.equals(&*other.interaction)
    }
}

impl Eq for WidgetRevisionComponents {}

impl WidgetRevisionComponents {
    pub(crate) fn structure_equal(&self, other: &Self) -> bool {
        self.structure.equals(&*other.structure)
    }

    pub(crate) fn geometry_equal(&self, other: &Self) -> bool {
        self.geometry.equals(&*other.geometry)
    }

    pub(crate) fn paint_equal(&self, other: &Self) -> bool {
        self.paint.equals(&*other.paint)
    }

    pub(crate) fn interaction_equal(&self, other: &Self) -> bool {
        self.interaction.equals(&*other.interaction)
    }
}

trait RevisionValue: Any {
    fn equals(&self, other: &dyn RevisionValue) -> bool;
}

impl<T> RevisionValue for T
where
    T: Eq + 'static,
{
    fn equals(&self, other: &dyn RevisionValue) -> bool {
        other
            .as_any()
            .downcast_ref::<T>()
            .is_some_and(|candidate| self == candidate)
    }
}

impl dyn RevisionValue {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl WidgetRevision {
    /// Return the safe fallback for widgets that cannot prove exact changes.
    #[must_use]
    pub const fn conservative() -> Self {
        Self {
            representation: Representation::Conservative,
        }
    }

    /// Build exact structure, geometry, paint, and interaction evidence.
    ///
    /// Each component is compared using its own `Eq` implementation. The
    /// component types are part of the evidence: changing a component's type
    /// is never treated as equal, even if the values happen to have a similar
    /// representation.
    #[must_use]
    pub fn exact<S, G, P, I>(structure: S, geometry: G, paint: P, interaction: I) -> Self
    where
        S: Eq + 'static,
        G: Eq + 'static,
        P: Eq + 'static,
        I: Eq + 'static,
    {
        Self {
            representation: Representation::Exact(WidgetRevisionComponents {
                structure: Rc::new(structure),
                geometry: Rc::new(geometry),
                paint: Rc::new(paint),
                interaction: Rc::new(interaction),
            }),
        }
    }

    pub(crate) fn exact_components(&self) -> Option<&WidgetRevisionComponents> {
        match &self.representation {
            Representation::Conservative => None,
            Representation::Exact(components) => Some(components),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::WidgetRevision;

    #[test]
    fn conservative_is_the_default_and_stable_fallback() {
        assert_eq!(WidgetRevision::default(), WidgetRevision::conservative());
        assert_eq!(
            WidgetRevision::conservative(),
            WidgetRevision::conservative()
        );
    }

    #[test]
    fn exact_values_compare_by_type_and_eq_without_fingerprints() {
        let first = WidgetRevision::exact("structure", 2_u32, false, vec![1_u8, 2]);
        let equal = WidgetRevision::exact("structure", 2_u32, false, vec![1_u8, 2]);
        let different_value = WidgetRevision::exact("structure", 3_u32, false, vec![1_u8, 2]);
        let different_type = WidgetRevision::exact("structure", 2_u64, false, vec![1_u8, 2]);

        assert_eq!(first, equal);
        assert_ne!(first, different_value);
        assert_ne!(first, different_type);
    }
}

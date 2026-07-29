//! Conservative, opaque widget revision descriptors.

#![allow(dead_code)]

/// Declarative revision metadata for a [`Widget`](super::Widget).
///
/// A revision is intentionally opaque. Custom widgets can opt into the
/// conservative default without exposing an invalidation hash or requiring
/// the runtime to trust an application-provided integer as proof of equality.
/// Exact component revisions remain an internal implementation detail until
/// their comparison and propagation contracts are ready for public use.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WidgetRevision {
    representation: Representation,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Representation {
    #[default]
    Conservative,
    Exact(WidgetRevisionComponents),
}

/// Exact component values used by the crate-local classifier fixtures.
///
/// This type is deliberately not public API. It gives the classifier a typed
/// relation to test without publishing arbitrary hash/u64 constructors for
/// application code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct WidgetRevisionComponents {
    pub(crate) structure: u64,
    pub(crate) geometry: u64,
    pub(crate) paint: u64,
    pub(crate) interaction: u64,
}

impl WidgetRevision {
    /// Return the safe fallback for widgets that cannot prove exact changes.
    #[must_use]
    pub const fn conservative() -> Self {
        Self {
            representation: Representation::Conservative,
        }
    }

    pub(crate) fn exact_components(&self) -> Option<WidgetRevisionComponents> {
        match self.representation {
            Representation::Conservative => None,
            Representation::Exact(components) => Some(components),
        }
    }

    #[cfg(test)]
    pub(crate) const fn exact_for_test(components: WidgetRevisionComponents) -> Self {
        Self {
            representation: Representation::Exact(components),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{WidgetRevision, WidgetRevisionComponents};

    #[test]
    fn conservative_is_the_default_and_stable_fallback() {
        assert_eq!(WidgetRevision::default(), WidgetRevision::conservative());
        assert_eq!(
            WidgetRevision::conservative(),
            WidgetRevision::conservative()
        );
    }

    #[test]
    fn exact_fixture_values_remain_internal() {
        let revision = WidgetRevision::exact_for_test(WidgetRevisionComponents {
            structure: 1,
            geometry: 2,
            paint: 3,
            interaction: 4,
        });
        assert_eq!(
            revision.exact_components(),
            Some(WidgetRevisionComponents {
                structure: 1,
                geometry: 2,
                paint: 3,
                interaction: 4,
            })
        );
    }
}

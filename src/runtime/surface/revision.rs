//! Pure widget revision relation used by future incremental reconciliation.

// This relation is deliberately staged without a production caller. Keep the
// pure foundation available for contract tests until incremental refresh owns
// its invocation.
#![allow(dead_code)]

use crate::widgets::{WidgetId, WidgetRevision, WidgetRevisionComponents};

/// The broadest safe effect of one retained widget revision comparison.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WidgetRevisionEffect {
    Structural,
    Geometry,
    Paint,
    Interaction,
    Unchanged,
}

/// Minimal comparison input for a same-ID retained widget pair.
#[derive(Clone, Copy, Debug)]
pub(crate) struct WidgetRevisionSnapshot {
    pub(crate) id: WidgetId,
    pub(crate) compatibility_kind: &'static str,
    pub(crate) revision: WidgetRevision,
}

/// Classify a widget pair without mutating or consulting runtime state.
///
/// Missing widgets, identity changes, compatibility changes, and conservative
/// revisions all take the structural/full fallback. Exact components are
/// compared in broadest-to-narrowest order so a multi-component change can
/// never be mistaken for a narrower effect.
pub(crate) fn classify_widget_revision(
    previous: Option<WidgetRevisionSnapshot>,
    current: Option<WidgetRevisionSnapshot>,
) -> WidgetRevisionEffect {
    let (Some(previous), Some(current)) = (previous, current) else {
        return WidgetRevisionEffect::Structural;
    };
    if previous.id != current.id || previous.compatibility_kind != current.compatibility_kind {
        return WidgetRevisionEffect::Structural;
    }

    let (Some(previous), Some(current)) = (
        previous.revision.exact_components(),
        current.revision.exact_components(),
    ) else {
        return WidgetRevisionEffect::Structural;
    };

    classify_exact_components(previous, current)
}

fn classify_exact_components(
    previous: WidgetRevisionComponents,
    current: WidgetRevisionComponents,
) -> WidgetRevisionEffect {
    if previous.structure != current.structure {
        WidgetRevisionEffect::Structural
    } else if previous.geometry != current.geometry {
        WidgetRevisionEffect::Geometry
    } else if previous.paint != current.paint {
        WidgetRevisionEffect::Paint
    } else if previous.interaction != current.interaction {
        WidgetRevisionEffect::Interaction
    } else {
        WidgetRevisionEffect::Unchanged
    }
}

#[cfg(test)]
mod tests {
    use super::{WidgetRevisionEffect, WidgetRevisionSnapshot, classify_widget_revision};
    use crate::widgets::{WidgetRevision, WidgetRevisionComponents};

    const KIND: &str = "test::Widget";

    fn snapshot(id: WidgetId, revision: WidgetRevision) -> WidgetRevisionSnapshot {
        WidgetRevisionSnapshot {
            id,
            compatibility_kind: KIND,
            revision,
        }
    }

    fn exact(structure: u64, geometry: u64, paint: u64, interaction: u64) -> WidgetRevision {
        WidgetRevision::exact_for_test(WidgetRevisionComponents {
            structure,
            geometry,
            paint,
            interaction,
        })
    }

    use crate::widgets::WidgetId;

    #[test]
    fn missing_identity_and_replacements_take_structural_fallback() {
        let current = Some(snapshot(7, exact(1, 1, 1, 1)));
        assert_eq!(
            classify_widget_revision(None, current),
            WidgetRevisionEffect::Structural
        );
        assert_eq!(
            classify_widget_revision(current, None),
            WidgetRevisionEffect::Structural
        );
        assert_eq!(
            classify_widget_revision(
                Some(snapshot(7, exact(1, 1, 1, 1))),
                Some(snapshot(8, exact(1, 1, 1, 1))),
            ),
            WidgetRevisionEffect::Structural
        );
        assert_eq!(
            classify_widget_revision(
                Some(snapshot(7, exact(1, 1, 1, 1))),
                Some(WidgetRevisionSnapshot {
                    id: 7,
                    compatibility_kind: "test::OtherWidget",
                    revision: exact(1, 1, 1, 1),
                }),
            ),
            WidgetRevisionEffect::Structural
        );
    }

    #[test]
    fn conservative_revision_always_takes_structural_fallback() {
        let exact = Some(snapshot(7, exact(1, 1, 1, 1)));
        let conservative = Some(snapshot(7, WidgetRevision::conservative()));
        assert_eq!(
            classify_widget_revision(exact, conservative),
            WidgetRevisionEffect::Structural
        );
        assert_eq!(
            classify_widget_revision(conservative, exact),
            WidgetRevisionEffect::Structural
        );
    }

    #[test]
    fn exact_components_choose_the_broadest_changed_effect() {
        let base = Some(snapshot(7, exact(1, 1, 1, 1)));
        for (revision, expected) in [
            (exact(2, 1, 1, 1), WidgetRevisionEffect::Structural),
            (exact(1, 2, 1, 1), WidgetRevisionEffect::Geometry),
            (exact(1, 1, 2, 1), WidgetRevisionEffect::Paint),
            (exact(1, 1, 1, 2), WidgetRevisionEffect::Interaction),
            (exact(1, 1, 1, 1), WidgetRevisionEffect::Unchanged),
        ] {
            assert_eq!(
                classify_widget_revision(base, Some(snapshot(7, revision))),
                expected
            );
        }
        assert_eq!(
            classify_widget_revision(base, Some(snapshot(7, exact(2, 2, 2, 2)))),
            WidgetRevisionEffect::Structural
        );
    }
}

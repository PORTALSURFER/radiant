//! Shared exact-evidence pieces for passive primitive revisions.

use super::WidgetCommon;
use crate::widgets::{PaintContract, WidgetRevision, WidgetStyle};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::widgets::primitives) struct ExactFloat(u32);

impl ExactFloat {
    pub(in crate::widgets::primitives) fn new(value: f32) -> Option<Self> {
        value.is_finite().then(|| Self(value.to_bits()))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::widgets::primitives) struct CommonGeometryRevision {
    pub(in crate::widgets::primitives) min: [ExactFloat; 2],
    pub(in crate::widgets::primitives) preferred: [ExactFloat; 2],
    pub(in crate::widgets::primitives) baseline: Option<ExactFloat>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::widgets::primitives) struct CommonPaintRevision {
    pub(in crate::widgets::primitives) paint: PaintContract,
    pub(in crate::widgets::primitives) style: WidgetStyle,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::widgets::primitives) struct CommonInteractionRevision {
    pub(in crate::widgets::primitives) focus: crate::widgets::FocusBehavior,
    pub(in crate::widgets::primitives) tooltip: Option<String>,
}

pub(in crate::widgets::primitives) fn common_geometry(
    common: &WidgetCommon,
) -> Option<CommonGeometryRevision> {
    Some(CommonGeometryRevision {
        min: [
            ExactFloat::new(common.sizing.min.x)?,
            ExactFloat::new(common.sizing.min.y)?,
        ],
        preferred: [
            ExactFloat::new(common.sizing.preferred.x)?,
            ExactFloat::new(common.sizing.preferred.y)?,
        ],
        baseline: match common.sizing.baseline {
            None => None,
            Some(value) => Some(ExactFloat::new(value)?),
        },
    })
}

pub(in crate::widgets::primitives) fn common_paint(common: &WidgetCommon) -> CommonPaintRevision {
    CommonPaintRevision {
        paint: common.paint,
        style: common.style,
    }
}

pub(in crate::widgets::primitives) fn common_interaction(
    common: &WidgetCommon,
) -> CommonInteractionRevision {
    CommonInteractionRevision {
        focus: common.focus,
        tooltip: common.tooltip.clone(),
    }
}

pub(in crate::widgets::primitives) fn exact_revision<G, P>(
    geometry: Option<G>,
    paint: P,
    interaction: CommonInteractionRevision,
) -> Option<WidgetRevision>
where
    G: Eq + 'static,
    P: Eq + 'static,
{
    Some(WidgetRevision::exact((), geometry?, paint, interaction))
}

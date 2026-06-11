use crate::gui::range::{
    NormalizedRange, normalized_fraction_to_micros, normalized_fraction_to_milli,
};

/// Editable range and fade handles for a normalized timeline or signal view.
///
/// The structure is deliberately host-neutral: it models a selected interval,
/// optional leading/trailing handle positions, and optional curve controls.
/// Hosts decide whether those controls represent animation ramps, trim previews,
/// easing handles, or other domain behavior.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct TimelineEditPreview {
    /// Range currently being edited.
    pub selection: Option<NormalizedRange>,
    /// End position for the leading/top handle in normalized milli-units.
    pub leading_end_milli: Option<u16>,
    /// End position for the leading/top handle in normalized micro-units.
    pub leading_end_micros: Option<u32>,
    /// Start position for the leading/bottom handle in normalized milli-units.
    pub leading_inner_start_milli: Option<u16>,
    /// Start position for the leading/bottom handle in normalized micro-units.
    pub leading_inner_start_micros: Option<u32>,
    /// Leading curve tension in normalized milli-units.
    pub leading_curve_milli: Option<u16>,
    /// Start position for the trailing/top handle in normalized milli-units.
    pub trailing_start_milli: Option<u16>,
    /// Start position for the trailing/top handle in normalized micro-units.
    pub trailing_start_micros: Option<u32>,
    /// End position for the trailing/bottom handle in normalized milli-units.
    pub trailing_inner_end_milli: Option<u16>,
    /// End position for the trailing/bottom handle in normalized micro-units.
    pub trailing_inner_end_micros: Option<u32>,
    /// Trailing curve tension in normalized milli-units.
    pub trailing_curve_milli: Option<u16>,
}

/// Standard edit-preview handles for a normalized timeline or signal surface.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TimelineEditHandle {
    /// Leading/top handle at the leading ramp end.
    LeadingEnd,
    /// Leading/bottom handle at the selected range start.
    LeadingStart,
    /// Leading outer handle before the selected range.
    LeadingOuterStart,
    /// Trailing/top handle at the trailing ramp start.
    TrailingStart,
    /// Trailing/bottom handle at the selected range end.
    TrailingEnd,
    /// Trailing outer handle after the selected range.
    TrailingOuterEnd,
}

impl TimelineEditHandle {
    /// Return the standard hit-test and paint order for timeline edit handles.
    ///
    /// Inner ramp handles are checked before selection-edge handles, and outer
    /// handles are checked last. This matches the default visual priority for
    /// compact timeline and signal editors while still letting hosts supply a
    /// custom order to [`TimelineEditPreview::handle_at`] when needed.
    pub const fn standard_order() -> [Self; 6] {
        [
            Self::LeadingEnd,
            Self::TrailingStart,
            Self::LeadingStart,
            Self::TrailingEnd,
            Self::LeadingOuterStart,
            Self::TrailingOuterEnd,
        ]
    }
}

/// Standard editable regions around a selected timeline interval.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TimelineEditRegion {
    /// Leading region inside the selected interval.
    LeadingInner,
    /// Leading region before the selected interval.
    LeadingOuter,
    /// Trailing region inside the selected interval.
    TrailingInner,
    /// Trailing region after the selected interval.
    TrailingOuter,
}

/// Standard editable ramp side for a normalized timeline interval.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TimelineEditRampSide {
    /// Leading ramp before or at the selected interval start.
    Leading,
    /// Trailing ramp at or after the selected interval end.
    Trailing,
}

impl TimelineEditRegion {
    /// Return the standard paint order for timeline edit regions.
    pub const fn standard_order() -> [Self; 4] {
        [
            Self::LeadingInner,
            Self::TrailingInner,
            Self::LeadingOuter,
            Self::TrailingOuter,
        ]
    }
}

/// Named edit-preview parts for timeline handle projection.
///
/// Hosts can fill only the handles they need while keeping range, leading
/// handles, trailing handles, and curve controls readable at call sites.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct TimelineEditPreviewParts {
    /// Range currently being edited.
    pub selection: Option<NormalizedRange>,
    /// End position for the leading/top handle in normalized milli-units.
    pub leading_end_milli: Option<u16>,
    /// End position for the leading/top handle in normalized micro-units.
    pub leading_end_micros: Option<u32>,
    /// Start position for the leading/bottom handle in normalized milli-units.
    pub leading_inner_start_milli: Option<u16>,
    /// Start position for the leading/bottom handle in normalized micro-units.
    pub leading_inner_start_micros: Option<u32>,
    /// Leading curve tension in normalized milli-units.
    pub leading_curve_milli: Option<u16>,
    /// Start position for the trailing/top handle in normalized milli-units.
    pub trailing_start_milli: Option<u16>,
    /// Start position for the trailing/top handle in normalized micro-units.
    pub trailing_start_micros: Option<u32>,
    /// End position for the trailing/bottom handle in normalized milli-units.
    pub trailing_inner_end_milli: Option<u16>,
    /// End position for the trailing/bottom handle in normalized micro-units.
    pub trailing_inner_end_micros: Option<u32>,
    /// Trailing curve tension in normalized milli-units.
    pub trailing_curve_milli: Option<u16>,
}

/// Optional ramp projection for a normalized timeline edit preview.
///
/// A ramp is deliberately domain-neutral: it may represent an audio fade, an
/// animation easing segment, an opacity transition, a trim preview, or any
/// other leading/trailing edit affordance attached to a selected interval.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct TimelineEditRamp {
    /// Ramp length as a fraction of the selected interval width.
    pub length_fraction: f32,
    /// Optional extension outside the selected interval as a fraction of the
    /// selected interval width.
    pub outer_fraction: f32,
    /// Optional curve/control value in normalized `0.0..=1.0` space.
    pub curve_fraction: Option<f32>,
}

impl TimelineEditRamp {
    /// Build a ramp from normalized length, outer extension, and optional curve.
    pub const fn new(
        length_fraction: f32,
        outer_fraction: f32,
        curve_fraction: Option<f32>,
    ) -> Self {
        Self {
            length_fraction,
            outer_fraction,
            curve_fraction,
        }
    }

    /// Build a ramp with no outer extension.
    pub const fn from_length(length_fraction: f32, curve_fraction: Option<f32>) -> Self {
        Self::new(length_fraction, 0.0, curve_fraction)
    }
}

impl TimelineEditPreview {
    /// Build an edit preview from named handle parts.
    pub fn from_parts(parts: TimelineEditPreviewParts) -> Self {
        Self {
            selection: parts.selection,
            leading_end_milli: parts.leading_end_milli,
            leading_end_micros: parts.leading_end_micros,
            leading_inner_start_milli: parts.leading_inner_start_milli,
            leading_inner_start_micros: parts.leading_inner_start_micros,
            leading_curve_milli: parts.leading_curve_milli,
            trailing_start_milli: parts.trailing_start_milli,
            trailing_start_micros: parts.trailing_start_micros,
            trailing_inner_end_milli: parts.trailing_inner_end_milli,
            trailing_inner_end_micros: parts.trailing_inner_end_micros,
            trailing_curve_milli: parts.trailing_curve_milli,
        }
    }

    /// Build an edit preview from a selected range and optional normalized ramps.
    ///
    /// The selected range supplies the durable timeline interval. Ramp lengths
    /// and outer extensions are fractions of that interval width, so hosts can
    /// project domain data into standard leading/trailing edit handles without
    /// duplicating milli/micro conversion and endpoint math.
    pub fn from_normalized_ramps(
        selection: NormalizedRange,
        leading: Option<TimelineEditRamp>,
        trailing: Option<TimelineEditRamp>,
    ) -> Self {
        let start = selection.start_fraction();
        let end = selection.end_fraction();
        let width = selection.width_fraction();
        Self::from_parts(TimelineEditPreviewParts {
            selection: Some(selection),
            leading_end_milli: leading
                .map(|ramp| normalized_fraction_to_milli(start + width * ramp.length_fraction)),
            leading_end_micros: leading
                .map(|ramp| normalized_fraction_to_micros(start + width * ramp.length_fraction)),
            leading_inner_start_milli: leading
                .map(|ramp| normalized_fraction_to_milli(start - width * ramp.outer_fraction)),
            leading_inner_start_micros: leading
                .map(|ramp| normalized_fraction_to_micros(start - width * ramp.outer_fraction)),
            leading_curve_milli: leading
                .and_then(|ramp| ramp.curve_fraction.map(normalized_fraction_to_milli)),
            trailing_start_milli: trailing
                .map(|ramp| normalized_fraction_to_milli(end - width * ramp.length_fraction)),
            trailing_start_micros: trailing
                .map(|ramp| normalized_fraction_to_micros(end - width * ramp.length_fraction)),
            trailing_inner_end_milli: trailing
                .map(|ramp| normalized_fraction_to_milli(end + width * ramp.outer_fraction)),
            trailing_inner_end_micros: trailing
                .map(|ramp| normalized_fraction_to_micros(end + width * ramp.outer_fraction)),
            trailing_curve_milli: trailing
                .and_then(|ramp| ramp.curve_fraction.map(normalized_fraction_to_milli)),
        })
    }

    /// Return the normalized micro-position for a standard edit handle.
    pub fn handle_micros(self, handle: TimelineEditHandle) -> Option<u32> {
        let selection = self.selection?;
        match handle {
            TimelineEditHandle::LeadingEnd => {
                Some(self.leading_end_micros.unwrap_or(selection.start_micros))
            }
            TimelineEditHandle::LeadingStart => {
                self.leading_end_micros.map(|_| selection.start_micros)
            }
            TimelineEditHandle::LeadingOuterStart => self.leading_end_micros.and(
                self.leading_inner_start_micros
                    .or(Some(selection.start_micros)),
            ),
            TimelineEditHandle::TrailingStart => {
                Some(self.trailing_start_micros.unwrap_or(selection.end_micros))
            }
            TimelineEditHandle::TrailingEnd => {
                self.trailing_start_micros.map(|_| selection.end_micros)
            }
            TimelineEditHandle::TrailingOuterEnd => self.trailing_start_micros.and(
                self.trailing_inner_end_micros
                    .or(Some(selection.end_micros)),
            ),
        }
    }

    pub(super) fn standard_ramp_curve_spans(
        self,
    ) -> impl Iterator<Item = (TimelineEditRampSide, u32, u32)> {
        let leading = self.leading_end_micros.map(|end| {
            (
                TimelineEditRampSide::Leading,
                self.leading_inner_start_micros
                    .or(self.selection.map(|selection| selection.start_micros))
                    .unwrap_or(end),
                end,
            )
        });
        let trailing = self.trailing_start_micros.map(|start| {
            (
                TimelineEditRampSide::Trailing,
                start,
                self.trailing_inner_end_micros
                    .or(self.selection.map(|selection| selection.end_micros))
                    .unwrap_or(start),
            )
        });
        [leading, trailing]
            .into_iter()
            .flatten()
            .filter(|(_, start, end)| end > start)
    }
}

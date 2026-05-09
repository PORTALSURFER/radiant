use crate::gui::range::NormalizedRange;

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

impl TimelineEditPreview {
    /// Build an edit preview with all handle positions supplied explicitly.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        selection: Option<NormalizedRange>,
        leading_end_milli: Option<u16>,
        leading_end_micros: Option<u32>,
        leading_inner_start_milli: Option<u16>,
        leading_inner_start_micros: Option<u32>,
        leading_curve_milli: Option<u16>,
        trailing_start_milli: Option<u16>,
        trailing_start_micros: Option<u32>,
        trailing_inner_end_milli: Option<u16>,
        trailing_inner_end_micros: Option<u32>,
        trailing_curve_milli: Option<u16>,
    ) -> Self {
        Self {
            selection,
            leading_end_milli,
            leading_end_micros,
            leading_inner_start_milli,
            leading_inner_start_micros,
            leading_curve_milli,
            trailing_start_milli,
            trailing_start_micros,
            trailing_inner_end_milli,
            trailing_inner_end_micros,
            trailing_curve_milli,
        }
    }
}

/// Explicit parts used to build timeline presentation metadata.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct TimelinePresentationParts {
    /// Optional guide spacing in normalized micro-units.
    pub guide_step_micros: Option<u32>,
    /// Guide origin in normalized micro-units.
    pub guide_origin_micros: u32,
    /// Whether repeat playback/review behavior is enabled.
    pub repeat_enabled: bool,
    /// Optional primary metadata label.
    pub primary_label: Option<String>,
    /// Optional viewport/zoom metadata label.
    pub viewport_label: Option<String>,
}

/// Presentation metadata for a normalized timeline.
///
/// This covers renderer-facing timeline guides, repeat state, and compact
/// labels without tying the primitive to any host domain concept.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct TimelinePresentationState {
    /// Optional guide spacing in normalized micro-units.
    pub guide_step_micros: Option<u32>,
    /// Guide origin in normalized micro-units.
    pub guide_origin_micros: u32,
    /// Whether repeat playback/review behavior is enabled.
    pub repeat_enabled: bool,
    /// Optional primary metadata label.
    pub primary_label: Option<String>,
    /// Optional viewport/zoom metadata label.
    pub viewport_label: Option<String>,
}

impl TimelinePresentationState {
    /// Build timeline presentation state from named guide and label values.
    pub fn from_parts(parts: TimelinePresentationParts) -> Self {
        Self {
            guide_step_micros: parts.guide_step_micros,
            guide_origin_micros: parts.guide_origin_micros,
            repeat_enabled: parts.repeat_enabled,
            primary_label: parts.primary_label,
            viewport_label: parts.viewport_label,
        }
    }

    /// Build timeline presentation state from explicit guide and label values.
    pub fn new(
        guide_step_micros: Option<u32>,
        guide_origin_micros: u32,
        repeat_enabled: bool,
        primary_label: Option<String>,
        viewport_label: Option<String>,
    ) -> Self {
        Self::from_parts(TimelinePresentationParts {
            guide_step_micros,
            guide_origin_micros,
            repeat_enabled,
            primary_label,
            viewport_label,
        })
    }
}

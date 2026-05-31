/// Approximate text-width inputs for deterministic layout before renderer
/// shaping metrics are available.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextWidthEstimate {
    /// Average logical width reserved for one displayed character.
    pub character_advance: f32,
    /// Extra horizontal logical width reserved around the text.
    pub horizontal_padding: f32,
}

impl TextWidthEstimate {
    /// Construct text-width estimate metrics from already-resolved tokens.
    pub fn new(character_advance: f32, horizontal_padding: f32) -> Self {
        Self {
            character_advance,
            horizontal_padding,
        }
    }

    /// Construct estimate metrics from a font size and average advance factor.
    pub fn from_font_size(
        font_size: f32,
        average_advance_factor: f32,
        horizontal_padding: f32,
    ) -> Self {
        Self::new(font_size * average_advance_factor, horizontal_padding)
    }
}

/// Approximate text width for a displayed string plus configured padding.
pub fn estimated_text_width(text: &str, metrics: TextWidthEstimate) -> f32 {
    estimated_text_width_for_char_count(text.chars().count(), metrics)
}

/// Approximate text width for a known displayed character count plus padding.
pub fn estimated_text_width_for_char_count(char_count: usize, metrics: TextWidthEstimate) -> f32 {
    let advance = finite_nonnegative_width(metrics.character_advance);
    let padding = finite_nonnegative_width(metrics.horizontal_padding);
    ((char_count as f32) * advance).ceil() + padding
}

/// Approximate text width clamped to a caller-defined logical-width range.
pub fn estimated_text_width_in_range(
    text: &str,
    metrics: TextWidthEstimate,
    min_width: f32,
    max_width: f32,
) -> f32 {
    estimated_text_width_for_char_count_in_range(
        text.chars().count(),
        metrics,
        min_width,
        max_width,
    )
}

/// Approximate text width for a known character count, clamped to a range.
pub fn estimated_text_width_for_char_count_in_range(
    char_count: usize,
    metrics: TextWidthEstimate,
    min_width: f32,
    max_width: f32,
) -> f32 {
    let min_width = finite_nonnegative_width(min_width);
    let max_width = finite_nonnegative_width(max_width).max(min_width);
    estimated_text_width_for_char_count(char_count, metrics).clamp(min_width, max_width)
}

pub(in crate::gui) fn finite_nonnegative_width(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

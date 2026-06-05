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

/// Deterministic width policy for inline single-line text inputs.
///
/// Token, recipient, filter, and chip editors often size an input from the
/// draft value plus optional inline completion text while still reserving room
/// for a placeholder or minimum visible character count. This policy keeps that
/// sizing logic reusable without requiring hosts to allocate temporary joined
/// strings before renderer text metrics are available.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextInputWidthPolicy {
    /// Approximate text-width inputs used before renderer shaping metrics exist.
    pub metrics: TextWidthEstimate,
    /// Minimum logical width reserved for the input.
    pub min_width: f32,
    /// Maximum logical width reserved for the input.
    pub max_width: f32,
    /// Minimum visible character count reserved before clamping to the width range.
    pub min_visible_chars: usize,
}

impl TextInputWidthPolicy {
    /// Construct a text-input width policy with no additional minimum visible
    /// character count beyond `min_width`.
    pub fn new(metrics: TextWidthEstimate, min_width: f32, max_width: f32) -> Self {
        Self {
            metrics,
            min_width,
            max_width,
            min_visible_chars: 0,
        }
    }

    /// Reserve space for at least `min_visible_chars` before clamping to the
    /// configured width range.
    pub fn with_min_visible_chars(mut self, min_visible_chars: usize) -> Self {
        self.min_visible_chars = min_visible_chars;
        self
    }

    /// Approximate input width for a known visible character count.
    pub fn width_for_char_count(self, char_count: usize) -> f32 {
        estimated_text_width_for_char_count_in_range(
            char_count.max(self.min_visible_chars),
            self.metrics,
            self.min_width,
            self.max_width,
        )
    }

    /// Approximate input width for a draft value.
    pub fn width_for_value(self, value: &str) -> f32 {
        self.width_for_char_count(value.chars().count())
    }

    /// Approximate input width for a draft value plus an optional inline
    /// completion suffix. Empty suffixes are ignored.
    pub fn width_for_value_and_completion_suffix(
        self,
        value: &str,
        completion_suffix: Option<&str>,
    ) -> f32 {
        self.width_for_char_count(value_completion_char_count(value, completion_suffix))
    }

    /// Approximate input width for a draft value plus an optional inline
    /// completion suffix, reserving at least enough room for `placeholder`.
    pub fn width_for_value_completion_or_placeholder(
        self,
        value: &str,
        completion_suffix: Option<&str>,
        placeholder: &str,
    ) -> f32 {
        self.width_for_char_count(
            value_completion_char_count(value, completion_suffix).max(placeholder.chars().count()),
        )
    }
}

/// Approximate text width for a displayed string plus configured padding.
pub fn estimated_text_width(text: &str, metrics: TextWidthEstimate) -> f32 {
    estimated_text_width_for_char_count(text.chars().count(), metrics)
}

/// Approximate text width for several displayed text segments plus configured
/// padding, without requiring callers to allocate a joined string.
pub fn estimated_text_width_for_segments<'a>(
    segments: impl IntoIterator<Item = &'a str>,
    metrics: TextWidthEstimate,
) -> f32 {
    estimated_text_width_for_char_count(text_segments_char_count(segments), metrics)
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

/// Approximate text width for several displayed text segments, clamped to a
/// caller-defined logical-width range.
pub fn estimated_text_width_for_segments_in_range<'a>(
    segments: impl IntoIterator<Item = &'a str>,
    metrics: TextWidthEstimate,
    min_width: f32,
    max_width: f32,
) -> f32 {
    estimated_text_width_for_char_count_in_range(
        text_segments_char_count(segments),
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

fn text_segments_char_count<'a>(segments: impl IntoIterator<Item = &'a str>) -> usize {
    segments
        .into_iter()
        .map(|segment| segment.chars().count())
        .sum()
}

fn value_completion_char_count(value: &str, completion_suffix: Option<&str>) -> usize {
    value.chars().count()
        + completion_suffix
            .filter(|suffix| !suffix.is_empty())
            .map(str::chars)
            .map(Iterator::count)
            .unwrap_or(0)
}

pub(in crate::gui) fn finite_nonnegative_width(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

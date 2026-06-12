use super::super::{PaintPrimitive, PaintTextRun, SurfacePaintPlan};
use crate::gui::types::{Rect, Rgba8};

impl SurfacePaintPlan {
    /// Iterate over text runs emitted by this paint plan in paint order.
    pub fn text_runs(&self) -> impl Iterator<Item = &PaintTextRun> {
        self.primitives.iter().filter_map(PaintPrimitive::text_run)
    }

    /// Iterate over visible text labels emitted by this paint plan in paint order.
    pub fn text_labels(&self) -> impl Iterator<Item = &str> {
        self.text_runs().map(|run| run.text.as_str())
    }

    /// Collect visible text labels emitted by this paint plan in paint order.
    ///
    /// Use this in tests, automation snapshots, or diagnostics that need owned
    /// labels for failure output without repeating text-run mapping boilerplate.
    pub fn text_label_strings(&self) -> Vec<String> {
        self.text_labels().map(str::to_string).collect()
    }

    /// Return the first text run with exactly matching visible text.
    pub fn first_text_run(&self, text: &str) -> Option<&PaintTextRun> {
        self.text_runs().find(|run| run.text.as_str() == text)
    }

    /// Return the first text run with exactly matching visible text whose
    /// rectangle begins at or after `min_x`.
    pub fn first_text_run_after_x(&self, text: &str, min_x: f32) -> Option<&PaintTextRun> {
        self.text_runs()
            .find(|run| run.text.as_str() == text && run.rect.min.x >= min_x)
    }

    /// Return whether this paint plan contains a text run with exactly matching
    /// visible text.
    pub fn contains_text(&self, text: &str) -> bool {
        self.first_text_run(text).is_some()
    }

    /// Return whether this paint plan contains exactly matching visible text
    /// whose rectangle begins at or after `min_x`.
    pub fn contains_text_after_x(&self, text: &str, min_x: f32) -> bool {
        self.first_text_run_after_x(text, min_x).is_some()
    }

    /// Return the rectangle for the first text run with exactly matching
    /// visible text.
    pub fn first_text_rect(&self, text: &str) -> Option<Rect> {
        self.first_text_run(text).map(|run| run.rect)
    }

    /// Return the color for the first text run with exactly matching visible
    /// text.
    pub fn first_text_color(&self, text: &str) -> Option<Rgba8> {
        self.first_text_run(text).map(|run| run.color)
    }
}

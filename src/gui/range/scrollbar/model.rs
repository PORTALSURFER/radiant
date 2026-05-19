use crate::gui::types::Rect;

/// Request used to resolve one horizontal normalized-range scrollbar.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NormalizedScrollbarRequest {
    /// Scrollbar track in window coordinates.
    pub track: Rect,
    /// Visible normalized start in micro-units (`0..=1_000_000`).
    pub start_micros: u32,
    /// Visible normalized end in micro-units (`0..=1_000_000`).
    pub end_micros: u32,
    /// Minimum thumb width in pixels.
    pub min_thumb_width: f32,
}

/// Resolved horizontal normalized-range scrollbar geometry.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NormalizedScrollbar {
    /// Scrollbar track in window coordinates.
    pub track: Rect,
    /// Scrollbar thumb in window coordinates.
    pub thumb: Rect,
}

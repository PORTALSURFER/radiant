//! Public model types for retained GPU-surface content.

/// Optional GPU-side gain envelope for retained signal rendering.
///
/// The preview is intentionally normalized and backend-neutral: hosts can
/// preview destructive fade/gain edits without rebuilding or re-uploading the
/// retained signal payload on each pointer update.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GpuSignalGainPreview {
    /// Normalized selection start.
    pub start: f32,
    /// Normalized selection end.
    pub end: f32,
    /// Gain applied inside the selection after fades.
    pub gain: f32,
    /// Fade-in length as a fraction of the selection width.
    pub fade_in_length: f32,
    /// Fade-in curve tension.
    pub fade_in_curve: f32,
    /// Fade-in outer extension length as a fraction of the selection width.
    ///
    /// Kept under the historical "mute" field name for API compatibility.
    pub fade_in_mute: f32,
    /// Fade-out length as a fraction of the selection width.
    pub fade_out_length: f32,
    /// Fade-out curve tension.
    pub fade_out_curve: f32,
    /// Fade-out outer extension length as a fraction of the selection width.
    ///
    /// Kept under the historical "mute" field name for API compatibility.
    pub fade_out_mute: f32,
}

/// Renderable shape resolved from a retained GPU signal payload.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GpuSignalRenderShape {
    /// Effective source frame count available to the renderer.
    pub frames: usize,
    /// Number of interleaved bands per frame.
    pub band_count: usize,
    /// Visible frame range as start/end frame offsets.
    pub frame_range: [f32; 2],
    /// Number of source sample or summary bucket entries.
    pub sample_count: usize,
}

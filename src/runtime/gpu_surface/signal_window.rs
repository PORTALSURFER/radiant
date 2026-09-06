//! Bounded, exact-origin summary windows for the custom-shader surface ABI.

use super::{
    GpuShaderSurfaceDescriptor, GpuShaderSurfaceDescriptorParts, GpuSignalSummaryBucket,
    GpuSignalViewport, GpuSignalViewportError, GpuSurfaceContent,
};
use std::{
    fmt,
    sync::{Arc, OnceLock},
};

/// Maximum number of bucket-major frames in a precise summary window.
pub const MAX_PRECISE_SIGNAL_BUCKETS: usize = 1 << 20;

/// Immutable bounded summary data with an exact source-frame origin.
///
/// Buckets begin at `first_frame`, with the final bucket optionally truncated at
/// the source end. Construction encodes at most 32 MiB of immutable min/max data.
/// Clones and presentation changes share that allocation.
#[derive(Clone, Debug)]
pub struct GpuSignalSummaryWindow {
    source_frames: u64,
    first_frame: u64,
    end_frame: u64,
    bucket_frames: u32,
    bucket_count: u32,
    band_count: u32,
    storage: Arc<[u8]>,
    uniforms: Arc<[u8]>,
    identity: u64,
    revision: u64,
}

/// Exact selection and gain/fade controls for a precise signal presentation.
///
/// Fade lengths and outer extensions are fractions of the selection span.
/// Selection positions are measured on the visual timeline before slide.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GpuPreciseSignalGainPreview {
    /// Exact visual-timeline selection before slide.
    pub selection: GpuSignalViewport,
    /// Non-negative gain inside the selection.
    pub gain: f32,
    /// Fade-in length as a selection fraction in [0, 1].
    pub fade_in_length: f32,
    /// Fade-in curve tension in [0, 1].
    pub fade_in_curve: f32,
    /// Non-negative outer fade-in extension as a selection fraction.
    pub fade_in_extension: f32,
    /// Gain at the outer fade-in edge in [0, 1].
    pub fade_in_outer_gain: f32,
    /// Fade-out length as a selection fraction in [0, 1].
    pub fade_out_length: f32,
    /// Fade-out curve tension in [0, 1].
    pub fade_out_curve: f32,
    /// Non-negative outer fade-out extension as a selection fraction.
    pub fade_out_extension: f32,
    /// Gain at the outer fade-out edge in [0, 1].
    pub fade_out_outer_gain: f32,
}

impl GpuPreciseSignalGainPreview {
    /// A unity gain selection with no fades or extensions.
    pub const fn new(selection: GpuSignalViewport) -> Self {
        Self {
            selection,
            gain: 1.0,
            fade_in_length: 0.0,
            fade_in_curve: 0.0,
            fade_in_extension: 0.0,
            fade_in_outer_gain: 1.0,
            fade_out_length: 0.0,
            fade_out_curve: 0.0,
            fade_out_extension: 0.0,
            fade_out_outer_gain: 1.0,
        }
    }
}

/// Volatile view state, independent of immutable summary storage identity.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GpuPreciseSignalPresentation {
    /// Exact visible source interval before slide.
    pub viewport: GpuSignalViewport,
    /// Signed whole-frame slide; sampling uses `(visual_frame - slide) mod source_frames`.
    pub slide_frames: i64,
    /// Optional precise selection gain and fades.
    pub gain_preview: Option<GpuPreciseSignalGainPreview>,
    /// Optional normalized cursor position in `[0, 1]`.
    pub cursor_ratio: Option<f32>,
    /// Cursor width in physical pixels.
    pub cursor_width: f32,
    /// Straight RGBA cursor color with each component in [0, 1].
    pub cursor_color: [f32; 4],
    /// Monotonic revision for the volatile presentation bytes.
    pub revision: u64,
}

impl GpuPreciseSignalPresentation {
    /// Construct a view without slide, gain preview, or cursor.
    pub const fn new(viewport: GpuSignalViewport) -> Self {
        Self {
            viewport,
            slide_frames: 0,
            gain_preview: None,
            cursor_ratio: None,
            cursor_width: 1.0,
            cursor_color: [1.0; 4],
            revision: 0,
        }
    }
}

/// A malformed precise window or a presentation that needs different data.
#[derive(Clone, Debug, PartialEq)]
pub enum GpuSignalWindowError {
    /// Zero source/bucket size, unsupported band count, or misaligned bucket data.
    InvalidShape,
    /// A bucket has non-finite or inverted extrema.
    InvalidBucket,
    /// The window exceeds the documented bucket limit.
    CapacityExceeded,
    /// A bucket begins at or beyond the declared source end.
    SourceRangeOverflow,
    /// The visible interval extends beyond the declared source.
    ViewportOutsideSource,
    /// The supplied immutable window does not cover the requested slid range.
    /// Request the required source window before trying this presentation again.
    MissingWindow,
    /// Non-finite, out-of-range, or collapsed presentation geometry.
    InvalidPresentation,
    /// An exact coordinate operation failed.
    Coordinate(GpuSignalViewportError),
}

impl fmt::Display for GpuSignalWindowError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidShape => f.write_str("invalid precise signal window shape"),
            Self::InvalidBucket => {
                f.write_str("signal buckets require finite ordered min/max values")
            }
            Self::CapacityExceeded => {
                f.write_str("precise signal window exceeds the bounded bucket capacity")
            }
            Self::SourceRangeOverflow => f.write_str("signal window is outside its source range"),
            Self::ViewportOutsideSource => f.write_str("signal viewport is outside its source"),
            Self::MissingWindow => {
                f.write_str("signal presentation requires summary data outside this window")
            }
            Self::InvalidPresentation => f.write_str("invalid precise signal presentation"),
            Self::Coordinate(error) => error.fmt(f),
        }
    }
}
impl std::error::Error for GpuSignalWindowError {}
impl From<GpuSignalViewportError> for GpuSignalWindowError {
    fn from(error: GpuSignalViewportError) -> Self {
        Self::Coordinate(error)
    }
}

impl GpuSignalSummaryWindow {
    /// Encode a bounded bucket window. Identity and revision describe immutable data;
    /// use a new revision when replacing its contents or source metadata.
    pub fn new(
        source_frames: u64,
        first_frame: u64,
        bucket_frames: u32,
        band_count: u32,
        buckets: &[GpuSignalSummaryBucket],
        identity: u64,
        revision: u64,
    ) -> Result<Self, GpuSignalWindowError> {
        if source_frames == 0
            || first_frame >= source_frames
            || bucket_frames == 0
            || !(1..=4).contains(&band_count)
            || buckets.is_empty()
            || !buckets.len().is_multiple_of(band_count as usize)
        {
            return Err(GpuSignalWindowError::InvalidShape);
        }
        let count = buckets.len() / band_count as usize;
        if count > MAX_PRECISE_SIGNAL_BUCKETS {
            return Err(GpuSignalWindowError::CapacityExceeded);
        }
        let nominal_end = u128::from(first_frame) + count as u128 * u128::from(bucket_frames);
        let last_start = nominal_end - u128::from(bucket_frames);
        if last_start >= u128::from(source_frames) {
            return Err(GpuSignalWindowError::SourceRangeOverflow);
        }
        let end_frame = nominal_end.min(u128::from(source_frames)) as u64;
        let mut storage = Vec::with_capacity(buckets.len() * 8);
        for bucket in buckets {
            if !bucket.min.is_finite() || !bucket.max.is_finite() || bucket.min > bucket.max {
                return Err(GpuSignalWindowError::InvalidBucket);
            }
            storage.extend_from_slice(&bucket.min.to_le_bytes());
            storage.extend_from_slice(&bucket.max.to_le_bytes());
        }
        let uniforms: Vec<u8> = [count as u32, band_count, bucket_frames, 0]
            .into_iter()
            .flat_map(u32::to_le_bytes)
            .collect();
        Ok(Self {
            source_frames,
            first_frame,
            end_frame,
            bucket_frames,
            bucket_count: count as u32,
            band_count,
            storage: storage.into(),
            uniforms: uniforms.into(),
            identity,
            revision,
        })
    }

    /// Total source length in frames.
    pub const fn source_frames(&self) -> u64 {
        self.source_frames
    }
    /// Exact origin of the first supplied bucket.
    pub const fn first_frame(&self) -> u64 {
        self.first_frame
    }
    /// Exclusive source-frame end covered by the supplied buckets.
    pub const fn end_frame(&self) -> u64 {
        self.end_frame
    }
    /// Number of source frames per bucket.
    pub const fn bucket_frames(&self) -> u32 {
        self.bucket_frames
    }
    /// Number of bucket-major frames retained.
    pub const fn bucket_count(&self) -> u32 {
        self.bucket_count
    }
    /// Number of interleaved bands per bucket.
    pub const fn band_count(&self) -> u32 {
        self.band_count
    }

    /// Build existing custom-shader content without adding a content enum variant.
    /// View changes share the immutable bytes and update only binding 3.
    pub fn content(
        &self,
        presentation: &GpuPreciseSignalPresentation,
    ) -> Result<GpuSurfaceContent, GpuSignalWindowError> {
        let bytes = self.presentation_bytes(presentation)?;
        Ok(GpuSurfaceContent::CustomShader {
            descriptor: GpuShaderSurfaceDescriptor::from_parts(GpuShaderSurfaceDescriptorParts {
                shader_key: "radiant.precise-signal.v1".into(),
                wgsl_source: Some(precise_shader()),
                entry_point: "vs_main".into(),
                fragment_entry_point: Some("fs_main".into()),
                uniform_bytes: Arc::clone(&self.uniforms),
                storage_bytes: Arc::clone(&self.storage),
                storage_identity: self.identity,
                storage_revision: self.revision,
                presentation_uniform_bytes: Some(bytes),
                presentation_uniform_revision: Some(presentation.revision),
                vertex_count: 6,
            })
            .into(),
        })
    }

    /// Encode volatile binding-3 bytes for the existing presentation update API.
    pub fn presentation_bytes(
        &self,
        p: &GpuPreciseSignalPresentation,
    ) -> Result<Arc<[u8]>, GpuSignalWindowError> {
        let end = p.viewport.end()?;
        if end.frame() > self.source_frames
            || (end.frame() == self.source_frames && end.fraction() > 0.0)
        {
            return Err(GpuSignalWindowError::ViewportOutsideSource);
        }
        let start = p.viewport.start();
        let slid_frame = (i128::from(start.frame()) - i128::from(p.slide_frames))
            .rem_euclid(i128::from(self.source_frames)) as u64;
        let remaining = self.source_frames - slid_frame;
        let span = p.viewport.span();
        // Subtract exact integers first; the converted delta is bounded by the window.
        let wraps = span > remaining as f64 - start.fraction();
        if slid_frame < self.first_frame
            || slid_frame >= self.end_frame
            || (wraps && (self.first_frame != 0 || self.end_frame != self.source_frames))
        {
            return Err(GpuSignalWindowError::MissingWindow);
        }
        let local = slid_frame - self.first_frame;
        let bucket_size = f64::from(self.bucket_frames);
        let query_start = (local / u64::from(self.bucket_frames)) as f64
            + ((local % u64::from(self.bucket_frames)) as f64 + start.fraction()) / bucket_size;
        let query_span = span / bucket_size;
        let available = (self.end_frame - slid_frame) as f64 - start.fraction();
        if !wraps && span > available {
            return Err(GpuSignalWindowError::MissingWindow);
        }
        let wrap_at = if wraps {
            (remaining as f64 - start.fraction()) / span
        } else {
            2.0
        };
        let wrap_subtract = if wraps {
            self.source_frames as f64 / bucket_size
        } else {
            0.0
        };
        let mut values = vec![
            query_start as f32,
            query_span as f32,
            wrap_at as f32,
            wrap_subtract as f32,
        ];
        encode_gain(p, &mut values)?;
        if p.cursor_ratio
            .is_some_and(|x| !x.is_finite() || !(0.0..=1.0).contains(&x))
            || !p.cursor_width.is_finite()
            || p.cursor_width < 0.0
            || p.cursor_color
                .iter()
                .any(|x| !x.is_finite() || !(0.0..=1.0).contains(x))
        {
            return Err(GpuSignalWindowError::InvalidPresentation);
        }
        values.extend_from_slice(&[p.cursor_ratio.unwrap_or(-1.0), p.cursor_width, 0.0, 0.0]);
        values.extend_from_slice(&p.cursor_color);
        if values.iter().any(|x| !x.is_finite()) {
            return Err(GpuSignalWindowError::InvalidPresentation);
        }
        Ok(values
            .into_iter()
            .flat_map(f32::to_le_bytes)
            .collect::<Vec<_>>()
            .into())
    }
}

fn encode_gain(
    p: &GpuPreciseSignalPresentation,
    out: &mut Vec<f32>,
) -> Result<(), GpuSignalWindowError> {
    let Some(g) = p.gain_preview else {
        out.extend_from_slice(&[0.0; 12]);
        return Ok(());
    };
    let controls = [
        g.gain,
        g.fade_in_length,
        g.fade_in_curve,
        g.fade_in_extension,
        g.fade_in_outer_gain,
        g.fade_out_length,
        g.fade_out_curve,
        g.fade_out_extension,
        g.fade_out_outer_gain,
    ];
    if controls.iter().any(|x| !x.is_finite() || *x < 0.0)
        || [
            g.fade_in_length,
            g.fade_in_curve,
            g.fade_in_outer_gain,
            g.fade_out_length,
            g.fade_out_curve,
            g.fade_out_outer_gain,
        ]
        .iter()
        .any(|x| *x > 1.0)
    {
        return Err(GpuSignalWindowError::InvalidPresentation);
    }
    let selection_start = p.viewport.local_offset(g.selection.start())? / p.viewport.span();
    let selection_end = p.viewport.local_offset(g.selection.end()?)? / p.viewport.span();
    let projected_start = selection_start as f32;
    let projected_end = selection_end as f32;
    let projected_width = projected_end - projected_start;
    if projected_end <= projected_start
        || !projected_width.is_finite()
        || !(projected_start - projected_width * g.fade_in_extension).is_finite()
        || !(projected_end + projected_width * g.fade_out_extension).is_finite()
    {
        return Err(GpuSignalWindowError::InvalidPresentation);
    }
    out.extend_from_slice(&[1.0, selection_start as f32, selection_end as f32, g.gain]);
    out.extend_from_slice(&[
        g.fade_in_length,
        g.fade_in_curve,
        g.fade_out_length,
        g.fade_out_curve,
    ]);
    out.extend_from_slice(&[
        g.fade_in_extension,
        g.fade_out_extension,
        g.fade_in_outer_gain,
        g.fade_out_outer_gain,
    ]);
    Ok(())
}

fn precise_shader() -> Arc<str> {
    static SHADER: OnceLock<Arc<str>> = OnceLock::new();
    Arc::clone(SHADER.get_or_init(|| {
        Arc::from(concat!(
            include_str!("shaders/signal_visual.wgsl"),
            "\n",
            include_str!("shaders/signal/precise.wgsl"),
        ))
    }))
}

#[cfg(test)]
#[path = "signal_window/tests.rs"]
mod tests;

use super::{GpuSignalSummaryBucket, GpuSignalSummaryLevel};
use std::sync::Arc;

pub(super) const MAX_OVERVIEW_BUCKETS: usize = 4096;
pub(super) const MAX_TILE_BUCKETS: usize = 8192;
const MAX_BYTES: usize = 256 * 1024;
const CANCEL_CHUNK_SAMPLES: usize = 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BoundedSignalError {
    InvalidShape,
    InvalidRange,
    Capacity,
    Cancelled,
}

#[derive(Clone, Debug)]
pub(crate) struct BoundedSignalOverview {
    pub(crate) frames: usize,
    pub(crate) band_count: usize,
    pub(crate) levels: Vec<GpuSignalSummaryLevel>,
}
#[derive(Clone, Debug)]
pub(crate) struct BoundedSignalTile {
    pub(crate) first_frame: usize,
    pub(crate) source_frames: usize,
    pub(crate) band_count: usize,
    pub(crate) bucket_frames: usize,
    pub(crate) buckets: Arc<[GpuSignalSummaryBucket]>,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct BoundedSignalTileRequest {
    pub(crate) first_frame: usize,
    pub(crate) bucket_frames: usize,
    pub(crate) bucket_count: usize,
    pub(crate) wrap: bool,
}

impl BoundedSignalOverview {
    #[cfg(test)]
    pub(crate) fn logical_summary_bytes(&self) -> usize {
        self.levels
            .iter()
            .map(|l| l.buckets.len() * std::mem::size_of::<GpuSignalSummaryBucket>())
            .sum()
    }
}
impl BoundedSignalTile {
    #[cfg(test)]
    pub(crate) fn logical_summary_bytes(&self) -> usize {
        self.buckets.len() * std::mem::size_of::<GpuSignalSummaryBucket>()
    }
}

pub(crate) fn build_bounded_overview(
    samples: &[f32],
    frames: usize,
    bands: usize,
    cancel: impl FnMut() -> bool,
) -> Result<BoundedSignalOverview, BoundedSignalError> {
    validate(samples, frames, bands)?;
    let (mut width, _) = bounded_overview_layout(frames, bands)?;
    let mut cancel = cancel;
    let mut levels = Vec::new();
    let mut buckets = scan(
        samples,
        frames,
        bands,
        BoundedSignalTileRequest {
            first_frame: 0,
            bucket_frames: width,
            bucket_count: frames.div_ceil(width),
            wrap: false,
        },
        &mut cancel,
    )?;
    loop {
        levels.push(GpuSignalSummaryLevel {
            bucket_frames: width,
            buckets: Arc::clone(&buckets),
        });
        if width > frames / 2 {
            break;
        }
        width = width.checked_mul(2).ok_or(BoundedSignalError::Capacity)?;
        buckets = merge(&buckets, bands, &mut cancel)?;
    }
    Ok(BoundedSignalOverview {
        frames,
        band_count: bands,
        levels,
    })
}

/// Return the exact logical bytes that `build_bounded_overview` would allocate.
pub(crate) fn bounded_overview_bytes(
    frames: usize,
    bands: usize,
) -> Result<usize, BoundedSignalError> {
    bounded_overview_layout(frames, bands).map(|(_, bytes)| bytes)
}

pub(crate) fn build_bounded_tile(
    samples: &[f32],
    source_frames: usize,
    bands: usize,
    request: BoundedSignalTileRequest,
    mut cancel: impl FnMut() -> bool,
) -> Result<BoundedSignalTile, BoundedSignalError> {
    validate(samples, source_frames, bands)?;
    if request.bucket_frames == 0
        || !request.bucket_frames.is_power_of_two()
        || request.bucket_count == 0
        || request.bucket_count > MAX_TILE_BUCKETS
    {
        return Err(BoundedSignalError::Capacity);
    }
    if !request.wrap && request.first_frame >= source_frames {
        return Err(BoundedSignalError::InvalidRange);
    }
    // A non-wrapping request may end in a truncated bucket, but it must not
    // include buckets wholly beyond the source.
    if !request.wrap
        && request.bucket_count
            > (source_frames - request.first_frame).div_ceil(request.bucket_frames)
    {
        return Err(BoundedSignalError::InvalidRange);
    }
    request
        .first_frame
        .checked_add(
            (request.bucket_count - 1)
                .checked_mul(request.bucket_frames)
                .and_then(|offset| offset.checked_add(request.bucket_frames - 1))
                .ok_or(BoundedSignalError::InvalidRange)?,
        )
        .ok_or(BoundedSignalError::InvalidRange)?;
    let total = request
        .bucket_count
        .checked_mul(bands)
        .and_then(|n| n.checked_mul(std::mem::size_of::<GpuSignalSummaryBucket>()))
        .ok_or(BoundedSignalError::Capacity)?;
    if total > MAX_BYTES {
        return Err(BoundedSignalError::Capacity);
    }
    let buckets = scan(samples, source_frames, bands, request, &mut cancel)?;
    Ok(BoundedSignalTile {
        first_frame: request.first_frame,
        source_frames,
        band_count: bands,
        bucket_frames: request.bucket_frames,
        buckets,
    })
}

fn validate(samples: &[f32], frames: usize, bands: usize) -> Result<(), BoundedSignalError> {
    if frames == 0
        || bands == 0
        || samples.len()
            < frames
                .checked_mul(bands)
                .ok_or(BoundedSignalError::InvalidShape)?
    {
        Err(BoundedSignalError::InvalidShape)
    } else {
        Ok(())
    }
}
fn bounded_overview_layout(
    frames: usize,
    bands: usize,
) -> Result<(usize, usize), BoundedSignalError> {
    if frames == 0 || bands == 0 {
        return Err(BoundedSignalError::InvalidShape);
    }
    let mut width = 1usize;
    loop {
        let bytes = estimated_pyramid_bytes(frames, width, bands)?;
        if frames.div_ceil(width) <= MAX_OVERVIEW_BUCKETS && bytes <= MAX_BYTES {
            return Ok((width, bytes));
        }
        width = width.checked_mul(2).ok_or(BoundedSignalError::Capacity)?;
    }
}
fn estimated_pyramid_bytes(
    frames: usize,
    mut width: usize,
    bands: usize,
) -> Result<usize, BoundedSignalError> {
    let mut entries = 0usize;
    loop {
        entries = entries
            .checked_add(
                frames
                    .div_ceil(width)
                    .checked_mul(bands)
                    .ok_or(BoundedSignalError::Capacity)?,
            )
            .ok_or(BoundedSignalError::Capacity)?;
        if width > frames / 2 {
            break;
        }
        width = width.checked_mul(2).ok_or(BoundedSignalError::Capacity)?;
    }
    entries
        .checked_mul(std::mem::size_of::<GpuSignalSummaryBucket>())
        .ok_or(BoundedSignalError::Capacity)
}
fn scan(
    samples: &[f32],
    frames: usize,
    bands: usize,
    request: BoundedSignalTileRequest,
    cancel: &mut impl FnMut() -> bool,
) -> Result<Arc<[GpuSignalSummaryBucket]>, BoundedSignalError> {
    let len = request
        .bucket_count
        .checked_mul(bands)
        .ok_or(BoundedSignalError::Capacity)?;
    if len
        .checked_mul(std::mem::size_of::<GpuSignalSummaryBucket>())
        .ok_or(BoundedSignalError::Capacity)?
        > MAX_BYTES
    {
        return Err(BoundedSignalError::Capacity);
    }
    if cancel() {
        return Err(BoundedSignalError::Cancelled);
    }
    let mut out = Vec::with_capacity(len);
    let mut seen = 0usize;
    for bucket in 0..request.bucket_count {
        for band in 0..bands {
            let mut value: Option<GpuSignalSummaryBucket> = None;
            for step in 0..request.bucket_frames {
                if seen.is_multiple_of(CANCEL_CHUNK_SAMPLES) && cancel() {
                    return Err(BoundedSignalError::Cancelled);
                }
                seen += 1;
                let frame = request
                    .first_frame
                    .checked_add(
                        bucket
                            .checked_mul(request.bucket_frames)
                            .ok_or(BoundedSignalError::InvalidRange)?,
                    )
                    .and_then(|n| n.checked_add(step))
                    .ok_or(BoundedSignalError::InvalidRange)?;
                if !request.wrap && frame >= frames {
                    break;
                }
                let sample =
                    samples[(if request.wrap { frame % frames } else { frame }) * bands + band];
                let sample = if sample.is_finite() { sample } else { 0.0 };
                let entry = value.get_or_insert(GpuSignalSummaryBucket {
                    min: sample,
                    max: sample,
                });
                entry.min = entry.min.min(sample);
                entry.max = entry.max.max(sample);
            }
            out.push(value.unwrap_or_default());
        }
    }
    if cancel() {
        return Err(BoundedSignalError::Cancelled);
    }
    Ok(out.into())
}
fn merge(
    previous: &[GpuSignalSummaryBucket],
    bands: usize,
    cancel: &mut impl FnMut() -> bool,
) -> Result<Arc<[GpuSignalSummaryBucket]>, BoundedSignalError> {
    let count = previous.len() / bands;
    if cancel() {
        return Err(BoundedSignalError::Cancelled);
    }
    let mut out = Vec::with_capacity(count.div_ceil(2) * bands);
    for bucket in 0..count.div_ceil(2) {
        for band in 0..bands {
            if out.len().is_multiple_of(CANCEL_CHUNK_SAMPLES) && cancel() {
                return Err(BoundedSignalError::Cancelled);
            }
            let mut v = previous[(bucket * 2) * bands + band];
            if bucket * 2 + 1 < count {
                let n = previous[(bucket * 2 + 1) * bands + band];
                v.min = v.min.min(n.min);
                v.max = v.max.max(n.max);
            }
            out.push(v);
        }
    }
    if cancel() {
        return Err(BoundedSignalError::Cancelled);
    }
    Ok(out.into())
}

#[cfg(test)]
#[path = "bounded/tests.rs"]
mod tests;

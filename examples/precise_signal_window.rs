//! Construct a bounded far-offset waveform and transform a local pointer position.
//!
//! Run with `cargo run --example precise_signal_window`. The returned custom
//! shader content can be attached to the same PaintGpuSurface used by
//! `waveform_view`; no source-sized allocation is needed.

use radiant::runtime::{
    GpuPreciseSignalPresentation, GpuSignalPosition, GpuSignalSummaryBucket,
    GpuSignalSummaryWindow, GpuSignalViewport,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let origin = 1_u64 << 40;
    let buckets: Vec<_> = (0..64)
        .map(|index| {
            let peak = ((index as f32 * 0.41).sin() * 0.8).abs();
            GpuSignalSummaryBucket {
                min: -peak,
                max: peak,
            }
        })
        .collect();
    let window = GpuSignalSummaryWindow::new(origin + 64, origin, 1, 1, &buckets, 1, 0)?;
    let viewport = GpuSignalViewport::new(GpuSignalPosition::new(origin + 8, 0.25)?, 32.0)?;

    // Convert only the widget-local pointer ratio; retain the source origin as u64.
    let pointer_x = 300.0_f64;
    let widget_left = 100.0_f64;
    let widget_width = 800.0_f64;
    let ratio = ((pointer_x - widget_left) / widget_width).clamp(0.0, 1.0);
    let hit = viewport.position_at(ratio)?;
    let zoomed = viewport.zoom_anchored(ratio, 16.0)?;
    let mut presentation = GpuPreciseSignalPresentation::new(zoomed);
    presentation.cursor_ratio = Some((zoomed.local_offset(hit)? / zoomed.span()) as f32);
    let content = window.content(&presentation)?;
    content.validate()?;
    println!(
        "Hit frame {} + {}; retained {} summary buckets",
        hit.frame(),
        hit.fraction(),
        window.bucket_count()
    );
    Ok(())
}

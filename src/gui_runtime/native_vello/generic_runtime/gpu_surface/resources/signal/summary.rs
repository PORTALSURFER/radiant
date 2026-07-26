use super::super::super::GpuSurfaceRenderer;
use super::super::super::gpu_surface_types::CachedSignalSummary;
use super::super::super::identity::RenderCanvasContentIdentity;
use super::super::super::stats::GpuSurfaceRenderStats;
use crate::runtime::GpuSignalSummary;
use std::sync::Arc;

impl GpuSurfaceRenderer {
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) fn cached_signal_summary(
        &mut self,
        key: u64,
        revision: u64,
        content_identity: RenderCanvasContentIdentity,
        frames: usize,
        band_count: usize,
        samples: &Arc<[f32]>,
        stats: &mut GpuSurfaceRenderStats,
    ) -> Arc<GpuSignalSummary> {
        if let Some(cached) = self.resources.signal_summaries.get(&key)
            && cached.revision == revision
            && cached.content_identity == content_identity
            && cached.frames == frames
            && cached.band_count == band_count
            && cached.sample_count == samples.len()
        {
            stats.signal.summary_cache_hits += 1;
            return Arc::clone(&cached.summary);
        }
        if let Some(cached) = self.resources.signal_summaries.get(&key) {
            if cached.revision != revision {
                stats.signal.summary_revision_mismatches += 1;
            } else if cached.content_identity != content_identity {
                stats.signal.summary_content_mismatches += 1;
            }
        }
        let summary = Arc::new(GpuSignalSummary::from_interleaved_samples(
            samples, frames, band_count,
        ));
        self.resources.signal_summaries.insert(
            key,
            CachedSignalSummary {
                revision,
                content_identity,
                frames,
                band_count,
                sample_count: samples.len(),
                summary: Arc::clone(&summary),
            },
        );
        stats.signal.summary_builds += 1;
        summary
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cached_signal_summary_reports_builds_and_hits() {
        let mut renderer = GpuSurfaceRenderer::default();
        let samples: Arc<[f32]> = [-0.5, 0.25, 0.75, -0.25].into_iter().collect();
        let mut stats = GpuSurfaceRenderStats::default();

        let identity = RenderCanvasContentIdentity::SignalBands {
            samples: Arc::as_ptr(&samples) as *const () as usize,
            frames: 4,
            band_count: 1,
            frame_range: [0.0f32.to_bits(), 4.0f32.to_bits()],
        };
        let first = renderer.cached_signal_summary(7, 1, identity, 4, 1, &samples, &mut stats);

        assert_eq!(stats.signal.summary_builds, 1);
        assert_eq!(stats.signal.summary_cache_hits, 0);

        let second = renderer.cached_signal_summary(7, 1, identity, 4, 1, &samples, &mut stats);

        assert!(Arc::ptr_eq(&first, &second));
        assert_eq!(stats.signal.summary_builds, 1);
        assert_eq!(stats.signal.summary_cache_hits, 1);
    }

    #[test]
    fn cached_signal_summary_rebuilds_when_source_shape_changes() {
        let mut renderer = GpuSurfaceRenderer::default();
        let samples: Arc<[f32]> = [-0.5, 0.25, 0.75, -0.25].into_iter().collect();
        let mut stats = GpuSurfaceRenderStats::default();

        let identity = RenderCanvasContentIdentity::SignalBands {
            samples: Arc::as_ptr(&samples) as *const () as usize,
            frames: 4,
            band_count: 1,
            frame_range: [0.0f32.to_bits(), 4.0f32.to_bits()],
        };
        let first = renderer.cached_signal_summary(7, 1, identity, 4, 1, &samples, &mut stats);
        let second_identity = RenderCanvasContentIdentity::SignalBands {
            samples: Arc::as_ptr(&samples) as *const () as usize,
            frames: 2,
            band_count: 2,
            frame_range: [0.0f32.to_bits(), 2.0f32.to_bits()],
        };
        let second =
            renderer.cached_signal_summary(7, 1, second_identity, 2, 2, &samples, &mut stats);

        assert!(!Arc::ptr_eq(&first, &second));
        assert_eq!(stats.signal.summary_builds, 2);
        assert_eq!(stats.signal.summary_cache_hits, 0);
    }

    #[test]
    fn cached_signal_summary_rebuilds_for_new_immutable_content_with_same_revision() {
        let mut renderer = GpuSurfaceRenderer::default();
        let first_samples: Arc<[f32]> = [-0.5, 0.25, 0.75, -0.25].into_iter().collect();
        let second_samples: Arc<[f32]> = [-0.5, 0.25, 0.75, -0.25].into_iter().collect();
        let first_identity = RenderCanvasContentIdentity::SignalBands {
            samples: Arc::as_ptr(&first_samples) as *const () as usize,
            frames: 4,
            band_count: 1,
            frame_range: [0.0f32.to_bits(), 4.0f32.to_bits()],
        };
        let second_identity = RenderCanvasContentIdentity::SignalBands {
            samples: Arc::as_ptr(&second_samples) as *const () as usize,
            frames: 4,
            band_count: 1,
            frame_range: [0.0f32.to_bits(), 4.0f32.to_bits()],
        };
        let mut stats = GpuSurfaceRenderStats::default();

        renderer.cached_signal_summary(7, 1, first_identity, 4, 1, &first_samples, &mut stats);
        renderer.cached_signal_summary(7, 1, second_identity, 4, 1, &second_samples, &mut stats);

        assert_eq!(stats.signal.summary_builds, 2);
        assert_eq!(stats.signal.summary_cache_hits, 0);
        assert_eq!(stats.signal.summary_revision_mismatches, 0);
        assert_eq!(stats.signal.summary_content_mismatches, 1);
    }
}

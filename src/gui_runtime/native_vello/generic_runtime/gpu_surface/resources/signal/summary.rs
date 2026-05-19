use super::super::super::*;

impl GpuSurfaceRenderer {
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) fn cached_signal_summary(
        &mut self,
        key: u64,
        revision: u64,
        frames: usize,
        band_count: usize,
        samples: &Arc<[f32]>,
        stats: &mut GpuSurfaceRenderStats,
    ) -> Arc<GpuSignalSummary> {
        if let Some(cached) = self.resources.signal_summaries.get(&key)
            && cached.revision == revision
            && cached.frames == frames
            && cached.band_count == band_count
            && cached.sample_count == samples.len()
        {
            stats.signal_summary_cache_hits += 1;
            return Arc::clone(&cached.summary);
        }
        let summary = Arc::new(GpuSignalSummary::from_interleaved_samples(
            samples, frames, band_count,
        ));
        self.resources.signal_summaries.insert(
            key,
            CachedSignalSummary {
                revision,
                frames,
                band_count,
                sample_count: samples.len(),
                summary: Arc::clone(&summary),
            },
        );
        stats.signal_summary_builds += 1;
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

        let first = renderer.cached_signal_summary(7, 1, 4, 1, &samples, &mut stats);

        assert_eq!(stats.signal_summary_builds, 1);
        assert_eq!(stats.signal_summary_cache_hits, 0);

        let second = renderer.cached_signal_summary(7, 1, 4, 1, &samples, &mut stats);

        assert!(Arc::ptr_eq(&first, &second));
        assert_eq!(stats.signal_summary_builds, 1);
        assert_eq!(stats.signal_summary_cache_hits, 1);
    }

    #[test]
    fn cached_signal_summary_rebuilds_when_source_shape_changes() {
        let mut renderer = GpuSurfaceRenderer::default();
        let samples: Arc<[f32]> = [-0.5, 0.25, 0.75, -0.25].into_iter().collect();
        let mut stats = GpuSurfaceRenderStats::default();

        let first = renderer.cached_signal_summary(7, 1, 4, 1, &samples, &mut stats);
        let second = renderer.cached_signal_summary(7, 1, 2, 2, &samples, &mut stats);

        assert!(!Arc::ptr_eq(&first, &second));
        assert_eq!(stats.signal_summary_builds, 2);
        assert_eq!(stats.signal_summary_cache_hits, 0);
    }
}

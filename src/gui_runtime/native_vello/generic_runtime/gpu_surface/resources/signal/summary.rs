use super::super::super::GpuSurfaceRenderer;
use super::super::super::gpu_surface_types::CachedSignalSummary;
use super::super::super::identity::SignalSourceIdentity;
use super::super::super::stats::GpuSurfaceRenderStats;
use super::super::super::upload_plan::GpuSurfaceRenderCanvasUploadSignalSummaryOperation;
use crate::runtime::GpuSignalSummary;
use std::sync::Arc;

pub(crate) struct CachedSignalSummaryRequest<'a> {
    pub(crate) key: u64,
    pub(crate) revision: u64,
    pub(crate) source_identity: SignalSourceIdentity,
    pub(crate) frames: usize,
    pub(crate) band_count: usize,
    pub(crate) samples: &'a Arc<[f32]>,
    pub(crate) stats: &'a mut GpuSurfaceRenderStats,
}

impl GpuSurfaceRenderer {
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) fn signal_summary_cache_operation(
        &self,
        key: u64,
        revision: u64,
        source_identity: SignalSourceIdentity,
        frames: usize,
        band_count: usize,
        sample_count: usize,
    ) -> GpuSurfaceRenderCanvasUploadSignalSummaryOperation {
        self.resources
            .signal_summaries
            .get(&key)
            .filter(|cached| {
                cached.revision == revision
                    && cached.source_identity == source_identity
                    && cached.frames == frames
                    && cached.band_count == band_count
                    && cached.sample_count == sample_count
            })
            .map(|_| GpuSurfaceRenderCanvasUploadSignalSummaryOperation::Reuse)
            .unwrap_or(GpuSurfaceRenderCanvasUploadSignalSummaryOperation::Build)
    }

    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) fn cached_signal_summary(
        &mut self,
        request: CachedSignalSummaryRequest<'_>,
    ) -> Arc<GpuSignalSummary> {
        let CachedSignalSummaryRequest {
            key,
            revision,
            source_identity,
            frames,
            band_count,
            samples,
            stats,
        } = request;
        if let Some(cached) = self.resources.signal_summaries.get(&key)
            && cached.revision == revision
            && cached.source_identity == source_identity
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
            } else if cached.source_identity != source_identity {
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
                source_identity,
                frames,
                band_count,
                sample_count: samples.len(),
                summary: Arc::clone(&summary),
                _source_samples: Arc::clone(samples),
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

        let identity = SignalSourceIdentity::Samples {
            samples: Arc::as_ptr(&samples) as *const () as usize,
            frames: 4,
            band_count: 1,
        };
        let first = renderer.cached_signal_summary(CachedSignalSummaryRequest {
            key: 7,
            revision: 1,
            source_identity: identity,
            frames: 4,
            band_count: 1,
            samples: &samples,
            stats: &mut stats,
        });

        assert_eq!(stats.signal.summary_builds, 1);
        assert_eq!(stats.signal.summary_cache_hits, 0);

        let second = renderer.cached_signal_summary(CachedSignalSummaryRequest {
            key: 7,
            revision: 1,
            source_identity: identity,
            frames: 4,
            band_count: 1,
            samples: &samples,
            stats: &mut stats,
        });

        assert!(Arc::ptr_eq(&first, &second));
        assert_eq!(stats.signal.summary_builds, 1);
        assert_eq!(stats.signal.summary_cache_hits, 1);
    }

    #[test]
    fn cached_signal_summary_rebuilds_when_source_shape_changes() {
        let mut renderer = GpuSurfaceRenderer::default();
        let samples: Arc<[f32]> = [-0.5, 0.25, 0.75, -0.25].into_iter().collect();
        let mut stats = GpuSurfaceRenderStats::default();

        let identity = SignalSourceIdentity::Samples {
            samples: Arc::as_ptr(&samples) as *const () as usize,
            frames: 4,
            band_count: 1,
        };
        let first = renderer.cached_signal_summary(CachedSignalSummaryRequest {
            key: 7,
            revision: 1,
            source_identity: identity,
            frames: 4,
            band_count: 1,
            samples: &samples,
            stats: &mut stats,
        });
        let second_identity = SignalSourceIdentity::Samples {
            samples: Arc::as_ptr(&samples) as *const () as usize,
            frames: 2,
            band_count: 2,
        };
        let second = renderer.cached_signal_summary(CachedSignalSummaryRequest {
            key: 7,
            revision: 1,
            source_identity: second_identity,
            frames: 2,
            band_count: 2,
            samples: &samples,
            stats: &mut stats,
        });

        assert!(!Arc::ptr_eq(&first, &second));
        assert_eq!(stats.signal.summary_builds, 2);
        assert_eq!(stats.signal.summary_cache_hits, 0);
    }

    #[test]
    fn cached_signal_summary_rebuilds_for_new_immutable_content_with_same_revision() {
        let mut renderer = GpuSurfaceRenderer::default();
        let first_samples: Arc<[f32]> = [-0.5, 0.25, 0.75, -0.25].into_iter().collect();
        let first_identity = SignalSourceIdentity::Samples {
            samples: Arc::as_ptr(&first_samples) as *const () as usize,
            frames: 4,
            band_count: 1,
        };
        let mut stats = GpuSurfaceRenderStats::default();

        renderer.cached_signal_summary(CachedSignalSummaryRequest {
            key: 7,
            revision: 1,
            source_identity: first_identity,
            frames: 4,
            band_count: 1,
            samples: &first_samples,
            stats: &mut stats,
        });
        drop(first_samples);
        assert_eq!(
            renderer
                .resources
                .signal_summaries
                .get(&7)
                .map(|cached| Arc::as_ptr(&cached._source_samples) as *const () as usize),
            Some(first_identity_sample_ptr(first_identity))
        );

        let second_samples: Arc<[f32]> = [-0.5, 0.25, 0.75, -0.25].into_iter().collect();
        let second_identity = SignalSourceIdentity::Samples {
            samples: Arc::as_ptr(&second_samples) as *const () as usize,
            frames: 4,
            band_count: 1,
        };
        renderer.cached_signal_summary(CachedSignalSummaryRequest {
            key: 7,
            revision: 1,
            source_identity: second_identity,
            frames: 4,
            band_count: 1,
            samples: &second_samples,
            stats: &mut stats,
        });

        assert_ne!(first_identity, second_identity);
        assert_eq!(stats.signal.summary_builds, 2);
        assert_eq!(stats.signal.summary_cache_hits, 0);
        assert_eq!(stats.signal.summary_revision_mismatches, 0);
        assert_eq!(stats.signal.summary_content_mismatches, 1);
    }

    fn first_identity_sample_ptr(identity: SignalSourceIdentity) -> usize {
        let SignalSourceIdentity::Samples { samples, .. } = identity else {
            unreachable!()
        };
        samples
    }
}

use super::super::super::GpuSurfaceRenderer;
use super::super::super::gpu_surface_types::CachedSignalSummary;
use super::super::super::identity::SignalSourceIdentity;
use super::super::super::stats::GpuSurfaceRenderStats;
use super::super::super::upload_plan::GpuSurfaceRenderCanvasUploadSignalSummaryOperation;
use crate::gui_runtime::native_vello::generic_runtime::signal_summary_prepare::PreparedSummary;
use crate::runtime::GpuSurfaceContent;
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
    /// Called only after terminal cleanup and successful transaction completion
    /// (or the committed legacy no-plan boundary). A pending replacement must
    /// release old source reservations even while its surface key remains live.
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) fn retire_stale_prepared_signals(
        &mut self,
        primitives: &[crate::runtime::PaintPrimitive],
    ) {
        let current = |key: &u64, prepared: &PreparedSummary| {
            primitives
                .iter()
                .rev()
                .find_map(|primitive| {
                    let crate::runtime::PaintPrimitive::GpuSurface(surface) = primitive else {
                        return None;
                    };
                    (surface.key == *key)
                        .then(|| prepared.matches_raw_surface(&surface.content, surface.revision))
                })
                .unwrap_or(false)
        };
        self.resources
            .signal_summaries
            .retain(|key, cached| current(key, &cached.prepared));
        self.resources
            .signals
            .retain(|key, buffer| match &buffer._content_owner {
                super::super::super::identity::RenderCanvasContentOwner::PreparedSignal(
                    prepared,
                ) => current(key, prepared),
                _ => true,
            });
        self.resources
            .signal_bodies
            .retain(|key, body| match &body._content_owner {
                super::super::super::identity::RenderCanvasContentOwner::PreparedSignal(
                    prepared,
                ) => current(key, prepared),
                _ => true,
            });
    }

    pub(in crate::gui_runtime::native_vello::generic_runtime) fn install_prepared_signal_summary(
        &mut self,
        key: u64,
        revision: u64,
        content: &GpuSurfaceContent,
        prepared: PreparedSummary,
    ) -> bool {
        if !prepared.matches_raw_surface(content, revision) {
            return false;
        }
        let GpuSurfaceContent::SignalBands {
            samples,
            frames,
            band_count,
            ..
        } = content
        else {
            return false;
        };
        let Some(shape) = content.signal_render_shape() else {
            return false;
        };
        let source_identity = SignalSourceIdentity::samples(samples, *frames, *band_count);
        if self
            .resources
            .signal_summaries
            .get(&key)
            .is_some_and(|cached| {
                cached.revision == revision
                    && cached.source_identity == source_identity
                    && cached.prepared.asset_key() == prepared.asset_key()
                    && cached.prepared.matches_raw_surface(content, revision)
            })
        {
            return true;
        }
        self.resources.signal_summaries.insert(
            key,
            CachedSignalSummary {
                revision,
                source_identity,
                frames: shape.frames,
                band_count: shape.band_count,
                sample_count: samples.len(),
                prepared,
            },
        );
        true
    }

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
    ) -> Option<PreparedSummary> {
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
            return Some(cached.prepared.clone());
        }
        if let Some(cached) = self.resources.signal_summaries.get(&key) {
            if cached.revision != revision {
                stats.signal.summary_revision_mismatches += 1;
            } else if cached.source_identity != source_identity {
                stats.signal.summary_content_mismatches += 1;
            }
        }
        None
    }
}

#[cfg(test)]
mod prepared_tests {
    use super::*;
    use crate::gui_runtime::native_vello::generic_runtime::signal_summary_prepare::SummaryBroker;

    fn raw(samples: Arc<[f32]>, range: [f32; 2]) -> GpuSurfaceContent {
        GpuSurfaceContent::SignalBands {
            frames: 4,
            band_count: 1,
            frame_range: range,
            samples,
        }
    }

    #[test]
    fn prepared_summary_is_pending_until_installed_then_reuses_viewport() {
        let samples: Arc<[f32]> = Arc::from([-0.5, 0.25, 0.75, -0.25]);
        let first = raw(Arc::clone(&samples), [0.0, 2.0]);
        let moved = raw(Arc::clone(&samples), [0.25, 2.25]);
        let (broker, prepared) = SummaryBroker::prepare_for_test(&first, 1);
        let mut renderer = GpuSurfaceRenderer::default();
        let mut stats = GpuSurfaceRenderStats::default();
        let request = CachedSignalSummaryRequest {
            key: 7,
            revision: 1,
            source_identity: SignalSourceIdentity::from_content(&first).expect("raw identity"),
            frames: 4,
            band_count: 1,
            samples: &samples,
            stats: &mut stats,
        };
        assert!(renderer.cached_signal_summary(request).is_none());
        assert!(renderer.install_prepared_signal_summary(7, 1, &first, prepared));
        let ready = renderer
            .cached_signal_summary(CachedSignalSummaryRequest {
                key: 7,
                revision: 1,
                source_identity: SignalSourceIdentity::from_content(&moved).expect("raw identity"),
                frames: 4,
                band_count: 1,
                samples: &samples,
                stats: &mut stats,
            })
            .expect("installed summary");
        assert!(ready.matches_raw_surface(&moved, 1));
        assert_eq!(stats.signal.summary_builds, 0);
        assert_eq!(stats.signal.summary_cache_hits, 1);
        drop(ready);
        drop(renderer);
        drop(broker);
    }

    #[test]
    fn installer_rejects_stale_revision_source_and_shape() {
        let samples: Arc<[f32]> = Arc::from([0.0, 0.25, -0.5, 1.0]);
        let content = raw(Arc::clone(&samples), [0.0, 4.0]);
        let (_broker, prepared) = SummaryBroker::prepare_for_test(&content, 1);
        let mut renderer = GpuSurfaceRenderer::default();
        assert!(!renderer.install_prepared_signal_summary(7, 2, &content, prepared.clone()));
        let replacement = raw(Arc::from([0.0, 0.25, -0.5, 1.0]), [0.0, 4.0]);
        assert!(!renderer.install_prepared_signal_summary(7, 1, &replacement, prepared.clone()));
        assert!(renderer.install_prepared_signal_summary(7, 1, &content, prepared.clone()));
        let changed = GpuSurfaceContent::SignalBands {
            frames: 2,
            band_count: 2,
            frame_range: [0.0, 2.0],
            samples,
        };
        assert!(!renderer.install_prepared_signal_summary(7, 1, &changed, prepared));
        assert!(
            renderer
                .resources
                .signal_summaries
                .get(&7)
                .expect("original retained")
                .prepared
                .matches_raw_surface(&content, 1)
        );
    }
}

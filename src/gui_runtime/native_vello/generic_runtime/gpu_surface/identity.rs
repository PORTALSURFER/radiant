use crate::gui::types::{ImageRgba, Rect};
use crate::runtime::{
    GpuShaderSurfaceDescriptor, GpuSignalGainPreview, GpuSignalSummary, GpuSurfaceContent,
};
use std::sync::Arc;

/// Ownership token retained with each cache entry whose identity uses an Arc
/// allocation address. Holding the immutable source keeps the address unique
/// for the lifetime of the cached resource and prevents allocator ABA reuse.
#[derive(Clone)]
pub(super) enum RenderCanvasContentOwner {
    RgbaAtlas(Arc<ImageRgba>),
    SignalBands(Arc<[f32]>),
    PreparedSignal(super::super::signal_summary_prepare::PreparedSummary),
    SignalSummaryBands(Arc<GpuSignalSummary>),
    CustomShader(Arc<GpuShaderSurfaceDescriptor>),
}

impl Drop for RenderCanvasContentOwner {
    fn drop(&mut self) {
        match self {
            Self::RgbaAtlas(source) => {
                let _ = source;
            }
            Self::PreparedSignal(source) => {
                let _ = source;
            }
            Self::SignalBands(source) => {
                let _ = source;
            }
            Self::SignalSummaryBands(source) => {
                let _ = source;
            }
            Self::CustomShader(source) => {
                let _ = source;
            }
        }
    }
}

/// Exact immutable identity used to authorize retained render-canvas reuse.
///
/// The identity deliberately records allocation addresses for shared payloads
/// instead of scanning their contents on the frame path. Immutable Arc-backed
/// payloads therefore reuse cheaply while a replacement allocation (even with
/// the same host revision and length) conservatively rebuilds its resources.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum RenderCanvasContentIdentity {
    RgbaAtlas {
        atlas: usize,
        source_rect: [u32; 4],
    },
    SignalBands {
        samples: usize,
        frames: usize,
        band_count: usize,
        frame_range: [u32; 2],
    },
    SignalSummaryBands {
        summary: usize,
        frames: usize,
        band_count: usize,
        frame_range: [u32; 2],
        gain_preview: [u32; 12],
        sample_slide_frame_offset: i64,
    },
    CustomShader {
        descriptor: usize,
    },
}

/// Immutable signal payload identity for resources whose contents do not
/// depend on presentation state.  Kept separate from
/// `RenderCanvasContentIdentity`: signal bodies must still invalidate for a
/// changed frame range, gain preview, or sample slide, while summary and
/// bucket-buffer reuse only depend on their immutable source and shape.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum SignalSourceIdentity {
    Samples {
        samples: usize,
        frames: usize,
        band_count: usize,
    },
    Summary {
        summary: usize,
        frames: usize,
        band_count: usize,
    },
}

impl SignalSourceIdentity {
    pub(super) fn samples(samples: &Arc<[f32]>, frames: usize, band_count: usize) -> Self {
        Self::Samples {
            samples: arc_ptr(samples),
            frames,
            band_count,
        }
    }

    pub(super) fn summary(
        summary: &Arc<GpuSignalSummary>,
        frames: usize,
        band_count: usize,
    ) -> Self {
        Self::Summary {
            summary: arc_ptr(summary),
            frames,
            band_count,
        }
    }

    #[cfg(test)]
    pub(super) fn from_content(content: &GpuSurfaceContent) -> Option<Self> {
        match content {
            GpuSurfaceContent::SignalBands {
                samples,
                frames,
                band_count,
                ..
            } => Some(Self::samples(samples, *frames, *band_count)),
            GpuSurfaceContent::SignalSummaryBands {
                summary,
                frames,
                band_count,
                ..
            } => Some(Self::summary(summary, *frames, *band_count)),
            GpuSurfaceContent::RgbaAtlas { .. } | GpuSurfaceContent::CustomShader { .. } => None,
        }
    }
}

impl Default for SignalSourceIdentity {
    fn default() -> Self {
        Self::Samples {
            samples: 0,
            frames: 0,
            band_count: 0,
        }
    }
}

impl RenderCanvasContentIdentity {
    pub(super) fn from_content(content: &GpuSurfaceContent) -> Self {
        match content {
            GpuSurfaceContent::RgbaAtlas { source_rect, atlas } => Self::RgbaAtlas {
                atlas: arc_ptr(atlas),
                source_rect: rect_bits(*source_rect),
            },
            GpuSurfaceContent::SignalBands {
                frames,
                band_count,
                frame_range,
                samples,
            } => Self::SignalBands {
                samples: arc_ptr(samples),
                frames: *frames,
                band_count: *band_count,
                frame_range: [frame_range[0].to_bits(), frame_range[1].to_bits()],
            },
            GpuSurfaceContent::SignalSummaryBands {
                frames,
                band_count,
                frame_range,
                summary,
                gain_preview,
                sample_slide_frame_offset,
            } => Self::SignalSummaryBands {
                summary: arc_ptr(summary),
                frames: *frames,
                band_count: *band_count,
                frame_range: [frame_range[0].to_bits(), frame_range[1].to_bits()],
                gain_preview: gain_preview_bits(*gain_preview),
                sample_slide_frame_offset: *sample_slide_frame_offset,
            },
            GpuSurfaceContent::CustomShader { descriptor } => Self::CustomShader {
                descriptor: arc_ptr(descriptor),
            },
        }
    }
}

impl RenderCanvasContentOwner {
    pub(super) fn from_content(content: &GpuSurfaceContent) -> Self {
        match content {
            GpuSurfaceContent::RgbaAtlas { atlas, .. } => Self::RgbaAtlas(Arc::clone(atlas)),
            GpuSurfaceContent::SignalBands { samples, .. } => {
                Self::SignalBands(Arc::clone(samples))
            }
            GpuSurfaceContent::SignalSummaryBands { summary, .. } => {
                Self::SignalSummaryBands(Arc::clone(summary))
            }
            GpuSurfaceContent::CustomShader { descriptor } => {
                Self::CustomShader(Arc::clone(descriptor))
            }
        }
    }
}

impl Default for RenderCanvasContentIdentity {
    fn default() -> Self {
        Self::RgbaAtlas {
            atlas: 0,
            source_rect: [0; 4],
        }
    }
}

fn arc_ptr<T: ?Sized>(value: &std::sync::Arc<T>) -> usize {
    std::sync::Arc::as_ptr(value) as *const () as usize
}

fn rect_bits(rect: Rect) -> [u32; 4] {
    [
        rect.min.x.to_bits(),
        rect.min.y.to_bits(),
        rect.max.x.to_bits(),
        rect.max.y.to_bits(),
    ]
}

fn gain_preview_bits(preview: Option<GpuSignalGainPreview>) -> [u32; 12] {
    let Some(preview) = preview else {
        return [0; 12];
    };
    [
        1,
        preview.start.to_bits(),
        preview.end.to_bits(),
        preview.gain.to_bits(),
        preview.fade_in_length.to_bits(),
        preview.fade_in_curve.to_bits(),
        preview.fade_in_mute.to_bits(),
        preview.fade_out_length.to_bits(),
        preview.fade_out_curve.to_bits(),
        preview.fade_out_mute.to_bits(),
        preview.fade_in_outer_gain.to_bits(),
        preview.fade_out_outer_gain.to_bits(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::types::{ImageRgba, Point, Vector2};
    use std::sync::Arc;

    #[test]
    fn immutable_arc_replacement_changes_identity_without_payload_scanning() {
        let first = Arc::new(ImageRgba::new(1, 1, vec![1, 2, 3, 4]).expect("valid image"));
        let second = Arc::new(ImageRgba::new(1, 1, vec![1, 2, 3, 4]).expect("valid image"));
        let first_identity =
            RenderCanvasContentIdentity::from_content(&GpuSurfaceContent::RgbaAtlas {
                source_rect: crate::gui::types::Rect::from_min_size(
                    Point::new(0.0, 0.0),
                    Vector2::new(1.0, 1.0),
                ),
                atlas: first,
            });
        let second_identity =
            RenderCanvasContentIdentity::from_content(&GpuSurfaceContent::RgbaAtlas {
                source_rect: crate::gui::types::Rect::from_min_size(
                    Point::new(0.0, 0.0),
                    Vector2::new(1.0, 1.0),
                ),
                atlas: second,
            });
        assert_ne!(first_identity, second_identity);
    }

    #[test]
    fn signal_source_identity_excludes_presentation_but_render_identity_keeps_it() {
        let samples: Arc<[f32]> = [0.0, 0.5, -0.5, 1.0].into_iter().collect();
        let first = GpuSurfaceContent::SignalBands {
            frames: 4,
            band_count: 1,
            frame_range: [0.0, 2.0],
            samples: Arc::clone(&samples),
        };
        let presented = GpuSurfaceContent::SignalBands {
            frames: 4,
            band_count: 1,
            frame_range: [1.0, 3.0],
            samples,
        };

        assert_eq!(
            SignalSourceIdentity::from_content(&first),
            SignalSourceIdentity::from_content(&presented)
        );
        assert_ne!(
            RenderCanvasContentIdentity::from_content(&first),
            RenderCanvasContentIdentity::from_content(&presented)
        );
    }
}

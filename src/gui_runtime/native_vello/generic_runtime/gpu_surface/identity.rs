use crate::gui::types::Rect;
use crate::runtime::{GpuSignalGainPreview, GpuSurfaceContent};

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
}

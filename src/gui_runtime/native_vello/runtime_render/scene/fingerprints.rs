//! Fingerprint builders used to decide which retained scene layers need rebuilds.

use super::super::*;

impl<B: NativeAppBridge> NativeVelloRunner<B> {
    /// Build candidate fingerprints for every retained static segment.
    pub(crate) fn build_static_segment_fingerprints(
        &self,
        layout: &ShellLayout,
        style: &StyleTokens,
        segment_revisions: SegmentRevisions,
    ) -> [StaticSegmentCacheFingerprint; StaticFrameSegment::COUNT] {
        let layout_width_bits = layout.root.rect.width().to_bits();
        let layout_height_bits = layout.root.rect.height().to_bits();
        let layout_scale_bits = layout.ui_scale.to_bits();
        let style_signature = static_segment_style_signature(style);
        std::array::from_fn(|idx| {
            let segment = Self::static_segment_from_cache_index(idx);
            StaticSegmentCacheFingerprint {
                segment,
                layout_width_bits,
                layout_height_bits,
                layout_scale_bits,
                style_signature,
                segment_revision: self.static_segment_revision(segment_revisions, segment),
            }
        })
    }

    pub(crate) fn state_overlay_cache_fingerprint(
        &self,
        model: &AppModel,
        _style: &StyleTokens,
        layout_width_bits: u32,
        layout_height_bits: u32,
        layout_scale_bits: u32,
    ) -> StateOverlayCacheFingerprint {
        StateOverlayCacheFingerprint {
            layout_width_bits,
            layout_height_bits,
            layout_scale_bits,
            shell: self.shell_state.state_overlay_fingerprint(),
            model_signature: state_overlay_model_signature(model),
        }
    }

    pub(crate) fn waveform_motion_overlay_cache_fingerprint(
        &self,
        motion_model: &NativeMotionModel,
        _style: &StyleTokens,
        layout_width_bits: u32,
        layout_height_bits: u32,
        layout_scale_bits: u32,
    ) -> WaveformMotionOverlayCacheFingerprint {
        WaveformMotionOverlayCacheFingerprint {
            layout_width_bits,
            layout_height_bits,
            layout_scale_bits,
            shell: self.shell_state.waveform_motion_overlay_fingerprint(),
            motion_signature: waveform_motion_overlay_model_signature(motion_model),
        }
    }

    pub(crate) fn chrome_motion_overlay_cache_fingerprint(
        &self,
        motion_model: &NativeMotionModel,
        _style: &StyleTokens,
        layout_width_bits: u32,
        layout_height_bits: u32,
        layout_scale_bits: u32,
    ) -> ChromeMotionOverlayCacheFingerprint {
        ChromeMotionOverlayCacheFingerprint {
            layout_width_bits,
            layout_height_bits,
            layout_scale_bits,
            shell: self.shell_state.chrome_motion_overlay_fingerprint(),
            motion_signature: chrome_motion_overlay_model_signature(motion_model),
        }
    }
}

use crate::gui_runtime::native_vello::*;
use crate::runtime::RetainedSurfaceCachePolicy;
use std::collections::VecDeque;

#[derive(Clone, Debug, Default)]
pub(in crate::gui_runtime::native_vello) struct RetainedSurfaceFrameCache {
    entries: VecDeque<RetainedSurfaceFrameCacheEntry>,
    policy: RetainedSurfaceCachePolicy,
}

#[derive(Clone, Debug)]
struct RetainedSurfaceFrameCacheEntry {
    descriptor: RetainedSurfaceDescriptor,
    rect: UiRect,
    viewport: Vector2,
    frame: PaintFrame,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(in crate::gui_runtime::native_vello) struct RetainedSurfaceEncodeStats {
    pub paint_plan_primitives: usize,
    pub clip_layer_count: usize,
    pub text_primitive_count: usize,
    pub text_input_count: usize,
    pub image_count: usize,
    pub svg_document_count: usize,
    pub gpu_surface_count: usize,
    pub custom_surface_count: usize,
    pub custom_surface_fallback_count: u32,
    pub bridge_calls: u32,
    pub cache_hits: u32,
    pub retained_surface_miss_count: u32,
    pub retained_frame_primitive_count: usize,
    pub retained_frame_text_run_count: usize,
    pub text_run_count: usize,
}

impl RetainedSurfaceEncodeStats {
    pub(in crate::gui_runtime::native_vello::generic_runtime::scene) fn record_text_runs(
        &mut self,
        count: usize,
    ) {
        self.text_run_count = self.text_run_count.saturating_add(count);
    }

    pub(in crate::gui_runtime::native_vello::generic_runtime::scene) fn record_retained_frame(
        &mut self,
        frame: &PaintFrame,
    ) {
        self.retained_frame_primitive_count = self
            .retained_frame_primitive_count
            .saturating_add(frame.primitives.len());
        self.retained_frame_text_run_count = self
            .retained_frame_text_run_count
            .saturating_add(frame.text_runs.len());
        self.text_run_count = self.text_run_count.saturating_add(frame.text_runs.len());
        self.image_count = self.image_count.saturating_add(
            frame
                .primitives
                .iter()
                .filter(|primitive| matches!(primitive, Primitive::Image(_)))
                .count(),
        );
        self.svg_document_count = self.svg_document_count.saturating_add(
            frame
                .primitives
                .iter()
                .filter(|primitive| matches!(primitive, Primitive::Svg(_)))
                .count(),
        );
    }
}

impl RetainedSurfaceFrameCache {
    pub(in crate::gui_runtime::native_vello) fn with_policy(
        policy: RetainedSurfaceCachePolicy,
    ) -> Self {
        Self {
            entries: VecDeque::new(),
            policy,
        }
    }

    pub(in crate::gui_runtime::native_vello) fn policy(&self) -> RetainedSurfaceCachePolicy {
        self.policy
    }

    pub(in crate::gui_runtime::native_vello) fn entry_count(&self) -> usize {
        self.entries.len()
    }

    pub(in crate::gui_runtime::native_vello::generic_runtime::scene) fn cached_frame(
        &mut self,
        descriptor: RetainedSurfaceDescriptor,
        rect: UiRect,
        viewport: Vector2,
    ) -> Option<&PaintFrame> {
        if !cacheable_descriptor(descriptor) {
            self.invalidate_descriptor_key(descriptor.key);
            return None;
        }
        self.entries
            .iter()
            .find(|entry| entry.matches(descriptor, rect, viewport))
            .map(|entry| &entry.frame)
    }

    pub(in crate::gui_runtime::native_vello::generic_runtime::scene) fn store(
        &mut self,
        descriptor: RetainedSurfaceDescriptor,
        rect: UiRect,
        viewport: Vector2,
        frame: PaintFrame,
    ) {
        if !cacheable_descriptor(descriptor) {
            self.invalidate_descriptor_key(descriptor.key);
            return;
        }
        if self.policy.max_frames == 0 {
            self.invalidate_descriptor_key(descriptor.key);
            return;
        }
        self.entries
            .retain(|entry| !entry.same_surface_geometry(descriptor, rect, viewport));
        self.entries.push_back(RetainedSurfaceFrameCacheEntry {
            descriptor,
            rect,
            viewport,
            frame,
        });
        while self.entries.len() > self.policy.max_frames {
            self.entries.pop_front();
        }
    }

    fn invalidate_descriptor_key(&mut self, key: u64) {
        self.entries.retain(|entry| entry.descriptor.key != key);
    }
}

impl RetainedSurfaceFrameCacheEntry {
    fn matches(
        &self,
        descriptor: RetainedSurfaceDescriptor,
        rect: UiRect,
        viewport: Vector2,
    ) -> bool {
        self.descriptor.key == descriptor.key
            && self.descriptor.revision == descriptor.revision
            && cacheable_descriptor(self.descriptor)
            && self.rect == rect
            && self.viewport == viewport
    }

    fn same_surface_geometry(
        &self,
        descriptor: RetainedSurfaceDescriptor,
        rect: UiRect,
        viewport: Vector2,
    ) -> bool {
        self.descriptor.key == descriptor.key && self.rect == rect && self.viewport == viewport
    }
}

fn cacheable_descriptor(descriptor: RetainedSurfaceDescriptor) -> bool {
    !descriptor.volatile && descriptor.dirty_mask == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn descriptor(key: u64) -> RetainedSurfaceDescriptor {
        RetainedSurfaceDescriptor {
            key,
            revision: 1,
            dirty_mask: 0,
            volatile: false,
        }
    }

    fn frame(red: u8) -> PaintFrame {
        PaintFrame {
            clear_color: Rgba8 {
                r: red,
                g: 0,
                b: 0,
                a: 255,
            },
            ..PaintFrame::default()
        }
    }

    #[test]
    fn retained_frame_cache_evicts_oldest_entry_without_shifting_storage() {
        let rect = UiRect::from_min_size(Point::new(0.0, 0.0), Vector2::new(20.0, 20.0));
        let viewport = Vector2::new(100.0, 100.0);
        let mut cache =
            RetainedSurfaceFrameCache::with_policy(RetainedSurfaceCachePolicy { max_frames: 64 });

        for key in 0..=64 {
            cache.store(descriptor(key), rect, viewport, frame(key as u8));
        }

        assert_eq!(cache.entries.len(), 64);
        assert!(cache.cached_frame(descriptor(0), rect, viewport).is_none());
        assert_eq!(
            cache
                .cached_frame(descriptor(1), rect, viewport)
                .map(|frame| frame.clear_color.r),
            Some(1)
        );
        assert_eq!(
            cache
                .cached_frame(descriptor(64), rect, viewport,)
                .map(|frame| frame.clear_color.r),
            Some(64)
        );
    }

    #[test]
    fn retained_frame_cache_policy_can_disable_storage() {
        let rect = UiRect::from_min_size(Point::new(0.0, 0.0), Vector2::new(20.0, 20.0));
        let viewport = Vector2::new(100.0, 100.0);
        let mut cache =
            RetainedSurfaceFrameCache::with_policy(RetainedSurfaceCachePolicy { max_frames: 0 });

        cache.store(descriptor(1), rect, viewport, frame(1));

        assert_eq!(cache.entry_count(), 0);
        assert!(cache.cached_frame(descriptor(1), rect, viewport).is_none());
    }
}

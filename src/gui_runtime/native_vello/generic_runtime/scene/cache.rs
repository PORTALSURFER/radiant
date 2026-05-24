use crate::{
    gui::{
        paint::{PaintFrame, Primitive},
        types::{Rect as UiRect, Vector2},
    },
    runtime::RetainedSurfaceCachePolicy,
    widgets::RetainedSurfaceDescriptor,
};
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
mod tests;

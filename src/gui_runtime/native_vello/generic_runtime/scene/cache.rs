use crate::gui_runtime::native_vello::*;

#[derive(Clone, Debug, Default)]
pub(in crate::gui_runtime::native_vello) struct RetainedSurfaceFrameCache {
    entry: Option<RetainedSurfaceFrameCacheEntry>,
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
    pub gpu_surface_count: usize,
    pub custom_surface_count: usize,
    pub bridge_calls: u32,
    pub cache_hits: u32,
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
    }
}

impl RetainedSurfaceFrameCache {
    pub(in crate::gui_runtime::native_vello::generic_runtime::scene) fn cached_frame(
        &self,
        descriptor: RetainedSurfaceDescriptor,
        rect: UiRect,
        viewport: Vector2,
    ) -> Option<&PaintFrame> {
        if descriptor.volatile || descriptor.dirty_mask != 0 {
            return None;
        }
        let entry = self.entry.as_ref()?;
        (entry.descriptor.key == descriptor.key
            && entry.descriptor.revision == descriptor.revision
            && entry.descriptor.dirty_mask == 0
            && !entry.descriptor.volatile
            && entry.rect == rect
            && entry.viewport == viewport)
            .then_some(&entry.frame)
    }

    pub(in crate::gui_runtime::native_vello::generic_runtime::scene) fn store(
        &mut self,
        descriptor: RetainedSurfaceDescriptor,
        rect: UiRect,
        viewport: Vector2,
        frame: PaintFrame,
    ) {
        if descriptor.volatile || descriptor.dirty_mask != 0 {
            return;
        }
        self.entry = Some(RetainedSurfaceFrameCacheEntry {
            descriptor,
            rect,
            viewport,
            frame,
        });
    }
}

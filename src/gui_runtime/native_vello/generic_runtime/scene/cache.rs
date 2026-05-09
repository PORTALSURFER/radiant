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
    pub bridge_calls: u32,
    pub cache_hits: u32,
    pub primitive_count: usize,
    pub text_run_count: usize,
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

//! Renderer-private persistent shader-storage selection and staging writes.
//!
//! CPU shadows remain owned by `SurfaceRuntime`; this module only derives the
//! exact bytes that one binding must upload and keeps its device-local cursor.

use super::gpu_surface_types::CustomShaderBinding;
use crate::runtime::{
    GpuPersistentStorageStore, GpuPersistentStorageTarget, GpuShaderSurfaceDescriptor,
    PaintGpuSurface, PersistentStorageEntry, PersistentStorageUploads,
};
use vello::wgpu;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) struct PersistentStorageRevision {
    pub(super) target: GpuPersistentStorageTarget,
    pub(super) incarnation: u64,
    pub(super) revision: u64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct PersistentStorageBindingCursor {
    committed: Option<PersistentStorageRevision>,
    // Some(None) means a speculative bulk fallback reset.
    pending: Option<Option<PersistentStorageRevision>>,
}

impl PersistentStorageBindingCursor {
    pub(super) fn effective(self) -> Option<PersistentStorageRevision> {
        self.pending.unwrap_or(self.committed)
    }
    pub(super) fn cursor_for(
        self,
        target: GpuPersistentStorageTarget,
        incarnation: u64,
    ) -> Option<u64> {
        self.effective()
            .filter(|current| current.target == target && current.incarnation == incarnation)
            .map(|current| current.revision)
    }
    pub(super) fn stage(&mut self, revision: PersistentStorageRevision) {
        self.pending = Some(Some(revision));
    }
    pub(super) fn stage_bulk_reset(&mut self) {
        self.pending = Some(None);
    }
    pub(super) fn commit(&mut self) {
        if let Some(next) = self.pending.take() {
            self.committed = next;
        }
    }
    pub(super) fn abort(&mut self) {
        self.pending = None;
    }
    pub(super) fn invalidate(&mut self) {
        *self = Self::default();
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct PersistentStorageRange {
    pub(super) offset: usize,
    pub(super) byte_len: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct PersistentStorageUploadPlan {
    pub(super) desired: PersistentStorageRevision,
    pub(super) ranges: Vec<PersistentStorageRange>,
}

pub(super) enum PersistentStorageSelection<'a> {
    Absent,
    Mismatch,
    Upload {
        entry: PersistentStorageEntry<'a>,
        plan: PersistentStorageUploadPlan,
    },
}

pub(super) fn select_persistent_storage<'a>(
    store: &'a GpuPersistentStorageStore,
    surface: &PaintGpuSurface,
    descriptor: &GpuShaderSurfaceDescriptor,
    cursor: PersistentStorageBindingCursor,
) -> PersistentStorageSelection<'a> {
    let Some(entry) = store.find(surface.widget_id, surface.key.into()) else {
        return PersistentStorageSelection::Absent;
    };
    let target = entry.target();
    if target.storage_identity() != descriptor.storage_identity
        || target.storage_generation() != descriptor.storage_revision
    {
        return PersistentStorageSelection::Absent;
    }
    if entry.capacity_bytes() != descriptor.storage_bytes.len() {
        return PersistentStorageSelection::Mismatch;
    }
    let incarnation = entry.incarnation();
    let (revision, ranges) = match entry.uploads_since(cursor.cursor_for(target, incarnation)) {
        PersistentStorageUploads::Full { revision, range } => (
            revision,
            vec![PersistentStorageRange {
                offset: range.start,
                byte_len: range.len(),
            }],
        ),
        PersistentStorageUploads::Ranges { revision, ranges } => (
            revision,
            ranges
                .into_iter()
                .map(|range| PersistentStorageRange {
                    offset: range.start,
                    byte_len: range.len(),
                })
                .collect(),
        ),
        PersistentStorageUploads::Empty { revision } => (revision, Vec::new()),
    };
    PersistentStorageSelection::Upload {
        entry,
        plan: PersistentStorageUploadPlan {
            desired: PersistentStorageRevision {
                target,
                incarnation,
                revision,
            },
            ranges,
        },
    }
}

pub(super) fn stage_selected_cursor(
    binding: &mut CustomShaderBinding,
    selection: &PersistentStorageSelection<'_>,
) {
    if let PersistentStorageSelection::Upload { plan, .. } = selection
        && !plan.ranges.is_empty()
    {
        binding.persistent_storage_cursor.stage(plan.desired);
    }
}

/// Encode one selected persistent-storage range. This deliberately uses the
/// staging belt, so an encoder veto before submission cannot publish a patch.
pub(super) fn write_selected_range(
    belt: &mut wgpu::util::StagingBelt,
    encoder: &mut wgpu::CommandEncoder,
    buffer: &wgpu::Buffer,
    entry: &PersistentStorageEntry<'_>,
    range: PersistentStorageRange,
) -> bool {
    let Some(end) = range.offset.checked_add(range.byte_len) else {
        return false;
    };
    let Some(bytes) = entry.bytes().get(range.offset..end) else {
        return false;
    };
    let Some(size) = wgpu::BufferSize::new(range.byte_len as wgpu::BufferAddress) else {
        return false;
    };
    let mut mapped = belt.write_buffer(encoder, buffer, range.offset as wgpu::BufferAddress, size);
    mapped.copy_from_slice(bytes);
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    fn revision(
        target: GpuPersistentStorageTarget,
        incarnation: u64,
        revision: u64,
    ) -> PersistentStorageRevision {
        PersistentStorageRevision {
            target,
            incarnation,
            revision,
        }
    }
    #[test]
    fn abort_keeps_the_full_committed_identity_after_a_different_target_stages() {
        let first = GpuPersistentStorageTarget::new(crate::widgets::WidgetId::from(1_u32), 2, 3, 4);
        let second =
            GpuPersistentStorageTarget::new(crate::widgets::WidgetId::from(1_u32), 2, 5, 6);
        let mut cursor = PersistentStorageBindingCursor::default();
        cursor.stage(revision(first, 7, 9));
        cursor.commit();
        cursor.stage(revision(second, 8, 10));
        cursor.abort();
        assert_eq!(cursor.effective(), Some(revision(first, 7, 9)));
    }
    #[test]
    fn bulk_reset_commits_only_after_the_outer_transaction() {
        let target =
            GpuPersistentStorageTarget::new(crate::widgets::WidgetId::from(1_u32), 2, 3, 4);
        let mut cursor = PersistentStorageBindingCursor::default();
        cursor.stage(revision(target, 7, 9));
        cursor.commit();
        cursor.stage_bulk_reset();
        cursor.abort();
        assert_eq!(cursor.effective(), Some(revision(target, 7, 9)));
        cursor.stage_bulk_reset();
        cursor.commit();
        assert_eq!(cursor.effective(), None);
    }
}

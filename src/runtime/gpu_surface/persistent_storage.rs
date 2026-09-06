//! Bounded CPU shadows for renderer-neutral persistent shader storage.
//!
//! The store owns desired bytes and a bounded upload journal. Renderers retain
//! independent cursors and only advance them after a successful present.

use super::CanvasKey;
use crate::widgets::WidgetId;
use std::collections::HashMap;

/// Maximum allocated byte capacity of one persistent storage shadow.
pub const MAX_GPU_PERSISTENT_STORAGE_BYTES: usize = 16 * 1024 * 1024;
/// Maximum payload accepted by one persistent storage patch.
pub const MAX_GPU_PERSISTENT_STORAGE_PATCH_BYTES: usize = 256 * 1024;
const MAX_RESOURCES: usize = 32;
const MAX_TOTAL_SHADOW_BYTES: usize = 64 * 1024 * 1024;
const MAX_JOURNAL_ENTRIES: usize = 64;
const ALIGNMENT: usize = 4;

/// Exact renderer-facing identity for one persistent storage allocation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GpuPersistentStorageTarget {
    widget_id: WidgetId,
    surface_key: CanvasKey,
    storage_identity: u64,
    storage_generation: u64,
}

impl GpuPersistentStorageTarget {
    /// Creates a target and its descriptor-derived resource fence.
    pub fn new(
        widget_id: WidgetId,
        surface_key: impl Into<CanvasKey>,
        storage_identity: u64,
        storage_generation: u64,
    ) -> Self {
        Self {
            widget_id,
            surface_key: surface_key.into(),
            storage_identity,
            storage_generation,
        }
    }

    /// Returns the owning widget.
    pub const fn widget_id(self) -> WidgetId {
        self.widget_id
    }
    /// Returns the canvas key within the widget.
    pub const fn surface_key(self) -> CanvasKey {
        self.surface_key
    }
    /// Returns the descriptor's stable storage identity.
    pub const fn storage_identity(self) -> u64 {
        self.storage_identity
    }
    /// Returns the descriptor's storage generation.
    pub const fn storage_generation(self) -> u64 {
        self.storage_generation
    }
}

/// Validation failure or synchronous admission rejection for persistent storage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GpuPersistentStorageError {
    /// Element stride was zero or not four-byte aligned.
    InvalidElementStride,
    /// Allocation capacity was zero, too large, or not stride aligned.
    InvalidCapacity,
    /// Logical length exceeded capacity or was not stride aligned.
    InvalidLogicalLength,
    /// Snapshot bytes did not exactly match its logical length.
    InvalidInitialBytes,
    /// Patch payload was empty, too large, or not four-byte aligned.
    InvalidPatchBytes,
    /// Patch revisions were not exactly consecutive.
    InvalidPatchRevision,
    /// Patch offset or range was not aligned or did not fit the shadow.
    InvalidPatchRange,
    /// The bounded number of persistent resource shadows was exhausted.
    ResourceLimit,
    /// The bounded allocated shadow-byte budget was exhausted.
    ShadowCapacity,
    /// Snapshot revision or generation regressed the current target.
    StaleSnapshot,
    /// Patch base revision regressed the current accepted revision.
    StalePatch,
    /// Patch target fence did not match the current target fence.
    FenceMismatch,
    /// The never-reused runtime entry incarnation counter overflowed.
    IncarnationExhausted,
}

/// Current admission state for a target.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GpuPersistentStorageStatus {
    /// The desired CPU shadow is current at this dynamic-data revision.
    Ready {
        /// Current dynamic-data revision.
        revision: u64,
    },
    /// A patch gap was observed and a newer full snapshot is required.
    NeedsSnapshot {
        /// Last accepted dynamic-data revision.
        revision: u64,
    },
}

/// Validated snapshot replacing the complete desired allocation state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GpuPersistentStorageSnapshot {
    target: GpuPersistentStorageTarget,
    element_stride: usize,
    capacity_bytes: usize,
    logical_len: usize,
    revision: u64,
    initial_bytes: Vec<u8>,
}

impl GpuPersistentStorageSnapshot {
    /// Validates a complete replacement allocation.
    ///
    /// A snapshot with a different identity supersedes the target's existing
    /// fence. For the same identity, generations and dynamic revisions must
    /// advance when the store applies it.
    pub fn new(
        target: GpuPersistentStorageTarget,
        element_stride: usize,
        capacity_bytes: usize,
        logical_len: usize,
        revision: u64,
        initial_bytes: impl AsRef<[u8]>,
    ) -> Result<Self, GpuPersistentStorageError> {
        validate_layout(element_stride, capacity_bytes, logical_len)?;
        let initial_bytes = initial_bytes.as_ref();
        if initial_bytes.len() != logical_len {
            return Err(GpuPersistentStorageError::InvalidInitialBytes);
        }
        Ok(Self {
            target,
            element_stride,
            capacity_bytes,
            logical_len,
            revision,
            initial_bytes: initial_bytes.to_vec(),
        })
    }

    /// Returns the target and descriptor fence.
    pub const fn target(&self) -> GpuPersistentStorageTarget {
        self.target
    }
    /// Returns the byte stride of one element.
    pub const fn element_stride(&self) -> usize {
        self.element_stride
    }
    /// Returns the fixed allocated shadow capacity.
    pub const fn capacity_bytes(&self) -> usize {
        self.capacity_bytes
    }
    /// Returns the initialized logical byte length.
    pub const fn logical_len(&self) -> usize {
        self.logical_len
    }
    /// Returns the dynamic-data revision.
    pub const fn revision(&self) -> u64 {
        self.revision
    }
    /// Returns the validated initial logical bytes.
    pub fn initial_bytes(&self) -> &[u8] {
        &self.initial_bytes
    }
}

/// One validated mutation operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GpuPersistentStoragePatchOperation {
    /// Replaces existing logical bytes at an explicit byte offset.
    Replace,
    /// Extends the logical length at its current end.
    Append,
}

/// Validated delta from one exact dynamic-data revision to the next.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GpuPersistentStoragePatch {
    target: GpuPersistentStorageTarget,
    base_revision: u64,
    result_revision: u64,
    operation: GpuPersistentStoragePatchOperation,
    byte_offset: Option<usize>,
    bytes: Vec<u8>,
}

impl GpuPersistentStoragePatch {
    /// Validates a replacement within the current logical range.
    pub fn replace(
        target: GpuPersistentStorageTarget,
        base_revision: u64,
        result_revision: u64,
        byte_offset: usize,
        bytes: impl AsRef<[u8]>,
    ) -> Result<Self, GpuPersistentStorageError> {
        validate_patch_bytes(bytes.as_ref())?;
        if !byte_offset.is_multiple_of(ALIGNMENT) {
            return Err(GpuPersistentStorageError::InvalidPatchRange);
        }
        Self::new(
            target,
            base_revision,
            result_revision,
            GpuPersistentStoragePatchOperation::Replace,
            Some(byte_offset),
            bytes.as_ref().to_vec(),
        )
    }

    /// Validates an append at the current logical end.
    pub fn append(
        target: GpuPersistentStorageTarget,
        base_revision: u64,
        result_revision: u64,
        bytes: impl AsRef<[u8]>,
    ) -> Result<Self, GpuPersistentStorageError> {
        validate_patch_bytes(bytes.as_ref())?;
        Self::new(
            target,
            base_revision,
            result_revision,
            GpuPersistentStoragePatchOperation::Append,
            None,
            bytes.as_ref().to_vec(),
        )
    }

    fn new(
        target: GpuPersistentStorageTarget,
        base_revision: u64,
        result_revision: u64,
        operation: GpuPersistentStoragePatchOperation,
        byte_offset: Option<usize>,
        bytes: Vec<u8>,
    ) -> Result<Self, GpuPersistentStorageError> {
        if result_revision
            != base_revision
                .checked_add(1)
                .ok_or(GpuPersistentStorageError::InvalidPatchRevision)?
        {
            return Err(GpuPersistentStorageError::InvalidPatchRevision);
        }
        Ok(Self {
            target,
            base_revision,
            result_revision,
            operation,
            byte_offset,
            bytes,
        })
    }

    /// Returns the target and descriptor fence.
    pub const fn target(&self) -> GpuPersistentStorageTarget {
        self.target
    }
    /// Returns the accepted base revision.
    pub const fn base_revision(&self) -> u64 {
        self.base_revision
    }
    /// Returns the required next revision.
    pub const fn result_revision(&self) -> u64 {
        self.result_revision
    }
    /// Returns whether this is a replacement or append.
    pub const fn operation(&self) -> GpuPersistentStoragePatchOperation {
        self.operation
    }
    /// Returns the replacement offset, or `None` for append.
    pub const fn byte_offset(&self) -> Option<usize> {
        self.byte_offset
    }
    /// Returns the validated patch payload.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// Declarative persistent-storage command payload.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GpuPersistentStorageUpdate {
    /// Replaces an allocation's complete desired state.
    Snapshot(GpuPersistentStorageSnapshot),
    /// Applies one exact consecutive revision.
    Patch(GpuPersistentStoragePatch),
    /// Releases one exact target and descriptor fence.
    Release(GpuPersistentStorageTarget),
}

/// One byte range a renderer must upload.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PersistentStorageUploadRange {
    /// Inclusive byte offset.
    pub start: usize,
    /// Exclusive byte offset.
    pub end: usize,
}

impl PersistentStorageUploadRange {
    /// Returns the range byte length.
    pub const fn len(self) -> usize {
        self.end - self.start
    }
    /// Reports whether the range has no bytes.
    pub const fn is_empty(self) -> bool {
        self.start == self.end
    }
}

/// Borrowed upload work for a renderer cursor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PersistentStorageUploads {
    /// The cursor is absent or too old, requiring the allocated capacity.
    Full {
        /// Current dynamic-data revision.
        revision: u64,
        /// Full allocated-capacity range.
        range: PersistentStorageUploadRange,
    },
    /// The journal still contains every change after the cursor.
    Ranges {
        /// Current dynamic-data revision.
        revision: u64,
        /// Sorted, non-overlapping changed ranges.
        ranges: Vec<PersistentStorageUploadRange>,
    },
    /// The renderer already has the current revision.
    Empty {
        /// Current dynamic-data revision.
        revision: u64,
    },
}

struct JournalEntry {
    revision: u64,
    range: PersistentStorageUploadRange,
}

struct StorageShadow {
    target: GpuPersistentStorageTarget,
    stride: usize,
    capacity: usize,
    logical_len: usize,
    revision: u64,
    incarnation: u64,
    needs_snapshot: bool,
    history_floor: u64,
    bytes: Vec<u8>,
    journal: Vec<JournalEntry>,
}

/// Private runtime-owned mutable desired-state store.
pub(crate) struct GpuPersistentStorageStore {
    entries: HashMap<GpuPersistentStorageTarget, StorageShadow>,
    target_fences: HashMap<(WidgetId, CanvasKey), GpuPersistentStorageTarget>,
    allocated_bytes: usize,
    next_incarnation: u64,
}

impl Default for GpuPersistentStorageStore {
    fn default() -> Self {
        Self {
            entries: HashMap::new(),
            target_fences: HashMap::new(),
            allocated_bytes: 0,
            next_incarnation: 1,
        }
    }
}

impl GpuPersistentStorageStore {
    pub(crate) fn apply(
        &mut self,
        update: GpuPersistentStorageUpdate,
    ) -> Result<Option<GpuPersistentStorageStatus>, GpuPersistentStorageError> {
        match update {
            GpuPersistentStorageUpdate::Snapshot(snapshot) => {
                self.apply_snapshot(snapshot).map(Some)
            }
            GpuPersistentStorageUpdate::Patch(patch) => self.apply_patch(patch).map(Some),
            GpuPersistentStorageUpdate::Release(target) => {
                self.release(target);
                Ok(None)
            }
        }
    }

    pub(crate) fn apply_snapshot(
        &mut self,
        snapshot: GpuPersistentStorageSnapshot,
    ) -> Result<GpuPersistentStorageStatus, GpuPersistentStorageError> {
        let target_key = (snapshot.target.widget_id, snapshot.target.surface_key);
        if let Some(current_target) = self.target_fences.get(&target_key).copied() {
            if current_target.storage_identity == snapshot.target.storage_identity {
                let current = self
                    .entries
                    .get(&current_target)
                    .expect("fence index must point to an entry");
                if snapshot.target.storage_generation < current_target.storage_generation
                    || (snapshot.target.storage_generation == current_target.storage_generation
                        && snapshot.revision <= current.revision)
                {
                    return Err(GpuPersistentStorageError::StaleSnapshot);
                }
            }
        }
        let prior_capacity = self
            .target_fences
            .get(&target_key)
            .and_then(|target| self.entries.get(target))
            .map_or(0, |entry| entry.capacity);
        let total = self
            .allocated_bytes
            .checked_sub(prior_capacity)
            .and_then(|bytes| bytes.checked_add(snapshot.capacity_bytes))
            .ok_or(GpuPersistentStorageError::ShadowCapacity)?;
        if total > MAX_TOTAL_SHADOW_BYTES {
            return Err(GpuPersistentStorageError::ShadowCapacity);
        }
        if !self.entries.contains_key(&snapshot.target)
            && self.entries.len() >= MAX_RESOURCES
            && prior_capacity == 0
        {
            return Err(GpuPersistentStorageError::ResourceLimit);
        }
        let incarnation = self.next_incarnation;
        self.next_incarnation = self
            .next_incarnation
            .checked_add(1)
            .ok_or(GpuPersistentStorageError::IncarnationExhausted)?;
        if let Some(previous) = self.target_fences.insert(target_key, snapshot.target) {
            self.entries.remove(&previous);
        }
        let mut bytes = vec![0; snapshot.capacity_bytes];
        bytes[..snapshot.logical_len].copy_from_slice(&snapshot.initial_bytes);
        self.entries.insert(
            snapshot.target,
            StorageShadow {
                target: snapshot.target,
                stride: snapshot.element_stride,
                capacity: snapshot.capacity_bytes,
                logical_len: snapshot.logical_len,
                revision: snapshot.revision,
                incarnation,
                needs_snapshot: false,
                history_floor: snapshot.revision,
                bytes,
                journal: Vec::new(),
            },
        );
        self.allocated_bytes = total;
        Ok(GpuPersistentStorageStatus::Ready {
            revision: snapshot.revision,
        })
    }

    pub(crate) fn apply_patch(
        &mut self,
        patch: GpuPersistentStoragePatch,
    ) -> Result<GpuPersistentStorageStatus, GpuPersistentStorageError> {
        let target_key = (patch.target.widget_id, patch.target.surface_key);
        let Some(current_target) = self.target_fences.get(&target_key).copied() else {
            return Ok(GpuPersistentStorageStatus::NeedsSnapshot {
                revision: patch.base_revision,
            });
        };
        if current_target != patch.target {
            return Err(GpuPersistentStorageError::FenceMismatch);
        }
        let entry = self
            .entries
            .get_mut(&patch.target)
            .expect("fence index must point to an entry");
        if patch.base_revision < entry.revision {
            return Err(GpuPersistentStorageError::StalePatch);
        }
        if entry.needs_snapshot || patch.base_revision > entry.revision {
            entry.needs_snapshot = true;
            return Ok(GpuPersistentStorageStatus::NeedsSnapshot {
                revision: entry.revision,
            });
        }
        let range = match patch.operation {
            GpuPersistentStoragePatchOperation::Replace => {
                let byte_offset = patch
                    .byte_offset
                    .expect("replacement patches always retain an offset");
                let end = byte_offset
                    .checked_add(patch.bytes.len())
                    .ok_or(GpuPersistentStorageError::InvalidPatchRange)?;
                if !byte_offset.is_multiple_of(entry.stride)
                    || end > entry.logical_len
                    || !end.is_multiple_of(entry.stride)
                {
                    return Err(GpuPersistentStorageError::InvalidPatchRange);
                }
                entry.bytes[byte_offset..end].copy_from_slice(&patch.bytes);
                PersistentStorageUploadRange {
                    start: byte_offset,
                    end,
                }
            }
            GpuPersistentStoragePatchOperation::Append => {
                let start = entry.logical_len;
                let end = start
                    .checked_add(patch.bytes.len())
                    .ok_or(GpuPersistentStorageError::InvalidPatchRange)?;
                if end > entry.capacity
                    || !start.is_multiple_of(entry.stride)
                    || !end.is_multiple_of(entry.stride)
                {
                    return Err(GpuPersistentStorageError::InvalidPatchRange);
                }
                entry.bytes[start..end].copy_from_slice(&patch.bytes);
                entry.logical_len = end;
                PersistentStorageUploadRange { start, end }
            }
        };
        entry.revision = patch.result_revision;
        entry.push_journal(range);
        Ok(GpuPersistentStorageStatus::Ready {
            revision: entry.revision,
        })
    }

    pub(crate) fn status(
        &self,
        target: GpuPersistentStorageTarget,
    ) -> Option<GpuPersistentStorageStatus> {
        self.entries.get(&target).map(|entry| {
            if entry.needs_snapshot {
                GpuPersistentStorageStatus::NeedsSnapshot {
                    revision: entry.revision,
                }
            } else {
                GpuPersistentStorageStatus::Ready {
                    revision: entry.revision,
                }
            }
        })
    }

    pub(crate) fn entry(
        &self,
        target: GpuPersistentStorageTarget,
    ) -> Option<PersistentStorageEntry<'_>> {
        self.entries
            .get(&target)
            .map(|entry| PersistentStorageEntry { entry })
    }
    pub(crate) fn find(
        &self,
        widget_id: WidgetId,
        surface_key: CanvasKey,
    ) -> Option<PersistentStorageEntry<'_>> {
        self.target_fences
            .get(&(widget_id, surface_key))
            .and_then(|target| self.entry(*target))
    }
    pub(crate) fn entries(&self) -> impl Iterator<Item = PersistentStorageEntry<'_>> {
        self.entries
            .values()
            .map(|entry| PersistentStorageEntry { entry })
    }
    pub(crate) fn release(&mut self, target: GpuPersistentStorageTarget) -> bool {
        let Some(entry) = self.entries.remove(&target) else {
            return false;
        };
        self.target_fences
            .remove(&(entry.target.widget_id, entry.target.surface_key));
        self.allocated_bytes -= entry.capacity;
        true
    }
}

impl StorageShadow {
    fn push_journal(&mut self, range: PersistentStorageUploadRange) {
        if self.journal.len() == MAX_JOURNAL_ENTRIES {
            let removed = self.journal.remove(0);
            self.history_floor = removed.revision;
        }
        self.journal.push(JournalEntry {
            revision: self.revision,
            range,
        });
    }
}

/// Immutable renderer-facing view of a CPU shadow.
pub(crate) struct PersistentStorageEntry<'a> {
    entry: &'a StorageShadow,
}

impl PersistentStorageEntry<'_> {
    pub(crate) const fn target(&self) -> GpuPersistentStorageTarget {
        self.entry.target
    }
    pub(crate) const fn revision(&self) -> u64 {
        self.entry.revision
    }
    pub(crate) const fn logical_len(&self) -> usize {
        self.entry.logical_len
    }
    pub(crate) const fn capacity_bytes(&self) -> usize {
        self.entry.capacity
    }
    pub(crate) const fn element_stride(&self) -> usize {
        self.entry.stride
    }
    pub(crate) const fn incarnation(&self) -> u64 {
        self.entry.incarnation
    }
    pub(crate) fn bytes(&self) -> &[u8] {
        &self.entry.bytes
    }
    pub(crate) fn uploads_since(&self, cursor: Option<u64>) -> PersistentStorageUploads {
        let Some(cursor) = cursor else {
            return PersistentStorageUploads::Full {
                revision: self.entry.revision,
                range: PersistentStorageUploadRange {
                    start: 0,
                    end: self.entry.capacity,
                },
            };
        };
        if cursor == self.entry.revision {
            return PersistentStorageUploads::Empty {
                revision: self.entry.revision,
            };
        }
        if cursor < self.entry.history_floor
            || cursor > self.entry.revision
            || self.entry.needs_snapshot
        {
            return PersistentStorageUploads::Full {
                revision: self.entry.revision,
                range: PersistentStorageUploadRange {
                    start: 0,
                    end: self.entry.capacity,
                },
            };
        }
        let mut ranges = Vec::new();
        for change in self
            .entry
            .journal
            .iter()
            .filter(|change| change.revision > cursor)
        {
            coalesce_range(&mut ranges, change.range);
        }
        if ranges.is_empty() {
            PersistentStorageUploads::Empty {
                revision: self.entry.revision,
            }
        } else {
            PersistentStorageUploads::Ranges {
                revision: self.entry.revision,
                ranges,
            }
        }
    }
}

fn coalesce_range(
    ranges: &mut Vec<PersistentStorageUploadRange>,
    next: PersistentStorageUploadRange,
) {
    let mut next = next;
    let mut index = 0;
    while index < ranges.len() {
        let current = ranges[index];
        if next.end < current.start || current.end < next.start {
            index += 1;
            continue;
        }
        next.start = next.start.min(current.start);
        next.end = next.end.max(current.end);
        ranges.remove(index);
    }
    let insert = ranges.partition_point(|range| range.start < next.start);
    ranges.insert(insert, next);
}

fn validate_layout(
    stride: usize,
    capacity: usize,
    logical_len: usize,
) -> Result<(), GpuPersistentStorageError> {
    if stride == 0 || !stride.is_multiple_of(ALIGNMENT) {
        return Err(GpuPersistentStorageError::InvalidElementStride);
    }
    if capacity == 0
        || capacity > MAX_GPU_PERSISTENT_STORAGE_BYTES
        || !capacity.is_multiple_of(stride)
    {
        return Err(GpuPersistentStorageError::InvalidCapacity);
    }
    if logical_len > capacity || !logical_len.is_multiple_of(stride) {
        return Err(GpuPersistentStorageError::InvalidLogicalLength);
    }
    Ok(())
}

fn validate_patch_bytes(bytes: &[u8]) -> Result<(), GpuPersistentStorageError> {
    if bytes.is_empty()
        || bytes.len() > MAX_GPU_PERSISTENT_STORAGE_PATCH_BYTES
        || !bytes.len().is_multiple_of(ALIGNMENT)
    {
        Err(GpuPersistentStorageError::InvalidPatchBytes)
    } else {
        Ok(())
    }
}

#[cfg(test)]
#[path = "persistent_storage/tests.rs"]
mod tests;

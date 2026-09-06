//! Private mutable desired-state storage for persistent shader allocations.

use super::*;
use std::collections::HashMap;

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
        if let Some(current_target) = self.target_fences.get(&target_key).copied()
            && current_target.storage_identity == snapshot.target.storage_identity
        {
            let current = self
                .entries
                .get(&current_target)
                .ok_or(GpuPersistentStorageError::FenceMismatch)?;
            if snapshot.target.storage_generation < current_target.storage_generation
                || (snapshot.target.storage_generation == current_target.storage_generation
                    && snapshot.revision <= current.revision)
            {
                return Err(GpuPersistentStorageError::StaleSnapshot);
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
            .ok_or(GpuPersistentStorageError::FenceMismatch)?;
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
                    .ok_or(GpuPersistentStorageError::InvalidPatchRange)?;
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

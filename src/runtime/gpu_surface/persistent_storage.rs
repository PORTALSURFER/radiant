//! Bounded CPU shadows for renderer-neutral persistent shader storage.
//!
//! The store owns desired bytes and a bounded upload journal. Renderers retain
//! independent cursors and only advance them after a successful present.

use super::CanvasKey;
use crate::widgets::WidgetId;

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

mod store;
pub(crate) use store::{GpuPersistentStorageStore, PersistentStorageEntry};

#[cfg(test)]
#[path = "persistent_storage/tests.rs"]
mod tests;

//! Bounded volatile presentation-uniform updates for custom shader surfaces.

use super::CanvasKey;
use crate::widgets::WidgetId;
use std::fmt;

/// Maximum number of bytes carried by one volatile custom-shader presentation
/// update.
pub const MAX_GPU_SHADER_PRESENTATION_UNIFORM_BYTES: usize = 256;

/// Required byte alignment for custom-shader presentation uniform payloads.
pub const GPU_SHADER_PRESENTATION_UNIFORM_ALIGNMENT: usize = 4;

/// Maximum number of pending presentation updates retained by one
/// [`crate::runtime::SurfaceRuntime`]. Updates are latest-only per target plus
/// immutable storage fence.
pub(crate) const GPU_SHADER_PRESENTATION_UNIFORM_MAILBOX_CAPACITY: usize = 32;

/// Error returned when a volatile custom-shader presentation update cannot be
/// represented by the bounded update contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GpuShaderPresentationUniformUpdateError {
    /// The update carried no bytes.
    Empty,
    /// The update length was not aligned for WGPU uniform writes.
    UnalignedBytes {
        /// Number of bytes supplied by the caller.
        actual_len: usize,
        /// Required byte alignment.
        alignment: usize,
    },
    /// The update exceeded [`MAX_GPU_SHADER_PRESENTATION_UNIFORM_BYTES`].
    TooManyBytes {
        /// Number of bytes supplied by the caller.
        actual_len: usize,
        /// Maximum number of bytes accepted by the contract.
        max_len: usize,
    },
}

impl fmt::Display for GpuShaderPresentationUniformUpdateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str(
                "custom-shader presentation uniform update must carry at least one byte",
            ),
            Self::UnalignedBytes {
                actual_len,
                alignment,
            } => write!(
                formatter,
                "custom-shader presentation uniform update has {actual_len} bytes; length must be a multiple of {alignment} for WGPU uniform writes"
            ),
            Self::TooManyBytes {
                actual_len,
                max_len,
            } => write!(
                formatter,
                "custom-shader presentation uniform update has {actual_len} bytes; maximum is {max_len}"
            ),
        }
    }
}

impl std::error::Error for GpuShaderPresentationUniformUpdateError {}

/// One fixed-size volatile presentation-uniform update.
///
/// The update is copied into inline storage during construction. Once admitted
/// by a runtime mailbox, replacing or draining it does not allocate for the
/// update payload. Mailbox admission is latest-only per target plus immutable
/// storage fence. `storage_identity` and `storage_revision` fence the update to
/// the immutable payload currently presented by the surface.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GpuShaderPresentationUniformUpdate {
    /// Widget that owns the target surface.
    pub widget_id: WidgetId,
    /// Stable surface key for the target custom shader.
    pub surface_key: CanvasKey,
    /// Immutable storage/payload identity fence.
    pub storage_identity: u64,
    /// Immutable storage/payload revision fence.
    pub storage_revision: u64,
    /// Monotonically increasing volatile presentation revision.
    pub presentation_revision: u64,
    bytes: [u8; MAX_GPU_SHADER_PRESENTATION_UNIFORM_BYTES],
    byte_len: u16,
}

impl GpuShaderPresentationUniformUpdate {
    /// Construct and validate one bounded presentation-uniform update.
    pub fn new(
        widget_id: WidgetId,
        surface_key: impl Into<CanvasKey>,
        storage_identity: u64,
        storage_revision: u64,
        presentation_revision: u64,
        bytes: impl AsRef<[u8]>,
    ) -> Result<Self, GpuShaderPresentationUniformUpdateError> {
        Self::try_new(
            widget_id,
            surface_key,
            storage_identity,
            storage_revision,
            presentation_revision,
            bytes,
        )
    }

    /// Construct and validate one bounded presentation-uniform update.
    pub fn try_new(
        widget_id: WidgetId,
        surface_key: impl Into<CanvasKey>,
        storage_identity: u64,
        storage_revision: u64,
        presentation_revision: u64,
        bytes: impl AsRef<[u8]>,
    ) -> Result<Self, GpuShaderPresentationUniformUpdateError> {
        let source = bytes.as_ref();
        if source.is_empty() {
            return Err(GpuShaderPresentationUniformUpdateError::Empty);
        }
        if source.len() > MAX_GPU_SHADER_PRESENTATION_UNIFORM_BYTES {
            return Err(GpuShaderPresentationUniformUpdateError::TooManyBytes {
                actual_len: source.len(),
                max_len: MAX_GPU_SHADER_PRESENTATION_UNIFORM_BYTES,
            });
        }
        if source.len() % GPU_SHADER_PRESENTATION_UNIFORM_ALIGNMENT != 0 {
            return Err(GpuShaderPresentationUniformUpdateError::UnalignedBytes {
                actual_len: source.len(),
                alignment: GPU_SHADER_PRESENTATION_UNIFORM_ALIGNMENT,
            });
        }

        let mut bytes = [0; MAX_GPU_SHADER_PRESENTATION_UNIFORM_BYTES];
        bytes[..source.len()].copy_from_slice(source);
        Ok(Self {
            widget_id,
            surface_key: surface_key.into(),
            storage_identity,
            storage_revision,
            presentation_revision,
            bytes,
            byte_len: source.len() as u16,
        })
    }

    /// Return the stable `(widget_id, surface_key)` target key.
    ///
    /// The mailbox combines this target key with the immutable storage fence for
    /// its slot identity.
    pub const fn key(self) -> (WidgetId, CanvasKey) {
        (self.widget_id, self.surface_key)
    }

    /// Return the bytes supplied to the constructor.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes[..usize::from(self.byte_len)]
    }

    /// Return the number of meaningful bytes in this update.
    pub const fn byte_len(self) -> usize {
        self.byte_len as usize
    }

    /// Return the maximum payload size accepted by this type.
    pub const fn max_bytes() -> usize {
        MAX_GPU_SHADER_PRESENTATION_UNIFORM_BYTES
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct MailboxSlotKey {
    target: (WidgetId, CanvasKey),
    storage_fence: (u64, u64),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct MailboxSlot {
    key: MailboxSlotKey,
    latest: Option<GpuShaderPresentationUniformUpdate>,
    last_presentation_revision: Option<u64>,
    snapshot_revision: Option<u64>,
}

/// Fixed-capacity mailbox owned by one [`SurfaceRuntime`]. Pending updates are
/// latest-only per target plus immutable storage fence.
pub(crate) struct GpuShaderPresentationUniformMailbox {
    slots: Vec<MailboxSlot>,
}

impl Default for GpuShaderPresentationUniformMailbox {
    fn default() -> Self {
        Self {
            slots: Vec::with_capacity(GPU_SHADER_PRESENTATION_UNIFORM_MAILBOX_CAPACITY),
        }
    }
}

impl GpuShaderPresentationUniformMailbox {
    /// Admit a newer update, replacing any pending update for the same target
    /// and immutable storage fence.
    pub(crate) fn admit(&mut self, update: GpuShaderPresentationUniformUpdate) -> bool {
        let key = MailboxSlotKey {
            target: update.key(),
            storage_fence: (update.storage_identity, update.storage_revision),
        };
        if let Some(slot) = self.slots.iter_mut().find(|slot| slot.key == key) {
            if slot
                .last_presentation_revision
                .is_some_and(|revision| update.presentation_revision <= revision)
            {
                return false;
            }
            slot.last_presentation_revision = Some(update.presentation_revision);
            slot.latest = Some(update);
            return true;
        }
        if let Some(slot) = self.slots.iter_mut().find(|slot| slot.latest.is_none()) {
            *slot = MailboxSlot {
                key,
                latest: Some(update),
                last_presentation_revision: Some(update.presentation_revision),
                snapshot_revision: None,
            };
            return true;
        }
        if self.slots.len() >= GPU_SHADER_PRESENTATION_UNIFORM_MAILBOX_CAPACITY {
            return false;
        }
        debug_assert!(self.slots.len() < self.slots.capacity());
        self.slots.push(MailboxSlot {
            key,
            latest: Some(update),
            last_presentation_revision: Some(update.presentation_revision),
            snapshot_revision: None,
        });
        true
    }

    /// Stage all pending updates into caller-owned fixed-capacity storage.
    ///
    /// Staging does not clear the mailbox.  Each selected slot retains its
    /// exact presentation revision until `commit_snapshot` succeeds, so a
    /// surface-acquisition, renderer, or lifecycle veto cannot lose the
    /// volatile update.  A newer update admitted while the snapshot is in
    /// flight remains pending and is not cleared by the older commit.
    pub(crate) fn snapshot_into(&mut self, updates: &mut Vec<GpuShaderPresentationUniformUpdate>) {
        updates.clear();
        debug_assert!(updates.capacity() >= self.slots.len());
        for slot in &mut self.slots {
            if slot.snapshot_revision.is_some() {
                continue;
            }
            if let Some(update) = slot.latest {
                updates.push(update);
                slot.snapshot_revision = Some(update.presentation_revision);
            }
        }
    }

    /// Commit only the exact revisions selected by the most recent snapshot.
    /// Newer updates in the same slot remain pending for the next frame.
    pub(crate) fn commit_snapshot(&mut self) {
        for slot in &mut self.slots {
            let Some(snapshot_revision) = slot.snapshot_revision.take() else {
                continue;
            };
            if slot
                .latest
                .is_some_and(|update| update.presentation_revision == snapshot_revision)
            {
                slot.latest = None;
            }
        }
    }

    /// Abort the most recent snapshot while retaining every selected update.
    pub(crate) fn abort_snapshot(&mut self) {
        for slot in &mut self.slots {
            slot.snapshot_revision = None;
        }
    }

    #[cfg(test)]
    pub(crate) fn drain_into(&mut self, updates: &mut Vec<GpuShaderPresentationUniformUpdate>) {
        self.snapshot_into(updates);
        self.commit_snapshot();
    }

    #[cfg(test)]
    fn pending_len(&self) -> usize {
        self.slots
            .iter()
            .filter(|slot| slot.latest.is_some())
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn update(key: u64, revision: u64, bytes: &[u8]) -> GpuShaderPresentationUniformUpdate {
        update_with_storage(key, 11, 13, revision, bytes)
    }

    fn update_with_storage(
        key: u64,
        storage_identity: u64,
        storage_revision: u64,
        presentation_revision: u64,
        bytes: &[u8],
    ) -> GpuShaderPresentationUniformUpdate {
        GpuShaderPresentationUniformUpdate::try_new(
            7,
            key,
            storage_identity,
            storage_revision,
            presentation_revision,
            bytes,
        )
        .expect("valid update")
    }

    #[test]
    fn update_rejects_empty_and_overflowing_payloads() {
        assert_eq!(
            GpuShaderPresentationUniformUpdate::try_new(1, 2, 3, 4, 5, []),
            Err(GpuShaderPresentationUniformUpdateError::Empty)
        );
        assert_eq!(
            GpuShaderPresentationUniformUpdate::try_new(1, 2, 3, 4, 5, [0, 1, 2]),
            Err(GpuShaderPresentationUniformUpdateError::UnalignedBytes {
                actual_len: 3,
                alignment: GPU_SHADER_PRESENTATION_UNIFORM_ALIGNMENT,
            })
        );
        let bytes = vec![
            0;
            MAX_GPU_SHADER_PRESENTATION_UNIFORM_BYTES
                + GPU_SHADER_PRESENTATION_UNIFORM_ALIGNMENT
        ];
        assert_eq!(
            GpuShaderPresentationUniformUpdate::try_new(1, 2, 3, 4, 5, &bytes),
            Err(GpuShaderPresentationUniformUpdateError::TooManyBytes {
                actual_len: MAX_GPU_SHADER_PRESENTATION_UNIFORM_BYTES
                    + GPU_SHADER_PRESENTATION_UNIFORM_ALIGNMENT,
                max_len: MAX_GPU_SHADER_PRESENTATION_UNIFORM_BYTES,
            })
        );
        assert_eq!(
            GpuShaderPresentationUniformUpdateError::UnalignedBytes {
                actual_len: 3,
                alignment: GPU_SHADER_PRESENTATION_UNIFORM_ALIGNMENT,
            }
            .to_string(),
            "custom-shader presentation uniform update has 3 bytes; length must be a multiple of 4 for WGPU uniform writes"
        );
    }

    #[test]
    fn mailbox_replaces_latest_and_rejects_stale_revisions() {
        let mut mailbox = GpuShaderPresentationUniformMailbox::default();
        assert!(mailbox.admit(update(2, 1, &[1, 1, 1, 1])));
        assert!(mailbox.admit(update(2, 3, &[3, 3, 3, 3])));
        assert!(!mailbox.admit(update(2, 2, &[2, 2, 2, 2])));
        assert!(!mailbox.admit(update(2, 3, &[3, 3, 3, 3])));

        let mut drained = Vec::with_capacity(GPU_SHADER_PRESENTATION_UNIFORM_MAILBOX_CAPACITY);
        mailbox.drain_into(&mut drained);
        assert_eq!(drained.len(), 1);
        assert_eq!(drained[0].presentation_revision, 3);
        assert_eq!(drained[0].bytes(), &[3, 3, 3, 3]);
        assert_eq!(mailbox.pending_len(), 0);
        assert!(!mailbox.admit(update(2, 2, &[2, 2, 2, 2])));
    }

    #[test]
    fn mailbox_keeps_capacity_bounded_and_reuses_drained_slots() {
        let mut mailbox = GpuShaderPresentationUniformMailbox::default();
        assert_eq!(
            mailbox.slots.capacity(),
            GPU_SHADER_PRESENTATION_UNIFORM_MAILBOX_CAPACITY
        );
        for storage_identity in 0..GPU_SHADER_PRESENTATION_UNIFORM_MAILBOX_CAPACITY as u64 {
            assert!(mailbox.admit(update_with_storage(
                2,
                storage_identity,
                13,
                1,
                &[1, 1, 1, 1],
            )));
        }
        assert_eq!(
            mailbox.pending_len(),
            GPU_SHADER_PRESENTATION_UNIFORM_MAILBOX_CAPACITY
        );
        assert_eq!(
            mailbox.slots.len(),
            GPU_SHADER_PRESENTATION_UNIFORM_MAILBOX_CAPACITY
        );

        let mut drained = Vec::with_capacity(GPU_SHADER_PRESENTATION_UNIFORM_MAILBOX_CAPACITY);
        mailbox.drain_into(&mut drained);
        assert_eq!(
            drained.len(),
            GPU_SHADER_PRESENTATION_UNIFORM_MAILBOX_CAPACITY
        );
        assert_eq!(
            drained.capacity(),
            GPU_SHADER_PRESENTATION_UNIFORM_MAILBOX_CAPACITY
        );
        assert_eq!(mailbox.pending_len(), 0);
        assert_eq!(
            mailbox.slots.capacity(),
            GPU_SHADER_PRESENTATION_UNIFORM_MAILBOX_CAPACITY
        );

        for storage_identity in GPU_SHADER_PRESENTATION_UNIFORM_MAILBOX_CAPACITY as u64
            ..(GPU_SHADER_PRESENTATION_UNIFORM_MAILBOX_CAPACITY * 2) as u64
        {
            assert!(mailbox.admit(update_with_storage(
                2,
                storage_identity,
                13,
                1,
                &[1, 1, 1, 1],
            )));
        }
        assert_eq!(
            mailbox.pending_len(),
            GPU_SHADER_PRESENTATION_UNIFORM_MAILBOX_CAPACITY
        );
        assert_eq!(
            mailbox.slots.len(),
            GPU_SHADER_PRESENTATION_UNIFORM_MAILBOX_CAPACITY
        );
        assert_eq!(
            mailbox.slots.capacity(),
            GPU_SHADER_PRESENTATION_UNIFORM_MAILBOX_CAPACITY
        );
        assert!(!mailbox.admit(update_with_storage(
            2,
            (GPU_SHADER_PRESENTATION_UNIFORM_MAILBOX_CAPACITY * 2) as u64,
            13,
            1,
            &[1, 1, 1, 1],
        )));
    }

    #[test]
    fn mailbox_keeps_storage_generations_in_separate_slots() {
        let mut mailbox = GpuShaderPresentationUniformMailbox::default();
        assert!(mailbox.admit(update_with_storage(2, 11, 13, 7, &[7, 7, 7, 7])));
        assert!(!mailbox.admit(update_with_storage(2, 11, 13, 7, &[7, 7, 7, 7])));

        // The new generation is allowed to restart its presentation revision,
        // including at a value lower than the previous generation's revision.
        assert!(mailbox.admit(update_with_storage(2, 12, 13, 1, &[1, 1, 1, 1])));
        assert!(!mailbox.admit(update_with_storage(2, 12, 13, 1, &[1, 1, 1, 1])));
        assert!(!mailbox.admit(update_with_storage(2, 12, 13, 0, &[0, 0, 0, 0])));
        assert!(mailbox.admit(update_with_storage(2, 12, 13, 2, &[2, 2, 2, 2])));

        let mut drained = Vec::with_capacity(GPU_SHADER_PRESENTATION_UNIFORM_MAILBOX_CAPACITY);
        mailbox.drain_into(&mut drained);
        assert_eq!(drained.len(), 2);
        assert!(drained.iter().any(|update| {
            update.storage_identity == 11
                && update.storage_revision == 13
                && update.presentation_revision == 7
        }));
        assert!(drained.iter().any(|update| {
            update.storage_identity == 12
                && update.storage_revision == 13
                && update.presentation_revision == 2
        }));
    }

    #[test]
    fn mailbox_snapshot_abort_is_lossless_and_newer_commit_survives() {
        let mut mailbox = GpuShaderPresentationUniformMailbox::default();
        assert!(mailbox.admit(update(2, 1, &[1, 1, 1, 1])));
        let mut staged = Vec::with_capacity(GPU_SHADER_PRESENTATION_UNIFORM_MAILBOX_CAPACITY);
        mailbox.snapshot_into(&mut staged);
        assert_eq!(staged[0].presentation_revision, 1);
        mailbox.abort_snapshot();
        mailbox.snapshot_into(&mut staged);
        assert_eq!(staged[0].presentation_revision, 1);
        mailbox.commit_snapshot();
        assert_eq!(mailbox.pending_len(), 0);

        assert!(mailbox.admit(update(2, 2, &[2, 2, 2, 2])));
        mailbox.snapshot_into(&mut staged);
        assert!(mailbox.admit(update(2, 3, &[3, 3, 3, 3])));
        mailbox.commit_snapshot();
        assert_eq!(mailbox.pending_len(), 1);
        mailbox.snapshot_into(&mut staged);
        assert_eq!(staged[0].presentation_revision, 3);
        mailbox.commit_snapshot();
        assert_eq!(mailbox.pending_len(), 0);
    }

    #[test]
    fn mailbox_retains_current_generation_when_late_old_generation_arrives() {
        let mut mailbox = GpuShaderPresentationUniformMailbox::default();
        let current_generation_b = update_with_storage(2, 12, 13, 5, &[5, 5, 5, 5]);
        let late_old_generation_a = update_with_storage(2, 11, 13, 6, &[6, 6, 6, 6]);
        assert!(mailbox.admit(current_generation_b));
        assert!(mailbox.admit(late_old_generation_a));

        let mut drained = Vec::with_capacity(GPU_SHADER_PRESENTATION_UNIFORM_MAILBOX_CAPACITY);
        mailbox.drain_into(&mut drained);

        assert_eq!(drained.len(), 2);
        let selected_for_b = drained
            .iter()
            .filter(|update| {
                update.widget_id == current_generation_b.widget_id
                    && update.surface_key == current_generation_b.surface_key
                    && update.storage_identity == current_generation_b.storage_identity
                    && update.storage_revision == current_generation_b.storage_revision
            })
            .max_by_key(|update| update.presentation_revision);
        assert_eq!(
            selected_for_b.map(|update| update.presentation_revision),
            Some(5)
        );
        assert!(drained.iter().any(|update| {
            update.storage_identity == late_old_generation_a.storage_identity
                && update.storage_revision == late_old_generation_a.storage_revision
                && update.presentation_revision == late_old_generation_a.presentation_revision
        }));
    }
}

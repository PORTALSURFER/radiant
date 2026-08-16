//! Bounded volatile presentation-uniform updates for custom shader surfaces.

use super::CanvasKey;
use crate::widgets::WidgetId;
use std::fmt;

/// Maximum number of bytes carried by one volatile custom-shader presentation
/// update.
pub const MAX_GPU_SHADER_PRESENTATION_UNIFORM_BYTES: usize = 256;

/// Maximum number of pending latest-only presentation updates retained by one
/// [`crate::runtime::SurfaceRuntime`].
pub(crate) const GPU_SHADER_PRESENTATION_UNIFORM_MAILBOX_CAPACITY: usize = 32;

/// Error returned when a volatile custom-shader presentation update cannot be
/// represented by the bounded update contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GpuShaderPresentationUniformUpdateError {
    /// The update carried no bytes.
    Empty,
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

/// One fixed-size, latest-only volatile presentation-uniform update.
///
/// The update is copied into inline storage during construction. Once admitted
/// by a runtime mailbox, replacing or draining it does not allocate for the
/// update payload. `storage_identity` and `storage_revision` fence the update
/// to the immutable payload currently presented by the surface.
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

    /// Return the stable `(widget_id, surface_key)` mailbox key.
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
struct MailboxSlot {
    key: (WidgetId, CanvasKey),
    latest: Option<GpuShaderPresentationUniformUpdate>,
    last_storage_fence: Option<(u64, u64)>,
    last_presentation_revision: Option<u64>,
}

/// Fixed-capacity latest-only mailbox owned by one [`SurfaceRuntime`].
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
    /// Admit a newer update, replacing any pending update for the same key.
    pub(crate) fn admit(&mut self, update: GpuShaderPresentationUniformUpdate) -> bool {
        let storage_fence = (update.storage_identity, update.storage_revision);
        if let Some(slot) = self.slots.iter_mut().find(|slot| slot.key == update.key()) {
            if slot.last_storage_fence == Some(storage_fence)
                && slot
                    .last_presentation_revision
                    .is_some_and(|revision| update.presentation_revision <= revision)
            {
                return false;
            }
            slot.last_storage_fence = Some(storage_fence);
            slot.last_presentation_revision = Some(update.presentation_revision);
            slot.latest = Some(update);
            return true;
        }
        if let Some(slot) = self.slots.iter_mut().find(|slot| slot.latest.is_none()) {
            *slot = MailboxSlot {
                key: update.key(),
                latest: Some(update),
                last_storage_fence: Some(storage_fence),
                last_presentation_revision: Some(update.presentation_revision),
            };
            return true;
        }
        if self.slots.len() >= GPU_SHADER_PRESENTATION_UNIFORM_MAILBOX_CAPACITY {
            return false;
        }
        debug_assert!(self.slots.len() < self.slots.capacity());
        self.slots.push(MailboxSlot {
            key: update.key(),
            latest: Some(update),
            last_storage_fence: Some(storage_fence),
            last_presentation_revision: Some(update.presentation_revision),
        });
        true
    }

    /// Drain all pending updates into caller-owned fixed-capacity storage.
    pub(crate) fn drain_into(&mut self, updates: &mut Vec<GpuShaderPresentationUniformUpdate>) {
        updates.clear();
        debug_assert!(updates.capacity() >= self.slots.len());
        for slot in &mut self.slots {
            if let Some(update) = slot.latest.take() {
                updates.push(update);
            }
        }
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
        let bytes = vec![0; MAX_GPU_SHADER_PRESENTATION_UNIFORM_BYTES + 1];
        assert_eq!(
            GpuShaderPresentationUniformUpdate::try_new(1, 2, 3, 4, 5, &bytes),
            Err(GpuShaderPresentationUniformUpdateError::TooManyBytes {
                actual_len: MAX_GPU_SHADER_PRESENTATION_UNIFORM_BYTES + 1,
                max_len: MAX_GPU_SHADER_PRESENTATION_UNIFORM_BYTES,
            })
        );
    }

    #[test]
    fn mailbox_replaces_latest_and_rejects_stale_revisions() {
        let mut mailbox = GpuShaderPresentationUniformMailbox::default();
        assert!(mailbox.admit(update(2, 1, &[1])));
        assert!(mailbox.admit(update(2, 3, &[3])));
        assert!(!mailbox.admit(update(2, 2, &[2])));
        assert!(!mailbox.admit(update(2, 3, &[3])));

        let mut drained = Vec::with_capacity(GPU_SHADER_PRESENTATION_UNIFORM_MAILBOX_CAPACITY);
        mailbox.drain_into(&mut drained);
        assert_eq!(drained.len(), 1);
        assert_eq!(drained[0].presentation_revision, 3);
        assert_eq!(drained[0].bytes(), &[3]);
        assert_eq!(mailbox.pending_len(), 0);
        assert!(!mailbox.admit(update(2, 2, &[2])));
    }

    #[test]
    fn mailbox_reuses_drained_slots_for_new_keys() {
        let mut mailbox = GpuShaderPresentationUniformMailbox::default();
        for key in 0..GPU_SHADER_PRESENTATION_UNIFORM_MAILBOX_CAPACITY as u64 {
            assert!(mailbox.admit(update(key, 1, &[1])));
        }

        let mut drained = Vec::with_capacity(GPU_SHADER_PRESENTATION_UNIFORM_MAILBOX_CAPACITY);
        mailbox.drain_into(&mut drained);
        assert_eq!(
            drained.len(),
            GPU_SHADER_PRESENTATION_UNIFORM_MAILBOX_CAPACITY
        );
        assert_eq!(mailbox.pending_len(), 0);

        for key in GPU_SHADER_PRESENTATION_UNIFORM_MAILBOX_CAPACITY as u64
            ..(GPU_SHADER_PRESENTATION_UNIFORM_MAILBOX_CAPACITY * 2) as u64
        {
            assert!(mailbox.admit(update(key, 1, &[1])));
        }
        assert!(!mailbox.admit(update(
            (GPU_SHADER_PRESENTATION_UNIFORM_MAILBOX_CAPACITY * 2) as u64,
            1,
            &[1]
        )));
    }

    #[test]
    fn mailbox_resets_presentation_revision_for_new_storage_generations() {
        let mut mailbox = GpuShaderPresentationUniformMailbox::default();
        assert!(mailbox.admit(update_with_storage(2, 11, 13, 7, &[7])));
        assert!(!mailbox.admit(update_with_storage(2, 11, 13, 7, &[7])));

        // The new generation is allowed to restart its presentation revision,
        // including at a value lower than the previous generation's revision.
        assert!(mailbox.admit(update_with_storage(2, 12, 13, 1, &[1])));
        assert!(!mailbox.admit(update_with_storage(2, 12, 13, 1, &[1])));
        assert!(!mailbox.admit(update_with_storage(2, 12, 13, 0, &[0])));
        assert!(mailbox.admit(update_with_storage(2, 12, 13, 2, &[2])));

        let mut drained = Vec::with_capacity(GPU_SHADER_PRESENTATION_UNIFORM_MAILBOX_CAPACITY);
        mailbox.drain_into(&mut drained);
        assert_eq!(drained.len(), 1);
        assert_eq!(drained[0].storage_identity, 12);
        assert_eq!(drained[0].presentation_revision, 2);
    }
}

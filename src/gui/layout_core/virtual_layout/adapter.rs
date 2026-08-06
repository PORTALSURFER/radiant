//! Private pure adapter from an accepted keyed window to retained payloads.
//!
//! This is the executable Slice 4 boundary.  It admits the complete shell and
//! active item batch before returning any lowered payload, but deliberately
//! stops before runtime registration and lifecycle callbacks.

#![expect(
    dead_code,
    reason = "The private retained adapter is shipped before runtime registration"
)]

use super::coordinator::VirtualLayoutCommit;
use super::materialization::VirtualLayoutSlotIdentity;
use super::{VIRTUAL_LAYOUT_MAX_QUERY_ENTRIES, VirtualLayoutItem, VirtualLayoutItemKey};
use crate::application::{View, VirtualLayoutViewAdmissionError, lower_virtual_layout_batch};
use crate::runtime::SurfaceNode;

/// Typed failures from complete shell-plus-batch admission.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum VirtualLayoutRetainedBatchError {
    InvalidCommit,
    MissingItem,
    ExtraItem,
    DuplicateItem,
    UnstableItemKey,
    SlotScopeMismatch,
    SlotCollision,
    Lowering(VirtualLayoutViewAdmissionError),
}

/// One immutable payload paired with the exact accepted item and slot tuple.
pub(super) struct VirtualLayoutRetainedItem<Message> {
    pub(super) item: VirtualLayoutItem,
    pub(super) slot: VirtualLayoutSlotIdentity,
    pub(super) payload: SurfaceNode<Message>,
}

/// Complete pure output of one shell-plus-active-batch admission.
pub(super) struct VirtualLayoutRetainedBatch<Message> {
    pub(super) shell: SurfaceNode<Message>,
    pub(super) items: Vec<VirtualLayoutRetainedItem<Message>>,
}

/// Admit and lower one complete accepted batch without lifecycle side effects.
pub(super) fn admit_virtual_layout_batch<Message: 'static>(
    commit: &VirtualLayoutCommit,
    shell: View<Message>,
    supplied: Vec<(
        VirtualLayoutItemKey,
        View<Message>,
        VirtualLayoutSlotIdentity,
    )>,
) -> Result<VirtualLayoutRetainedBatch<Message>, VirtualLayoutRetainedBatchError> {
    if commit.accepted_revision() == 0
        || commit.view().fallback
        || commit.view().clip.is_some()
        || commit.view().extent.is_none()
        || commit.view().accepted_revision != Some(commit.accepted_revision())
        || commit.view().entries.len() > VIRTUAL_LAYOUT_MAX_QUERY_ENTRIES
    {
        return Err(VirtualLayoutRetainedBatchError::InvalidCommit);
    }

    let mut accepted: Vec<VirtualLayoutItem> = commit.view().entries.clone();
    accepted.sort_by_key(VirtualLayoutItem::logical_index);
    for first in 0..accepted.len() {
        for second in (first + 1)..accepted.len() {
            match accepted[first].key().stable_equals(accepted[second].key()) {
                Some(false) => {}
                Some(true) => return Err(VirtualLayoutRetainedBatchError::DuplicateItem),
                None => return Err(VirtualLayoutRetainedBatchError::UnstableItemKey),
            }
        }
    }

    if supplied.len() < accepted.len() {
        return Err(VirtualLayoutRetainedBatchError::MissingItem);
    }
    if supplied.len() > accepted.len() {
        return Err(VirtualLayoutRetainedBatchError::ExtraItem);
    }

    let mut used = vec![false; accepted.len()];
    let mut ordered = Vec::with_capacity(supplied.len());
    for (key, view, slot) in supplied {
        let mut match_index = None;
        for (index, item) in accepted.iter().enumerate() {
            match key.stable_equals(item.key()) {
                Some(true) if used[index] => {
                    return Err(VirtualLayoutRetainedBatchError::DuplicateItem);
                }
                Some(true) => {
                    match_index = Some(index);
                    break;
                }
                Some(false) => {}
                None => return Err(VirtualLayoutRetainedBatchError::UnstableItemKey),
            }
        }
        let Some(index) = match_index else {
            return Err(VirtualLayoutRetainedBatchError::ExtraItem);
        };
        used[index] = true;
        ordered.push((index, accepted[index].clone(), view, slot));
    }
    if used.iter().any(|was_used| !was_used) {
        return Err(VirtualLayoutRetainedBatchError::MissingItem);
    }
    ordered.sort_by_key(|(index, ..)| *index);

    let fence = commit.fence();
    for (_, _, _, slot) in &ordered {
        if slot.container_id() != fence.container_id()
            || slot.mount_generation() != fence.mount_generation()
        {
            return Err(VirtualLayoutRetainedBatchError::SlotScopeMismatch);
        }
        if slot.checked_generation() == 0
            || ordered.iter().any(|(_, _, _, other)| {
                other.slot_index() == slot.slot_index() && !std::ptr::eq(other, slot)
            })
        {
            return Err(VirtualLayoutRetainedBatchError::SlotCollision);
        }
    }

    let mut metadata = Vec::with_capacity(ordered.len());
    let mut lowering_inputs = Vec::with_capacity(ordered.len());
    for (_, item, view, slot) in ordered {
        metadata.push((item, slot));
        lowering_inputs.push((
            view,
            slot.mount_generation(),
            slot.slot_index(),
            slot.checked_generation(),
        ));
    }
    let lowered = lower_virtual_layout_batch(shell, fence.container_id(), lowering_inputs)
        .map_err(VirtualLayoutRetainedBatchError::Lowering)?;

    let items = metadata
        .into_iter()
        .zip(lowered.items)
        .map(|((item, slot), payload)| VirtualLayoutRetainedItem {
            item,
            slot,
            payload,
        })
        .collect();
    Ok(VirtualLayoutRetainedBatch {
        shell: lowered.shell,
        items,
    })
}

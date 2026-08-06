//! Private pure adapter from an accepted keyed window to retained payloads.
//!
//! This is the executable Slice 4 boundary.  It admits the complete shell and
//! active item batch before returning any lowered payload, but deliberately
//! stops before runtime registration and lifecycle callbacks.

use super::coordinator::VirtualLayoutCommit;
use super::materialization::{
    VirtualLayoutHostProjector, VirtualLayoutProjection, VirtualLayoutProjectionEvidence,
    VirtualLayoutProjectionKind, VirtualLayoutSlotIdentity,
};
use super::{VIRTUAL_LAYOUT_MAX_QUERY_ENTRIES, VirtualLayoutItem, VirtualLayoutItemKey};
use crate::application::{View, VirtualLayoutViewAdmissionError, lower_virtual_layout_batch};
use crate::layout::NodeId;
use crate::runtime::SurfaceNode;
use std::{cell::RefCell, rc::Rc};

type VirtualLayoutShellProjector<Message> = Rc<dyn Fn() -> View<Message>>;
type VirtualLayoutItemProjector<Message> = Rc<dyn Fn(&VirtualLayoutItem) -> View<Message>>;
type VirtualLayoutItemKindProjector =
    Rc<dyn Fn(&VirtualLayoutItem) -> super::VirtualLayoutPolicyIdentity>;
type VirtualLayoutItemLowerer<Message> = Rc<
    dyn Fn(
        View<Message>,
        NodeId,
        u64,
        usize,
        u64,
    ) -> Result<SurfaceNode<Message>, VirtualLayoutRetainedBatchError>,
>;
type VirtualLayoutBatchAdmitter<Message> = Rc<
    dyn Fn(
        &VirtualLayoutCommit,
        View<Message>,
        Vec<(
            VirtualLayoutItemKey,
            View<Message>,
            VirtualLayoutSlotIdentity,
        )>,
    ) -> Result<VirtualLayoutRetainedBatch<Message>, VirtualLayoutRetainedBatchError>,
>;

/// Typed failures from complete shell-plus-batch admission.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum VirtualLayoutRetainedBatchError {
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

/// Whole-batch host projector used by the runtime bridge.
///
/// The store supplies the exact slot identities selected during its private
/// planning phase. This projector consumes those identities in one call so
/// shell and item identity admission completes before lifecycle callbacks.
pub(crate) struct VirtualLayoutBatchProjector<Message> {
    shell: RefCell<Option<View<Message>>>,
    item_projector: VirtualLayoutItemProjector<Message>,
    kind_projector: VirtualLayoutItemKindProjector,
    item_lowerer: VirtualLayoutItemLowerer<Message>,
    batch_admitter: VirtualLayoutBatchAdmitter<Message>,
    projected_shell: RefCell<Option<SurfaceNode<Message>>>,
}

impl<Message> VirtualLayoutBatchProjector<Message> {
    fn new(
        shell: View<Message>,
        item_projector: VirtualLayoutItemProjector<Message>,
        kind_projector: VirtualLayoutItemKindProjector,
        item_lowerer: VirtualLayoutItemLowerer<Message>,
        batch_admitter: VirtualLayoutBatchAdmitter<Message>,
    ) -> Self {
        Self {
            shell: RefCell::new(Some(shell)),
            item_projector,
            kind_projector,
            item_lowerer,
            batch_admitter,
            projected_shell: RefCell::new(None),
        }
    }

    pub(crate) fn factory(
        shell: VirtualLayoutShellProjector<Message>,
        item_projector: VirtualLayoutItemProjector<Message>,
        kind_projector: VirtualLayoutItemKindProjector,
    ) -> Rc<dyn Fn() -> Self>
    where
        Message: 'static,
    {
        Rc::new(move || {
            let item_lowerer = Rc::new(
                |node: View<Message>,
                 container_id: NodeId,
                 mount_generation: u64,
                 slot_index: usize,
                 checked_generation: u64| {
                    crate::application::lower_virtual_layout_item(
                        node,
                        container_id,
                        mount_generation,
                        slot_index,
                        checked_generation,
                    )
                    .map_err(VirtualLayoutRetainedBatchError::Lowering)
                },
            );
            let batch_admitter = Rc::new(
                |commit: &VirtualLayoutCommit,
                 shell: View<Message>,
                 supplied: Vec<(
                    VirtualLayoutItemKey,
                    View<Message>,
                    VirtualLayoutSlotIdentity,
                )>| { admit_virtual_layout_batch(commit, shell, supplied) },
            );
            Self::new(
                shell(),
                Rc::clone(&item_projector),
                Rc::clone(&kind_projector),
                item_lowerer,
                batch_admitter,
            )
        })
    }

    pub(crate) fn take_shell(&self) -> Option<SurfaceNode<Message>> {
        self.projected_shell.borrow_mut().take()
    }
}

impl<Message> VirtualLayoutHostProjector for VirtualLayoutBatchProjector<Message> {
    type Payload = SurfaceNode<Message>;
    type Error = VirtualLayoutRetainedBatchError;

    fn projection_kind(
        &self,
        _item: &VirtualLayoutItem,
    ) -> Result<VirtualLayoutProjectionKind, Self::Error> {
        Ok(VirtualLayoutProjectionKind::new((self.kind_projector)(
            _item,
        )))
    }

    fn project<'a>(
        &self,
        evidence: VirtualLayoutProjectionEvidence<'a>,
    ) -> Result<VirtualLayoutProjection<Self::Payload>, Self::Error> {
        let payload = (self.item_lowerer)(
            (self.item_projector)(evidence.item()),
            evidence.fence().container_id(),
            evidence.proposed_slot().mount_generation(),
            evidence.proposed_slot().slot_index(),
            evidence.proposed_slot().checked_generation(),
        )?;
        Ok(VirtualLayoutProjection::new(
            VirtualLayoutProjectionKind::new((self.kind_projector)(evidence.item())),
            payload,
        ))
    }

    fn project_batch<'a>(
        &self,
        commit: &VirtualLayoutCommit,
        evidence: &[VirtualLayoutProjectionEvidence<'a>],
    ) -> Result<Option<Vec<VirtualLayoutProjection<Self::Payload>>>, Self::Error> {
        let shell =
            self.shell
                .borrow_mut()
                .take()
                .ok_or(VirtualLayoutRetainedBatchError::Lowering(
                    VirtualLayoutViewAdmissionError::LoweringPanicked,
                ))?;
        let supplied = evidence
            .iter()
            .map(|evidence| {
                (
                    evidence.key().clone(),
                    (self.item_projector)(evidence.item()),
                    evidence.proposed_slot(),
                )
            })
            .collect();
        let batch = (self.batch_admitter)(commit, shell, supplied)?;
        self.projected_shell.replace(Some(batch.shell));
        Ok(Some(
            batch
                .items
                .into_iter()
                .map(|item| {
                    let _ = item.slot;
                    VirtualLayoutProjection::new(
                        VirtualLayoutProjectionKind::new((self.kind_projector)(&item.item)),
                        item.payload,
                    )
                })
                .collect(),
        ))
    }
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

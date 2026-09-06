//! Deferred terminal delivery through the outgoing scroll mapper.
use super::super::ScrollDragCapture;
use super::{ScrollEditBatch, SurfaceRuntime};
use crate::{
    gui::types::Vector2,
    layout::NodeId,
    runtime::{RuntimeBridge, ScrollEditMessageMapper},
    widgets::{EditEvent, EditTransaction, InteractionProvenance, PointerModifiers},
};

struct RetiringEdit<Message> {
    node_id: NodeId,
    event: EditEvent<Vector2>,
    mapper: ScrollEditMessageMapper<Message>,
}
struct PointerRetirement<Message> {
    capture: ScrollDragCapture,
    mapper: Option<ScrollEditMessageMapper<Message>>,
    source: Option<crate::runtime::FrozenSourceMetadata>,
}
struct WheelRetirement<Message> {
    transaction: EditTransaction,
    sources: Vec<(NodeId, Option<crate::runtime::FrozenSourceMetadata>)>,
    published: Vec<RetiringEdit<Message>>,
}
pub(in crate::runtime::controller) struct ScrollEditRetirement<Message> {
    pointer: Option<PointerRetirement<Message>>,
    wheel: Option<WheelRetirement<Message>>,
}

fn cancellation_provenance() -> InteractionProvenance {
    InteractionProvenance::Pointer {
        modifiers: PointerModifiers::default(),
        timestamp: None,
        sequence_range: None,
    }
}
fn queue_cancel<Message>(edit: RetiringEdit<Message>, messages: &mut Vec<Message>) {
    if let Some(cancel) = edit.event.cancel(cancellation_provenance())
        && let Some(batch) = ScrollEditBatch::new(edit.node_id, &[cancel], None)
        && let Some(message) = (edit.mapper)(batch)
    {
        messages.push(message);
    }
}

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    /// Clone only active edit routing evidence, without invoking application code.
    pub(in crate::runtime::controller) fn prepare_scroll_edit_retirement(
        &self,
    ) -> ScrollEditRetirement<Message> {
        let pointer =
            self.interaction
                .pointer
                .scroll_drag_capture
                .map(|capture| PointerRetirement {
                    capture,
                    mapper: self.surface.root().scroll_edit_mapper(capture.node_id),
                    source: self.surface.root().scroll_edit_source(capture.node_id),
                });
        let wheel = self
            .interaction
            .wheel
            .scroll_edit
            .as_ref()
            .map(|sequence| WheelRetirement {
                transaction: sequence.transaction,
                sources: sequence
                    .owners
                    .iter()
                    .map(|owner| {
                        (
                            owner.node_id,
                            self.surface.root().scroll_edit_source(owner.node_id),
                        )
                    })
                    .collect(),
                published: sequence
                    .owners
                    .iter()
                    .filter(|owner| owner.published)
                    .filter_map(|owner| {
                        self.surface
                            .root()
                            .scroll_edit_mapper(owner.node_id)
                            .map(|mapper| RetiringEdit {
                                node_id: owner.node_id,
                                event: owner.edit,
                                mapper,
                            })
                    })
                    .collect(),
            });
        ScrollEditRetirement { pointer, wheel }
    }

    /// New surface/layout state is committed before any terminal message reduces.
    pub(in crate::runtime::controller) fn retire_scroll_edits_after_refresh(
        &mut self,
        snapshot: ScrollEditRetirement<Message>,
        messages: &mut Vec<Message>,
    ) {
        if let Some(pointer) = snapshot.pointer {
            let old = pointer.capture;
            let same = self
                .interaction
                .pointer
                .scroll_drag_capture
                .is_some_and(|current| current.edit.transaction == old.edit.transaction);
            let live = same
                && self.scrollbar_edit_geometry_matches(old)
                && self.layout_state.scroll_offset(old.node_id) == old.edit.value
                && self.surface.root().scroll_edit_source(old.node_id) == pointer.source;
            if !live {
                if same {
                    self.interaction.pointer.scroll_drag_capture = None;
                    self.interaction
                        .pointer
                        .set_release_tombstone(old.button, true);
                }
                if old.edit_started
                    && let Some(mapper) = pointer.mapper
                {
                    queue_cancel(
                        RetiringEdit {
                            node_id: old.node_id,
                            event: old.edit,
                            mapper,
                        },
                        messages,
                    );
                }
            }
        }
        if let Some(wheel) = snapshot.wheel {
            let same = self
                .interaction
                .wheel
                .scroll_edit
                .as_ref()
                .is_some_and(|current| current.transaction == wheel.transaction);
            if !same
                || !self.scroll_wheel_edit_is_live()
                || wheel.sources.iter().any(|(node_id, source)| {
                    self.surface.root().scroll_edit_source(*node_id) != *source
                })
            {
                self.retire_scroll_wheel_for_refresh(wheel.transaction);
                for edit in wheel.published {
                    queue_cancel(edit, messages);
                }
            }
        }
    }
}

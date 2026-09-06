//! Controlled details-column width edits using the shared edit protocol.
use super::DetailsColumnWidthUpdate;
use crate::widgets::{
    DragHandleMessage, DragHandleMetadata, EditEvent, EditTransaction, InteractionProvenance,
    SliderEditBatch,
};
use std::ops::RangeInclusive;

/// Caller-owned state for one admitted details-column resize.
///
/// Keep this state beside the durable column model. It carries no pointer capture
/// authority; input must come from the column's qualified drag-handle route.
#[derive(Clone, Debug, PartialEq)]
pub struct DetailsColumnResizeEdit {
    column_id: String,
    start_x: f32,
    limits: RangeInclusive<f32>,
    event: EditEvent<f32>,
}
impl DetailsColumnResizeEdit {
    /// Stable identity of the column being resized.
    pub fn column_id(&self) -> &str {
        &self.column_id
    }
    /// Current edit transaction.
    pub const fn transaction(&self) -> EditTransaction {
        self.event.transaction
    }
}

/// One ordered width-edit batch for a stable details column.
#[derive(Clone, Debug, PartialEq)]
pub struct DetailsColumnResizeEditBatch {
    column_id: String,
    events: SliderEditBatch,
    width_change: Option<f32>,
}
impl DetailsColumnResizeEditBatch {
    /// Column identity qualified when this edit was admitted.
    pub fn column_id(&self) -> &str {
        &self.column_id
    }
    /// One to three ordered edit events, all in the same transaction.
    pub fn events(&self) -> &[EditEvent<f32>] {
        self.events.events()
    }
    /// Transaction shared by the events.
    pub const fn transaction(&self) -> EditTransaction {
        self.events.transaction()
    }
    /// Project an effective width update or meaningful rollback for existing reducers.
    pub fn width_update(&self) -> Option<DetailsColumnWidthUpdate> {
        self.width_change.map(|width| DetailsColumnWidthUpdate {
            column_id: self.column_id.clone(),
            width,
        })
    }
    fn new(column_id: &str, events: &[EditEvent<f32>], width_change: Option<f32>) -> Option<Self> {
        Some(Self {
            column_id: column_id.into(),
            events: SliderEditBatch::from_events(events)?,
            width_change,
        })
    }
}

/// Apply one qualified drag-handle sample to a controlled column-width edit.
///
/// Supply the current projected width on every sample. An echo of the last
/// emitted width preserves the edit. A replacement width, changed limits, or
/// removed column (`None`) emits Cancel without rolling back the replacement
/// model. Wrong-column samples and duplicate starts/terminals remain inert.
/// Ordinary cancellation restores the starting width. No-op cancellation is
/// still typed, but has no concise width update.
pub fn update_details_column_resize_edit(
    active: &mut Option<DetailsColumnResizeEdit>,
    column_id: impl AsRef<str>,
    message: DragHandleMessage,
    current_width: Option<f32>,
    min_width: f32,
    max_width: f32,
) -> Option<DetailsColumnResizeEditBatch> {
    let column_id = column_id.as_ref();
    let limits = min_width..=max_width;
    if active
        .as_ref()
        .is_some_and(|edit| edit.column_id != column_id)
    {
        return None;
    }
    if let Some(edit) = active.as_ref()
        && (current_width != Some(edit.event.value)
            || edit.limits != limits
            || !valid_limits(&limits))
    {
        let edit = active.take()?;
        return DetailsColumnResizeEditBatch::new(
            column_id,
            &[edit.event.cancel(provenance(DragHandleMetadata::empty()))?],
            None,
        );
    }
    match message {
        DragHandleMessage::Started {
            origin,
            position,
            metadata,
        } if active.is_none() => {
            let width = current_width.filter(|width| valid_width(*width, &limits))?;
            if !origin.x.is_finite() || !position.x.is_finite() {
                return None;
            }
            let candidate = candidate_width(width, origin.x, position.x, &limits)?;
            let provenance = provenance(metadata);
            let begin = EditEvent::begin(width, provenance);
            let changed = differs(width, candidate);
            let event = if changed {
                begin.update(candidate, provenance)?
            } else {
                begin
            };
            let batch = if changed {
                DetailsColumnResizeEditBatch::new(column_id, &[begin, event], Some(candidate))
            } else {
                DetailsColumnResizeEditBatch::new(column_id, &[begin], None)
            };
            *active = Some(DetailsColumnResizeEdit {
                column_id: column_id.into(),
                start_x: origin.x,
                limits,
                event,
            });
            batch
        }
        DragHandleMessage::Moved { position, metadata }
        | DragHandleMessage::Ended { position, metadata } => {
            let previous = active.as_ref()?;
            let Some(candidate) = candidate_width(
                previous.event.start_value,
                previous.start_x,
                position.x,
                &limits,
            ) else {
                // An invalid terminal still retires the edit; malformed motion stays inert.
                return if message.is_ended() {
                    cancel(active)
                } else {
                    None
                };
            };
            let changed = differs(previous.event.value, candidate);
            let provenance = provenance(metadata);
            let event = if changed {
                previous.event.update(candidate, provenance)?
            } else {
                previous.event
            };
            if message.is_ended() {
                let commit = event.commit(candidate, provenance)?;
                *active = None;
                if changed {
                    DetailsColumnResizeEditBatch::new(column_id, &[event, commit], Some(candidate))
                } else {
                    DetailsColumnResizeEditBatch::new(column_id, &[commit], None)
                }
            } else {
                active.as_mut()?.event = event;
                if changed {
                    DetailsColumnResizeEditBatch::new(column_id, &[event], Some(candidate))
                } else {
                    None
                }
            }
        }
        DragHandleMessage::Cancelled { .. } => cancel(active),
        _ => None,
    }
}

/// Produce an atomic keyboard, wheel, semantic, or programmatic column-width edit.
///
/// The caller supplies the already-qualified candidate and its actual provenance;
/// this helper invents no key or pointer samples and no product-specific step policy.
/// It refuses to join an active pointer resize. Out-of-range finite candidates
/// clamp to the supplied limits; invalid values and effective no-ops are inert.
pub fn details_column_width_edit(
    active: &Option<DetailsColumnResizeEdit>,
    column_id: impl AsRef<str>,
    current_width: f32,
    candidate: f32,
    limits: RangeInclusive<f32>,
    provenance: InteractionProvenance,
) -> Option<DetailsColumnResizeEditBatch> {
    if active.is_some() || !valid_width(current_width, &limits) || !candidate.is_finite() {
        return None;
    }
    let candidate = candidate.clamp(*limits.start(), *limits.end());
    if !differs(current_width, candidate) {
        return None;
    }
    let begin = EditEvent::begin(current_width, provenance);
    let update = begin.update(candidate, provenance)?;
    let commit = update.commit(candidate, provenance)?;
    DetailsColumnResizeEditBatch::new(
        column_id.as_ref(),
        &[begin, update, commit],
        Some(candidate),
    )
}

fn cancel(active: &mut Option<DetailsColumnResizeEdit>) -> Option<DetailsColumnResizeEditBatch> {
    let edit = active.take()?;
    let changed = differs(edit.event.value, edit.event.start_value);
    DetailsColumnResizeEditBatch::new(
        &edit.column_id,
        &[edit.event.cancel(provenance(DragHandleMetadata::empty()))?],
        changed.then_some(edit.event.start_value),
    )
}
fn provenance(metadata: DragHandleMetadata) -> InteractionProvenance {
    InteractionProvenance::Pointer {
        modifiers: metadata.modifiers,
        timestamp: metadata.timestamp,
        sequence_range: metadata.sequence_range,
    }
}
fn valid_limits(limits: &RangeInclusive<f32>) -> bool {
    limits.start().is_finite()
        && limits.end().is_finite()
        && *limits.start() >= 0.0
        && limits.start() <= limits.end()
}
fn valid_width(width: f32, limits: &RangeInclusive<f32>) -> bool {
    valid_limits(limits) && width.is_finite() && limits.contains(&width)
}
fn candidate_width(
    start: f32,
    start_x: f32,
    pointer_x: f32,
    limits: &RangeInclusive<f32>,
) -> Option<f32> {
    if !pointer_x.is_finite() {
        return None;
    }
    // Wider intermediate arithmetic avoids overflow for finite but distant positions.
    let candidate = (f64::from(pointer_x) - f64::from(start_x)) + f64::from(start);
    candidate
        .is_finite()
        .then(|| candidate.clamp(f64::from(*limits.start()), f64::from(*limits.end())) as f32)
}
fn differs(a: f32, b: f32) -> bool {
    (a - b).abs() > f32::EPSILON
}

#[cfg(test)]
mod tests;

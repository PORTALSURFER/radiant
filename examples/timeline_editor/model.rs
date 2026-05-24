#[path = "model/projection.rs"]
mod projection;
#[path = "model/types.rs"]
mod types;
#[path = "model/update.rs"]
mod update;

#[cfg(test)]
pub(super) use projection::beat_to_normalized;
pub(super) use projection::{timeline_label, timeline_surface};
#[cfg(test)]
pub(super) use types::TimelineClipParts;
pub(super) use types::{
    BeatRange, TimelineClip, TimelineEditorState, TimelineMessage, TimelineSurfaceMessage,
};
pub(super) use update::update;
#[cfg(test)]
pub(super) use update::{delete_selected_clip, update_surface};

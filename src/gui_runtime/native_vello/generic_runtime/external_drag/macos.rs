//! macOS outgoing file-drag platform implementation.
//!
//! The public runtime entrypoint stays here. Objective-C/AppKit ABI calls, pasteboard payload
//! construction, and `NSDraggingSource` ownership live in focused sibling modules.

#[path = "macos/bridge.rs"]
mod bridge;
#[path = "macos/payload.rs"]
mod payload;
#[path = "macos/source.rs"]
mod source;

use crate::runtime::{ExternalDragPayload, ExternalDragRequest};
use std::time::Instant;
use tracing::debug;

pub(super) fn start_external_drag(
    request: &ExternalDragRequest,
    context: super::ExternalDragLaunchContext,
) -> Result<super::ExternalDragLaunchDisposition, String> {
    let startup_started_at = Instant::now();
    let ExternalDragPayload::Files(paths) = &request.payload;
    if paths.is_empty() {
        return Err(String::from("No files to drag"));
    }
    let window_id = context
        .window_id
        .ok_or_else(|| String::from("Native external drag has no originating window"))?;
    let event_proxy = context
        .event_proxy
        .ok_or_else(|| String::from("Native external drag event-loop proxy was not installed"))?;

    let _pool = bridge::AutoreleasePool::new()?;
    let app = unsafe { bridge::shared_application()? };
    let (window, view) = unsafe { bridge::key_window_and_content_view(app)? };
    let event = unsafe { bridge::external_drag_event(app, window)? };
    let items_started_at = Instant::now();
    let items = unsafe { payload::dragging_items(paths)? };
    let items_elapsed = items_started_at.elapsed();
    let mut source = unsafe { source::dragging_source(event_proxy, window_id, context.identity)? };
    unsafe { bridge::begin_dragging_session(view, items, event, source.source())? };
    source.commit_to_session();
    debug!(
        target: "radiant::external_drag",
        event = "external_drag.macos.session_started",
        path_count = paths.len(),
        preview = ?payload::drag_preview_kind(paths.len()),
        item_build_ms = items_elapsed.as_secs_f64() * 1000.0,
        startup_ms = startup_started_at.elapsed().as_secs_f64() * 1000.0,
        "macOS external drag session started"
    );

    Ok(super::ExternalDragLaunchDisposition::Pending)
}

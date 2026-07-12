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

use crate::runtime::{
    ExternalDragEffect, ExternalDragOutcome, ExternalDragPayload, ExternalDragRequest,
};
use std::time::Instant;
use tracing::debug;

pub(super) fn start_external_drag(
    request: &ExternalDragRequest,
) -> Result<ExternalDragOutcome, String> {
    let startup_started_at = Instant::now();
    let ExternalDragPayload::Files(paths) = &request.payload;
    if paths.is_empty() {
        return Err(String::from("No files to drag"));
    }

    let _pool = bridge::AutoreleasePool::new()?;
    let app = unsafe { bridge::shared_application()? };
    let (window, view) = unsafe { bridge::key_window_and_content_view(app)? };
    let event = unsafe { bridge::external_drag_event(app, window)? };
    let items_started_at = Instant::now();
    let items = unsafe { payload::dragging_items(paths)? };
    let items_elapsed = items_started_at.elapsed();
    let source = unsafe { source::dragging_source()? };
    unsafe { bridge::begin_dragging_session(view, items, event, source)? };
    debug!(
        target: "radiant::external_drag",
        event = "external_drag.macos.session_started",
        path_count = paths.len(),
        preview = ?payload::drag_preview_kind(paths.len()),
        item_build_ms = items_elapsed.as_secs_f64() * 1000.0,
        startup_ms = startup_started_at.elapsed().as_secs_f64() * 1000.0,
        "macOS external drag session started"
    );

    Ok(ExternalDragOutcome {
        effect: ExternalDragEffect::Copy,
    })
}

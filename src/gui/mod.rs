//! Backend-agnostic GUI primitives for host applications.
//!
//! Architectural guarantees:
//! - This module owns input normalization, hit testing policy, and the retained
//!   update model interface used by application code.
//! - The host application supplies domain actions and state; `radiant` owns
//!   layout, repaint invalidation, and scene reconciliation.
//! - Rendering orchestration is performed by `radiant` runtimes in `gui_runtime`.
//!
//! Update/diff model:
//! - App code emits a declarative UI model plus action payloads.
//! - `radiant` diffs successive models to identify invalidated subtrees.
//! - Only invalidated regions are rebuilt and sent to the active render backend.
//!
//! Event propagation model:
//! - Host-native events are normalized into the token set in this module.
//! - `radiant` performs deterministic focus, pointer capture, and key routing.
//! - App code receives action callbacks and updates domain state only.

/// Input event primitives shared by UI code.
pub mod input;
/// Strict slot-based layout core used by retained containers.
pub(crate) mod layout_core;
/// Native shell layout + scene model used by the Vello backend.
pub(crate) mod native_shell;
/// Backend-neutral repaint signaling primitives used by runtimes and background jobs.
pub mod repaint;
/// Geometry and image buffer types shared by UI code.
pub mod types;

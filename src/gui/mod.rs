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

/// Frame feedback primitives shared by runtimes and render backends.
pub mod frame;
/// Input event primitives shared by UI code.
pub mod input;
/// Domain-neutral invalidation primitives for retained UI updates.
pub mod invalidation;
/// Public slot-based layout engine and container model.
pub mod layout_core;
/// Native shell layout + scene model kept as Sempal compatibility infrastructure.
pub(crate) mod native_shell;
/// Backend-neutral repaint signaling primitives used by runtimes and background jobs.
pub mod repaint;
/// Retained snapshot storage primitives.
pub mod retained;
/// Geometry and image buffer types shared by UI code.
pub mod types;

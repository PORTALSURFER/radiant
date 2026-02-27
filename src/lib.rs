//! `radiant`: reusable GUI primitives and runtimes for Sempal.
//!
//! The crate is organized as a thin runtime boundary:
//! - `app`: model/action contracts emitted and consumed by the host.
//! - `gui`: retained layout, input mapping, and paint generation.
//! - `gui_runtime`: platform host bindings and frame scheduling.
//!
//! The host builds an [`AppModel`](crate::app::AppModel) for each frame and applies
//! [`UiAction`](crate::app::UiAction) events produced by input and interactions.
//! All GUI-specific layout, diffing, and render orchestration stay inside `radiant`.

/// App-facing model/action contracts for runtime integration.
pub mod app;
/// Shared environment-flag parsing helpers used by runtime internals.
mod env_flags;
/// Backend-agnostic GUI primitives.
pub mod gui;
/// Shared runtime host implementations.
pub mod gui_runtime;

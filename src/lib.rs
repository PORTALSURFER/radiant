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

// `radiant` is in the middle of the staged cleanup backlog (`tmp/cleanup_plan.md`
// items 20-23, 26, and 30). The current crate still carries large transitional
// state/layout modules, so newer clippy structural lints would block unrelated
// work until those splits land. Keep this list narrow and remove entries as the
// corresponding cleanup items are completed.
#![allow(clippy::collapsible_if)]
#![allow(clippy::derivable_impls)]
#![allow(clippy::double_ended_iterator_last)]
#![allow(clippy::field_reassign_with_default)]
#![allow(clippy::if_same_then_else)]
#![allow(clippy::into_iter_on_ref)]
#![allow(clippy::manual_clamp)]
#![allow(clippy::manual_is_multiple_of)]
#![allow(clippy::needless_borrow)]
#![allow(clippy::question_mark)]
#![allow(clippy::too_many_arguments)]

/// App-facing model/action contracts for runtime integration.
pub mod app;
/// Shared environment-flag parsing helpers used by runtime internals.
mod env_flags;
/// Backend-agnostic GUI primitives.
pub mod gui;
/// Shared runtime host implementations.
pub mod gui_runtime;

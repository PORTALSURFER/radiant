//! `radiant`: reusable GUI primitives and runtimes for host applications.
//!
//! The crate is organized as a thin runtime boundary:
//! - `app`: compatibility-facing Sempal shell contracts emitted and consumed by the host.
//! - `gui`: retained layout, input mapping, and paint generation.
//! - `gui_runtime`: platform host bindings and frame scheduling.
//! - `runtime`: generic declarative view/message surfaces for new host applications.
//!
//! New host applications should prefer [`runtime`](crate::runtime), which lets
//! them project generic declarative UI trees built from public containers and
//! widgets, then reduce host-defined messages without depending on
//! Sempal-specific [`AppModel`](crate::app::AppModel) or
//! [`UiAction`](crate::app::UiAction) shapes.
//!
//! The legacy [`app`](crate::app) surface remains available as the
//! compatibility path while Sempal migrates onto the generic surface.
//! All GUI-specific layout, diffing, and render orchestration stay inside `radiant`.
//!
//! Generic host-facing entry points:
//! - [`layout`]: stable slot-based layout primitives
//! - [`widgets`]: first-class reusable widget taxonomy and contracts
//! - [`app`]: compatibility-facing app model and action contracts
//! - [`gui_runtime`]: backend runtimes and scheduling
//! - [`runtime`]: generic declarative view/message bridge for new hosts

// `radiant` still carries several large transitional runtime and native-shell
// modules. Keep this list narrow while the active cleanup lane continues to
// split those surfaces into smaller focused modules.
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
/// Stable public slot-based layout API.
pub mod layout {
    pub use crate::gui::layout_core::*;
}
/// Shared runtime host implementations.
pub mod gui_runtime;
/// Generic declarative view/message runtime surface for new hosts.
pub mod runtime;
/// Stable public widget taxonomy and contracts.
pub mod widgets;

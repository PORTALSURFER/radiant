//! `radiant`: reusable GUI primitives and runtimes for host applications.
//!
//! New host applications should start with [`runtime`](crate::runtime),
//! [`widgets`](crate::widgets), [`layout`](crate::layout), and
//! [`theme`](crate::theme). That path lets hosts project generic declarative UI
//! trees, reduce host-defined messages, and run through the native Vello backend
//! without depending on transitional shell DTOs. See the checked `generic_native`
//! example for a small standalone native app.
//! The current compatibility shell is opt-in through the `legacy-shell` feature
//! and is not part of Radiant's default standalone build.
//! See `docs/API.md` for the checked public API boundary and lifecycle model.
//!
//! Generic host-facing modules:
//! - [`layout`]: stable slot-based layout primitives
//! - [`widgets`]: first-class reusable widget taxonomy and contracts
//! - [`gui_runtime`]: backend runtimes and scheduling
//! - [`runtime`]: generic declarative view/message bridge for new hosts
//! - [`theme`]: reusable visual tokens for generic widgets and containers
//!
//! Transitional compatibility lives behind the `legacy-shell` feature while
//! host applications migrate onto generic Radiant APIs.

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
// Included transitional composition modules still refer to `crate::app` when they
// are compiled inside Radiant. Keep this crate-private and feature-gated; do not
// expose it as `radiant::app`.
#[cfg(feature = "legacy-shell")]
pub(crate) mod app {
    pub(crate) use crate::compat_app_contract::*;
}
/// Explicit compatibility namespace for migration-time shell APIs.
#[cfg(feature = "legacy-shell")]
pub mod compat;
/// Shared environment-flag parsing helpers used by runtime internals.
mod env_flags;
/// Backend-agnostic GUI primitives.
pub mod gui;
/// Stable public slot-based layout API.
pub mod layout {
    pub use crate::gui::layout_core::*;
}
#[cfg(feature = "legacy-shell")]
#[path = "compat/legacy_shell/mod.rs"]
pub(crate) mod compat_app_contract;
/// Shared runtime host implementations.
pub mod gui_runtime;
/// Generic declarative view/message runtime surface for new hosts.
pub mod runtime;
/// Generic theme tokens for reusable Radiant widgets and containers.
pub mod theme;
/// Stable public widget taxonomy and contracts.
pub mod widgets;

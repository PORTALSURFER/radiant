//! `radiant`: reusable GUI primitives and runtimes for host applications.
//!
//! New host applications should start with [`runtime`](crate::runtime),
//! [`widgets`](crate::widgets), [`layout`](crate::layout), and
//! [`theme`](crate::theme). That path lets hosts project generic declarative UI
//! trees, reduce host-defined messages, and run through the native Vello backend
//! without depending on Sempal-specific shell DTOs. See the checked
//! `generic_native` example for a small standalone native app.
//!
//! Generic host-facing modules:
//! - [`layout`]: stable slot-based layout primitives
//! - [`widgets`]: first-class reusable widget taxonomy and contracts
//! - [`gui_runtime`]: backend runtimes and scheduling
//! - [`runtime`]: generic declarative view/message bridge for new hosts
//! - [`theme`]: reusable visual tokens for generic widgets and containers
//!
//! Transitional compatibility lives under [`compat::sempal_shell`](crate::compat::sempal_shell).
//! The crate-root [`app`](crate::app) module is a deprecated alias for that
//! Sempal compatibility contract and should not be used by new code.

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
// Transitional compatibility internals still include Sempal-owned shared
// modules that refer to the crate-root alias when compiled inside Radiant.
// External callers still see the deprecation warning on `radiant::app`.
#![allow(deprecated)]

/// Deprecated alias for the Sempal compatibility model/action contract.
///
/// New host applications should use [`runtime`], [`widgets`], [`layout`], and
/// [`theme`]. Migration-time Sempal shell consumers should import the explicit
/// [`compat::sempal_shell`] namespace instead.
#[deprecated(
    since = "0.1.0",
    note = "use radiant::runtime for generic apps, or radiant::compat::sempal_shell for the transitional Sempal shell"
)]
pub mod app {
    pub use crate::sempal_app::*;
}
/// Explicit compatibility namespace for migration-time Sempal shell APIs.
pub mod compat;
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
#[path = "app/mod.rs"]
pub(crate) mod sempal_app;
/// Generic theme tokens for reusable Radiant widgets and containers.
pub mod theme;
/// Stable public widget taxonomy and contracts.
pub mod widgets;

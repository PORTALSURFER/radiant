//! Compatibility surfaces preserved while host applications migrate onto generic Radiant APIs.
//!
//! This namespace is transitional. Host-shaped legacy shell contracts remain
//! here only to keep existing applications running while they move those DTOs
//! and adapters into the host codebase and consume Radiant's generic runtime,
//! layout, and widget APIs directly.
//!
//! New host applications should prefer [`crate::layout`], [`crate::widgets`],
//! and [`crate::runtime`]. This module exists so the current compatibility
//! shell can keep running without pretending to be the preferred core API
//! interface.

#[cfg(feature = "legacy-shell")]
pub mod runtime_artifacts;

/// Compatibility namespace for the current legacy shell contract.
///
/// This namespace groups the legacy model/action bridge plus the native-shell
/// runtime entry points that still depend on those compatibility contracts.
/// Keep new generic work out of this module unless it is explicitly about
/// compatibility or migration support. The final disposition for the
/// `legacy_shell` contract is host-owned.
#[cfg(feature = "legacy-shell")]
pub mod legacy_shell {
    pub use crate::compat_app_contract::*;
}

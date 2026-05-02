//! Compatibility surfaces preserved while host applications migrate onto generic Radiant APIs.
//!
//! New host applications should prefer [`crate::layout`], [`crate::widgets`],
//! and [`crate::runtime`]. This module exists so the current compatibility
//! shell can keep running without pretending to be the preferred core API
//! interface.

/// Compatibility namespace for the current legacy shell contract.
///
/// This namespace groups the legacy model/action bridge plus the native-shell
/// runtime entry points that still depend on those compatibility contracts.
/// Keep new generic work out of this module unless it is explicitly about
/// compatibility or migration support.
#[cfg(feature = "legacy-shell")]
pub mod legacy_shell {
    pub use crate::compat_app_contract::*;
    pub use crate::gui_runtime::{NativeRunOptions, NativeStartupTimingArtifact, WindowIconRgba};
}

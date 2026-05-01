//! Compatibility surfaces preserved while Sempal migrates onto generic Radiant APIs.
//!
//! New host applications should prefer [`crate::layout`], [`crate::widgets`],
//! and [`crate::runtime`]. This module exists so the current Sempal-shaped
//! shell can keep running without pretending to be the preferred core library
//! interface.

/// Compatibility namespace for the current Sempal-shaped shell contract.
///
/// This namespace groups the legacy model/action bridge plus the native-shell
/// runtime entry points that still depend on those Sempal-specific contracts.
/// Keep new generic work out of this module unless it is explicitly about
/// compatibility or migration support.
pub mod sempal_shell {
    pub use crate::compat_app_contract::*;
    pub use crate::gui_runtime::{
        NativeRunOptions, NativeRunReport, NativeRuntimeArtifacts, NativeStartupTimingArtifact,
        WindowIconRgba, capture_gui_automation_snapshot, capture_native_shell_shot_snapshot,
        run_native_vello_app, run_native_vello_app_declarative,
        run_native_vello_app_declarative_with_artifacts, run_native_vello_app_with_artifacts,
        run_native_vello_preview,
    };
}

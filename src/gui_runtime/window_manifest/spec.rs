use crate::gui_runtime::NativeRunOptions;

mod accessors;
mod builders;
mod conversion;

use super::{WindowSpecError, validation::validate_window_spec};

/// Platform-neutral descriptor for one application window.
///
/// `WindowSpec` is intentionally a manifest object, not an event-loop runtime.
/// Hosts that need multiple windows can keep a collection of specs, attach a
/// separate runtime bridge per spec, and let a platform adapter decide how to
/// open or embed each surface.
#[derive(Clone, Debug, PartialEq)]
pub struct WindowSpec {
    /// Stable host-owned key for this window.
    pub key: String,
    /// Native launch options for this window.
    pub options: NativeRunOptions,
}

/// Named fields for constructing a platform-neutral window descriptor.
#[derive(Clone, Debug, PartialEq)]
pub struct WindowSpecParts {
    /// Stable host-owned key for this window.
    pub key: String,
    /// Native launch options for this window.
    pub options: NativeRunOptions,
}

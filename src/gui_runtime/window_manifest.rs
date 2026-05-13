//! Host-owned native window descriptors and manifest validation.

mod manifest;
mod spec;

pub use manifest::{WindowManifest, WindowManifestError};
pub use spec::{WindowSpec, WindowSpecError};

//! Host-owned native window descriptors and manifest validation.

mod manifest;
mod spec;
mod validation;

pub use manifest::{WindowManifest, WindowManifestError};
pub use spec::{WindowSpec, WindowSpecParts};
pub use validation::WindowSpecError;

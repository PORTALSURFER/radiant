//! Common imports for Radiant applications.
//!
//! The public prelude is grouped by owning subsystem so additions stay close to
//! the API area they belong to instead of accumulating in one giant export list.
//! Keep this file as the facade and add new exports to the matching sibling
//! module under `src/prelude/`.
//!
//! Advanced host-control APIs stay on their owning modules:
//!
//! ```compile_fail
//! use radiant::prelude::*;
//! let _: Option<NativeFrameDiagnostics> = None;
//! ```
//!
//! ```compile_fail
//! use radiant::prelude::*;
//! let _: Option<SurfacePaintPlan> = None;
//! ```
//!
//! ```compile_fail
//! use radiant::prelude::*;
//! let _: Option<TimelineViewport> = None;
//! ```

mod application;
mod gui;
mod layout;
mod runtime;
mod theme;
mod widgets;

pub use application::*;
pub use gui::*;
pub use layout::*;
pub use runtime::*;
pub use theme::*;
pub use widgets::*;

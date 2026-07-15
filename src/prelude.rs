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
//!
//! Specialist details-list APIs stay on `radiant::application`:
//!
//! ```compile_fail
//! use radiant::prelude::*;
//! let _: Option<DetailsColumnResizeDrag> = None;
//! ```
//!
//! Low-level paint construction stays on `radiant::runtime` even though the
//! `PaintPrimitive` signature type remains common for `Widget` implementations:
//!
//! ```compile_fail
//! use radiant::prelude::*;
//! let _: Option<PaintFillRect> = None;
//! ```
//!
//! Raw platform protocols and external-drag models also require explicit
//! `radiant::runtime` imports:
//!
//! ```compile_fail
//! use radiant::prelude::*;
//! let _: Option<PlatformRequest> = None;
//! ```
//!
//! ```compile_fail
//! use radiant::prelude::*;
//! let _: Option<ExternalDragRequest> = None;
//! ```
//!
//! Named-parts construction models stay on their owning modules:
//!
//! ```compile_fail
//! use radiant::prelude::*;
//! let _: Option<StatusSegmentsParts> = None;
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

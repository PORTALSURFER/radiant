//! `radiant`: reusable GUI primitives and runtimes for host applications.
//!
//! New host applications should start with [`prelude`](crate::prelude) for
//! simple windows and stateful apps, then graduate to [`runtime`](crate::runtime),
//! [`widgets`](crate::widgets), [`layout`](crate::layout), and
//! [`theme`](crate::theme) when they need explicit runtime control. Both paths
//! lower into the same generic declarative UI tree and native Vello backend
//! without depending on host-shaped shell DTOs. See the checked `hello_world`
//! example for the beginner path and `generic_native` for explicit runtime use.
//! See `docs/API.md` for the checked public API boundary and lifecycle model.
//!
//! Generic host-facing modules:
//! - [`layout`]: stable slot-based layout primitives
//! - [`widgets`]: first-class reusable widget taxonomy and contracts
//! - [`gui_runtime`]: backend runtimes and scheduling
//! - [`runtime`]: generic declarative view/message bridge for new hosts
//! - [`theme`]: reusable visual tokens for generic widgets and containers

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

/// Shared environment-flag parsing helpers used by runtime internals.
mod env_flags;
/// Beginner-facing application and view builders.
pub mod ergonomic;
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
/// Generic theme tokens for reusable Radiant widgets and containers.
pub mod theme;
/// Stable public widget taxonomy and contracts.
pub mod widgets;

/// Beginner-facing imports for simple Radiant apps.
pub mod prelude {
    pub use crate::Result;
    pub use crate::ergonomic::{
        IntoView, RunnableStatefulApp, StatefulAppBuilder, StatefulAppWithView, ViewNode,
        WindowBuilder, app, button, button_mapped, column, row, text, text_input, toggle, window,
    };
    pub use crate::runtime::Command;
}

pub use ergonomic::{Result, app, window};

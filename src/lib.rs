//! `radiant`: reusable GUI primitives and runtimes for host applications.
//!
//! Radiant exposes one public API with progressive control. Applications can
//! start with [`prelude`](crate::prelude) for readable window, app, and view
//! builders, then name [`runtime`](crate::runtime), [`widgets`](crate::widgets),
//! [`layout`](crate::layout), and [`theme`](crate::theme) objects when they need
//! more explicit control. All of those entry points lower into the same generic
//! declarative UI tree and native Vello backend without depending on host-shaped
//! shell DTOs. See the checked `hello_world`, `counter`, and `generic_native`
//! examples for application patterns.
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

/// Readable application and view builder implementation.
mod application;
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
/// Generic theme tokens for reusable Radiant widgets and containers.
pub mod theme;
/// Stable public widget taxonomy and contracts.
pub mod widgets;

/// Common imports for Radiant apps.
pub mod prelude {
    pub use crate::Result;
    pub use crate::application::{
        ButtonBuilder, IntoView, RunnableStatefulApp, StateAction, StateView, StatefulAppBuilder,
        StatefulAppWithView, TextInputBuilder, ToggleBuilder, View, ViewNode, WindowBuilder, app,
        button, button_mapped, button_message, checkbox, column, column_key, list, list_row, row,
        row_key, scroll, scroll_column, text, text_input, text_input_mapped, toggle, toggle_mapped,
        window,
    };
    pub use crate::runtime::Command;
}

pub use application::{Result, app, window};

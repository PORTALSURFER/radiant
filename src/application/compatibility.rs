//! Compatibility-only direct state-callback application types.
//!
//! New application code should use `View<Message>` plus explicit update
//! handlers. This module remains only for older composite helpers while they
//! are migrated to message-first APIs.

pub use super::state::StateAction;

/// Application view node type for direct state-callback apps.
pub type StateView<State> = super::View<StateAction<State>>;

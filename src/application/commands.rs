//! Contextual semantic commands and portable, data-only keymaps.
//!
//! Registries own static metadata. Applications project active immutable scopes;
//! resolution selects one typed invocation without performing a domain action.

mod declarative;
mod dispatch;
mod host;
mod keymap;
mod keys;
mod presentation;
mod registry;
mod resolution;
mod scope;
mod validation;

pub use declarative::CommandScopeProjection;
pub(crate) use declarative::{CommandScopeAttachment, ResolvedCommandScope};
pub use dispatch::{CommandDispatch, CommandDispatchStatus, CommandDispatcher};
pub use host::{CommandFocus, CommandRequest, CommandSnapshot};
pub use keymap::{Keymap, KeymapDiagnostic, KeymapError, KeymapProblem};
pub use keys::{CommandInput, CommandKey, CommandModifiers, CommandShortcut};
pub use presentation::CommandPresentation;
pub use registry::{CommandDescriptor, CommandId, CommandRegistrationError, CommandRegistry};
pub use resolution::{CommandConflict, CommandResolution, CommandSuppression};
pub use scope::{
    CommandBinding, CommandInvocation, CommandScope, CommandScopeError, CommandScopeKind,
    CommandSource, CommandTarget,
};
pub use validation::{
    CommandShortcutPresentation, KeymapConflict, KeymapConflictKind, KeymapResolutionChoice,
    KeymapValidation,
};

#[cfg(test)]
mod tests;

#[cfg(test)]
mod host_tests;

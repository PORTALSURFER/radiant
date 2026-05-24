//! Command-returning declarative runtime bridges.

mod owned;
mod shared;

pub use owned::{
    DeclarativeOwnedCommandRuntimeBridge, DeclarativeOwnedCommandRuntimeBridgeParts,
    declarative_owned_command_runtime_bridge,
};
pub use shared::{
    DeclarativeCommandRuntimeBridge, DeclarativeCommandRuntimeBridgeParts,
    declarative_command_runtime_bridge,
};

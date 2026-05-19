//! Closure-driven runtime bridge implementations.

mod command;
mod message;

pub use command::{
    DeclarativeCommandRuntimeBridge, DeclarativeCommandRuntimeBridgeParts,
    DeclarativeOwnedCommandRuntimeBridge, DeclarativeOwnedCommandRuntimeBridgeParts,
    declarative_command_runtime_bridge, declarative_owned_command_runtime_bridge,
};
pub use message::{
    DeclarativeOwnedRuntimeBridge, DeclarativeOwnedRuntimeBridgeParts, DeclarativeRuntimeBridge,
    DeclarativeRuntimeBridgeParts, declarative_owned_runtime_bridge, declarative_runtime_bridge,
};

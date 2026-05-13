//! Closure-driven runtime bridge implementations.

mod command;
mod message;

pub use command::{
    DeclarativeCommandRuntimeBridge, DeclarativeOwnedCommandRuntimeBridge,
    declarative_command_runtime_bridge, declarative_owned_command_runtime_bridge,
};
pub use message::{
    DeclarativeOwnedRuntimeBridge, DeclarativeRuntimeBridge, declarative_owned_runtime_bridge,
    declarative_runtime_bridge,
};

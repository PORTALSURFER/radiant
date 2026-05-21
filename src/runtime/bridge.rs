//! Generic declarative bridge traits for message-driven Radiant hosts.

mod animation;
mod app;
mod contract;
mod declarative;

pub use animation::RuntimeAnimationActivity;
pub use app::App;
pub use contract::{AuxiliaryWindow, RuntimeBridge};
pub use declarative::{
    DeclarativeCommandRuntimeBridge, DeclarativeCommandRuntimeBridgeParts,
    DeclarativeOwnedCommandRuntimeBridge, DeclarativeOwnedCommandRuntimeBridgeParts,
    DeclarativeOwnedRuntimeBridge, DeclarativeOwnedRuntimeBridgeParts, DeclarativeRuntimeBridge,
    DeclarativeRuntimeBridgeParts, declarative_command_runtime_bridge,
    declarative_owned_command_runtime_bridge, declarative_owned_runtime_bridge,
    declarative_runtime_bridge,
};

//! Message-reducing declarative runtime bridges.

mod owned;
mod shared;

pub use owned::{DeclarativeOwnedRuntimeBridge, DeclarativeOwnedRuntimeBridgeParts};
pub use shared::{
    DeclarativeRuntimeBridge, DeclarativeRuntimeBridgeParts, declarative_runtime_bridge,
};

pub use owned::declarative_owned_runtime_bridge;

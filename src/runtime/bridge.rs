//! Generic declarative bridge traits for message-driven Radiant hosts.

mod animation;
mod app;
mod auxiliary;
mod capabilities;
mod contract;
mod declarative;

pub use animation::{RuntimeAnimationActivity, RuntimeAnimationDemand};
pub use app::App;
pub use auxiliary::AuxiliaryWindow;
pub use auxiliary::AuxiliaryWindowClosePolicy;
pub use capabilities::{
    RuntimeAnimationHost, RuntimeDiagnosticsHost, RuntimeFrameDiagnosticsHost,
    RuntimeHostCapabilities, RuntimeInputHost, RuntimeLifecycleHost, RuntimePlatformHost,
    RuntimeQueueHost, RuntimeRetainedSurfaceHost, RuntimeTaskHost, RuntimeTransientOverlayHost,
    RuntimeWindowHost,
};
pub(crate) use capabilities::{RuntimeQueueCapability, RuntimeRetainedSurfaceCapability};
pub use contract::RuntimeBridge;
pub use declarative::{
    DeclarativeCommandRuntimeBridge, DeclarativeCommandRuntimeBridgeParts,
    DeclarativeOwnedCommandRuntimeBridge, DeclarativeOwnedCommandRuntimeBridgeParts,
    DeclarativeOwnedRuntimeBridge, DeclarativeOwnedRuntimeBridgeParts, DeclarativeRuntimeBridge,
    DeclarativeRuntimeBridgeParts, declarative_command_runtime_bridge,
    declarative_owned_command_runtime_bridge, declarative_owned_runtime_bridge,
    declarative_runtime_bridge,
};

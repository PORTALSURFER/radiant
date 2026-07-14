//! Explicit opt-in capabilities for custom runtime hosts.

mod animation;
mod auxiliary;
mod diagnostics;
mod input;
mod lifecycle;
mod platform;
mod presentation;
mod queues;
mod tasks;

pub use animation::RuntimeAnimationHost;
pub use auxiliary::RuntimeWindowHost;
pub use diagnostics::{RuntimeDiagnosticsHost, RuntimeFrameDiagnosticsHost};
pub use input::RuntimeInputHost;
pub use lifecycle::RuntimeLifecycleHost;
pub use platform::RuntimePlatformHost;
pub use presentation::{RuntimeRetainedSurfaceHost, RuntimeTransientOverlayHost};
pub use queues::RuntimeQueueHost;
pub use tasks::RuntimeTaskHost;

use crate::{
    gui::{
        focus::FocusSurface, input::KeyPress, repaint::RepaintSignal, shortcuts::ShortcutResolution,
    },
    runtime::{
        NativeFrameDiagnostics, PaintPrimitive, RuntimeAnimationActivity, TransientOverlayContext,
    },
};
pub(crate) use animation::RuntimeAnimationCapability;
pub(crate) use auxiliary::RuntimeWindowCapability;
pub(crate) use diagnostics::{RuntimeDiagnosticsCapability, RuntimeFrameDiagnosticsCapability};
pub(crate) use input::RuntimeInputCapability;
pub(crate) use lifecycle::RuntimeLifecycleCapability;
pub(crate) use platform::RuntimePlatformCapability;
pub(crate) use presentation::{
    RuntimeRetainedSurfaceCapability, RuntimeTransientOverlayCapability,
};
pub(crate) use queues::RuntimeQueueCapability;
use std::sync::Arc;
pub(crate) use tasks::RuntimeTaskCapability;

/// Stable capability table cached by [`super::RuntimeBridge`] consumers.
///
/// A minimal bridge returns [`Self::new`]. Advanced hosts opt into only the
/// focused traits they implement. The table stores monomorphized function
/// pointers, so dispatch does not allocate or perform trait-object lookup.
pub struct RuntimeHostCapabilities<Bridge, Message> {
    pub(crate) input: Option<RuntimeInputCapability<Bridge, Message>>,
    pub(crate) tasks: Option<RuntimeTaskCapability<Bridge, Message>>,
    pub(crate) platform: Option<RuntimePlatformCapability<Bridge, Message>>,
    pub(crate) queues: Option<RuntimeQueueCapability<Bridge, Message>>,
    pub(crate) animation: Option<RuntimeAnimationCapability<Bridge>>,
    pub(crate) windows: Option<RuntimeWindowCapability<Bridge, Message>>,
    pub(crate) retained_surface: Option<RuntimeRetainedSurfaceCapability<Bridge>>,
    pub(crate) transient_overlay: Option<RuntimeTransientOverlayCapability<Bridge>>,
    pub(crate) runtime_diagnostics: Option<RuntimeDiagnosticsCapability<Bridge>>,
    pub(crate) frame_diagnostics: Option<RuntimeFrameDiagnosticsCapability<Bridge>>,
    pub(crate) lifecycle: Option<RuntimeLifecycleCapability<Bridge>>,
}

impl<Bridge, Message> RuntimeHostCapabilities<Bridge, Message> {
    /// Build an empty capability table for a minimal custom host.
    pub const fn new() -> Self {
        Self {
            input: None,
            tasks: None,
            platform: None,
            queues: None,
            animation: None,
            windows: None,
            retained_surface: None,
            transient_overlay: None,
            runtime_diagnostics: None,
            frame_diagnostics: None,
            lifecycle: None,
        }
    }

    /// Enable host-owned input policy callbacks.
    pub fn with_input(mut self) -> Self
    where
        Bridge: RuntimeInputHost<Message>,
    {
        self.input = Some(RuntimeInputCapability::new());
        self
    }

    /// Enable host-owned task scheduling and repaint signaling.
    pub fn with_tasks(mut self) -> Self
    where
        Bridge: RuntimeTaskHost<Message>,
    {
        self.tasks = Some(RuntimeTaskCapability::new());
        self
    }

    /// Enable typed platform-service dispatch.
    pub fn with_platform(mut self) -> Self
    where
        Bridge: RuntimePlatformHost<Message>,
    {
        self.platform = Some(RuntimePlatformCapability::new());
        self
    }

    /// Enable bridge-owned runtime command and message queues.
    pub fn with_queues(mut self) -> Self
    where
        Bridge: RuntimeQueueHost<Message>,
    {
        self.queues = Some(RuntimeQueueCapability::new());
        self
    }

    /// Enable host-owned animation polling and frame messages.
    pub fn with_animation(mut self) -> Self
    where
        Bridge: RuntimeAnimationHost,
    {
        self.animation = Some(RuntimeAnimationCapability::new());
        self
    }

    /// Enable auxiliary top-level window projection.
    pub fn with_windows(mut self) -> Self
    where
        Bridge: RuntimeWindowHost<Message>,
    {
        self.windows = Some(RuntimeWindowCapability::new());
        self
    }

    /// Enable retained custom-surface rendering.
    pub fn with_retained_surfaces(mut self) -> Self
    where
        Bridge: RuntimeRetainedSurfaceHost,
    {
        self.retained_surface = Some(RuntimeRetainedSurfaceCapability::new());
        self
    }

    /// Enable transient overlay painting.
    pub fn with_transient_overlays(mut self) -> Self
    where
        Bridge: RuntimeTransientOverlayHost,
    {
        self.transient_overlay = Some(RuntimeTransientOverlayCapability::new());
        self
    }

    /// Enable application-runtime diagnostics snapshots.
    pub fn with_runtime_diagnostics(mut self) -> Self
    where
        Bridge: RuntimeDiagnosticsHost,
    {
        self.runtime_diagnostics = Some(RuntimeDiagnosticsCapability::new());
        self
    }

    /// Enable native per-frame diagnostics delivery.
    pub fn with_frame_diagnostics(mut self) -> Self
    where
        Bridge: RuntimeFrameDiagnosticsHost,
    {
        self.frame_diagnostics = Some(RuntimeFrameDiagnosticsCapability::new());
        self
    }

    /// Enable runtime-exit and close-request lifecycle hooks.
    pub fn with_lifecycle(mut self) -> Self
    where
        Bridge: RuntimeLifecycleHost,
    {
        self.lifecycle = Some(RuntimeLifecycleCapability::new());
        self
    }

    /// Return whether transient overlay painting was explicitly enabled.
    pub const fn has_transient_overlay(&self) -> bool {
        self.transient_overlay.is_some()
    }

    /// Return whether native per-frame diagnostics were explicitly enabled.
    pub const fn has_frame_diagnostics(&self) -> bool {
        self.frame_diagnostics.is_some()
    }

    /// Poll explicitly enabled host animation activity.
    pub fn animation_activity(&self, bridge: &mut Bridge) -> Option<RuntimeAnimationActivity> {
        self.animation
            .as_ref()
            .map(|capability| (capability.animation_activity)(bridge))
    }

    /// Queue an explicitly enabled host animation-frame message.
    pub fn queue_animation_frame(&self, bridge: &mut Bridge) -> Option<bool> {
        self.animation
            .as_ref()
            .map(|capability| (capability.queue_animation_frame)(bridge))
    }

    /// Install a repaint signal when task hosting is enabled.
    pub fn install_repaint_signal(
        &self,
        bridge: &mut Bridge,
        signal: Arc<dyn RepaintSignal>,
    ) -> bool {
        let Some(capability) = self.tasks.as_ref() else {
            return false;
        };
        (capability.install_repaint_signal)(bridge, signal);
        true
    }

    /// Resolve one key press when host input policy is enabled.
    pub fn resolve_key_press(
        &self,
        bridge: &mut Bridge,
        pending_chord: Option<KeyPress>,
        press: KeyPress,
        focus: FocusSurface,
    ) -> Option<ShortcutResolution<Message>> {
        self.input
            .as_ref()
            .map(|capability| (capability.resolve_key_press)(bridge, pending_chord, press, focus))
    }

    /// Paint an explicitly enabled transient overlay.
    pub fn paint_transient_overlay(
        &self,
        bridge: &mut Bridge,
        context: TransientOverlayContext<'_>,
        primitives: &mut Vec<PaintPrimitive>,
    ) -> bool {
        let Some(capability) = self.transient_overlay.as_ref() else {
            return false;
        };
        (capability.paint_transient_overlay)(bridge, context, primitives);
        true
    }

    /// Deliver native frame diagnostics when the observer capability is enabled.
    pub fn observe_frame_diagnostics(
        &self,
        bridge: &mut Bridge,
        diagnostics: NativeFrameDiagnostics,
    ) -> bool {
        let Some(capability) = self.frame_diagnostics.as_ref() else {
            return false;
        };
        (capability.observe_frame_diagnostics)(bridge, diagnostics);
        true
    }

    /// Run the optional runtime-exit hook.
    pub fn on_runtime_exit(&self, bridge: &mut Bridge) -> Option<serde_json::Value> {
        let capability = self.lifecycle.as_ref()?;
        (capability.on_runtime_exit)(bridge)
    }
}

impl<Bridge, Message> Default for RuntimeHostCapabilities<Bridge, Message> {
    fn default() -> Self {
        Self::new()
    }
}

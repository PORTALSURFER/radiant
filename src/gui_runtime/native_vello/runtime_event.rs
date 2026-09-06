use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use winit::window::WindowId;

use super::generic_runtime::{
    NativeAdapterGeneration, NativeRecoveryEpisodeToken, NativeRenderDeviceErrorKind,
};

/// Owner-scoped identity for one native WGPU device callback pair.
///
/// The witness is intentionally opaque and compared by allocation identity.
/// An event carrying an old witness therefore cannot be admitted after its
/// owner has been replaced, removed, or recreated.
#[derive(Debug)]
struct SurfaceAcquireCorrelation {
    active: bool,
    uncaptured_error: Option<NativeRenderDeviceErrorKind>,
}

#[derive(Debug)]
pub(in crate::gui_runtime::native_vello) struct DeviceLossRegistration {
    surface_acquire: Mutex<SurfaceAcquireCorrelation>,
}

impl DeviceLossRegistration {
    pub(in crate::gui_runtime::native_vello) fn new() -> Self {
        Self {
            surface_acquire: Mutex::new(SurfaceAcquireCorrelation {
                active: false,
                uncaptured_error: None,
            }),
        }
    }

    pub(in crate::gui_runtime::native_vello) fn begin_surface_acquire(&self) {
        let mut correlation = self
            .surface_acquire
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        correlation.active = true;
        correlation.uncaptured_error = None;
    }

    pub(in crate::gui_runtime::native_vello) fn finish_surface_acquire(
        &self,
    ) -> Option<NativeRenderDeviceErrorKind> {
        let mut correlation = self
            .surface_acquire
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        correlation.active = false;
        correlation.uncaptured_error.take()
    }

    pub(in crate::gui_runtime::native_vello) fn observe_uncaptured_error(
        &self,
        kind: NativeRenderDeviceErrorKind,
    ) {
        let mut correlation = self
            .surface_acquire
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if correlation.active
            && (correlation.uncaptured_error.is_none()
                || matches!(kind, NativeRenderDeviceErrorKind::OutOfMemory))
        {
            correlation.uncaptured_error = Some(kind);
        }
    }
}

#[cfg(target_os = "macos")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::gui_runtime::native_vello) enum NativeSemanticAccessibilityQuery {
    /// An explicit AppKit child-range request.  The callback has already
    /// bounded `max_count`; the owned runtime turn performs the remaining
    /// cardinality, declaration-budget, and aggregate-budget validation.
    ChildrenRange {
        token: u64,
        start_index: usize,
        max_count: usize,
        explicit_retry: bool,
    },
}

#[cfg(target_os = "macos")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::gui_runtime::native_vello) enum NativeNumericAccessibilityAction {
    Increment,
    Decrement,
    SetValueText(String),
}

#[derive(Clone, Debug)]
pub(in crate::gui_runtime::native_vello) enum RuntimeUserEvent {
    RepaintRequested,
    /// Coalesced parent-owned pump for CPU signal-summary preparation.
    SignalSummaryWorkReady,
    /// Coalesced parent-owned pump for device-bound custom shader preparation.
    CustomShaderWorkReady,
    OpenFiles(Vec<PathBuf>),
    ApplicationReopenRequested,
    DeviceLost {
        registration: Arc<DeviceLossRegistration>,
        generation: NativeAdapterGeneration,
        message: String,
    },
    DeviceRecoveryReady {
        episode: NativeRecoveryEpisodeToken,
    },
    RenderDeviceError {
        registration: Arc<DeviceLossRegistration>,
        generation: NativeAdapterGeneration,
        kind: NativeRenderDeviceErrorKind,
        message: String,
    },
    #[allow(
        dead_code,
        reason = "The macOS dragging source is the only native producer of this event."
    )]
    ExternalDragCompleted {
        window_id: WindowId,
        identity: crate::runtime::ExternalDragIdentity,
        result: Result<crate::runtime::ExternalDragOutcome, String>,
    },
    NativeResourceMaintenanceRequested,
    /// Wake-only completion for one exact-generation frame GPU timing slot.
    /// Conversion, delivery, unmapping, and recycling remain event-loop work.
    NativeGpuTimingReady {
        route: NativeGpuTimingRoute,
        generation: NativeAdapterGeneration,
        resource_identity: u64,
        slot: u8,
        token: u64,
    },
    #[cfg(target_os = "macos")]
    AccessibilityDisplayChanged,
    #[cfg(target_os = "macos")]
    NativeSemanticAccessibilityQuery {
        window_id: WindowId,
        generation: u64,
        query: NativeSemanticAccessibilityQuery,
    },
    #[cfg(target_os = "macos")]
    NativeNumericAccessibilityAction {
        window_id: WindowId,
        generation: u64,
        token: u64,
        target: Box<crate::gui::automation::AutomationTarget>,
        action: NativeNumericAccessibilityAction,
    },
}

/// Exact private route for one native window's asynchronous GPU timing pool.
///
/// The auxiliary key is stable across the window's resource reconstruction, so
/// a completion cannot be redirected to the primary or to a sibling child.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::gui_runtime::native_vello) enum NativeGpuTimingRoute {
    Primary,
    Auxiliary(String),
}

impl PartialEq for RuntimeUserEvent {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::RepaintRequested, Self::RepaintRequested)
            | (Self::SignalSummaryWorkReady, Self::SignalSummaryWorkReady)
            | (Self::CustomShaderWorkReady, Self::CustomShaderWorkReady)
            | (Self::ApplicationReopenRequested, Self::ApplicationReopenRequested) => true,
            (Self::OpenFiles(left), Self::OpenFiles(right)) => left == right,
            (
                Self::NativeResourceMaintenanceRequested,
                Self::NativeResourceMaintenanceRequested,
            ) => true,
            (
                Self::DeviceRecoveryReady { episode: left },
                Self::DeviceRecoveryReady { episode: right },
            ) => left == right,
            (
                Self::DeviceLost {
                    registration: left_registration,
                    generation: left_generation,
                    message: left_message,
                },
                Self::DeviceLost {
                    registration: right_registration,
                    generation: right_generation,
                    message: right_message,
                },
            ) => {
                Arc::ptr_eq(left_registration, right_registration)
                    && left_generation == right_generation
                    && left_message == right_message
            }
            (
                Self::RenderDeviceError {
                    registration: left_registration,
                    generation: left_generation,
                    kind: left_kind,
                    message: left_message,
                },
                Self::RenderDeviceError {
                    registration: right_registration,
                    generation: right_generation,
                    kind: right_kind,
                    message: right_message,
                },
            ) => {
                Arc::ptr_eq(left_registration, right_registration)
                    && left_generation == right_generation
                    && left_kind == right_kind
                    && left_message == right_message
            }
            (
                Self::NativeGpuTimingReady {
                    route: left_route,
                    generation: left_generation,
                    resource_identity: left_resource_identity,
                    slot: left_slot,
                    token: left_token,
                },
                Self::NativeGpuTimingReady {
                    route: right_route,
                    generation: right_generation,
                    resource_identity: right_resource_identity,
                    slot: right_slot,
                    token: right_token,
                },
            ) => {
                left_route == right_route
                    && left_generation == right_generation
                    && left_resource_identity == right_resource_identity
                    && left_slot == right_slot
                    && left_token == right_token
            }
            (
                Self::ExternalDragCompleted {
                    window_id: left_window_id,
                    identity: left_identity,
                    result: left_result,
                },
                Self::ExternalDragCompleted {
                    window_id: right_window_id,
                    identity: right_identity,
                    result: right_result,
                },
            ) => {
                left_window_id == right_window_id
                    && left_identity == right_identity
                    && left_result == right_result
            }
            #[cfg(target_os = "macos")]
            (Self::AccessibilityDisplayChanged, Self::AccessibilityDisplayChanged) => true,
            #[cfg(target_os = "macos")]
            (
                Self::NativeSemanticAccessibilityQuery {
                    window_id: left_window_id,
                    generation: left_generation,
                    query: left_query,
                },
                Self::NativeSemanticAccessibilityQuery {
                    window_id: right_window_id,
                    generation: right_generation,
                    query: right_query,
                },
            ) => {
                left_window_id == right_window_id
                    && left_generation == right_generation
                    && left_query == right_query
            }
            #[cfg(target_os = "macos")]
            (
                Self::NativeNumericAccessibilityAction {
                    window_id: left_window_id,
                    generation: left_generation,
                    token: left_token,
                    target: left_target,
                    action: left_action,
                },
                Self::NativeNumericAccessibilityAction {
                    window_id: right_window_id,
                    generation: right_generation,
                    token: right_token,
                    target: right_target,
                    action: right_action,
                },
            ) => {
                left_window_id == right_window_id
                    && left_generation == right_generation
                    && left_token == right_token
                    && left_target == right_target
                    && left_action == right_action
            }
            _ => false,
        }
    }
}

impl Eq for RuntimeUserEvent {}

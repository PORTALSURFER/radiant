use std::path::PathBuf;
use std::sync::Arc;

use super::generic_runtime::{NativeAdapterGeneration, NativeRenderDeviceErrorKind};

/// Owner-scoped identity for one native WGPU device callback pair.
///
/// The witness is intentionally opaque and compared by allocation identity.
/// An event carrying an old witness therefore cannot be admitted after its
/// owner has been replaced, removed, or recreated.
#[derive(Debug)]
pub(in crate::gui_runtime::native_vello) struct DeviceLossRegistration;

impl DeviceLossRegistration {
    pub(in crate::gui_runtime::native_vello) const fn new() -> Self {
        Self
    }
}

#[derive(Clone, Debug)]
pub(in crate::gui_runtime::native_vello) enum RuntimeUserEvent {
    RepaintRequested,
    OpenFiles(Vec<PathBuf>),
    ApplicationReopenRequested,
    DeviceLost {
        registration: Arc<DeviceLossRegistration>,
        generation: NativeAdapterGeneration,
        message: String,
    },
    RenderDeviceError {
        registration: Arc<DeviceLossRegistration>,
        generation: NativeAdapterGeneration,
        kind: NativeRenderDeviceErrorKind,
        message: String,
    },
    NativeResourceMaintenanceRequested,
    #[cfg(target_os = "macos")]
    AccessibilityDisplayChanged,
}

impl PartialEq for RuntimeUserEvent {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::RepaintRequested, Self::RepaintRequested)
            | (Self::ApplicationReopenRequested, Self::ApplicationReopenRequested) => true,
            (Self::OpenFiles(left), Self::OpenFiles(right)) => left == right,
            (
                Self::NativeResourceMaintenanceRequested,
                Self::NativeResourceMaintenanceRequested,
            ) => true,
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
            #[cfg(target_os = "macos")]
            (Self::AccessibilityDisplayChanged, Self::AccessibilityDisplayChanged) => true,
            _ => false,
        }
    }
}

impl Eq for RuntimeUserEvent {}

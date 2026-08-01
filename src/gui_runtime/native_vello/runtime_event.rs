use std::path::PathBuf;
use std::sync::Arc;

/// Owner-scoped identity for one native WGPU device-loss callback.
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
        message: String,
    },
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
                Self::DeviceLost {
                    registration: left_registration,
                    message: left_message,
                },
                Self::DeviceLost {
                    registration: right_registration,
                    message: right_message,
                },
            ) => {
                Arc::ptr_eq(left_registration, right_registration) && left_message == right_message
            }
            #[cfg(target_os = "macos")]
            (Self::AccessibilityDisplayChanged, Self::AccessibilityDisplayChanged) => true,
            _ => false,
        }
    }
}

impl Eq for RuntimeUserEvent {}

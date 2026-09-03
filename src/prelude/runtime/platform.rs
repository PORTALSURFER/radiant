//! Common platform-service request and callback prelude exports.

pub use crate::runtime::{
    ClipboardContent, ClipboardContentFormat, ClipboardFormat, ClipboardIdentity, ClipboardValue,
    ClipboardValueError, ConfirmDialogRequest, ConfirmationButtons, ConfirmationLevel,
    ConfirmationResponse, DragPreview, DragPreviewTextSizing, DragRequest, FileDialogFilter,
    FileDialogRequest, NotificationLevel, NotificationRequest, PlatformError, PlatformFailure,
    PlatformResult, PlatformResultExt, PlatformService,
};

//! Platform-neutral native runtime policy types.

mod gpu;
mod icon;
mod popup;
mod run;
mod text;

pub use gpu::{NativeGpuBackend, NativeGpuOptions};
pub use icon::WindowIconRgba;
pub use popup::{NativePopupOptions, NativeWindowMode};
pub(crate) use run::normalize_native_target_fps;
pub use run::{
    DEFAULT_NATIVE_WINDOW_TITLE, MAX_NATIVE_TARGET_FPS, MIN_NATIVE_TARGET_FPS, NativeFrameOptions,
    NativeRunOptions, NativeRunOptionsError, NativeWindowBehavior, NativeWindowGeometry,
    NativeWindowOptions,
};
pub use text::{EmbeddedFont, NativeTextOptions};

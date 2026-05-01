//! Compatibility keypress DTOs for host-owned shortcut resolution.

pub use crate::gui::input::KeyPress;
pub use crate::gui::shortcuts::ShortcutResolution;

/// Compatibility alias for the generic shortcut resolution DTO.
pub type HotkeyResolution = ShortcutResolution<crate::sempal_app::UiAction>;

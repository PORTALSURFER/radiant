//! Compatibility result DTO for host-owned hotkey resolution.

use super::KeyPress;
use crate::sempal_app::UiAction;

/// Result of resolving one keypress against the host shortcut catalog.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HotkeyResolution {
    /// Action produced by this keypress, if any.
    pub action: Option<UiAction>,
    /// Whether the keypress was consumed by the hotkey system.
    pub handled: bool,
    /// Pending chord starter to carry into the next keypress, if any.
    pub pending_chord: Option<KeyPress>,
}

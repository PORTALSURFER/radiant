/// Generic enabled/visible tool state for a signal visualization surface.
///
/// The fields intentionally describe interaction roles rather than domain
/// operations. Hosts map these booleans to product-specific tools such as snap
/// modes, overlays, review modes, or cleanup availability.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SignalToolState {
    /// Whether the visualization's current mode is locked against host updates.
    pub lock_enabled: bool,
    /// Whether alternate preview behavior is enabled.
    pub alternate_preview_enabled: bool,
    /// Whether the primary snap behavior is enabled.
    pub primary_snap_enabled: bool,
    /// Whether grid/guide alignment uses a relative anchor.
    pub relative_grid_enabled: bool,
    /// Whether the secondary snap behavior is enabled.
    pub secondary_snap_enabled: bool,
    /// Whether marker overlays are visible.
    pub markers_visible: bool,
    /// Whether marker editing mode is active.
    pub marker_mode_enabled: bool,
    /// Whether a host-defined batch action is available.
    pub batch_action_available: bool,
}

/// Explicit flags used to build signal visualization tool state.
///
/// Prefer this over positional boolean constructors so host projections remain
/// readable as the generic visualization model grows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SignalToolFlags {
    /// Whether the visualization's current mode is locked against host updates.
    pub lock_enabled: bool,
    /// Whether alternate preview behavior is enabled.
    pub alternate_preview_enabled: bool,
    /// Whether the primary snap behavior is enabled.
    pub primary_snap_enabled: bool,
    /// Whether grid/guide alignment uses a relative anchor.
    pub relative_grid_enabled: bool,
    /// Whether the secondary snap behavior is enabled.
    pub secondary_snap_enabled: bool,
    /// Whether marker overlays are visible.
    pub markers_visible: bool,
    /// Whether marker editing mode is active.
    pub marker_mode_enabled: bool,
    /// Whether a host-defined batch action is available.
    pub batch_action_available: bool,
}

impl Default for SignalToolFlags {
    fn default() -> Self {
        Self {
            lock_enabled: false,
            alternate_preview_enabled: false,
            primary_snap_enabled: false,
            relative_grid_enabled: false,
            secondary_snap_enabled: false,
            markers_visible: true,
            marker_mode_enabled: false,
            batch_action_available: false,
        }
    }
}

impl Default for SignalToolState {
    fn default() -> Self {
        Self::from_flags(SignalToolFlags::default())
    }
}

impl SignalToolState {
    /// Build signal tool state from explicitly named generic flags.
    pub fn from_flags(flags: SignalToolFlags) -> Self {
        Self {
            lock_enabled: flags.lock_enabled,
            alternate_preview_enabled: flags.alternate_preview_enabled,
            primary_snap_enabled: flags.primary_snap_enabled,
            relative_grid_enabled: flags.relative_grid_enabled,
            secondary_snap_enabled: flags.secondary_snap_enabled,
            markers_visible: flags.markers_visible,
            marker_mode_enabled: flags.marker_mode_enabled,
            batch_action_available: flags.batch_action_available,
        }
    }
}

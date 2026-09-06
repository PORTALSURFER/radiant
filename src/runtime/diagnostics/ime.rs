use super::NativeWindowDiagnosticIdentity;

/// Native backend reported by the created window handle for IME diagnostics.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NativeImeBackend {
    /// The window exposes an AppKit handle.
    AppKit,
    /// The window exposes a Win32 handle.
    Win32,
    /// The window exposes a Wayland handle.
    Wayland,
    /// The window exposes an X11 handle.
    X11,
    /// The native handle was unavailable or did not identify a known backend.
    #[default]
    Unknown,
}

/// Why an IME adapter capability cannot be asserted for one window.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NativeImeAdapterUnavailableReason {
    /// The created window did not expose a raw window handle.
    WindowHandleUnavailable,
    /// The raw handle did not identify a backend with locked-Winit evidence.
    #[default]
    UnknownBackend,
}

/// Locked-Winit capability for composition transport.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeImeCompositionCapability {
    /// Winit exposes composition through its `Ime` window events.
    SupportedByWinit,
    /// Composition transport cannot be asserted for this adapter.
    Unavailable(NativeImeAdapterUnavailableReason),
}

/// Locked-Winit capability for candidate-window placement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeImeCandidateCapability {
    /// Winit supports the complete IME cursor area on this adapter.
    FullCursorAreaByWinit,
    /// Winit supports only the IME cursor position on this adapter.
    PositionOnlyByWinit,
    /// Candidate placement cannot be asserted for this adapter.
    Unavailable(NativeImeAdapterUnavailableReason),
}

/// Why matching-key suppression cannot be asserted for one window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeImeMatchingKeySuppressionUnavailableReason {
    /// The created window did not expose a raw window handle.
    WindowHandleUnavailable,
    /// Win32 suppression has not been verified against the locked Winit source.
    Win32,
    /// Wayland suppression has not been verified against the locked Winit source.
    Wayland,
    /// X11 suppression has not been verified against the locked Winit source.
    X11,
    /// The backend could not be identified from the raw window handle.
    UnknownBackend,
}

/// Locked-Winit evidence for whether an IME-handled key suppresses its matching
/// keyboard input event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeImeMatchingKeySuppression {
    /// The created AppKit window uses Winit's verified IME suppression path.
    VerifiedWinitAppKit,
    /// Suppression is not asserted for this window.
    Unavailable(NativeImeMatchingKeySuppressionUnavailableReason),
}

/// One-shot IME adapter observation for an admitted native window.
///
/// This records capabilities exposed by the locked Winit adapter. It does not
/// alter focus, composition ownership, input routing, or native IME policy.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NativeImeAdapterObservation {
    /// Opaque identity of the admitted native window.
    pub window_identity: Option<NativeWindowDiagnosticIdentity>,
    /// Actual backend classified from the created window's raw handle.
    pub backend: NativeImeBackend,
    /// Composition transport capability for this actual locked-Winit adapter.
    pub composition: NativeImeCompositionCapability,
    /// Candidate-window placement capability for this actual locked-Winit adapter.
    pub candidate: NativeImeCandidateCapability,
    /// Matching-key suppression evidence for this actual backend.
    pub matching_key_suppression: NativeImeMatchingKeySuppression,
}

impl Default for NativeImeMatchingKeySuppression {
    fn default() -> Self {
        Self::Unavailable(NativeImeMatchingKeySuppressionUnavailableReason::UnknownBackend)
    }
}

impl Default for NativeImeCompositionCapability {
    fn default() -> Self {
        Self::Unavailable(NativeImeAdapterUnavailableReason::UnknownBackend)
    }
}

impl Default for NativeImeCandidateCapability {
    fn default() -> Self {
        Self::Unavailable(NativeImeAdapterUnavailableReason::UnknownBackend)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_observation_fails_closed_for_unverified_capabilities() {
        let observation = NativeImeAdapterObservation::default();
        let unavailable = NativeImeAdapterUnavailableReason::UnknownBackend;
        assert_eq!(
            observation.composition,
            NativeImeCompositionCapability::Unavailable(unavailable)
        );
        assert_eq!(
            observation.candidate,
            NativeImeCandidateCapability::Unavailable(unavailable)
        );
        assert_eq!(
            observation.matching_key_suppression,
            NativeImeMatchingKeySuppression::Unavailable(
                NativeImeMatchingKeySuppressionUnavailableReason::UnknownBackend
            )
        );
    }
}

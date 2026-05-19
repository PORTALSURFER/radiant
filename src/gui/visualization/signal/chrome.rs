/// Channel layout for visualizing one stream as a combined or split view.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChannelViewMode {
    /// Collapse channels into one combined envelope.
    Mono,
    /// Render channels in a split stereo view.
    Stereo,
}

/// Explicit parts used to build generic signal chrome state.
///
/// This avoids positional status/reference/channel construction as the generic
/// visualization chrome grows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SignalChromeParts {
    /// Extra status hint shown alongside visualization labels.
    pub status_hint: String,
    /// Whether a host-defined reference anchor is currently available.
    pub reference_anchor_available: bool,
    /// Label for the host-defined reference anchor, when available.
    pub reference_anchor_label: Option<String>,
    /// Channel layout used by the signal visualization.
    pub channel_view: ChannelViewMode,
}

impl Default for SignalChromeParts {
    fn default() -> Self {
        Self {
            status_hint: String::from("idle"),
            reference_anchor_available: false,
            reference_anchor_label: None,
            channel_view: ChannelViewMode::Mono,
        }
    }
}

/// Generic chrome/status state for a signal visualization surface.
///
/// This captures reusable display state such as a transport/status hint,
/// optional reference-anchor metadata, and channel layout. Host-specific tools
/// and edit modes should remain in host state or compatibility adapters.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SignalChromeState {
    /// Extra status hint shown alongside visualization labels.
    pub status_hint: String,
    /// Whether a host-defined reference anchor is currently available.
    pub reference_anchor_available: bool,
    /// Label for the host-defined reference anchor, when available.
    pub reference_anchor_label: Option<String>,
    /// Channel layout used by the signal visualization.
    pub channel_view: ChannelViewMode,
}

impl Default for SignalChromeState {
    fn default() -> Self {
        Self::from_parts(SignalChromeParts::default())
    }
}

impl SignalChromeState {
    /// Build signal chrome state from named generic display parts.
    pub fn from_parts(parts: SignalChromeParts) -> Self {
        Self {
            status_hint: parts.status_hint,
            reference_anchor_available: parts.reference_anchor_available,
            reference_anchor_label: parts.reference_anchor_label,
            channel_view: parts.channel_view,
        }
    }

    /// Build signal chrome state from explicit display values.
    pub fn new(
        status_hint: impl Into<String>,
        reference_anchor_available: bool,
        reference_anchor_label: Option<String>,
        channel_view: ChannelViewMode,
    ) -> Self {
        Self::from_parts(SignalChromeParts {
            status_hint: status_hint.into(),
            reference_anchor_available,
            reference_anchor_label,
            channel_view,
        })
    }
}

//! Launch model shared by the popup launcher and child popup host.

use std::env;

pub(super) const POPUP_ARG: &str = "--popup";
pub(super) const POPUP_MODE_ARG: &str = "--popup-mode";
pub(super) const POPUP_PREWARM_ARG: &str = "--popup-prewarm";
pub(super) const POPUP_POSITION: [f32; 2] = [460.0, 280.0];
pub(super) const POPUP_PREWARM_POSITION: [f32; 2] = [-20_000.0, -20_000.0];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PopupMode {
    DragPreview,
    Tooltip,
    CommandPalette,
}

impl PopupMode {
    pub(super) const ALL: [Self; 3] = [Self::DragPreview, Self::Tooltip, Self::CommandPalette];

    pub(super) fn arg(self) -> &'static str {
        match self {
            Self::DragPreview => "drag-preview",
            Self::Tooltip => "tooltip",
            Self::CommandPalette => "command-palette",
        }
    }

    fn from_arg(value: &str) -> Self {
        match value {
            "tooltip" => Self::Tooltip,
            "command-palette" => Self::CommandPalette,
            _ => Self::DragPreview,
        }
    }

    pub(super) fn label(self) -> &'static str {
        match self {
            Self::DragPreview => "Drag preview",
            Self::Tooltip => "Tooltip",
            Self::CommandPalette => "Command palette",
        }
    }

    pub(super) fn detail(self) -> &'static str {
        match self {
            Self::DragPreview => "Audio Clip 04 is being dragged outside the main window.",
            Self::Tooltip => "Transient help can use the same borderless popup surface.",
            Self::CommandPalette => "Popup windows can host focused command surfaces too.",
        }
    }

    pub(super) fn badge(self) -> &'static str {
        match self {
            Self::DragPreview => "Dragging",
            Self::Tooltip => "Hint",
            Self::CommandPalette => "Commands",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct PopupLaunch {
    pub(super) mode: PopupMode,
    pub(super) prewarmed: bool,
}

impl Default for PopupLaunch {
    fn default() -> Self {
        Self {
            mode: PopupMode::DragPreview,
            prewarmed: false,
        }
    }
}

pub(super) fn popup_launch_from_args() -> Option<PopupLaunch> {
    let mut args = env::args().skip(1);
    let mut popup = false;
    let mut prewarmed = false;
    let mut mode = PopupMode::DragPreview;
    while let Some(arg) = args.next() {
        if arg == POPUP_ARG {
            popup = true;
        } else if arg == POPUP_PREWARM_ARG {
            prewarmed = true;
        } else if arg == POPUP_MODE_ARG
            && let Some(value) = args.next()
        {
            mode = PopupMode::from_arg(value.as_str());
        }
    }
    popup.then_some(PopupLaunch { mode, prewarmed })
}

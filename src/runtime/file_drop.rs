//! Backend-neutral native file-drop events.

use crate::{gui::types::Point, widgets::WidgetId};
use std::path::PathBuf;

/// Native file drag/drop phase reported by the host window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeFileDropPhase {
    /// A native file drag is hovering over the application window.
    Hover,
    /// A native file drag left the application window or was cancelled.
    Cancel,
    /// A native file was dropped onto the application window.
    Drop,
}

/// Native file drag/drop event with the current pointer target when available.
#[derive(Clone, Debug, PartialEq)]
pub struct NativeFileDrop {
    /// Event phase.
    pub phase: NativeFileDropPhase,
    /// File path supplied by the operating system.
    pub path: Option<PathBuf>,
    /// Last known logical pointer position in the surface.
    pub position: Option<Point>,
    /// Widget under the last known pointer position.
    pub target_widget: Option<WidgetId>,
}

impl NativeFileDrop {
    /// Build a hover event.
    pub fn hover(path: PathBuf, position: Option<Point>, target_widget: Option<WidgetId>) -> Self {
        Self {
            phase: NativeFileDropPhase::Hover,
            path: Some(path),
            position,
            target_widget,
        }
    }

    /// Build a cancellation event.
    pub fn cancel(position: Option<Point>, target_widget: Option<WidgetId>) -> Self {
        Self {
            phase: NativeFileDropPhase::Cancel,
            path: None,
            position,
            target_widget,
        }
    }

    /// Build a drop event.
    pub fn dropped(
        path: PathBuf,
        position: Option<Point>,
        target_widget: Option<WidgetId>,
    ) -> Self {
        Self {
            phase: NativeFileDropPhase::Drop,
            path: Some(path),
            position,
            target_widget,
        }
    }
}

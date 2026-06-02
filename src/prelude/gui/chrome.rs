//! Generic chrome and panel-state prelude exports.

pub use crate::gui::{
    chrome::{
        ContentViewActivityChrome, ContentViewChrome, ContentViewFooterChrome,
        ContentViewSearchChrome, ContentViewSortChrome, ContentViewTabs, StatusSegments,
        StatusSegmentsParts,
    },
    disclosure::ExclusiveOpen,
    focus::FocusSurface,
    frame::{FrameCadenceConfig, FrameCadenceKind, FrameCadenceMonitor, FrameCadenceReport},
    panel::{PanelResizeDrag, PanelResizeEdge, update_panel_resize_drag},
};

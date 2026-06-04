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
    panel::{
        CollapsiblePanelResizeConstraints, PanelResizeConstraints, PanelResizeDrag,
        PanelResizeEdge, PanelResizeState, update_collapsible_panel_resize_drag,
        update_panel_resize_drag,
    },
};

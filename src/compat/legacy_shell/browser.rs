//! Browser/list/map-facing models exposed by the `radiant` app contract.

pub use crate::gui::chrome::ContentViewChrome as BrowserChromeModel;
pub use crate::gui::list::ContentListActions as BrowserActionsModel;
pub use crate::gui::list::ContentListRow as BrowserRowModel;
pub use crate::gui::list::RecencyBucket as PlaybackAgeBucket;
pub use crate::gui::list::RecencyFilterChip as PlaybackAgeFilterChip;
pub use crate::gui::list::RowProcessingState as BrowserRowProcessingState;
pub use crate::gui::selection::TriState as BrowserPillState;
pub use crate::gui::visualization::PointRenderMode as MapRenderModeModel;
pub use crate::gui::visualization::SpatialPanel as MapPanelModel;
pub use crate::gui::visualization::SpatialPoint as MapPointModel;
/// One clickable pill projected into the browser metadata sidebar.
pub type BrowserPillModel = crate::gui::badge::SelectablePill<BrowserPillState>;
/// Browser-local metadata sidebar shown beside the content list.
pub type BrowserPillEditorModel = crate::gui::badge::PillEditorPanel<BrowserPillState>;

/// Summary of browser/list state consumed by the native shell.
pub type BrowserPanelModel =
    crate::gui::list::ContentListPanel<BrowserRowModel, BrowserPillEditorModel>;

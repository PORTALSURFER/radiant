//! Browser/list/map-facing models exposed by the `radiant` app contract.

pub use crate::gui::chrome::ContentViewChrome as BrowserChromeModel;
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

/// Browser action availability consumed by the native shell action strip.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct BrowserActionsModel {
    /// Whether rename can be started for the focused row.
    pub can_rename: bool,
    /// Whether delete can be applied to focused/selected rows.
    pub can_delete: bool,
    /// Whether pill-editor actions can be applied to focused/selected rows.
    pub can_edit_pills: bool,
    /// Whether the focused browser row can be normalized in place.
    pub can_normalize_focused_item: bool,
    /// Whether the focused browser row can open the seamless loop-crossfade flow.
    pub can_loop_crossfade_focused_item: bool,
    /// Whether sticky random navigation mode is currently enabled.
    pub random_navigation_enabled: bool,
    /// Whether browser duplicate cleanup mode is currently enabled.
    pub duplicate_cleanup_active: bool,
    /// Whether the browser-local pill editor is currently open.
    pub pill_editor_open: bool,
}

impl BrowserActionsModel {
    /// Whether generic browser pill edits can be applied.
    pub fn can_edit_pills(&self) -> bool {
        self.can_edit_pills
    }

    /// Whether the generic browser pill editor is currently open.
    pub fn pill_editor_open(&self) -> bool {
        self.pill_editor_open
    }
}

//! Container-specific layout policy values.

use super::{CrossAlign, Insets, MainAlign, OverflowPolicy, VirtualizationPolicy};

/// Grid-specific policy values.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GridPolicy {
    /// Number of columns used by the grid.
    pub columns: usize,
    /// Gap between columns.
    pub column_gap: f32,
    /// Gap between rows.
    pub row_gap: f32,
}

impl Default for GridPolicy {
    fn default() -> Self {
        Self {
            columns: 1,
            column_gap: 0.0,
            row_gap: 0.0,
        }
    }
}

/// Wrap/flow policy values.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WrapPolicy {
    /// Horizontal gap between items.
    pub item_gap: f32,
    /// Vertical gap between wrapped rows.
    pub line_gap: f32,
}

impl Default for WrapPolicy {
    fn default() -> Self {
        Self {
            item_gap: 0.0,
            line_gap: 0.0,
        }
    }
}

/// One switch-layout branch width range.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SwitchBreakpoint {
    /// Inclusive minimum viewport/content width.
    pub min_width: f32,
    /// Inclusive maximum viewport/content width.
    pub max_width: f32,
}

impl SwitchBreakpoint {
    /// Create a normalized width range.
    #[allow(dead_code)]
    pub fn new(min_width: f32, max_width: f32) -> Self {
        let min = min_width.max(0.0);
        let max = max_width.max(min);
        Self {
            min_width: min,
            max_width: max,
        }
    }

    /// Return true when `width` is covered by this range.
    pub fn contains(self, width: f32) -> bool {
        width >= self.min_width && width <= self.max_width
    }
}

/// Container kind used to select a deterministic layout algorithm.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContainerKind {
    /// Lay out children horizontally using slot main-axis sizing rules.
    Row,
    /// Lay out children vertically using slot main-axis sizing rules.
    Column,
    /// Overlay children in shared bounds using slot order as z-order.
    Stack,
    /// Apply container padding around one logical content child.
    PaddingBox,
    /// Align one logical content child inside the assigned bounds.
    AlignBox,
    /// Maintain a fixed aspect ratio for one logical content child.
    AspectBox,
    /// Place children in deterministic rows and columns.
    Grid,
    /// Create a scrollable viewport around one content child.
    ScrollView,
    /// Wrap children onto additional lines when space runs out.
    Wrap,
    /// Select one branch child based on the available width.
    SwitchLayout,
}

/// Shared policy configuration for container nodes.
#[derive(Clone, Debug, PartialEq)]
pub struct ContainerPolicy {
    /// Layout algorithm kind.
    pub kind: ContainerKind,
    /// Child spacing on the main axis.
    pub spacing: f32,
    /// Container-level content padding.
    pub padding: Insets,
    /// Default main-axis alignment.
    pub align_main: MainAlign,
    /// Default cross-axis alignment.
    pub align_cross: CrossAlign,
    /// Explicit overflow policy.
    pub overflow: OverflowPolicy,
    /// Grid-specific options.
    pub grid: GridPolicy,
    /// Wrap-specific options.
    pub wrap: WrapPolicy,
    /// Aspect ratio used by `AspectBox` (width / height).
    pub aspect_ratio: Option<f32>,
    /// Branch selection ranges for `SwitchLayout`.
    pub switch_breakpoints: Vec<SwitchBreakpoint>,
    /// Optional virtualization policy for scroll containers.
    pub virtualization: Option<VirtualizationPolicy>,
}

impl Default for ContainerPolicy {
    fn default() -> Self {
        Self {
            kind: ContainerKind::Column,
            spacing: 0.0,
            padding: Insets::default(),
            align_main: MainAlign::Start,
            align_cross: CrossAlign::Stretch,
            overflow: OverflowPolicy::Clip,
            grid: GridPolicy::default(),
            wrap: WrapPolicy::default(),
            aspect_ratio: None,
            switch_breakpoints: Vec::new(),
            virtualization: None,
        }
    }
}

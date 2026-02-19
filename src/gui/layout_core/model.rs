//! Slot and container policy definitions for the slot-based layout engine.

use super::constraints::Constraints;

/// Main-axis sizing mode for a slot.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SizeModeMain {
    /// Fixed logical pixels.
    Fixed(f32),
    /// Fill remaining space by weight.
    Fill(f32),
    /// Percentage of parent content space.
    Percent(f32),
    /// Resolve from child intrinsic measurement.
    Intrinsic,
}

/// Cross-axis sizing mode for a slot.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SizeModeCross {
    /// Fixed logical pixels.
    Fixed(f32),
    /// Fill available cross-axis space.
    Fill,
    /// Resolve from child intrinsic measurement.
    Intrinsic,
}

/// Main-axis alignment within a container.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MainAlign {
    Start,
    Center,
    End,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

/// Cross-axis alignment for children within a container.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrossAlign {
    Start,
    Center,
    End,
    Stretch,
}

/// Explicit overflow policy for containers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OverflowPolicy {
    Clip,
    Scroll,
    Wrap,
    Shrink,
}

/// Scroll virtualization axis selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum VirtualizationAxis {
    /// Virtualize children along the horizontal axis.
    Horizontal,
    /// Virtualize children along the vertical axis.
    Vertical,
}

/// Optional virtualization policy for large scrollable child lists.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VirtualizationPolicy {
    /// Enables virtualization when `true`.
    pub enabled: bool,
    /// Main-axis used to compute visible windows.
    pub axis: VirtualizationAxis,
    /// Extra pixels before/after the viewport to pre-materialize.
    pub overscan_px: f32,
}

/// Axis-agnostic insets.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Insets {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

impl Insets {
    /// Build symmetrical insets for every edge.
    pub fn all(value: f32) -> Self {
        Self {
            left: value,
            right: value,
            top: value,
            bottom: value,
        }
    }

    /// Return total horizontal inset.
    pub fn horizontal(self) -> f32 {
        self.left + self.right
    }

    /// Return total vertical inset.
    pub fn vertical(self) -> f32 {
        self.top + self.bottom
    }
}

/// Parent-owned slot configuration for a single child.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SlotParams {
    /// Main-axis sizing mode.
    pub size_main: SizeModeMain,
    /// Cross-axis sizing mode.
    pub size_cross: SizeModeCross,
    /// Child constraints applied by this slot.
    pub constraints: Constraints,
    /// Slot margins around the child.
    pub margin: Insets,
    /// Optional per-slot cross-axis alignment override.
    pub align_cross_override: Option<CrossAlign>,
    /// Whether fixed-size slots can be compressed under hard overflow.
    pub allow_fixed_compress: bool,
}

impl SlotParams {
    /// Create a fill/fill slot with unconstrained limits and zero margin.
    pub fn fill() -> Self {
        Self {
            size_main: SizeModeMain::Fill(1.0),
            size_cross: SizeModeCross::Fill,
            constraints: Constraints::unconstrained(),
            margin: Insets::default(),
            align_cross_override: None,
            allow_fixed_compress: false,
        }
    }
}

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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContainerKind {
    Row,
    Column,
    Stack,
    PaddingBox,
    AlignBox,
    AspectBox,
    Grid,
    ScrollView,
    Wrap,
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

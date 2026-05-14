use super::super::super::model::{MainAlign, OverflowPolicy};

/// Overflow metadata recorded for one node.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OverflowInfo {
    /// True when width overflowed.
    pub x: bool,
    /// True when height overflowed.
    pub y: bool,
    /// Policy used when handling overflow.
    pub policy: OverflowPolicy,
}

impl Default for OverflowInfo {
    fn default() -> Self {
        Self {
            x: false,
            y: false,
            policy: OverflowPolicy::Clip,
        }
    }
}

/// Virtualized window metadata for a scroll container.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VirtualWindowInfo {
    /// Total children available in the virtualized content list.
    pub total_children: usize,
    /// First materialized child index.
    pub first_index: usize,
    /// Exclusive end index of materialized children.
    pub last_index_exclusive: usize,
    /// Number of children culled before the window.
    pub culled_before: usize,
    /// Number of children culled after the window.
    pub culled_after: usize,
    /// Viewport start on the virtualization axis.
    pub viewport_main_start: f32,
    /// Viewport end on the virtualization axis.
    pub viewport_main_end: f32,
    /// Window start on the virtualization axis.
    pub window_main_start: f32,
    /// Window end on the virtualization axis.
    pub window_main_end: f32,
    /// Total resolved main-axis extent for the content container.
    pub resolved_total_main: f32,
    /// Resolved main-axis alignment mode.
    pub alignment_mode: MainAlign,
}

impl Default for VirtualWindowInfo {
    fn default() -> Self {
        Self {
            total_children: 0,
            first_index: 0,
            last_index_exclusive: 0,
            culled_before: 0,
            culled_after: 0,
            viewport_main_start: 0.0,
            viewport_main_end: 0.0,
            window_main_start: 0.0,
            window_main_end: 0.0,
            resolved_total_main: 0.0,
            alignment_mode: MainAlign::Start,
        }
    }
}

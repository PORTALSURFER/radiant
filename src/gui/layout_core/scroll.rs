//! Backend-neutral scroll policy and request values.

use super::controlled::Controlled;
use crate::gui::layout_core::virtual_layout::VirtualLayoutItemKey;
use crate::gui::types::{Rect, Vector2};

/// Axes on which a scroll viewport may consume movement.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ScrollAxis {
    /// Only horizontal movement is accepted.
    Horizontal,
    /// Only vertical movement is accepted.
    #[default]
    Vertical,
    /// Both axes are accepted.
    Both,
}

impl ScrollAxis {
    /// Return whether this selection includes the horizontal axis.
    pub const fn includes_horizontal(self) -> bool {
        matches!(self, Self::Horizontal | Self::Both)
    }
    /// Return whether this selection includes the vertical axis.
    pub const fn includes_vertical(self) -> bool {
        matches!(self, Self::Vertical | Self::Both)
    }
}

/// Optional dominant-axis lock for wheel and gesture input.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ScrollAxisLock {
    /// Preserve both components.
    #[default]
    None,
    /// Consume horizontal movement only.
    Horizontal,
    /// Consume vertical movement only.
    Vertical,
}

/// Scrollbar placement relative to the viewport.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ScrollbarPlacement {
    /// Paint the bar over the content edge.
    #[default]
    Overlay,
    /// Reserve a logical gutter for the bar.
    Reserved,
}

/// Scrollbar visibility policy.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ScrollbarVisibility {
    /// Show bars while the viewport is scrolling or hovered.
    #[default]
    Auto,
    /// Keep bars visible whenever the corresponding axis overflows.
    Always,
    /// Do not paint or hit-test bars.
    Hidden,
}

/// Runtime behavior for a scroll viewport.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollPolicy {
    /// Axes that may scroll.
    pub axes: ScrollAxis,
    /// Dominant-axis lock for wheel/gesture deltas.
    pub axis_lock: ScrollAxisLock,
    /// Placement of horizontal and vertical bars.
    pub scrollbar_placement: ScrollbarPlacement,
    /// Visibility of horizontal and vertical bars.
    pub scrollbar_visibility: ScrollbarVisibility,
    /// Fraction of the viewport used for PageUp/PageDown.
    pub page_fraction: f32,
    /// Whether unconsumed wheel movement may bubble to an ancestor.
    pub chaining: bool,
    legacy_both_axes: bool,
}

impl Default for ScrollPolicy {
    fn default() -> Self {
        Self {
            axes: ScrollAxis::Vertical,
            axis_lock: ScrollAxisLock::None,
            scrollbar_placement: ScrollbarPlacement::Overlay,
            scrollbar_visibility: ScrollbarVisibility::Auto,
            page_fraction: 1.0,
            chaining: true,
            legacy_both_axes: true,
        }
    }
}

impl ScrollPolicy {
    /// Set the scrollable axes.
    pub const fn axes(mut self, axes: ScrollAxis) -> Self {
        self.axes = axes;
        self.legacy_both_axes = false;
        self
    }
    /// Set the axis lock.
    pub const fn axis_lock(mut self, lock: ScrollAxisLock) -> Self {
        self.axis_lock = lock;
        self
    }
    /// Set overlay or reserved bar placement.
    pub const fn scrollbar_placement(mut self, placement: ScrollbarPlacement) -> Self {
        self.scrollbar_placement = placement;
        self
    }
    /// Set bar visibility.
    pub const fn scrollbar_visibility(mut self, visibility: ScrollbarVisibility) -> Self {
        self.scrollbar_visibility = visibility;
        self
    }
    /// Set the page amount as a viewport fraction.
    pub const fn page_fraction(mut self, fraction: f32) -> Self {
        self.page_fraction = fraction;
        self
    }
    /// Enable or disable ancestor chaining.
    pub const fn chaining(mut self, chaining: bool) -> Self {
        self.chaining = chaining;
        self
    }
    /// Alias for [`Self::scrollbar_placement`].
    pub const fn bars(mut self, placement: ScrollbarPlacement) -> Self {
        self.scrollbar_placement = placement;
        self
    }
    /// Alias for [`Self::scrollbar_visibility`].
    pub const fn visibility(mut self, visibility: ScrollbarVisibility) -> Self {
        self.scrollbar_visibility = visibility;
        self
    }
    /// Normalize values supplied by an untrusted builder boundary.
    pub fn normalized(self) -> Self {
        Self {
            page_fraction: if self.page_fraction.is_finite() {
                self.page_fraction.clamp(0.1, 4.0)
            } else {
                1.0
            },
            ..self
        }
    }

    /// Preserve the pre-policy scroll view's ability to consume horizontal
    /// wheel deltas when no explicit axis policy was supplied.
    pub(crate) const fn allows_legacy_horizontal(self) -> bool {
        self.legacy_both_axes
    }
}

/// Alignment requested when revealing a target.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ScrollAlignment {
    /// Move only enough to make the target visible.
    #[default]
    Nearest,
    /// Align the target's leading edge with the viewport's leading edge.
    Start,
    /// Center the target in the viewport.
    Center,
    /// Align the target's trailing edge with the viewport's trailing edge.
    End,
}

/// An edge of the scrollable content.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ScrollEdge {
    /// Content top edge.
    Top,
    /// Content bottom edge.
    Bottom,
    /// Content left edge.
    Left,
    /// Content right edge.
    Right,
    /// Content leading edge along the configured axis.
    Start,
    /// Content trailing edge along the configured axis.
    End,
}

/// A materialized target for a reveal request.
#[derive(Clone, Debug, PartialEq)]
pub enum ScrollTarget {
    /// A stable keyed item; unavailable keys fail closed.
    Keyed(VirtualLayoutItemKey),
    /// A finite rectangle in the scroll content's logical coordinates.
    Rect(Rect),
    /// A content edge.
    Edge(ScrollEdge),
}

impl ScrollTarget {
    /// Construct a keyed target.
    pub fn keyed(key: VirtualLayoutItemKey) -> Self {
        Self::Keyed(key)
    }
    /// Construct a rectangle target.
    pub const fn rect(rect: Rect) -> Self {
        Self::Rect(rect)
    }
    /// Construct an edge target.
    pub const fn edge(edge: ScrollEdge) -> Self {
        Self::Edge(edge)
    }
}

/// One generation-bearing, one-shot request to reveal a target.
#[derive(Clone, Debug, PartialEq)]
pub struct ScrollRequest {
    /// Target to resolve against committed materialized geometry.
    pub target: ScrollTarget,
    /// Alignment to apply after target resolution.
    pub alignment: ScrollAlignment,
    /// Caller-owned monotonic request generation.
    pub generation: u64,
}

impl ScrollRequest {
    /// Construct a request with explicit generation and alignment.
    pub const fn new(target: ScrollTarget, alignment: ScrollAlignment, generation: u64) -> Self {
        Self {
            target,
            alignment,
            generation,
        }
    }
    /// Construct a rectangle reveal request.
    pub const fn rect(rect: Rect, alignment: ScrollAlignment, generation: u64) -> Self {
        Self::new(ScrollTarget::Rect(rect), alignment, generation)
    }
    /// Construct an edge reveal request.
    pub const fn edge(edge: ScrollEdge, alignment: ScrollAlignment, generation: u64) -> Self {
        Self::new(ScrollTarget::Edge(edge), alignment, generation)
    }
}

/// Declarative scroll inputs carried by a mounted container.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ScrollDeclaration {
    /// Optional one-time mount seed.
    pub initial_offset: Option<Vector2>,
    /// Optional strictly newer controlled value.
    pub controlled_offset: Option<Controlled<Vector2>>,
    /// Optional one-shot reveal request.
    pub request: Option<ScrollRequest>,
}

/// Resolve one finite axis target according to an alignment.
pub fn resolve_scroll_alignment(
    current: f32,
    viewport: f32,
    target_min: f32,
    target_max: f32,
    alignment: ScrollAlignment,
) -> f32 {
    let viewport = viewport.max(0.0);
    match alignment {
        ScrollAlignment::Nearest => {
            if target_min < current {
                target_min
            } else if target_max > current + viewport {
                target_max - viewport
            } else {
                current
            }
        }
        ScrollAlignment::Start => target_min,
        ScrollAlignment::Center => (target_min + target_max - viewport) * 0.5,
        ScrollAlignment::End => target_max - viewport,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alignment_resolves_all_modes_without_mutating_input() {
        assert_eq!(
            resolve_scroll_alignment(10.0, 100.0, 0.0, 20.0, ScrollAlignment::Nearest),
            0.0
        );
        assert_eq!(
            resolve_scroll_alignment(10.0, 100.0, 20.0, 40.0, ScrollAlignment::Start),
            20.0
        );
        assert_eq!(
            resolve_scroll_alignment(10.0, 100.0, 20.0, 40.0, ScrollAlignment::Center),
            -20.0
        );
        assert_eq!(
            resolve_scroll_alignment(10.0, 100.0, 20.0, 40.0, ScrollAlignment::End),
            -60.0
        );
    }

    #[test]
    fn policy_normalizes_non_finite_page_fraction() {
        let policy = ScrollPolicy::default().page_fraction(f32::NAN).normalized();
        assert_eq!(policy.page_fraction, 1.0);
    }
}

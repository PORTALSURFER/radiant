//! Generic tree-guide geometry and paint helpers for virtualized dense trees.

use crate::{
    application::{View, custom_widget, spacer},
    gui::types::{Point, Rect, Rgba8, Vector2},
    layout::LayoutOutput,
    runtime::{PaintPrimitive, push_fill_rect},
    theme::ThemeTokens,
    widgets::{PaintBounds, Widget, WidgetCommon, WidgetInput, WidgetOutput, WidgetSizing},
};

const TREE_GUIDE_OVERLAY_WIDGET_ID: u64 = 0x7261_6469_616e_7404;

/// Domain-neutral metadata for one projected tree row.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct TreeGuideRow {
    /// Row nesting depth where zero is the root level.
    pub depth: usize,
    /// Whether this row starts a visible descendant group that should receive a guide line.
    pub starts_descendant_group: bool,
}

impl TreeGuideRow {
    /// Build tree-guide row metadata from depth and descendant-group state.
    pub const fn new(depth: usize, starts_descendant_group: bool) -> Self {
        Self {
            depth,
            starts_descendant_group,
        }
    }
}

/// A continuous vertical guide segment spanning a descendant group.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TreeGuideSegment {
    /// Nesting level that owns the guide line.
    pub level: usize,
    /// Inclusive row index where the guide begins.
    pub start_row: usize,
    /// Exclusive row index where the guide ends.
    pub end_row_exclusive: usize,
}

/// Visual and sizing parameters for tree guide projection.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TreeGuideStyle {
    /// Per-level horizontal indent in logical pixels.
    pub indent_width: f32,
    /// Fixed tree row height in logical pixels.
    pub row_height: f32,
    /// Guide stroke width in logical pixels.
    pub guide_width: f32,
    /// Gap trimmed from the guide end when the final row is materialized.
    pub end_gap: f32,
    /// Guide color.
    pub color: Rgba8,
}

impl TreeGuideStyle {
    /// Build tree-guide style from the required row geometry and color.
    pub const fn new(indent_width: f32, row_height: f32, color: Rgba8) -> Self {
        Self {
            indent_width,
            row_height,
            guide_width: 1.0,
            end_gap: 5.0,
            color,
        }
    }

    /// Set guide stroke width.
    pub const fn guide_width(mut self, width: f32) -> Self {
        self.guide_width = width;
        self
    }

    /// Set the gap trimmed from the guide end when the final row is visible.
    pub const fn end_gap(mut self, gap: f32) -> Self {
        self.end_gap = gap;
        self
    }
}

/// Paint-only widget for continuous vertical guides over a materialized tree window.
#[derive(Clone, Debug, PartialEq)]
pub struct TreeGuideOverlay {
    common: WidgetCommon,
    first_row: usize,
    row_count: usize,
    segments: Vec<TreeGuideSegment>,
    style: TreeGuideStyle,
}

impl TreeGuideOverlay {
    /// Build an overlay for the supplied materialized window and full-list segments.
    pub fn new(
        first_row: usize,
        row_count: usize,
        segments: Vec<TreeGuideSegment>,
        style: TreeGuideStyle,
    ) -> Self {
        let mut common = WidgetCommon::new(
            TREE_GUIDE_OVERLAY_WIDGET_ID,
            WidgetSizing::fixed(Vector2::new(0.0, row_count as f32 * style.row_height)),
        )
        .without_default_chrome();
        common.paint.bounds = PaintBounds::AllowOverflow;
        common.state.disabled = true;
        Self {
            common,
            first_row,
            row_count,
            segments,
            style,
        }
    }
}

impl Widget for TreeGuideOverlay {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, _bounds: Rect, _input: WidgetInput) -> Option<WidgetOutput> {
        None
    }

    fn needs_state_synchronization(&self) -> bool {
        false
    }

    fn accepts_pointer_move(&self) -> bool {
        false
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        _theme: &ThemeTokens,
    ) {
        let Some(style) = normalized_tree_guide_style(self.style) else {
            return;
        };
        let last_row = self.first_row + self.row_count;
        for segment in &self.segments {
            let start = segment.start_row.max(self.first_row);
            let end = segment.end_row_exclusive.min(last_row);
            if start >= end {
                continue;
            }

            let x = tree_guide_x(bounds, segment.level, style);
            let y0 = bounds.min.y + (start - self.first_row) as f32 * style.row_height;
            let raw_y1 = bounds.min.y + (end - self.first_row) as f32 * style.row_height;
            let y1 = if segment.end_row_exclusive <= last_row {
                raw_y1 - style.end_gap
            } else {
                raw_y1
            };
            if y1 <= y0 {
                continue;
            }
            push_fill_rect(
                primitives,
                self.common.id,
                Rect::from_min_max(Point::new(x, y0), Point::new(x + style.guide_width, y1)),
                style.color,
            );
        }
    }
}

/// Build a passive overlay view for a materialized tree window.
pub fn tree_guide_overlay<Message: 'static>(
    rows: &[TreeGuideRow],
    first_row: usize,
    end_row: usize,
    style: TreeGuideStyle,
) -> View<Message> {
    let row_count = end_row.saturating_sub(first_row);
    custom_widget(
        TreeGuideOverlay::new(first_row, row_count, tree_guide_segments(rows), style),
        |_| None,
    )
    .key(format!("tree-guide-overlay-{first_row}-{end_row}"))
    .fill_width()
    .height(row_count as f32 * style.row_height.max(0.0))
}

/// Build a passive indent spacer for a tree row depth.
pub fn tree_guide_indent<Message: 'static>(depth: usize, style: TreeGuideStyle) -> View<Message> {
    spacer()
        .width(depth as f32 * style.indent_width.max(0.0))
        .height(style.row_height.max(0.0))
}

/// Project continuous vertical guide segments from tree row metadata.
pub fn tree_guide_segments(rows: &[TreeGuideRow]) -> Vec<TreeGuideSegment> {
    let mut segments = Vec::new();
    let mut open_groups = Vec::new();

    for (index, row) in rows.iter().enumerate() {
        close_finished_tree_guide_groups(index, row.depth, &mut open_groups, &mut segments);
        if row.starts_descendant_group {
            open_groups.push(OpenTreeGuideGroup {
                parent_depth: row.depth,
                segment_index: segments.len(),
            });
            segments.push(TreeGuideSegment {
                level: row.depth.saturating_sub(1),
                start_row: index,
                end_row_exclusive: rows.len(),
            });
        }
    }

    segments.retain(|segment| segment.end_row_exclusive > segment.start_row.saturating_add(1));
    segments
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct OpenTreeGuideGroup {
    parent_depth: usize,
    segment_index: usize,
}

fn close_finished_tree_guide_groups(
    end_row: usize,
    row_depth: usize,
    open_groups: &mut Vec<OpenTreeGuideGroup>,
    segments: &mut [TreeGuideSegment],
) {
    while open_groups
        .last()
        .is_some_and(|group| row_depth <= group.parent_depth)
    {
        let Some(group) = open_groups.pop() else {
            break;
        };
        segments[group.segment_index].end_row_exclusive = end_row;
    }
}

fn tree_guide_x(bounds: Rect, level: usize, style: TreeGuideStyle) -> f32 {
    bounds.min.x
        + level as f32 * style.indent_width
        + (style.indent_width - style.guide_width) * 0.5
}

fn normalized_tree_guide_style(style: TreeGuideStyle) -> Option<TreeGuideStyle> {
    let valid = style.indent_width.is_finite()
        && style.indent_width > 0.0
        && style.row_height.is_finite()
        && style.row_height > 0.0
        && style.guide_width.is_finite()
        && style.guide_width > 0.0
        && style.end_gap.is_finite()
        && style.end_gap >= 0.0;
    valid.then_some(style)
}

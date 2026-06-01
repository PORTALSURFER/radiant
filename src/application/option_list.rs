use super::{BoundedScrollColumnParts, ViewNode, bounded_scroll_column_from_parts, row, text};
use crate::widgets::{WidgetProminence, WidgetStyle, WidgetTone};

const DEFAULT_COMPACT_OPTION_LIST_MAX_VISIBLE_ROWS: usize = 6;
const DEFAULT_COMPACT_OPTION_LIST_ROW_HEIGHT: f32 = 18.0;
const DEFAULT_COMPACT_OPTION_LIST_VERTICAL_CHROME: f32 = 6.0;
const DEFAULT_COMPACT_OPTION_LIST_PADDING: f32 = 3.0;
const DEFAULT_COMPACT_OPTION_LIST_GAP: f32 = 6.0;

/// One display row in a compact option list.
#[derive(Clone, Debug, PartialEq)]
pub struct CompactOptionListItem {
    /// Main option label.
    pub primary_label: String,
    /// Optional secondary label for group, category, shortcut, or metadata text.
    pub secondary_label: Option<String>,
    /// Whether this row is the active keyboard or current selection.
    pub selected: bool,
}

impl CompactOptionListItem {
    /// Build a compact option-list item.
    pub fn new(primary_label: impl Into<String>) -> Self {
        Self {
            primary_label: primary_label.into(),
            secondary_label: None,
            selected: false,
        }
    }

    /// Set the optional secondary label.
    pub fn secondary_label(mut self, secondary_label: impl Into<String>) -> Self {
        self.secondary_label = Some(secondary_label.into());
        self
    }

    /// Set whether this row is selected.
    pub const fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }
}

/// Named construction fields for a compact fixed-row option list.
#[derive(Clone, Debug, PartialEq)]
pub struct CompactOptionListParts {
    /// Ordered option rows.
    pub items: Vec<CompactOptionListItem>,
    /// Maximum number of rows visible before scrolling.
    pub max_visible_rows: usize,
    /// Fixed row height.
    pub row_height: f32,
    /// Vertical chrome included in the capped scroll height.
    pub vertical_chrome: f32,
    /// Fixed width for the primary label column.
    pub primary_label_width: f32,
    /// Gap between primary and secondary columns.
    pub column_gap: f32,
    /// Style applied to the scroll viewport.
    pub style: WidgetStyle,
    /// Padding applied inside the scroll viewport.
    pub padding: f32,
}

impl CompactOptionListParts {
    /// Build compact option-list parts with standard autocomplete/menu metrics.
    pub fn new(items: Vec<CompactOptionListItem>, primary_label_width: f32) -> Self {
        Self {
            items,
            max_visible_rows: DEFAULT_COMPACT_OPTION_LIST_MAX_VISIBLE_ROWS,
            row_height: DEFAULT_COMPACT_OPTION_LIST_ROW_HEIGHT,
            vertical_chrome: DEFAULT_COMPACT_OPTION_LIST_VERTICAL_CHROME,
            primary_label_width,
            column_gap: DEFAULT_COMPACT_OPTION_LIST_GAP,
            style: WidgetStyle::new(WidgetTone::Neutral, WidgetProminence::Subtle),
            padding: DEFAULT_COMPACT_OPTION_LIST_PADDING,
        }
    }

    /// Set the maximum number of visible rows before scrolling.
    pub const fn max_visible_rows(mut self, max_visible_rows: usize) -> Self {
        self.max_visible_rows = max_visible_rows;
        self
    }

    /// Set fixed row height.
    pub const fn row_height(mut self, row_height: f32) -> Self {
        self.row_height = row_height;
        self
    }

    /// Set vertical chrome included in the capped scroll height.
    pub const fn vertical_chrome(mut self, vertical_chrome: f32) -> Self {
        self.vertical_chrome = vertical_chrome;
        self
    }

    /// Set the primary label column width.
    pub const fn primary_label_width(mut self, primary_label_width: f32) -> Self {
        self.primary_label_width = primary_label_width;
        self
    }

    /// Set the gap between primary and secondary columns.
    pub const fn column_gap(mut self, column_gap: f32) -> Self {
        self.column_gap = column_gap;
        self
    }

    /// Set the style applied to the scroll viewport.
    pub const fn style(mut self, style: WidgetStyle) -> Self {
        self.style = style;
        self
    }

    /// Set uniform padding inside the scroll viewport.
    pub const fn padding(mut self, padding: f32) -> Self {
        self.padding = padding;
        self
    }
}

/// Build a compact selected option list from ordered items.
pub fn compact_option_list<Message: 'static>(
    items: Vec<CompactOptionListItem>,
    primary_label_width: f32,
) -> ViewNode<Message> {
    compact_option_list_from_parts(CompactOptionListParts::new(items, primary_label_width))
}

/// Build a compact selected option list from named parts.
pub fn compact_option_list_from_parts<Message: 'static>(
    parts: CompactOptionListParts,
) -> ViewNode<Message> {
    let max_visible_rows = parts.max_visible_rows;
    let row_height = parts.row_height;
    let vertical_chrome = parts.vertical_chrome;
    let primary_label_width = parts.primary_label_width;
    let column_gap = parts.column_gap;
    let style = parts.style;
    let padding = parts.padding;
    let rows = parts
        .items
        .into_iter()
        .enumerate()
        .map(|(index, item)| {
            compact_option_list_row(index, item, row_height, primary_label_width, column_gap)
        })
        .collect::<Vec<_>>();
    bounded_scroll_column_from_parts(
        BoundedScrollColumnParts::new(rows, max_visible_rows, row_height, vertical_chrome)
            .style(style)
            .padding(padding),
    )
}

fn compact_option_list_row<Message: 'static>(
    index: usize,
    item: CompactOptionListItem,
    row_height: f32,
    primary_label_width: f32,
    column_gap: f32,
) -> ViewNode<Message> {
    row([
        text(item.primary_label)
            .height(row_height)
            .width(primary_label_width.max(0.0))
            .truncate(),
        text(item.secondary_label.unwrap_or_default())
            .height(row_height)
            .fill_width()
            .truncate(),
    ])
    .key(format!("compact-option-list-row-{index}"))
    .style(if item.selected {
        WidgetStyle::new(WidgetTone::Accent, WidgetProminence::Strong)
    } else {
        WidgetStyle::default()
    })
    .height(row_height)
    .fill_width()
    .spacing(column_gap.max(0.0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        application::{IntoView, column},
        layout::{LayoutNode, SizeModeMain},
    };

    #[test]
    fn compact_option_list_caps_height_and_keeps_empty_lists_hidden() {
        let empty = compact_option_list::<()>(Vec::new(), 80.0);
        let layout = column([empty]).into_surface().layout_node();
        let LayoutNode::Container(parent_column) = layout else {
            panic!("parent should lower to a column container");
        };
        assert!(matches!(
            parent_column.children[0].slot.size_main,
            SizeModeMain::Fixed(height) if height == 0.0
        ));

        let items = (0..12)
            .map(|index| {
                CompactOptionListItem::new(format!("Item {index}"))
                    .secondary_label("Group")
                    .selected(index == 1)
            })
            .collect::<Vec<_>>();
        let view = compact_option_list::<()>(items, 80.0);
        let layout = column([view]).into_surface().layout_node();
        let LayoutNode::Container(parent_column) = layout else {
            panic!("parent should lower to a column container");
        };
        assert!(matches!(
            parent_column.children[0].slot.size_main,
            SizeModeMain::Fixed(height) if (height - 114.0).abs() < 0.01
        ));
    }
}

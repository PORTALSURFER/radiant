use crate::application::TextContent;
use crate::gui::list::bounded_list_height;
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
    pub primary_label: TextContent,
    /// Optional secondary label for group, category, shortcut, or metadata text.
    pub secondary_label: Option<TextContent>,
    /// Whether this row is the active keyboard or current selection.
    pub selected: bool,
}

impl CompactOptionListItem {
    /// Build a compact option-list item.
    pub fn new(primary_label: impl Into<TextContent>) -> Self {
        Self {
            primary_label: primary_label.into(),
            secondary_label: None,
            selected: false,
        }
    }

    /// Set the optional secondary label.
    pub fn secondary_label(mut self, secondary_label: impl Into<TextContent>) -> Self {
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

    /// Return the fixed viewport height implied by these option-list parts.
    pub fn height(&self) -> f32 {
        bounded_list_height(
            self.items.len(),
            self.max_visible_rows,
            self.row_height,
            self.vertical_chrome,
        )
    }
}

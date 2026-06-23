//! Generic chrome and status-surface primitives.

#[cfg(test)]
#[path = "chrome/tests.rs"]
mod tests;

/// Structured status content for left/center/right chrome segments.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct StatusSegments {
    /// Left-aligned status segment.
    pub left: String,
    /// Center-aligned status segment.
    pub center: String,
    /// Right-aligned status segment.
    pub right: String,
}

/// Named fields for constructing left, center, and right status segments.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct StatusSegmentsParts {
    /// Left-aligned status segment.
    pub left: String,
    /// Center-aligned status segment.
    pub center: String,
    /// Right-aligned status segment.
    pub right: String,
}

impl StatusSegments {
    /// Build status segments from named parts.
    pub fn from_parts(parts: StatusSegmentsParts) -> Self {
        Self {
            left: parts.left,
            center: parts.center,
            right: parts.right,
        }
    }

    /// Build status segments from explicit left, center, and right labels.
    pub fn new(
        left: impl Into<String>,
        center: impl Into<String>,
        right: impl Into<String>,
    ) -> Self {
        Self::from_parts(StatusSegmentsParts {
            left: left.into(),
            center: center.into(),
            right: right.into(),
        })
    }

    /// Build a status segment set with only the primary left label populated.
    pub fn primary(left: impl Into<String>) -> Self {
        Self::new(left, "", "")
    }

    /// Build status segments with left and center labels populated.
    ///
    /// Use this when trailing controls or progress content occupy the right
    /// side of a status bar and no right text segment is needed.
    pub fn left_center(left: impl Into<String>, center: impl Into<String>) -> Self {
        Self::new(left, center, "")
    }

    /// Return these segments with a replaced left label.
    pub fn with_left(mut self, left: impl Into<String>) -> Self {
        self.left = left.into();
        self
    }

    /// Return these segments with a replaced center label.
    pub fn with_center(mut self, center: impl Into<String>) -> Self {
        self.center = center.into();
        self
    }

    /// Return these segments with a replaced right label.
    pub fn with_right(mut self, right: impl Into<String>) -> Self {
        self.right = right.into();
        self
    }
}

/// Product-neutral tab and primary action copy for a searchable content view.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContentViewTabs {
    /// Label for the primary item/list tab.
    pub items_tab_label: String,
    /// Label for the primary item/list column header.
    pub item_column_label: String,
    /// Label for the secondary map or visualization tab.
    pub map_tab_label: String,
    /// Label for the pill/badge editor action.
    pub pill_editor_label: String,
}

impl Default for ContentViewTabs {
    fn default() -> Self {
        Self {
            items_tab_label: String::from("Items"),
            item_column_label: String::from("Item"),
            map_tab_label: String::from("Map"),
            pill_editor_label: String::from("Pills"),
        }
    }
}

/// Product-neutral search copy for a searchable content view.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContentViewSearchChrome {
    /// Prefix label shown before active search queries.
    pub search_prefix_label: String,
    /// Placeholder label shown when no search query is active.
    pub search_placeholder: String,
}

impl Default for ContentViewSearchChrome {
    fn default() -> Self {
        Self {
            search_prefix_label: String::from("Search"),
            search_placeholder: String::from("Search items (Ctrl+F)"),
        }
    }
}

/// Product-neutral activity copy for a searchable content view.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContentViewActivityChrome {
    /// Status label shown when background work is idle.
    pub activity_ready_label: String,
    /// Status label shown when background work is running.
    pub activity_busy_label: String,
}

impl Default for ContentViewActivityChrome {
    fn default() -> Self {
        Self {
            activity_ready_label: String::from("Ready"),
            activity_busy_label: String::from("Filtering"),
        }
    }
}

/// Product-neutral sort and relatedness copy for a searchable content view.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContentViewSortChrome {
    /// Prefix label shown before active sort order labels.
    pub sort_prefix_label: String,
    /// Label describing the active sort order.
    pub sort_order_label: String,
    /// Label describing relatedness or map mode in view chrome.
    pub similarity_toggle_label: String,
}

impl Default for ContentViewSortChrome {
    fn default() -> Self {
        Self {
            sort_prefix_label: String::from("Sort"),
            sort_order_label: String::from("List order"),
            similarity_toggle_label: String::from("points"),
        }
    }
}

/// Product-neutral footer copy for a searchable content view.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContentViewFooterChrome {
    /// Footer/status label for total item counts.
    pub item_count_label: String,
}

impl Default for ContentViewFooterChrome {
    fn default() -> Self {
        Self {
            item_count_label: String::from("0 items"),
        }
    }
}

/// Product-neutral chrome copy for a searchable content view.
///
/// Hosts provide product-specific wording by mapping their own labels into
/// these generic slots before rendering.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct ContentViewChrome {
    /// Tab and primary action labels.
    pub tabs: ContentViewTabs,
    /// Search labels.
    pub search: ContentViewSearchChrome,
    /// Background activity labels.
    pub activity: ContentViewActivityChrome,
    /// Sort and relatedness labels.
    pub sort: ContentViewSortChrome,
    /// Footer/status labels.
    pub footer: ContentViewFooterChrome,
}

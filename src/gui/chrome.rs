//! Generic chrome and status-surface primitives.

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

/// Product-neutral chrome copy for a searchable content view.
///
/// Hosts provide product-specific wording by mapping their own labels into
/// these generic slots before rendering.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContentViewChrome {
    /// Label for the primary item/list tab.
    pub items_tab_label: String,
    /// Label for the secondary map or visualization tab.
    pub map_tab_label: String,
    /// Prefix label shown before active search queries.
    pub search_prefix_label: String,
    /// Placeholder label shown when no search query is active.
    pub search_placeholder: String,
    /// Status label shown when background work is idle.
    pub activity_ready_label: String,
    /// Status label shown when background work is running.
    pub activity_busy_label: String,
    /// Prefix label shown before active sort order labels.
    pub sort_prefix_label: String,
    /// Label describing the active sort order.
    pub sort_order_label: String,
    /// Label describing relatedness or map mode in view chrome.
    pub similarity_toggle_label: String,
    /// Footer/status label for total item counts.
    pub item_count_label: String,
}

impl Default for ContentViewChrome {
    fn default() -> Self {
        Self {
            items_tab_label: String::from("Items"),
            map_tab_label: String::from("Map"),
            search_prefix_label: String::from("Search"),
            search_placeholder: String::from("Search items (Ctrl+F)"),
            activity_ready_label: String::from("Ready"),
            activity_busy_label: String::from("Filtering"),
            sort_prefix_label: String::from("Sort"),
            sort_order_label: String::from("List order"),
            similarity_toggle_label: String::from("points"),
            item_count_label: String::from("0 items"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ContentViewChrome, StatusSegments};

    #[test]
    fn status_segments_default_to_empty_text() {
        assert_eq!(StatusSegments::default().left, "");
        assert_eq!(StatusSegments::default().center, "");
        assert_eq!(StatusSegments::default().right, "");
    }

    #[test]
    fn content_view_chrome_defaults_to_product_neutral_copy() {
        let chrome = ContentViewChrome::default();

        assert_eq!(chrome.items_tab_label, "Items");
        assert_eq!(chrome.map_tab_label, "Map");
        assert_eq!(chrome.search_placeholder, "Search items (Ctrl+F)");
        assert_eq!(chrome.activity_busy_label, "Filtering");
        assert_eq!(chrome.item_count_label, "0 items");
    }
}

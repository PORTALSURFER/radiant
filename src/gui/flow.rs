//! Generic data-side helpers for wrapping variable-width inline items.

/// Geometry policy for a wrapped inline flow.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FlowLayoutMetricsParts {
    /// Horizontal gap between adjacent items in one row.
    pub item_gap: f32,
    /// Vertical gap between rows.
    pub line_gap: f32,
    /// Height of one row.
    pub item_height: f32,
}

/// Geometry policy for a wrapped inline flow.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FlowLayoutMetrics {
    /// Horizontal gap between adjacent items in one row.
    pub item_gap: f32,
    /// Vertical gap between rows.
    pub line_gap: f32,
    /// Height of one row.
    pub item_height: f32,
}

impl FlowLayoutMetrics {
    /// Construct metrics from named already-resolved logical-pixel values.
    pub const fn from_parts(parts: FlowLayoutMetricsParts) -> Self {
        Self {
            item_gap: parts.item_gap,
            line_gap: parts.line_gap,
            item_height: parts.item_height,
        }
    }

    /// Construct metrics from resolved logical-pixel values.
    pub const fn new(item_gap: f32, line_gap: f32, item_height: f32) -> Self {
        Self::from_parts(FlowLayoutMetricsParts {
            item_gap,
            line_gap,
            item_height,
        })
    }
}

/// One item and its desired main-axis width for flow packing.
#[derive(Clone, Debug, PartialEq)]
pub struct FlowItem<T> {
    /// Caller-owned item payload.
    pub value: T,
    /// Desired width in logical pixels.
    pub width: f32,
}

impl<T> FlowItem<T> {
    /// Construct one flow item from a caller payload and desired width.
    pub const fn new(value: T, width: f32) -> Self {
        Self { value, width }
    }
}

/// Named trailing editor/control policy for wrapped inline flows.
pub struct FlowTrailingItemParts<Create> {
    /// Build the caller-owned trailing payload from the resolved width.
    pub create: Create,
    /// Desired width when the trailing item stays on a row with other items.
    pub width: f32,
    /// Desired width when the trailing item starts an otherwise empty row.
    pub standalone_width: f32,
    /// Minimum remaining row width required before keeping the trailing item
    /// on the current row.
    pub min_remaining_width: f32,
}

impl<Create> FlowTrailingItemParts<Create> {
    /// Construct a named trailing editor/control policy.
    pub const fn new(
        create: Create,
        width: f32,
        standalone_width: f32,
        min_remaining_width: f32,
    ) -> Self {
        Self {
            create,
            width,
            standalone_width,
            min_remaining_width,
        }
    }
}

impl<T> FlowItemWidth for FlowItem<T> {
    fn flow_width(&self) -> f32 {
        self.width
    }
}

/// Pack variable-width items into rows for a wrapping inline flow.
///
/// This mirrors Radiant's wrap-layout row break policy for data-driven widgets
/// that need to know row count before constructing a view tree.
pub fn pack_flow_rows<T>(
    items: impl IntoIterator<Item = FlowItem<T>>,
    content_width: f32,
    metrics: FlowLayoutMetrics,
) -> Vec<Vec<T>> {
    let mut rows = Vec::new();
    let mut current_width = 0.0;
    let content_width = content_width.max(0.0);
    let item_gap = metrics.item_gap.max(0.0);

    for item in items {
        let width = item.width.max(0.0);
        let proposed = proposed_row_width(current_width, width, item_gap);
        if proposed > content_width && current_width > 0.0 {
            rows.push(Vec::new());
            current_width = 0.0;
        }

        current_width = proposed_row_width(current_width, width, item_gap);
        if rows.is_empty() {
            rows.push(Vec::new());
        }
        if let Some(row) = rows.last_mut() {
            row.push(item.value);
        }
    }

    rows
}

/// Pack variable-width items and append one trailing editor/control item.
///
/// This is useful for chip, pill, tag, recipient, and token editors where a
/// final text field or action control should stay on the current row when
/// enough usable space remains, but start on a new row when editing would be
/// cramped. `trailing.create` receives the width that should be embedded in the
/// caller-owned trailing payload. When the trailing item starts an otherwise
/// empty row, `trailing.standalone_width` is used.
pub fn pack_flow_rows_with_trailing_item<T, Create>(
    items: impl IntoIterator<Item = FlowItem<T>>,
    trailing: FlowTrailingItemParts<Create>,
    content_width: f32,
    metrics: FlowLayoutMetrics,
) -> Vec<Vec<T>>
where
    T: FlowItemWidth,
    Create: FnOnce(f32) -> T,
{
    let items = items.into_iter().collect::<Vec<_>>();
    let trailing_starts_new_row = flow_trailing_item_starts_new_row(
        items.iter().map(|item| item.width),
        trailing.width,
        trailing.min_remaining_width,
        content_width,
        metrics,
    );
    let mut rows = pack_flow_rows(items, content_width, metrics);
    if trailing_starts_new_row || rows.is_empty() {
        rows.push(Vec::new());
    }

    let width = if rows.last().is_some_and(Vec::is_empty) {
        trailing.standalone_width
    } else {
        trailing.width
    };
    push_flow_row_item(
        &mut rows,
        (trailing.create)(width),
        width,
        content_width,
        metrics,
    );
    rows
}

/// Push one variable-width item onto existing flow rows.
pub fn push_flow_row_item<T>(
    rows: &mut Vec<Vec<T>>,
    item: T,
    width: f32,
    content_width: f32,
    metrics: FlowLayoutMetrics,
) where
    T: FlowItemWidth,
{
    if rows.is_empty() {
        rows.push(Vec::new());
    }

    let item_gap = metrics.item_gap.max(0.0);
    let current_width = rows
        .last()
        .map(|row| flow_row_width(row, item_gap))
        .unwrap_or(0.0);
    let proposed = proposed_row_width(current_width, width.max(0.0), item_gap);
    if proposed > content_width.max(0.0) && current_width > 0.0 {
        rows.push(Vec::new());
    }
    if let Some(row) = rows.last_mut() {
        row.push(item);
    }
}

/// Push a group of variable-width items that should stay on the same row.
///
/// This is useful for inline token editors where a prefix token and an editor,
/// or a label and action, should wrap as one unit instead of leaving the prefix
/// stranded at the end of the previous row. If the group is wider than the
/// content width it is still placed on one row so callers can apply their own
/// overflow policy at the view layer.
pub fn push_flow_row_group<T>(
    rows: &mut Vec<Vec<T>>,
    items: impl IntoIterator<Item = FlowItem<T>>,
    content_width: f32,
    metrics: FlowLayoutMetrics,
) where
    T: FlowItemWidth,
{
    let items = items.into_iter().collect::<Vec<_>>();
    if items.is_empty() {
        return;
    }
    if rows.is_empty() {
        rows.push(Vec::new());
    }

    let item_gap = metrics.item_gap.max(0.0);
    let current_width = rows
        .last()
        .map(|row| flow_row_width(row, item_gap))
        .unwrap_or(0.0);
    let group_width = flow_widths_total(items.iter().map(|item| item.width), item_gap);
    let proposed = proposed_row_width(current_width, group_width, item_gap);
    if proposed > content_width.max(0.0) && current_width > 0.0 {
        rows.push(Vec::new());
    }

    if let Some(row) = rows.last_mut() {
        row.extend(items.into_iter().map(|item| item.value));
    }
}

/// Return the packed width of a row of flow items.
pub fn flow_row_width<T>(row: &[T], item_gap: f32) -> f32
where
    T: FlowItemWidth,
{
    flow_widths_total(row.iter().map(FlowItemWidth::flow_width), item_gap)
}

/// Return the total height for a known number of flow rows.
pub fn flow_rows_height(row_count: usize, metrics: FlowLayoutMetrics) -> f32 {
    if row_count == 0 {
        return 0.0;
    }
    row_count as f32 * metrics.item_height.max(0.0)
        + row_count.saturating_sub(1) as f32 * metrics.line_gap.max(0.0)
}

/// Return the visible height for a capped wrapped flow plus surrounding chrome.
///
/// This is useful for chip, pill, tag, recipient, and token editors that grow
/// with wrapped rows until a maximum visible row count, then switch the content
/// area to scrolling. `min_visible_rows` keeps empty editors tall enough for
/// their trailing input or placeholder row.
pub fn capped_flow_rows_height(
    row_count: usize,
    min_visible_rows: usize,
    max_visible_rows: usize,
    chrome_height: f32,
    metrics: FlowLayoutMetrics,
) -> f32 {
    let min_visible_rows = min_visible_rows.min(max_visible_rows);
    let visible_rows = row_count.clamp(min_visible_rows, max_visible_rows);
    flow_rows_height(visible_rows, metrics) + chrome_height.max(0.0)
}

/// Return whether a trailing item should start on a new row.
///
/// This is useful for editable pill/tag fields where the text input should move
/// to its own line when the current row leaves too little editing room.
pub fn flow_trailing_item_starts_new_row(
    item_widths: impl IntoIterator<Item = f32>,
    trailing_width: f32,
    min_remaining_width: f32,
    content_width: f32,
    metrics: FlowLayoutMetrics,
) -> bool {
    let content_width = content_width.max(0.0);
    let item_gap = metrics.item_gap.max(0.0);
    let mut row_width = 0.0;

    for width in item_widths {
        let proposed = proposed_row_width(row_width, width.max(0.0), item_gap);
        if proposed > content_width && row_width > 0.0 {
            row_width = width.max(0.0);
        } else {
            row_width = proposed;
        }
    }

    row_width > 0.0
        && content_width - row_width - item_gap < trailing_width.max(min_remaining_width).max(0.0)
}

fn proposed_row_width(current_width: f32, item_width: f32, item_gap: f32) -> f32 {
    if current_width <= 0.0 {
        item_width
    } else {
        current_width + item_gap + item_width
    }
}

fn flow_widths_total(widths: impl IntoIterator<Item = f32>, item_gap: f32) -> f32 {
    widths
        .into_iter()
        .map(|width| width.max(0.0))
        .reduce(|total, width| total + item_gap.max(0.0) + width)
        .unwrap_or(0.0)
}

/// Trait for row payloads that expose a desired flow width.
pub trait FlowItemWidth {
    /// Desired width in logical pixels.
    fn flow_width(&self) -> f32;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn metrics() -> FlowLayoutMetrics {
        FlowLayoutMetrics::new(3.0, 5.0, 18.0)
    }

    #[test]
    fn pack_flow_rows_wraps_variable_width_items() {
        let rows = pack_flow_rows(
            [
                FlowItem::new("one", 40.0),
                FlowItem::new("two", 40.0),
                FlowItem::new("three", 40.0),
            ],
            90.0,
            metrics(),
        );

        assert_eq!(rows, vec![vec!["one", "two"], vec!["three"]]);
    }

    #[test]
    fn flow_rows_height_includes_line_gaps_between_rows() {
        assert_eq!(flow_rows_height(0, metrics()), 0.0);
        assert_eq!(flow_rows_height(1, metrics()), 18.0);
        assert_eq!(flow_rows_height(3, metrics()), 64.0);
    }

    #[test]
    fn capped_flow_rows_height_clamps_rows_and_adds_chrome() {
        assert_eq!(capped_flow_rows_height(0, 1, 6, 6.0, metrics()), 24.0);
        assert_eq!(capped_flow_rows_height(3, 1, 6, 6.0, metrics()), 70.0);
        assert_eq!(capped_flow_rows_height(9, 1, 6, 6.0, metrics()), 139.0);
        assert_eq!(capped_flow_rows_height(2, 4, 3, -12.0, metrics()), 64.0);
    }

    #[test]
    fn push_flow_row_item_appends_to_existing_rows() {
        #[derive(Clone)]
        struct SizedItem(&'static str, f32);

        impl FlowItemWidth for SizedItem {
            fn flow_width(&self) -> f32 {
                self.1
            }
        }

        let mut rows = vec![vec![SizedItem("one", 40.0)]];
        push_flow_row_item(&mut rows, SizedItem("two", 40.0), 40.0, 90.0, metrics());
        push_flow_row_item(&mut rows, SizedItem("three", 40.0), 40.0, 90.0, metrics());

        assert_eq!(rows.len(), 2);
        assert_eq!(
            rows[0].iter().map(|item| item.0).collect::<Vec<_>>(),
            ["one", "two"]
        );
        assert_eq!(
            rows[1].iter().map(|item| item.0).collect::<Vec<_>>(),
            ["three"]
        );
    }

    #[test]
    fn pack_flow_rows_with_trailing_item_keeps_editor_on_roomy_row() {
        #[derive(Clone, Debug, PartialEq)]
        struct SizedItem(&'static str, f32);

        impl FlowItemWidth for SizedItem {
            fn flow_width(&self) -> f32 {
                self.1
            }
        }

        let rows = pack_flow_rows_with_trailing_item(
            [FlowItem::new(SizedItem("pill", 38.0), 38.0)],
            FlowTrailingItemParts::new(|width| SizedItem("input", width), 61.0, 180.0, 100.0),
            180.0,
            metrics(),
        );

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0], [SizedItem("pill", 38.0), SizedItem("input", 61.0)]);
    }

    #[test]
    fn pack_flow_rows_with_trailing_item_uses_standalone_width_on_new_row() {
        #[derive(Clone, Debug, PartialEq)]
        struct SizedItem(&'static str, f32);

        impl FlowItemWidth for SizedItem {
            fn flow_width(&self) -> f32 {
                self.1
            }
        }

        let rows = pack_flow_rows_with_trailing_item(
            [
                FlowItem::new(SizedItem("one", 38.0), 38.0),
                FlowItem::new(SizedItem("two", 42.0), 42.0),
            ],
            FlowTrailingItemParts::new(|width| SizedItem("input", width), 61.0, 180.0, 100.0),
            180.0,
            metrics(),
        );

        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0], [SizedItem("one", 38.0), SizedItem("two", 42.0)]);
        assert_eq!(rows[1], [SizedItem("input", 180.0)]);
    }

    #[test]
    fn pack_flow_rows_with_trailing_item_handles_empty_items() {
        #[derive(Clone, Debug, PartialEq)]
        struct SizedItem(&'static str, f32);

        impl FlowItemWidth for SizedItem {
            fn flow_width(&self) -> f32 {
                self.1
            }
        }

        let rows = pack_flow_rows_with_trailing_item(
            [],
            FlowTrailingItemParts::new(|width| SizedItem("input", width), 61.0, 180.0, 100.0),
            180.0,
            metrics(),
        );

        assert_eq!(rows, vec![vec![SizedItem("input", 180.0)]]);
    }

    #[test]
    fn push_flow_row_group_keeps_items_together_when_wrapping() {
        #[derive(Clone, Debug, PartialEq)]
        struct SizedItem(&'static str, f32);

        impl FlowItemWidth for SizedItem {
            fn flow_width(&self) -> f32 {
                self.1
            }
        }

        let mut rows = pack_flow_rows(
            [FlowItem::new(SizedItem("pill", 86.0), 86.0)],
            150.0,
            metrics(),
        );
        push_flow_row_group(
            &mut rows,
            [
                FlowItem::new(SizedItem("prefix", 50.0), 50.0),
                FlowItem::new(SizedItem("input", 70.0), 70.0),
            ],
            150.0,
            metrics(),
        );

        assert_eq!(
            rows,
            vec![
                vec![SizedItem("pill", 86.0)],
                vec![SizedItem("prefix", 50.0), SizedItem("input", 70.0)]
            ]
        );
    }

    #[test]
    fn trailing_item_moves_when_remaining_width_is_too_small() {
        assert!(flow_trailing_item_starts_new_row(
            [38.0, 42.0],
            61.0,
            100.0,
            180.0,
            metrics()
        ));
        assert!(!flow_trailing_item_starts_new_row(
            [38.0],
            61.0,
            100.0,
            180.0,
            metrics()
        ));
    }
}

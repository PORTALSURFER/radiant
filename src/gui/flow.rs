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

/// Geometry policy for a bounded wrapped inline field.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FlowFieldMetricsParts {
    /// Row-packing metrics for the wrapped inline items.
    pub flow: FlowLayoutMetrics,
    /// Width reserved by field chrome outside the packed content area.
    pub horizontal_chrome: f32,
    /// Height reserved by field chrome outside the packed content area.
    pub vertical_chrome: f32,
    /// Minimum usable content width after chrome is removed.
    pub min_content_width: f32,
    /// Maximum number of rows shown before the host should use scrolling.
    pub max_visible_rows: usize,
}

/// Geometry policy for a bounded wrapped inline field.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FlowFieldMetrics {
    /// Row-packing metrics for the wrapped inline items.
    pub flow: FlowLayoutMetrics,
    /// Width reserved by field chrome outside the packed content area.
    pub horizontal_chrome: f32,
    /// Height reserved by field chrome outside the packed content area.
    pub vertical_chrome: f32,
    /// Minimum usable content width after chrome is removed.
    pub min_content_width: f32,
    /// Maximum number of rows shown before the host should use scrolling.
    pub max_visible_rows: usize,
}

/// Resolved geometry summary for a bounded wrapped inline field.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FlowFieldLayout {
    /// Usable row-packing width inside the field chrome.
    pub content_width: f32,
    /// Number of rows produced by caller-owned flow packing.
    pub row_count: usize,
    /// Number of rows visible before scrolling is needed.
    pub visible_row_count: usize,
    /// Visible content height before adding field chrome.
    pub content_height: f32,
    /// Visible field height including field chrome.
    pub field_height: f32,
    /// Whether the packed rows exceed the visible row limit.
    pub requires_scroll: bool,
}

impl FlowFieldMetrics {
    /// Construct field metrics from named already-resolved logical-pixel values.
    pub const fn from_parts(parts: FlowFieldMetricsParts) -> Self {
        Self {
            flow: parts.flow,
            horizontal_chrome: parts.horizontal_chrome,
            vertical_chrome: parts.vertical_chrome,
            min_content_width: parts.min_content_width,
            max_visible_rows: parts.max_visible_rows,
        }
    }

    /// Construct field metrics from resolved logical-pixel values.
    pub const fn new(
        flow: FlowLayoutMetrics,
        horizontal_chrome: f32,
        vertical_chrome: f32,
        min_content_width: f32,
        max_visible_rows: usize,
    ) -> Self {
        Self::from_parts(FlowFieldMetricsParts {
            flow,
            horizontal_chrome,
            vertical_chrome,
            min_content_width,
            max_visible_rows,
        })
    }

    /// Return the available row-packing width inside a containing field.
    pub fn content_width(self, container_width: f32) -> f32 {
        (container_width - self.horizontal_chrome.max(0.0)).max(self.min_content_width.max(0.0))
    }

    /// Return the resolved field layout for a containing width and row count.
    pub fn layout(self, container_width: f32, row_count: usize) -> FlowFieldLayout {
        self.layout_for_content_width(self.content_width(container_width), row_count)
    }

    /// Return the resolved field layout for an already-computed content width.
    pub fn layout_for_content_width(self, content_width: f32, row_count: usize) -> FlowFieldLayout {
        let visible_row_count = self.visible_row_count(row_count);
        let content_height = flow_rows_height(visible_row_count, self.flow);
        FlowFieldLayout {
            content_width: content_width.max(self.min_content_width.max(0.0)),
            row_count,
            visible_row_count,
            content_height,
            field_height: content_height + self.vertical_chrome.max(0.0),
            requires_scroll: self.requires_scroll(row_count),
        }
    }

    /// Return the row count visible before the field should scroll.
    pub fn visible_row_count(self, row_count: usize) -> usize {
        row_count.clamp(1, self.max_visible_rows.max(1))
    }

    /// Return the visible content height for the supplied packed row count.
    pub fn visible_rows_height(self, row_count: usize) -> f32 {
        flow_rows_height(self.visible_row_count(row_count), self.flow)
    }

    /// Return the full field height for the supplied packed row count.
    pub fn visible_field_height(self, row_count: usize) -> f32 {
        self.visible_rows_height(row_count) + self.vertical_chrome.max(0.0)
    }

    /// Return whether the supplied packed row count exceeds the visible limit.
    pub fn requires_scroll(self, row_count: usize) -> bool {
        row_count > self.max_visible_rows.max(1)
    }
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

/// Stateful row packer for incrementally building wrapped inline flows.
///
/// This keeps the current row width as items are appended, avoiding repeated
/// scans of the last row when a caller builds a token, chip, pill, or recipient
/// editor one item at a time. Use [`pack_flow_rows`] for one-shot packing.
#[derive(Clone, Debug, PartialEq)]
pub struct FlowRowPacker<T> {
    rows: Vec<Vec<T>>,
    content_width: f32,
    item_gap: f32,
    current_width: f32,
}

impl<T> FlowRowPacker<T> {
    /// Create an empty incremental flow-row packer.
    pub fn new(content_width: f32, metrics: FlowLayoutMetrics) -> Self {
        Self {
            rows: Vec::new(),
            content_width: content_width.max(0.0),
            item_gap: metrics.item_gap.max(0.0),
            current_width: 0.0,
        }
    }

    /// Borrow the packed rows.
    pub fn rows(&self) -> &[Vec<T>] {
        &self.rows
    }

    /// Return the width of the current trailing row.
    pub fn current_row_width(&self) -> f32 {
        self.current_width
    }

    /// Consume the packer and return its packed rows.
    pub fn into_rows(self) -> Vec<Vec<T>> {
        self.rows
    }

    /// Push one variable-width item onto the packed rows.
    pub fn push_item(&mut self, item: T, width: f32) {
        let width = width.max(0.0);
        let proposed = proposed_row_width(self.current_width, width, self.item_gap);
        if proposed > self.content_width && self.current_width > 0.0 {
            self.rows.push(Vec::new());
            self.current_width = 0.0;
        }
        self.current_width = proposed_row_width(self.current_width, width, self.item_gap);
        self.push_value(item);
    }

    /// Push a group of items that should stay on the same row.
    pub fn push_group(&mut self, items: impl IntoIterator<Item = FlowItem<T>>) {
        let (values, group_width) = collect_flow_group(items, self.item_gap);
        if values.is_empty() {
            return;
        }
        let proposed = proposed_row_width(self.current_width, group_width, self.item_gap);
        if proposed > self.content_width && self.current_width > 0.0 {
            self.rows.push(Vec::new());
            self.current_width = 0.0;
        }
        self.current_width = proposed_row_width(self.current_width, group_width, self.item_gap);
        self.ensure_current_row();
        if let Some(row) = self.rows.last_mut() {
            row.extend(values);
        }
    }

    fn push_value(&mut self, item: T) {
        self.ensure_current_row();
        if let Some(row) = self.rows.last_mut() {
            row.push(item);
        }
    }

    fn ensure_current_row(&mut self) {
        if self.rows.is_empty() {
            self.rows.push(Vec::new());
        }
    }
}

impl<T> FlowRowPacker<T>
where
    T: FlowItemWidth,
{
    /// Create a packer from already-built rows, computing the current row width once.
    pub fn from_rows(rows: Vec<Vec<T>>, content_width: f32, metrics: FlowLayoutMetrics) -> Self {
        let item_gap = metrics.item_gap.max(0.0);
        let current_width = rows
            .last()
            .map(|row| flow_row_width(row, item_gap))
            .unwrap_or(0.0);
        Self {
            rows,
            content_width: content_width.max(0.0),
            item_gap,
            current_width,
        }
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
    let mut packer = FlowRowPacker::new(content_width, metrics);
    for item in items {
        packer.push_item(item.value, item.width);
    }
    packer.into_rows()
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
    let mut packer = FlowRowPacker::new(content_width, metrics);
    for item in items {
        packer.push_item(item.value, item.width);
    }
    let trailing_starts_new_row = trailing_item_starts_new_row_after_width(
        packer.current_row_width(),
        trailing.width,
        trailing.min_remaining_width,
        content_width,
        metrics.item_gap,
    );
    let mut rows = packer.into_rows();
    if trailing_starts_new_row || rows.is_empty() {
        rows.push(Vec::new());
    }

    let width = if rows.last().is_some_and(Vec::is_empty) {
        trailing.standalone_width
    } else {
        trailing.width
    };
    let mut packer = FlowRowPacker::from_rows(rows, content_width, metrics);
    packer.push_item((trailing.create)(width), width);
    packer.into_rows()
}

/// Pack variable-width items and append an atomically-wrapped trailing group.
///
/// This is useful for chip, pill, tag, recipient, and token editors where a
/// trailing prefix plus editor/control must stay together instead of leaving
/// the prefix stranded at the end of the previous row.
pub fn pack_flow_rows_with_trailing_group<T>(
    items: impl IntoIterator<Item = FlowItem<T>>,
    trailing: impl IntoIterator<Item = FlowItem<T>>,
    content_width: f32,
    metrics: FlowLayoutMetrics,
) -> Vec<Vec<T>>
where
    T: FlowItemWidth,
{
    let mut rows = pack_flow_rows(items, content_width, metrics);
    push_flow_row_group(&mut rows, trailing, content_width, metrics);
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
    let mut packer = FlowRowPacker::from_rows(std::mem::take(rows), content_width, metrics);
    packer.push_item(item, width);
    *rows = packer.into_rows();
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
    let mut packer = FlowRowPacker::from_rows(std::mem::take(rows), content_width, metrics);
    packer.push_group(items);
    *rows = packer.into_rows();
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

    trailing_item_starts_new_row_after_width(
        row_width,
        trailing_width,
        min_remaining_width,
        content_width,
        item_gap,
    )
}

fn proposed_row_width(current_width: f32, item_width: f32, item_gap: f32) -> f32 {
    if current_width <= 0.0 {
        item_width
    } else {
        current_width + item_gap + item_width
    }
}

fn trailing_item_starts_new_row_after_width(
    row_width: f32,
    trailing_width: f32,
    min_remaining_width: f32,
    content_width: f32,
    item_gap: f32,
) -> bool {
    row_width > 0.0
        && content_width.max(0.0) - row_width - item_gap.max(0.0)
            < trailing_width.max(min_remaining_width).max(0.0)
}

fn flow_widths_total(widths: impl IntoIterator<Item = f32>, item_gap: f32) -> f32 {
    widths
        .into_iter()
        .map(|width| width.max(0.0))
        .reduce(|total, width| total + item_gap.max(0.0) + width)
        .unwrap_or(0.0)
}

fn collect_flow_group<T>(
    items: impl IntoIterator<Item = FlowItem<T>>,
    item_gap: f32,
) -> (Vec<T>, f32) {
    let items = items.into_iter();
    let (lower, upper) = items.size_hint();
    let mut values = Vec::with_capacity(upper.unwrap_or(lower));
    let mut total_width = 0.0;
    let item_gap = item_gap.max(0.0);

    for item in items {
        let width = item.width.max(0.0);
        total_width = proposed_row_width(total_width, width, item_gap);
        values.push(item.value);
    }

    (values, total_width)
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
    fn flow_field_metrics_clamps_content_width_and_visible_rows() {
        let field = FlowFieldMetrics::new(metrics(), 26.0, 6.0, 120.0, 3);

        assert_eq!(field.content_width(400.0), 374.0);
        assert_eq!(field.content_width(80.0), 120.0);
        assert_eq!(field.visible_row_count(0), 1);
        assert_eq!(field.visible_row_count(2), 2);
        assert_eq!(field.visible_row_count(8), 3);
        assert_eq!(field.visible_rows_height(8), 64.0);
        assert_eq!(field.visible_field_height(8), 70.0);
        assert!(field.requires_scroll(4));
        assert!(!field.requires_scroll(3));
    }

    #[test]
    fn flow_field_layout_resolves_content_height_and_scroll_policy() {
        let field = FlowFieldMetrics::new(metrics(), 26.0, 6.0, 120.0, 3);

        assert_eq!(
            field.layout(400.0, 8),
            FlowFieldLayout {
                content_width: 374.0,
                row_count: 8,
                visible_row_count: 3,
                content_height: 64.0,
                field_height: 70.0,
                requires_scroll: true,
            }
        );
        assert_eq!(field.layout(80.0, 0).content_width, 120.0);
        assert_eq!(field.layout_for_content_width(80.0, 0).content_width, 120.0);
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
    fn flow_row_packer_tracks_width_without_rescanning_rows() {
        #[derive(Clone, Debug, PartialEq)]
        struct SizedItem(&'static str, f32);

        let mut packer = FlowRowPacker::new(90.0, metrics());
        packer.push_item(SizedItem("one", 40.0), 40.0);
        assert_eq!(packer.current_row_width(), 40.0);
        packer.push_item(SizedItem("two", 40.0), 40.0);
        assert_eq!(packer.current_row_width(), 83.0);
        packer.push_item(SizedItem("three", 40.0), 40.0);

        assert_eq!(packer.current_row_width(), 40.0);
        assert_eq!(
            packer.into_rows(),
            vec![
                vec![SizedItem("one", 40.0), SizedItem("two", 40.0)],
                vec![SizedItem("three", 40.0)]
            ]
        );
    }

    #[test]
    fn flow_row_packer_keeps_group_items_atomic() {
        #[derive(Clone, Debug, PartialEq)]
        struct SizedItem(&'static str, f32);

        let mut packer = FlowRowPacker::new(150.0, metrics());
        packer.push_item(SizedItem("pill", 86.0), 86.0);
        packer.push_group([
            FlowItem::new(SizedItem("prefix", 50.0), 50.0),
            FlowItem::new(SizedItem("input", 70.0), 70.0),
        ]);

        assert_eq!(packer.current_row_width(), 123.0);
        assert_eq!(
            packer.rows(),
            &[
                vec![SizedItem("pill", 86.0)],
                vec![SizedItem("prefix", 50.0), SizedItem("input", 70.0)]
            ]
        );
    }

    #[test]
    fn flow_group_collection_computes_width_while_extracting_payloads() {
        let (values, width) = collect_flow_group(
            [
                FlowItem::new("prefix", 50.0),
                FlowItem::new("input", 70.0),
                FlowItem::new("negative", -10.0),
            ],
            metrics().item_gap,
        );

        assert_eq!(values, ["prefix", "input", "negative"]);
        assert_eq!(width, 126.0);
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
    fn pack_flow_rows_with_trailing_group_wraps_group_atomically() {
        #[derive(Clone, Debug, PartialEq)]
        struct SizedItem(&'static str, f32);

        impl FlowItemWidth for SizedItem {
            fn flow_width(&self) -> f32 {
                self.1
            }
        }

        let rows = pack_flow_rows_with_trailing_group(
            [FlowItem::new(SizedItem("pill", 86.0), 86.0)],
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

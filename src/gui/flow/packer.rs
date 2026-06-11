use super::{
    item::{FlowItem, FlowItemWidth},
    metrics::FlowLayoutMetrics,
};
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

pub(super) fn proposed_row_width(current_width: f32, item_width: f32, item_gap: f32) -> f32 {
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

pub(super) fn collect_flow_group<T>(
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

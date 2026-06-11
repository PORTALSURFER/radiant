use super::{
    item::{FlowItem, FlowItemWidth},
    metrics::FlowLayoutMetrics,
    packer::{FlowRowPacker, proposed_row_width},
};
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

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

    /// Reserve horizontal room after the trailing item for a following action.
    ///
    /// Use this when a chip, token, recipient, or pill editor has a text input
    /// followed by a compact control such as a picker, library toggle, or
    /// add-menu button. The compact and standalone widths both reserve the
    /// follow-up control while keeping at least `min_width`, capped by the
    /// original requested width.
    pub fn reserve_following_item(
        mut self,
        following_width: f32,
        following_gap: f32,
        min_width: f32,
    ) -> Self {
        self.width = flow_width_with_following_item_reserved(
            self.width,
            following_width,
            following_gap,
            min_width,
        );
        self.standalone_width = flow_width_with_following_item_reserved(
            self.standalone_width,
            following_width,
            following_gap,
            min_width,
        );
        self
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

/// Pack variable-width items, append one trailing editor/control item, then an
/// optional following control.
///
/// This is the common token-editor shape where a final text editor should
/// reserve room for a compact action button on the same row. If the following
/// item is present, the trailing editor's compact and standalone widths are
/// reduced by `metrics.item_gap + following.width` before packing, but never
/// below `trailing_min_width.min(original_width)`. The following item is then
/// pushed through the normal row packer so it wraps consistently with other
/// flow items.
pub fn pack_flow_rows_with_trailing_item_and_following_item<T, Create>(
    items: impl IntoIterator<Item = FlowItem<T>>,
    trailing: FlowTrailingItemParts<Create>,
    following: Option<FlowItem<T>>,
    trailing_min_width: f32,
    content_width: f32,
    metrics: FlowLayoutMetrics,
) -> Vec<Vec<T>>
where
    T: FlowItemWidth,
    Create: FnOnce(f32) -> T,
{
    let trailing = if let Some(following) = &following {
        trailing.reserve_following_item(following.width, metrics.item_gap, trailing_min_width)
    } else {
        trailing
    };
    let mut rows = pack_flow_rows_with_trailing_item(items, trailing, content_width, metrics);
    if let Some(following) = following {
        let mut packer = FlowRowPacker::from_rows(rows, content_width, metrics);
        packer.push_item(following.value, following.width);
        rows = packer.into_rows();
    }
    rows
}

/// Pack variable-width items and append an atomic trailing group containing a
/// flexible editor/control and an optional following control.
///
/// This is useful for token, recipient, chip, and pill editors where a prefix
/// token plus final editor must wrap together, while the final editor should
/// reserve space for a compact following action such as a picker, add button,
/// or library toggle.
pub fn pack_flow_rows_with_flexible_trailing_group<T, Create>(
    items: impl IntoIterator<Item = FlowItem<T>>,
    group_leading: impl IntoIterator<Item = FlowItem<T>>,
    trailing: FlowTrailingItemParts<Create>,
    following: Option<FlowItem<T>>,
    trailing_min_width: f32,
    content_width: f32,
    metrics: FlowLayoutMetrics,
) -> Vec<Vec<T>>
where
    T: FlowItemWidth,
    Create: FnOnce(f32) -> T,
{
    let trailing = if let Some(following) = &following {
        trailing.reserve_following_item(following.width, metrics.item_gap, trailing_min_width)
    } else {
        trailing
    };
    let group_leading = group_leading.into_iter().collect::<Vec<_>>();
    let required_width = flexible_group_required_width(
        &group_leading,
        trailing.width.max(trailing.min_remaining_width),
        following.as_ref(),
        metrics.item_gap,
    );

    let mut packer = FlowRowPacker::new(content_width, metrics);
    for item in items {
        packer.push_item(item.value, item.width);
    }
    let group_starts_new_row = trailing_item_starts_new_row_after_width(
        packer.current_row_width(),
        required_width,
        0.0,
        content_width,
        metrics.item_gap,
    );
    let mut rows = packer.into_rows();
    if group_starts_new_row || rows.is_empty() {
        rows.push(Vec::new());
    }

    let trailing_width = if rows.last().is_some_and(Vec::is_empty) {
        trailing.standalone_width
    } else {
        trailing.width
    };
    let mut group = group_leading;
    group.push(FlowItem::new(
        (trailing.create)(trailing_width),
        trailing_width,
    ));
    if let Some(following) = following {
        group.push(following);
    }
    let mut packer = FlowRowPacker::from_rows(rows, content_width, metrics);
    packer.push_group(group);
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

/// Return an item width after reserving room for a following item.
///
/// This keeps inline editor width-reservation policy in Radiant instead of
/// forcing host apps to repeat `available - gap - button` math. Non-finite and
/// negative widths are normalized to zero, and the result keeps at least
/// `min_width.min(width)` so very small controls do not grow unexpectedly.
pub fn flow_width_with_following_item_reserved(
    width: f32,
    following_width: f32,
    following_gap: f32,
    min_width: f32,
) -> f32 {
    let width = finite_nonnegative_width(width);
    let reserved =
        finite_nonnegative_width(following_width) + finite_nonnegative_width(following_gap);
    (width - reserved).max(finite_nonnegative_width(min_width).min(width))
}

fn finite_nonnegative_width(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

fn flexible_group_required_width<T>(
    leading: &[FlowItem<T>],
    trailing_width: f32,
    following: Option<&FlowItem<T>>,
    item_gap: f32,
) -> f32 {
    let item_gap = item_gap.max(0.0);
    let mut width = 0.0;
    for item in leading {
        width = proposed_row_width(width, item.width.max(0.0), item_gap);
    }
    width = proposed_row_width(width, trailing_width.max(0.0), item_gap);
    if let Some(following) = following {
        width = proposed_row_width(width, following.width.max(0.0), item_gap);
    }
    width
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

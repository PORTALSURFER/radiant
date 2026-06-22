//! Generic data-side helpers for wrapping variable-width inline items.

mod field;
mod item;
mod metrics;
mod packer;
#[cfg(test)]
mod tests;
mod trailing;

pub use field::{
    FlowFieldLayout, FlowFieldMetrics, FlowFieldMetricsParts, capped_flow_rows_height,
};
pub use item::{FlowItem, FlowItemWidth};
pub use metrics::{FlowLayoutMetrics, flow_rows_height};
pub use packer::{
    FlowRowPacker, flow_row_width, pack_flow_rows, pack_flow_rows_with_trailing_group,
    push_flow_row_group, push_flow_row_item,
};
pub use trailing::{
    FlowTrailingItemParts, flow_trailing_item_starts_new_row,
    flow_width_with_following_item_reserved, pack_flow_rows_with_trailing_item,
    pack_flow_rows_with_trailing_item_and_following_item,
};

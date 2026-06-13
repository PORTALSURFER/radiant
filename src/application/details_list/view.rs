mod compact;
mod header;
mod list;

pub use compact::{
    CompactDetailsAnchoredCellParts, compact_details_anchored_cell_from_parts,
    compact_details_cell, compact_details_row,
};
pub use header::{
    CompactDetailsHeaderCellIds, compact_details_header_row, compact_resizable_details_header_cell,
    compact_resizable_details_header_cell_with_ids,
};
pub use list::{message_selectable_sortable_details_list, message_sortable_details_list};

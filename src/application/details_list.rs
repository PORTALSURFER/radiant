mod model;
mod view;

pub use model::{
    DetailsColumn, DetailsColumnParts, DetailsRow, DetailsRowParts, DetailsSort, SortDirection,
};
pub use view::{selectable_sortable_details_list, sortable_details_list};

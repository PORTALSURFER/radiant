//! Browser toolbar layout and hover helpers.

mod buttons;
mod colors;
mod layout;

#[allow(unused_imports)]
pub(in crate::gui::native_shell::state) use buttons::browser_action_buttons;
#[allow(unused_imports)]
pub(in crate::gui::native_shell::state) use colors::{
    marked_filter_chip_border, marked_filter_chip_contains_point,
    marked_filter_chip_fill, marked_filter_chip_hover_border,
    marked_filter_chip_hover_fill, recency_filter_chip_border,
    recency_filter_chip_fill, recency_filter_chip_hover_border,
    recency_filter_chip_hover_fill, rating_filter_chip_border,
    rating_filter_chip_fill, rating_filter_chip_hover_border,
    rating_filter_chip_hover_fill, search_field_hover_border,
    search_field_hover_fill, render_recency_filter_chip_hover_overlay,
    render_rating_filter_chip_hover_overlay, render_search_field_hover_overlay,
};
#[allow(unused_imports)]
pub(in crate::gui::native_shell::state) use layout::{
    column_chips, recency_filter_chip_at_point,
    recency_filter_chip_index, rating_filter_chip_index,
    rating_filter_level_at_point, browser_toolbar_layout,
};

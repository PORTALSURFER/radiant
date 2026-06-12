use super::super::containers::{row, row_key};
use crate::application::ViewNode;
use crate::widgets::WidgetStyle;

/// Build a keyed list row with full-width, fixed-height defaults.
pub fn list_row<Message>(
    key: impl ToString,
    children: impl IntoIterator<Item = ViewNode<Message>>,
) -> ViewNode<Message> {
    apply_list_row_chrome(row_key(key, children))
}

/// Build a list row with a direct numeric id instead of a string key.
///
/// Prefer this for large numeric collections when the caller already owns
/// stable item ids; it avoids per-row key string allocation during projection.
pub fn list_row_id<Message>(
    id: u64,
    children: impl IntoIterator<Item = ViewNode<Message>>,
) -> ViewNode<Message> {
    apply_list_row_chrome(row(children).id(id))
}

fn apply_list_row_chrome<Message>(row: ViewNode<Message>) -> ViewNode<Message> {
    row.style(WidgetStyle::default())
        .hoverable()
        .fill_width()
        .height(44.0)
        .padding_x(12.0)
        .padding_y(7.0)
        .spacing(10.0)
}

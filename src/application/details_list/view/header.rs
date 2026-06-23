use crate::{
    application::{View, button, drag_handle, input_underlay, row, text},
    widgets::{
        DragHandleMessage, WidgetId, WidgetProminence, WidgetStyle, WidgetTone, stable_widget_id,
    },
};

/// Stable widget ids for a compact resizable details header cell.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct CompactDetailsHeaderCellIds {
    /// Widget id for the sort/reorder input surface.
    pub sort_drag: Option<WidgetId>,
    /// Widget id for the trailing resize handle.
    pub resize: Option<WidgetId>,
}

impl CompactDetailsHeaderCellIds {
    /// Build explicit ids for the sort/reorder and resize surfaces.
    pub const fn new(sort_drag: Option<WidgetId>, resize: Option<WidgetId>) -> Self {
        Self { sort_drag, resize }
    }

    /// Derive sort/reorder and resize ids from caller-owned scopes and a stable column key.
    ///
    /// Use this for dynamic details-list headers where each column needs
    /// retained interaction identity, but the app should not repeat the
    /// sort/resize id derivation at every header call site.
    pub fn from_stable_key(sort_drag_scope: u64, resize_scope: u64, key: impl AsRef<str>) -> Self {
        let key = key.as_ref();
        Self::new(
            Some(stable_widget_id(sort_drag_scope, key)),
            Some(stable_widget_id(resize_scope, key)),
        )
    }
}

/// Build a compact details-list header row.
///
/// This matches Radiant's dense details-list header chrome: fixed 24px height,
/// accent subtle background, small vertical padding, left/right chrome, and
/// compact cell spacing. Host apps can use it when they need custom sortable,
/// resizable, or reorderable header cells but should not repeat details-list
/// header styling.
pub fn compact_details_header_row<Message>(
    children: impl IntoIterator<Item = View<Message>>,
) -> View<Message> {
    row(children)
        .style(WidgetStyle::new(
            WidgetTone::Accent,
            WidgetProminence::Subtle,
        ))
        .fill_width()
        .height(24.0)
        .padding_x(8.0)
        .padding_y(2.0)
        .spacing(10.0)
}

/// Build a compact sortable, reorderable, and resizable details-list header cell.
///
/// This provides the standard composition used by dense details-list headers:
/// visible truncated label text, a transparent click-or-drag input layer for
/// sort and column reorder gestures, and a trailing resize handle.
pub fn compact_resizable_details_header_cell<Message>(
    key: impl Into<String>,
    label: impl Into<String>,
    width: f32,
    sort_message: Message,
    drag_message: impl Fn(DragHandleMessage) -> Message + Send + Sync + 'static,
    resize_message: impl Fn(DragHandleMessage) -> Message + Send + Sync + 'static,
) -> View<Message>
where
    Message: Clone + Send + Sync + 'static,
{
    compact_resizable_details_header_cell_with_ids(
        key,
        label,
        width,
        CompactDetailsHeaderCellIds::default(),
        sort_message,
        drag_message,
        resize_message,
    )
}

/// Build a compact sortable, reorderable, and resizable details-list header
/// cell with explicit stable widget ids for the interactive surfaces.
#[allow(
    clippy::too_many_arguments,
    reason = "compatibility API with two optional stable widget ids"
)]
pub fn compact_resizable_details_header_cell_with_ids<Message>(
    key: impl Into<String>,
    label: impl Into<String>,
    width: f32,
    ids: CompactDetailsHeaderCellIds,
    sort_message: Message,
    drag_message: impl Fn(DragHandleMessage) -> Message + Send + Sync + 'static,
    resize_message: impl Fn(DragHandleMessage) -> Message + Send + Sync + 'static,
) -> View<Message>
where
    Message: Clone + Send + Sync + 'static,
{
    let key = key.into();
    let label = label.into();
    let mut sort_drag = button("")
        .hover_chrome_only()
        .click_or_drag(sort_message, drag_message)
        .key(format!("{key}-sort-drag"))
        .fill_width()
        .height(20.0);
    if let Some(id) = ids.sort_drag {
        sort_drag = sort_drag.id(id);
    }
    let mut resize = drag_handle()
        .mapped(resize_message)
        .key(format!("{key}-resize"))
        .size(4.0, 20.0);
    if let Some(id) = ids.resize {
        resize = resize.id(id);
    }
    row([
        input_underlay(
            text(label.clone())
                .key(format!("{key}-label"))
                .align_text(crate::widgets::TextAlign::Left)
                .fill_width()
                .height(20.0)
                .truncate(),
            sort_drag,
        )
        .fill_width()
        .height(20.0),
        resize,
    ])
    .key(key)
    .width(width)
    .height(20.0)
    .spacing(1.0)
}

#[cfg(test)]
mod tests {
    use super::CompactDetailsHeaderCellIds;
    use crate::widgets::stable_widget_id;

    #[test]
    fn compact_details_header_cell_ids_can_derive_stable_column_ids() {
        let ids = CompactDetailsHeaderCellIds::from_stable_key(11, 22, "name");

        assert_eq!(ids.sort_drag, Some(stable_widget_id(11, "name")));
        assert_eq!(ids.resize, Some(stable_widget_id(22, "name")));
        assert_ne!(ids.sort_drag, ids.resize);
    }
}

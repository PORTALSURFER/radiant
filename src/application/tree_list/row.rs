use crate::{
    application::{View, button, disclosure_button, drag_handle, row, spacer, text},
    widgets::{ButtonMessage, WidgetProminence, WidgetStyle, WidgetTone},
};
use std::sync::Arc;

use super::TreeListItem;

pub(super) fn message_tree_list_row<Message>(
    item: TreeListItem,
    select_message: Arc<dyn Fn(String) -> Message + Send + Sync>,
    toggle_message: Arc<dyn Fn(String) -> Message + Send + Sync>,
    context_message: Option<Arc<dyn Fn(String) -> Message + Send + Sync>>,
    drag_message: Option<
        Arc<dyn Fn(String, crate::widgets::DragHandleMessage) -> Message + Send + Sync>,
    >,
) -> View<Message>
where
    Message: Clone + Send + Sync + 'static,
{
    let select_id = item.id.clone();
    let context_id = item.id.clone();
    let toggle_id = item.id.clone();
    let drag_id = item.id.clone();
    let key = item.id.clone();

    let mut label = if let Some(context_message) = context_message {
        button(item.label)
            .secondary_clicks()
            .filter_mapped(move |message| match message {
                ButtonMessage::Activate => Some(select_message(select_id.clone())),
                ButtonMessage::SecondaryActivate { .. } => {
                    Some(context_message(context_id.clone()))
                }
                ButtonMessage::Drag(_) => None,
            })
    } else {
        button(item.label).message(select_message(select_id))
    }
    .key(format!("tree-list-label-{key}"))
    .fill_width()
    .height(22.0);
    if item.selected || item.drop_target {
        label = label.primary();
    } else {
        label = label.subtle();
    }

    row([
        text("").size((item.depth as f32) * 12.0, 22.0),
        if item.draggable {
            if let Some(drag_message) = drag_message {
                drag_handle()
                    .mapped(move |message| drag_message(drag_id.clone(), message))
                    .key(format!("tree-list-drag-{}", item.id))
                    .size(22.0, 22.0)
            } else {
                text("")
                    .key(format!("tree-list-drag-spacer-{}", item.id))
                    .size(22.0, 22.0)
            }
        } else {
            text("")
                .key(format!("tree-list-drag-spacer-{}", item.id))
                .size(22.0, 22.0)
        },
        if item.has_children {
            disclosure_button(item.expanded)
                .message(toggle_message(toggle_id))
                .key(format!("tree-list-toggle-{}", item.id))
                .size(26.0, 22.0)
                .subtle()
        } else {
            spacer()
                .key(format!("tree-list-spacer-{}", item.id))
                .size(26.0, 22.0)
        },
        label,
    ])
    .key(format!("tree-list-row-{}", item.id))
    .style(if item.drop_target {
        WidgetStyle::new(WidgetTone::Accent, WidgetProminence::Subtle)
    } else {
        WidgetStyle::default()
    })
    .fill_width()
    .height(23.0)
    .spacing(1.0)
    .hoverable()
}

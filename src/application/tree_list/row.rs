use crate::{
    application::{
        StateDragCallback, StateStringCallback, StateView, button, disclosure_button, drag_handle,
        row, spacer, text,
    },
    widgets::{WidgetProminence, WidgetStyle, WidgetTone},
};

use super::TreeListItem;

pub(super) fn tree_list_row<State: 'static>(
    item: TreeListItem,
    on_select: StateStringCallback<State>,
    on_toggle: StateStringCallback<State>,
    on_context: Option<StateStringCallback<State>>,
    on_drag: Option<StateDragCallback<State>>,
) -> StateView<State> {
    let select_id = item.id.clone();
    let context_id = item.id.clone();
    let toggle_id = item.id.clone();
    let drag_id = item.id.clone();
    let key = item.id.clone();

    let mut label = if let Some(on_context) = on_context {
        button(item.label).on_click_or_secondary(
            move |state: &mut State| on_select(state, select_id.clone()),
            move |state: &mut State| on_context(state, context_id.clone()),
        )
    } else {
        button(item.label).on_click(move |state: &mut State| on_select(state, select_id.clone()))
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
            if let Some(on_drag) = on_drag {
                drag_handle()
                    .on_drag(move |state: &mut State, message| {
                        on_drag(state, drag_id.clone(), message);
                    })
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
                .on_click(move |state: &mut State| on_toggle(state, toggle_id.clone()))
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

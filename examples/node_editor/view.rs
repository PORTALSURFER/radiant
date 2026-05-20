use radiant::prelude::*;

use crate::model::{
    NodeEditorState, begin_connection_from, connect_pending_output_to, connection_summary,
    connection_text, move_node_from_drag, node_base_id, node_body, node_label,
};

pub(super) fn project_surface(state: &mut NodeEditorState) -> StateView<NodeEditorState> {
    column([
        row([
            text("Node Editor").height(30.0).fill_width(),
            badge(format!("rev {}", state.revision))
                .primary()
                .on_click(|state: &mut NodeEditorState| {
                    state.revision += 1;
                    state.status = "canvas refreshed".to_string();
                })
                .size(88.0, 28.0),
        ])
        .fill_width()
        .spacing(10.0),
        stack([
            retained_canvas(900)
                .revision(state.revision)
                .dirty_mask(1)
                .view()
                .id(20)
                .fill(),
            connection_markers(state),
            row(state.node_order.iter().map(|node_id| {
                node_card(
                    node_base_id(node_id),
                    node_id,
                    node_label(node_id),
                    node_body(node_id),
                    connection_summary(state, node_id),
                    state.pending_output == Some(*node_id),
                    state.selected_node == *node_id,
                )
            }))
            .padding(28.0)
            .spacing(34.0)
            .fill(),
        ])
        .style(WidgetStyle::default())
        .height(232.0)
        .fill_width(),
        row([
            toggle("Filter enabled", state.filter_enabled)
                .on_change(|state: &mut NodeEditorState, enabled| {
                    state.filter_enabled = enabled;
                    state.revision += 1;
                    state.status = if enabled {
                        "filter on"
                    } else {
                        "filter bypassed"
                    }
                    .to_string();
                })
                .size(148.0, 30.0),
            text(format!(
                "selected={} connections={} status={}",
                state.selected_node,
                connection_text(&state.connections),
                state.status
            ))
            .id(500)
            .height(30.0)
            .fill_width(),
        ])
        .fill_width()
        .spacing(12.0),
    ])
    .style(WidgetStyle::default())
    .padding(16.0)
    .spacing(12.0)
    .fill()
}

fn node_card(
    base_id: u64,
    node_id: &'static str,
    label: &'static str,
    body: &'static str,
    connection_summary: String,
    wiring_from_here: bool,
    selected: bool,
) -> StateView<NodeEditorState> {
    stack([
        card().id(base_id).fill(),
        column([
            row([
                drag_handle()
                    .on_drag(move |state: &mut NodeEditorState, message| {
                        move_node_from_drag(state, node_id, message);
                    })
                    .id(base_id + 1)
                    .size(24.0, 24.0),
                selectable(label, selected)
                    .on_change(move |state: &mut NodeEditorState, selected| {
                        if selected {
                            state.selected_node = node_id;
                            state.status = format!("{node_id} selected");
                        }
                    })
                    .id(base_id + 2)
                    .fill_width(),
            ])
            .fill_width()
            .spacing(8.0),
            text(body).wrap().height(48.0).fill_width(),
            row([
                badge("input")
                    .on_click(move |state: &mut NodeEditorState| {
                        connect_pending_output_to(state, node_id);
                    })
                    .id(base_id + 3),
                badge(if wiring_from_here {
                    "output armed"
                } else {
                    "output"
                })
                .primary()
                .on_click(move |state: &mut NodeEditorState| {
                    begin_connection_from(state, node_id);
                })
                .id(base_id + 4),
            ])
            .spacing(8.0),
            text(connection_summary)
                .truncate()
                .height(22.0)
                .fill_width(),
        ])
        .padding(12.0)
        .spacing(8.0)
        .fill(),
    ])
    .height(156.0)
    .fill_width()
}

fn connection_markers(state: &NodeEditorState) -> StateView<NodeEditorState> {
    stack(
        state
            .connections
            .iter()
            .enumerate()
            .filter_map(|(index, connection)| {
                let from = state
                    .node_order
                    .iter()
                    .position(|node_id| node_id == &connection.from)?;
                let to = state
                    .node_order
                    .iter()
                    .position(|node_id| node_id == &connection.to)?;
                let left = from.min(to) as f32;
                let width = ((from.max(to) - from.min(to)) as f32 * 228.0 + 132.0).max(132.0);
                Some(
                    drop_marker(
                        180.0 + left * 262.0,
                        128.0 + index as f32 * 12.0,
                        width,
                        3.0,
                    )
                    .id(30 + index as u64),
                )
            })
            .collect::<Vec<_>>(),
    )
    .fill()
}

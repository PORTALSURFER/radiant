//! Node-editor-style workspace built from public Radiant application builders.

use radiant::prelude::*;

const NODE_IDS: [&str; 3] = ["input", "filter", "output"];

#[derive(Clone, Debug)]
struct NodeEditorState {
    selected_node: &'static str,
    node_order: Vec<&'static str>,
    connections: Vec<NodeConnection>,
    pending_output: Option<&'static str>,
    filter_enabled: bool,
    revision: u64,
    status: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct NodeConnection {
    from: &'static str,
    to: &'static str,
}

impl Default for NodeEditorState {
    fn default() -> Self {
        Self {
            selected_node: "filter",
            node_order: NODE_IDS.to_vec(),
            connections: vec![
                NodeConnection {
                    from: "input",
                    to: "filter",
                },
                NodeConnection {
                    from: "filter",
                    to: "output",
                },
            ],
            pending_output: None,
            filter_enabled: true,
            revision: 1,
            status: "ready".to_string(),
        }
    }
}

fn main() -> radiant::Result {
    radiant::app(NodeEditorState::default())
        .title("Radiant Node Editor")
        .size(780, 420)
        .min_size(560, 320)
        .view(project_surface)
        .run()
}

fn project_surface(state: &mut NodeEditorState) -> StateView<NodeEditorState> {
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

fn move_node_from_drag(
    state: &mut NodeEditorState,
    node_id: &'static str,
    message: DragHandleMessage,
) {
    match message {
        DragHandleMessage::Started { .. } => {
            state.selected_node = node_id;
            state.status = format!("{node_id} drag started");
        }
        DragHandleMessage::Moved { position } | DragHandleMessage::Ended { position } => {
            let phase = drag_phase(message);
            let target_index = slot_index_for_x(position.x, state.node_order.len());
            reorder_node(state, node_id, target_index);
            state.selected_node = node_id;
            state.revision += 1;
            state.status = format!("{node_id} {phase} slot {}", target_index + 1);
        }
    }
}

fn drag_phase(message: DragHandleMessage) -> &'static str {
    match message {
        DragHandleMessage::Started { .. } => "drag started",
        DragHandleMessage::Moved { .. } => "drag moved",
        DragHandleMessage::Ended { .. } => "drag ended",
    }
}

fn slot_index_for_x(x: f32, count: usize) -> usize {
    if count == 0 {
        return 0;
    }
    let canvas_left = 28.0;
    let slot_width = 262.0;
    (((x - canvas_left) / slot_width).round() as isize).clamp(0, count as isize - 1) as usize
}

fn reorder_node(state: &mut NodeEditorState, node_id: &'static str, target_index: usize) {
    let Some(current_index) = state
        .node_order
        .iter()
        .position(|candidate| *candidate == node_id)
    else {
        return;
    };
    let node = state.node_order.remove(current_index);
    let insert_at = target_index.min(state.node_order.len());
    state.node_order.insert(insert_at, node);
}

fn begin_connection_from(state: &mut NodeEditorState, node_id: &'static str) {
    state.selected_node = node_id;
    state.pending_output = Some(node_id);
    state.status = format!("{node_id} output armed");
}

fn connect_pending_output_to(state: &mut NodeEditorState, node_id: &'static str) {
    let Some(from) = state.pending_output.take() else {
        state.selected_node = node_id;
        state.status = format!("{node_id} input selected; choose an output first");
        return;
    };
    state.selected_node = node_id;
    if from == node_id {
        state.status = format!("{node_id} cannot connect to itself");
        return;
    }
    if let Some(connection) = state
        .connections
        .iter_mut()
        .find(|connection| connection.from == from)
    {
        connection.to = node_id;
    } else {
        state.connections.push(NodeConnection { from, to: node_id });
    }
    state.revision += 1;
    state.status = format!("{from} wired to {node_id}");
}

fn connection_summary(state: &NodeEditorState, node_id: &'static str) -> String {
    let incoming = state
        .connections
        .iter()
        .filter(|connection| connection.to == node_id)
        .map(|connection| connection.from)
        .collect::<Vec<_>>();
    let outgoing = state
        .connections
        .iter()
        .filter(|connection| connection.from == node_id)
        .map(|connection| connection.to)
        .collect::<Vec<_>>();
    format!(
        "in: {} out: {}",
        endpoint_list(&incoming),
        endpoint_list(&outgoing)
    )
}

fn endpoint_list(values: &[&'static str]) -> String {
    if values.is_empty() {
        String::from("-")
    } else {
        values.join(",")
    }
}

fn connection_text(connections: &[NodeConnection]) -> String {
    connections
        .iter()
        .map(|connection| format!("{}>{}", connection.from, connection.to))
        .collect::<Vec<_>>()
        .join(",")
}

fn node_base_id(node_id: &str) -> u64 {
    match node_id {
        "input" => 100,
        "filter" => 200,
        "output" => 300,
        _ => 900,
    }
}

fn node_label(node_id: &str) -> &'static str {
    match node_id {
        "input" => "Audio In",
        "filter" => "Filter",
        "output" => "Output",
        _ => "Node",
    }
}

fn node_body(node_id: &str) -> &'static str {
    match node_id {
        "input" => "Source node feeding the graph.",
        "filter" => "Toggle and shape the signal before output.",
        "output" => "Terminal node receiving the final signal.",
        _ => "Custom processing node.",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use radiant::{
        layout::{Point, Vector2},
        runtime::{RuntimeBridge, SurfaceRuntime},
        widgets::{PointerButton, TextWidget, WidgetInput, WidgetKey},
    };

    #[test]
    fn node_editor_routes_drag_selection_and_rewiring_through_public_builders() {
        let bridge = radiant::app(NodeEditorState::default())
            .view(project_surface)
            .into_bridge();
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(780.0, 420.0));

        assert!(runtime.surface().find_widget(20).is_some());
        assert!(runtime.surface().find_widget(101).is_some());
        assert!(runtime.surface().find_widget(202).is_some());
        assert!(runtime.surface().find_widget(204).is_some());
        assert!(runtime.surface().find_widget(303).is_some());
        assert!(runtime.surface().keyboard_focus_order().contains(&102));

        let pressed_selectable = runtime.dispatch_input(
            102,
            WidgetInput::PointerPress {
                position: Point::new(40.0, 40.0),
                button: PointerButton::Primary,
            },
        );
        let released_selectable = runtime.dispatch_input(
            102,
            WidgetInput::PointerRelease {
                position: Point::new(40.0, 40.0),
                button: PointerButton::Primary,
            },
        );
        let dragged = runtime.dispatch_input(
            101,
            WidgetInput::PointerPress {
                position: Point::new(34.0, 34.0),
                button: PointerButton::Primary,
            },
        );
        let moved = runtime.dispatch_input(
            101,
            WidgetInput::PointerMove {
                position: Point::new(650.0, 64.0),
            },
        );
        let ended = runtime.dispatch_input(
            101,
            WidgetInput::PointerRelease {
                position: Point::new(650.0, 64.0),
                button: PointerButton::Primary,
            },
        );

        assert!(pressed_selectable);
        assert!(released_selectable);
        assert!(dragged);
        assert!(moved);
        assert!(ended);
        assert!(status_text(&runtime).contains("input drag ended slot 3"));

        click(&mut runtime, 204);
        click(&mut runtime, 303);

        let status = status_text(&runtime);
        assert!(status.contains("filter>output"));
        assert!(status.contains("filter wired to output"));
    }

    fn click<Bridge>(
        runtime: &mut SurfaceRuntime<Bridge, StateAction<NodeEditorState>>,
        widget_id: u64,
    ) where
        Bridge: RuntimeBridge<StateAction<NodeEditorState>>,
    {
        assert!(runtime.focus_widget(widget_id));
        assert!(runtime.dispatch_input(widget_id, WidgetInput::KeyPress(WidgetKey::Enter),));
    }

    fn status_text<Bridge>(runtime: &SurfaceRuntime<Bridge, StateAction<NodeEditorState>>) -> String
    where
        Bridge: RuntimeBridge<StateAction<NodeEditorState>>,
    {
        runtime
            .surface()
            .find_widget(500)
            .expect("status widget exists")
            .widget_object()
            .as_any()
            .downcast_ref::<TextWidget>()
            .expect("status widget is text")
            .text
            .clone()
    }
}

//! Node graph ordering and connection behavior for the node editor example.

use super::NodeEditorState;
use radiant::prelude::DragHandleMessage;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct NodeConnection {
    pub(crate) from: &'static str,
    pub(crate) to: &'static str,
}

pub(crate) fn move_node_from_drag(
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

pub(crate) fn begin_connection_from(state: &mut NodeEditorState, node_id: &'static str) {
    state.selected_node = node_id;
    state.pending_output = Some(node_id);
    state.status = format!("{node_id} output armed");
}

pub(crate) fn connect_pending_output_to(state: &mut NodeEditorState, node_id: &'static str) {
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

pub(crate) fn connection_summary(state: &NodeEditorState, node_id: &'static str) -> String {
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

pub(crate) fn connection_text(connections: &[NodeConnection]) -> String {
    connections
        .iter()
        .map(|connection| format!("{}>{}", connection.from, connection.to))
        .collect::<Vec<_>>()
        .join(",")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_graph_model_reorders_and_rewires_without_duplicate_outputs() {
        let mut state = NodeEditorState::default();

        reorder_node(&mut state, "input", 2);
        assert_eq!(state.node_order, vec!["filter", "output", "input"]);

        begin_connection_from(&mut state, "input");
        connect_pending_output_to(&mut state, "output");

        assert_eq!(
            state.connections,
            vec![
                NodeConnection {
                    from: "input",
                    to: "output"
                },
                NodeConnection {
                    from: "filter",
                    to: "output"
                },
            ]
        );
        assert_eq!(state.pending_output, None);
        assert!(state.status.contains("input wired to output"));
    }
}

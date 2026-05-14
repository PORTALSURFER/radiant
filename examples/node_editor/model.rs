use radiant::prelude::DragHandleMessage;

pub(super) const NODE_IDS: [&str; 3] = ["input", "filter", "output"];

#[derive(Clone, Debug)]
pub(super) struct NodeEditorState {
    pub(super) selected_node: &'static str,
    pub(super) node_order: Vec<&'static str>,
    pub(super) connections: Vec<NodeConnection>,
    pub(super) pending_output: Option<&'static str>,
    pub(super) filter_enabled: bool,
    pub(super) revision: u64,
    pub(super) status: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct NodeConnection {
    pub(super) from: &'static str,
    pub(super) to: &'static str,
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

pub(super) fn move_node_from_drag(
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

pub(super) fn drag_phase(message: DragHandleMessage) -> &'static str {
    match message {
        DragHandleMessage::Started { .. } => "drag started",
        DragHandleMessage::Moved { .. } => "drag moved",
        DragHandleMessage::Ended { .. } => "drag ended",
    }
}

pub(super) fn slot_index_for_x(x: f32, count: usize) -> usize {
    if count == 0 {
        return 0;
    }
    let canvas_left = 28.0;
    let slot_width = 262.0;
    (((x - canvas_left) / slot_width).round() as isize).clamp(0, count as isize - 1) as usize
}

pub(super) fn reorder_node(
    state: &mut NodeEditorState,
    node_id: &'static str,
    target_index: usize,
) {
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

pub(super) fn begin_connection_from(state: &mut NodeEditorState, node_id: &'static str) {
    state.selected_node = node_id;
    state.pending_output = Some(node_id);
    state.status = format!("{node_id} output armed");
}

pub(super) fn connect_pending_output_to(state: &mut NodeEditorState, node_id: &'static str) {
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

pub(super) fn connection_summary(state: &NodeEditorState, node_id: &'static str) -> String {
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

pub(super) fn endpoint_list(values: &[&'static str]) -> String {
    if values.is_empty() {
        String::from("-")
    } else {
        values.join(",")
    }
}

pub(super) fn connection_text(connections: &[NodeConnection]) -> String {
    connections
        .iter()
        .map(|connection| format!("{}>{}", connection.from, connection.to))
        .collect::<Vec<_>>()
        .join(",")
}

pub(super) fn node_base_id(node_id: &str) -> u64 {
    match node_id {
        "input" => 100,
        "filter" => 200,
        "output" => 300,
        _ => 900,
    }
}

pub(super) fn node_label(node_id: &str) -> &'static str {
    match node_id {
        "input" => "Audio In",
        "filter" => "Filter",
        "output" => "Output",
        _ => "Node",
    }
}

pub(super) fn node_body(node_id: &str) -> &'static str {
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

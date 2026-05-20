#[path = "model/graph.rs"]
mod graph;
#[path = "model/metadata.rs"]
mod metadata;

pub(super) use graph::{
    NodeConnection, begin_connection_from, connect_pending_output_to, connection_summary,
    connection_text, move_node_from_drag,
};
pub(super) use metadata::{NODE_IDS, node_base_id, node_body, node_label};

/// Host-owned state for the node editor example.
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

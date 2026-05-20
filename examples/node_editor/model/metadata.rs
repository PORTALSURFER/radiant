//! Static node metadata for the node editor example.

pub(crate) const NODE_IDS: [&str; 3] = ["input", "filter", "output"];

pub(crate) fn node_base_id(node_id: &str) -> u64 {
    match node_id {
        "input" => 100,
        "filter" => 200,
        "output" => 300,
        _ => 900,
    }
}

pub(crate) fn node_label(node_id: &str) -> &'static str {
    match node_id {
        "input" => "Audio In",
        "filter" => "Filter",
        "output" => "Output",
        _ => "Node",
    }
}

pub(crate) fn node_body(node_id: &str) -> &'static str {
    match node_id {
        "input" => "Source node feeding the graph.",
        "filter" => "Toggle and shape the signal before output.",
        "output" => "Terminal node receiving the final signal.",
        _ => "Custom processing node.",
    }
}

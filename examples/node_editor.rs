//! Node-editor-style workspace built from public Radiant application builders.

#[path = "node_editor/model.rs"]
mod model;
#[path = "node_editor/view.rs"]
mod view;

use model::NodeEditorState;
use view::{project_surface, update};

#[cfg(test)]
#[path = "node_editor/tests.rs"]
mod tests;

fn main() -> radiant::Result {
    radiant::app(NodeEditorState::default())
        .title("Radiant Node Editor")
        .size(780, 420)
        .min_size(560, 320)
        .view(project_surface)
        .update(update)
        .run()
}

//! Standalone todo-list app built with Radiant application builders.

use radiant::prelude as ui;

#[path = "todo_list/model.rs"]
mod model;
#[path = "todo_list/view.rs"]
mod view;

#[cfg(test)]
#[path = "todo_list/tests.rs"]
mod tests;

use model::TodoState;
use view::{project_surface, update};

fn main() -> radiant::Result {
    radiant::app(TodoState::default())
        .title("Radiant Todo List")
        .size(700, 480)
        .min_size(520, 340)
        .view(project_surface)
        .update(update)
        .run()
}

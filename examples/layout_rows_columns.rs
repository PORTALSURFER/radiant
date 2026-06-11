//! Dynamic row and column panels showing fill behavior on both axes.

use radiant::prelude as ui;

#[path = "layout_rows_columns/model.rs"]
mod model;
#[path = "layout_rows_columns/view.rs"]
mod view;

#[cfg(test)]
#[path = "layout_rows_columns/tests.rs"]
mod tests;

use model::LayoutDemoState;
use view::{project_surface, update};

fn main() -> radiant::Result {
    radiant::app(LayoutDemoState::default())
        .title("Radiant Rows and Columns")
        .size(860, 620)
        .min_size(620, 420)
        .view(project_surface)
        .update(update)
        .run()
}

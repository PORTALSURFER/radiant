use super::*;
use crate::{application::IntoView, layout::LayoutNode};

#[path = "tests/bounded_scroll.rs"]
mod bounded_scroll;
#[path = "tests/row_chrome.rs"]
mod row_chrome;
#[path = "tests/scroll_update.rs"]
mod scroll_update;
#[path = "tests/tree_window.rs"]
mod tree_window;
#[path = "tests/virtual_window.rs"]
mod virtual_window;

fn count_layout_nodes(node: &LayoutNode) -> usize {
    match node {
        LayoutNode::Widget(_) => 1,
        LayoutNode::Container(container) => {
            1 + container
                .children
                .iter()
                .map(|child| count_layout_nodes(&child.child))
                .sum::<usize>()
        }
    }
}

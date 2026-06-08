//! Large virtualized list using application builders.

use radiant::prelude::*;

const ROW_COUNT: usize = 10_000;
const ROW_HEIGHT: f32 = 32.0;
const VIEWPORT_ROWS: usize = 18;
const OVERSCAN_ROWS: usize = 6;
const SCROLL_OVERSCAN_PX: f32 = ROW_HEIGHT * OVERSCAN_ROWS as f32;
const LIST_ID: u64 = 100;
const ROW_ID_BASE: u64 = 10_000;

#[derive(Clone, Debug, PartialEq)]
enum Message {
    Select(usize),
    WindowChanged(VirtualListWindowChange),
}

#[derive(Default)]
struct DemoState {
    selected: Option<usize>,
    view_start: usize,
}

fn main() -> radiant::Result {
    radiant::app(DemoState::default())
        .title("Radiant Virtualized List")
        .size(420, 420)
        .min_size(320, 260)
        .view(project_surface)
        .update(|state, message| match message {
            Message::Select(index) => {
                state.selected = Some(index);
            }
            Message::WindowChanged(change) => {
                state.view_start = change.window.viewport_start;
            }
        })
        .run()
}

fn project_surface(state: &mut DemoState) -> View<Message> {
    let selected = state
        .selected
        .map(|index| format!("Selected: {index:05}"))
        .unwrap_or_else(|| String::from("Select a row"));
    let window = resolve_virtual_list_window(VirtualListWindowRequest {
        total_items: ROW_COUNT,
        viewport_len: VIEWPORT_ROWS,
        requested_start: state.view_start,
        overscan: OVERSCAN_ROWS,
        ..VirtualListWindowRequest::default()
    });
    state.view_start = window.viewport_start;

    column([
        text(selected).height(28.0).fill_width(),
        virtual_list_windowed(|index| {
            let label = if Some(index) == state.selected {
                format!("Selected row {index:05}")
            } else {
                format!("Row {index:05}")
            };
            selectable(label, Some(index) == state.selected)
                .message(move |_| Message::Select(index))
                .id(index as u64 + ROW_ID_BASE)
                .fill_width()
        })
        .row_height(ROW_HEIGHT)
        .window(window)
        .overscan_px(SCROLL_OVERSCAN_PX)
        .on_window_changed(Message::WindowChanged)
        .view()
        .id(LIST_ID)
        .fill_height(),
    ])
    .padding(16.0)
    .spacing(10.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use radiant::layout::LayoutNode;

    #[test]
    fn virtualized_list_projects_bounded_window_from_scroll_state() {
        let mut state = DemoState {
            selected: Some(4_020),
            view_start: 4_000,
        };
        let surface = project_surface(&mut state).into_surface();
        let layout = surface.layout_node();

        assert_eq!(state.view_start, 4_000);
        assert!(
            node_count(&layout) < 96,
            "windowed example should project only visible rows plus overscan"
        );
        assert!(surface.find_widget(ROW_ID_BASE + 4_000).is_some());
        assert!(surface.find_widget(ROW_ID_BASE + 4_023).is_some());
        assert!(surface.find_widget(ROW_ID_BASE + 100).is_none());
    }

    #[test]
    fn virtualized_list_scroll_change_maps_to_logical_rows() {
        let current = resolve_virtual_list_window(VirtualListWindowRequest {
            total_items: ROW_COUNT,
            viewport_len: VIEWPORT_ROWS,
            overscan: OVERSCAN_ROWS,
            ..VirtualListWindowRequest::default()
        });
        let change = virtual_list_window_change_for_scroll(
            ScrollUpdate {
                node_id: LIST_ID,
                position: Point::default(),
                delta: Vector2::new(0.0, 128_000.0),
                previous_offset: Vector2::default(),
                offset: Vector2::new(0.0, 128_000.0),
                viewport: Vector2::new(400.0, ROW_HEIGHT * VIEWPORT_ROWS as f32),
            },
            ROW_HEIGHT,
            current,
            OVERSCAN_ROWS,
        );

        assert_eq!(change.window.viewport_start, 4_000);
    }

    fn node_count(node: &LayoutNode) -> usize {
        match node {
            LayoutNode::Widget(_) => 1,
            LayoutNode::Container(container) => {
                1 + container
                    .children
                    .iter()
                    .map(|child| node_count(&child.child))
                    .sum::<usize>()
            }
        }
    }
}

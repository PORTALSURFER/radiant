//! Node-editor-style workspace built from public Radiant application builders.

use radiant::prelude::*;

#[derive(Clone, Debug)]
struct NodeEditorState {
    selected_node: &'static str,
    filter_enabled: bool,
    revision: u64,
    status: String,
}

impl Default for NodeEditorState {
    fn default() -> Self {
        Self {
            selected_node: "filter",
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
            drop_marker(180.0, 128.0, 132.0, 3.0).id(30),
            drop_marker(442.0, 128.0, 132.0, 3.0).id(31),
            row([
                node_card(
                    100,
                    "input",
                    "Audio In",
                    "Source node feeding the graph.",
                    state.selected_node == "input",
                ),
                node_card(
                    200,
                    "filter",
                    "Filter",
                    "Toggle and shape the signal before output.",
                    state.selected_node == "filter",
                ),
                node_card(
                    300,
                    "output",
                    "Output",
                    "Terminal node receiving the final signal.",
                    state.selected_node == "output",
                ),
            ])
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
                "selected={} status={}",
                state.selected_node, state.status
            ))
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
    selected: bool,
) -> StateView<NodeEditorState> {
    stack([
        card().id(base_id).fill(),
        column([
            row([
                drag_handle()
                    .on_drag(move |state: &mut NodeEditorState, message| {
                        state.status = format!("{node_id} {}", drag_status(message));
                        state.revision += 1;
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
                badge("input").on_click(move |state: &mut NodeEditorState| {
                    state.status = format!("{node_id} input inspected");
                }),
                badge("output")
                    .primary()
                    .on_click(move |state: &mut NodeEditorState| {
                        state.status = format!("{node_id} output inspected");
                    }),
            ])
            .spacing(8.0),
        ])
        .padding(12.0)
        .spacing(8.0)
        .fill(),
    ])
    .height(156.0)
    .fill_width()
}

fn drag_status(message: DragHandleMessage) -> &'static str {
    match message {
        DragHandleMessage::Started { .. } => "drag started",
        DragHandleMessage::Moved { .. } => "drag moved",
        DragHandleMessage::Ended { .. } => "drag ended",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use radiant::{
        layout::{Point, Vector2},
        runtime::SurfaceRuntime,
        widgets::{PointerButton, WidgetInput},
    };

    #[test]
    fn node_editor_routes_drag_and_selection_through_public_builders() {
        let bridge = radiant::app(NodeEditorState::default())
            .view(project_surface)
            .into_bridge();
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(780.0, 420.0));

        assert!(runtime.surface().find_widget(20).is_some());
        assert!(runtime.surface().find_widget(101).is_some());
        assert!(runtime.surface().find_widget(202).is_some());
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

        assert!(pressed_selectable);
        assert!(released_selectable);
        assert!(dragged);
    }
}

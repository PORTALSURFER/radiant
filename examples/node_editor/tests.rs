use radiant::{
    layout::{Point, Vector2},
    runtime::{RuntimeBridge, SurfaceRuntime},
    widgets::{PointerButton, TextWidget, WidgetInput, WidgetKey},
};

use crate::{
    model::NodeEditorState,
    view::{NodeEditorMessage, project_surface, update},
};

#[test]
fn node_editor_routes_drag_selection_and_rewiring_through_public_builders() {
    let bridge = radiant::app(NodeEditorState::default())
        .view(project_surface)
        .update(update)
        .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(780.0, 420.0));

    assert!(runtime.surface().find_widget(20).is_some());
    assert!(runtime.surface().find_widget(101).is_some());
    assert!(runtime.surface().find_widget(202).is_some());
    assert!(runtime.surface().find_widget(204).is_some());
    assert!(runtime.surface().find_widget(303).is_some());
    assert!(runtime.surface().keyboard_focus_order().contains(&102));

    let pressed_selectable = runtime.dispatch_input(
        102,
        WidgetInput::PointerPress {
            position: Point::new(40.0, 40.0),
            button: PointerButton::Primary,
            modifiers: Default::default(),
        },
    );
    let released_selectable = runtime.dispatch_input(
        102,
        WidgetInput::PointerRelease {
            position: Point::new(40.0, 40.0),
            button: PointerButton::Primary,
            modifiers: Default::default(),
        },
    );
    let dragged = runtime.dispatch_input(
        101,
        WidgetInput::PointerPress {
            position: Point::new(34.0, 34.0),
            button: PointerButton::Primary,
            modifiers: Default::default(),
        },
    );
    let moved = runtime.dispatch_input(
        101,
        WidgetInput::PointerMove {
            position: Point::new(650.0, 64.0),
        },
    );
    let ended = runtime.dispatch_input(
        101,
        WidgetInput::PointerRelease {
            position: Point::new(650.0, 64.0),
            button: PointerButton::Primary,
            modifiers: Default::default(),
        },
    );

    assert!(pressed_selectable);
    assert!(released_selectable);
    assert!(dragged);
    assert!(moved);
    assert!(ended);
    assert!(status_text(&runtime).contains("input drag ended slot 3"));

    click(&mut runtime, 204);
    click(&mut runtime, 303);

    let status = status_text(&runtime);
    assert!(status.contains("filter>output"));
    assert!(status.contains("filter wired to output"));
}

fn click<Bridge>(runtime: &mut SurfaceRuntime<Bridge, NodeEditorMessage>, widget_id: u64)
where
    Bridge: RuntimeBridge<NodeEditorMessage>,
{
    assert!(runtime.focus_widget(widget_id));
    assert!(runtime.dispatch_input(widget_id, WidgetInput::KeyPress(WidgetKey::Enter),));
}

fn status_text<Bridge>(runtime: &SurfaceRuntime<Bridge, NodeEditorMessage>) -> String
where
    Bridge: RuntimeBridge<NodeEditorMessage>,
{
    runtime
        .surface()
        .find_widget(500)
        .expect("status widget exists")
        .widget_object()
        .as_any()
        .downcast_ref::<TextWidget>()
        .expect("status widget is text")
        .text
        .to_string()
}

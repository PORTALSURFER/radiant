use crate::model::{DemoMessage, DemoState};
use crate::selection_overlay::SelectionOverlay;
use crate::view::{SURFACE_HEIGHT, SURFACE_WIDTH};
use radiant::gui::visualization::DragHandleRole;
use radiant::prelude::*;
use radiant::runtime::{Event, RuntimeBridge, SurfaceRuntime};
use radiant::widgets::{TextWidget, WidgetId};

#[test]
fn resize_selection_keeps_minimum_width() {
    let mut overlay = SelectionOverlay::new(&DemoState {
        selection_start: 0.22,
        selection_end: 0.68,
        ..DemoState::default()
    });
    overlay.drag_handle = Some(DragHandleRole::Start);

    overlay.resize_selection(0.67);

    assert!(overlay.selection_end - overlay.selection_start >= 0.04);
}

#[test]
fn resize_preview_stays_widget_local_until_release() {
    let mut overlay = SelectionOverlay::new(&DemoState::default());
    overlay.drag_handle = Some(DragHandleRole::Start);
    let bounds = Rect::from_min_size(
        Point::new(0.0, 0.0),
        Vector2::new(SURFACE_WIDTH, SURFACE_HEIGHT),
    );

    let output = overlay.handle_input(
        bounds,
        WidgetInput::PointerMove {
            position: Point::new(SURFACE_WIDTH * 0.32, 10.0),
        },
    );

    assert!(output.is_none());
    assert_eq!(overlay.selection_start, 0.32);
    assert_eq!(overlay.drag_handle, Some(DragHandleRole::Start));
}

#[test]
fn resize_release_commits_final_selection_once() {
    let mut overlay = SelectionOverlay::new(&DemoState::default());
    overlay.drag_handle = Some(DragHandleRole::End);
    overlay.selection_end = 0.74;
    let bounds = Rect::from_min_size(
        Point::new(0.0, 0.0),
        Vector2::new(SURFACE_WIDTH, SURFACE_HEIGHT),
    );

    let output = overlay
        .handle_input(
            bounds,
            WidgetInput::PointerRelease {
                position: Point::new(SURFACE_WIDTH * 0.74, 10.0),
                button: PointerButton::Primary,
                modifiers: Default::default(),
            },
        )
        .expect("release should emit a commit message");

    assert_eq!(overlay.drag_handle, None);
    assert_eq!(
        output.custom_ref::<DemoMessage>(),
        Some(&DemoMessage::CommitResize {
            start: 0.22,
            end: 0.74
        })
    );
}

#[test]
fn runtime_resize_drag_previews_locally_and_commits_once() {
    let mut runtime = SurfaceRuntime::new(
        radiant::app(DemoState::default())
            .view(|state| {
                column([
                    text(format!(
                        "selection {:.0}% - {:.0}%",
                        state.selection_start * 100.0,
                        state.selection_end * 100.0
                    ))
                    .id(2)
                    .height(32.0),
                    custom_widget_mapped(SelectionOverlay::new(state), |message: DemoMessage| {
                        message
                    })
                    .id(11)
                    .size(SURFACE_WIDTH, SURFACE_HEIGHT),
                ])
            })
            .update_command(|state: &mut DemoState, message| match message {
                DemoMessage::CommitResize { start, end } => {
                    state.commit_selection(start, end);
                    Command::request_repaint()
                }
                _ => Command::none(),
            })
            .into_bridge(),
        Vector2::new(SURFACE_WIDTH, SURFACE_HEIGHT + 32.0),
    );

    let handle_position = Point::new(SURFACE_WIDTH * 0.22, 44.0);
    let preview_position = Point::new(SURFACE_WIDTH * 0.32, 44.0);

    runtime.dispatch_event(Event::PointerPress {
        position: handle_position,
        button: PointerButton::Primary,
        modifiers: Default::default(),
    });
    runtime.dispatch_event(Event::PointerMove {
        position: preview_position,
    });

    let overlay = selection_overlay(&runtime);
    assert_eq!(overlay.selection_start, 0.32);
    assert_eq!(overlay.drag_handle, Some(DragHandleRole::Start));
    assert_eq!(
        text_widget(&runtime, 2).text,
        "selection 22% - 68%",
        "host state should not refresh on every drag pixel"
    );

    runtime.dispatch_event(Event::PointerRelease {
        position: preview_position,
        button: PointerButton::Primary,
        modifiers: Default::default(),
    });

    assert_eq!(selection_overlay(&runtime).drag_handle, None);
    assert_eq!(text_widget(&runtime, 2).text, "selection 32% - 68%");
}

fn selection_overlay<Bridge, Message>(
    runtime: &SurfaceRuntime<Bridge, Message>,
) -> &SelectionOverlay
where
    Bridge: RuntimeBridge<Message>,
{
    runtime
        .surface()
        .find_widget(11)
        .expect("selection overlay should exist")
        .widget()
        .as_any()
        .downcast_ref::<SelectionOverlay>()
        .expect("widget should be selection overlay")
}

fn text_widget<Bridge, Message>(
    runtime: &SurfaceRuntime<Bridge, Message>,
    id: WidgetId,
) -> &TextWidget
where
    Bridge: RuntimeBridge<Message>,
{
    runtime
        .surface()
        .find_widget(id)
        .expect("text widget should exist")
        .widget()
        .as_any()
        .downcast_ref::<TextWidget>()
        .expect("widget should be text")
}

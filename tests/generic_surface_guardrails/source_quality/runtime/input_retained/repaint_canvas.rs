use super::*;

#[test]
fn repaint_signaling_keeps_coalescing_and_callback_tests_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source = fs::read_to_string(manifest_dir.join("src/gui/repaint.rs"))
        .expect("repaint signaling source should be readable");
    let tests = fs::read_to_string(manifest_dir.join("src/gui/repaint/tests.rs"))
        .expect("repaint signaling behavior tests should be readable");

    assert!(
        source.contains("pub trait RepaintSignal")
            && source.contains("pub fn try_mark_repaint_pending")
            && source.contains("pub struct CoalescingRepaintSignal")
            && source.contains("pub struct SharedRepaintSignal")
            && source.contains("#[path = \"repaint/tests.rs\"]")
            && !source.contains("fn shared_repaint_signal_forwards_request_to_active_callback"),
        "repaint signaling primitives should live in gui/repaint.rs while behavior tests stay delegated"
    );
    assert!(
        tests.contains("fn shared_repaint_signal_forwards_request_to_active_callback")
            && tests.contains("fn coalescing_repaint_signal_clears_pending_when_queue_fails"),
        "repaint behavior coverage should live in gui/repaint/tests.rs"
    );
}

#[test]
fn canvas_gesture_primitives_stay_in_event_pointer_and_state_modules() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/widgets/interaction/canvas_gesture.rs"))
        .expect("canvas gesture root should be readable");
    let event =
        fs::read_to_string(manifest_dir.join("src/widgets/interaction/canvas_gesture/event.rs"))
            .expect("canvas gesture event module should be readable");
    let pointer =
        fs::read_to_string(manifest_dir.join("src/widgets/interaction/canvas_gesture/pointer.rs"))
            .expect("canvas gesture pointer module should be readable");
    let state =
        fs::read_to_string(manifest_dir.join("src/widgets/interaction/canvas_gesture/state.rs"))
            .expect("canvas gesture state module should be readable");
    let active_press = fs::read_to_string(
        manifest_dir.join("src/widgets/interaction/canvas_gesture/state/active_press.rs"),
    )
    .expect("canvas gesture active press module should be readable");
    let state_tests = fs::read_to_string(
        manifest_dir.join("src/widgets/interaction/canvas_gesture/state/tests.rs"),
    )
    .expect("canvas gesture state tests should be readable");

    for required in [
        "mod event;",
        "mod pointer;",
        "mod state;",
        "pub use event::CanvasGestureEvent;",
        "pub use pointer::CanvasPointer;",
        "pub use state::CanvasGestureState;",
    ] {
        assert!(
            root.contains(required),
            "canvas gesture root should delegate `{required}`"
        );
    }
    assert!(
        !root.contains("pub enum CanvasGestureEvent")
            && !root.contains("pub struct CanvasPointer")
            && !root.contains("pub struct CanvasGestureState"),
        "canvas gesture root should re-export public primitives instead of owning their implementations"
    );
    assert!(
        event.contains("pub enum CanvasGestureEvent")
            && event.contains("Hover(CanvasPointer)")
            && event.contains("FocusChanged(bool)"),
        "canvas gesture event variants should live in canvas_gesture/event.rs"
    );
    assert!(
        pointer.contains("pub struct CanvasPointer")
            && pointer.contains("fn canvas_pointer")
            && pointer.contains("fn point_delta"),
        "canvas pointer projection and delta helpers should live in canvas_gesture/pointer.rs"
    );
    assert!(
        state.contains("mod active_press;")
            && state.contains("#[cfg(test)]")
            && state.contains("mod tests;")
            && state.contains("pub struct CanvasGestureState")
            && state.contains("pub fn handle_input"),
        "canvas retained state and input resolution should live in canvas_gesture/state.rs"
    );
    assert!(
        !state.contains("struct ActiveCanvasPress")
            && active_press.contains("struct ActiveCanvasPress")
            && active_press.contains("origin: CanvasPointer")
            && active_press.contains("button: PointerButton")
            && active_press.contains("modifiers: PointerModifiers"),
        "canvas retained press metadata should live in canvas_gesture/state/active_press.rs"
    );
    assert!(
        !state.contains("fn canvas_gesture_state_tracks_press_drag_and_release")
            && state_tests
                .contains("fn canvas_gesture_state_projects_local_and_normalized_positions")
            && state_tests.contains("fn canvas_gesture_state_tracks_press_drag_and_release")
            && state_tests.contains("fn canvas_gesture_state_clears_drag_on_focus_loss"),
        "canvas gesture state regression tests should live in canvas_gesture/state/tests.rs"
    );
}

use super::*;

#[test]
fn scrollbar_primitive_keeps_surface_builders_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/widgets/primitives/scrollbar.rs"))
        .expect("scrollbar primitive root should be readable");
    let builders =
        fs::read_to_string(manifest_dir.join("src/widgets/primitives/scrollbar/builders.rs"))
            .expect("scrollbar primitive builders should be readable");
    let tests = fs::read_to_string(manifest_dir.join("src/widgets/primitives/scrollbar/tests.rs"))
        .expect("scrollbar primitive tests should be readable");

    assert!(
        root.contains("mod builders;")
            && root.contains("pub struct ScrollbarWidget")
            && root.contains("impl Widget for ScrollbarWidget")
            && root.contains("#[path = \"scrollbar/tests.rs\"]")
            && !root.contains("impl<Message> SurfaceNode<Message>")
            && !root.contains("impl<Message> WidgetMessageMapper<Message>")
            && !root.contains("fn scrollbar_drag_emits_clamped_offset_changes"),
        "scrollbar primitive root should own widget behavior while delegating runtime builders and behavior tests"
    );
    assert!(
        builders.contains("impl<Message> SurfaceNode<Message>")
            && builders.contains("pub fn scrollbar(")
            && builders.contains("pub fn scrollbar_mapped(")
            && builders.contains("impl<Message> WidgetMessageMapper<Message>"),
        "scrollbar runtime builder helpers should live in scrollbar/builders.rs"
    );
    assert!(
        tests.contains("fn scrollbar_drag_emits_clamped_offset_changes")
            && tests.contains("fn scrollbar_track_click_centers_thumb"),
        "scrollbar behavior tests should live in scrollbar/tests.rs"
    );
}

#[test]
fn drag_handle_primitive_keeps_surface_builders_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/widgets/primitives/drag_handle.rs"))
        .expect("drag-handle primitive root should be readable");
    let builders =
        fs::read_to_string(manifest_dir.join("src/widgets/primitives/drag_handle/builders.rs"))
            .expect("drag-handle primitive builders should be readable");

    assert!(
        root.contains("mod builders;")
            && root.contains("pub struct DragHandleWidget")
            && root.contains("impl Widget for DragHandleWidget")
            && !root.contains("impl<Message> SurfaceNode<Message>")
            && !root.contains("impl<Message> WidgetMessageMapper<Message>"),
        "drag-handle primitive root should own widget behavior and delegate runtime builders"
    );
    assert!(
        builders.contains("impl<Message> SurfaceNode<Message>")
            && builders.contains("pub fn drag_handle_mapped(")
            && builders.contains("impl<Message> WidgetMessageMapper<Message>")
            && builders.contains("pub fn drag_handle("),
        "drag-handle runtime builder helpers should live in drag_handle/builders.rs"
    );
}

#[test]
fn slider_primitive_keeps_surface_builders_and_tests_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/widgets/primitives/slider.rs"))
        .expect("slider primitive root should be readable");
    let builders =
        fs::read_to_string(manifest_dir.join("src/widgets/primitives/slider/builders.rs"))
            .expect("slider primitive builders should be readable");
    let tests = fs::read_to_string(manifest_dir.join("src/widgets/primitives/slider/tests.rs"))
        .expect("slider primitive tests should be readable");

    assert!(
        root.contains("mod builders;")
            && root.contains("pub struct SliderWidget")
            && root.contains("impl Widget for SliderWidget")
            && root.contains("#[path = \"slider/tests.rs\"]")
            && !root.contains("impl<Message> SurfaceNode<Message>")
            && !root.contains("impl<Message> WidgetMessageMapper<Message>")
            && !root.contains("fn slider_pointer_drag_emits_clamped_values"),
        "slider primitive root should own widget behavior while delegating runtime builders and behavior tests"
    );
    assert!(
        builders.contains("impl<Message> SurfaceNode<Message>")
            && builders.contains("pub fn slider(")
            && builders.contains("pub fn slider_mapped(")
            && builders.contains("impl<Message> WidgetMessageMapper<Message>"),
        "slider runtime builder helpers should live in slider/builders.rs"
    );
    assert!(
        tests.contains("fn slider_pointer_drag_emits_clamped_values")
            && tests.contains("fn focused_slider_responds_to_keyboard_steps"),
        "slider behavior tests should live in slider/tests.rs"
    );
}

#[test]
fn toggle_primitive_keeps_surface_builders_and_tests_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/widgets/primitives/toggle.rs"))
        .expect("toggle primitive root should be readable");
    let builders =
        fs::read_to_string(manifest_dir.join("src/widgets/primitives/toggle/builders.rs"))
            .expect("toggle primitive builders should be readable");
    let tests = fs::read_to_string(manifest_dir.join("src/widgets/primitives/toggle/tests.rs"))
        .expect("toggle primitive tests should be readable");

    assert!(
        root.contains("mod builders;")
            && root.contains("pub struct ToggleWidget")
            && root.contains("impl Widget for ToggleWidget")
            && root.contains("#[path = \"toggle/tests.rs\"]")
            && !root.contains("impl<Message> SurfaceNode<Message>")
            && !root.contains("impl<Message> WidgetMessageMapper<Message>")
            && !root.contains("fn toggle_keyboard_activation_flips_active_state"),
        "toggle primitive root should own widget behavior while delegating runtime builders and behavior tests"
    );
    assert!(
        builders.contains("impl<Message> SurfaceNode<Message>")
            && builders.contains("pub fn toggle(")
            && builders.contains("pub fn toggle_with_checked(")
            && builders.contains("pub fn toggle_mapped(")
            && builders.contains("pub fn toggle_mapped_with_checked(")
            && builders.contains("impl<Message> WidgetMessageMapper<Message>"),
        "toggle runtime builder helpers should live in toggle/builders.rs"
    );
    assert!(
        tests.contains("fn toggle_keyboard_activation_flips_active_state"),
        "toggle behavior tests should live in toggle/tests.rs"
    );
}

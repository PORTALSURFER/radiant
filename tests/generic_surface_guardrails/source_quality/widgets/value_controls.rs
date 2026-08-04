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
    let input =
        fs::read_to_string(manifest_dir.join("src/widgets/primitives/drag_handle/input.rs"))
            .expect("drag-handle input module should be readable");

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
    assert!(
        input.contains("DragHandleMetadata")
            && input.contains("modifiers")
            && input.contains("timestamp")
            && input.contains("sequence_range")
            && input.contains("pointer_move_with_metadata")
            && input.contains("sequence_range: None")
            && input
                .contains("drag_handle_preserves_native_metadata_and_only_moves_carry_sequences")
            && input.contains("DragHandleMetadata::empty()"),
        "drag-handle input should preserve native modifiers/timestamps, restrict sequences to moves, and test synthetic/cancellation absence"
    );
}

#[test]
fn knob_pointer_provenance_stays_incremental_and_observational() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let messages =
        fs::read_to_string(manifest_dir.join("src/widgets/interaction/messages/range.rs"))
            .expect("range interaction messages should be readable");
    let knob = fs::read_to_string(manifest_dir.join("src/widgets/primitives/knob.rs"))
        .expect("knob primitive should be readable");
    let state_start = knob
        .find("pub struct KnobState")
        .expect("knob state should remain a public model");
    let state_end = knob[state_start..]
        .find("}\n")
        .expect("knob state should have a closed field list");
    let state = &knob[state_start..state_start + state_end];

    assert!(
        messages.contains("pub struct KnobPointerMetadata")
            && messages.contains("#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]")
            && messages.contains("pub const fn pointer_gesture_metadata")
            && messages.contains("Self::Reset { metadata, .. } => Some(*metadata)")
            && messages.contains("Self::KeyboardGesture(_) | Self::WheelGesture(_) => None"),
        "knob pointer provenance should be a copy-only pointer-message API"
    );
    assert!(
        knob.contains("KnobPointerMetadata {")
            && knob.contains("sequence_range: None")
            && knob.contains("PointerMove {")
            && knob.contains("finish_terminal_gesture"),
        "knob input routing should forward pointer metadata at the existing lifecycle boundaries"
    );
    assert!(
        !state.contains("KnobPointerMetadata") && !state.contains("metadata"),
        "pointer provenance should not become retained knob widget state"
    );
    for test in [
        "fn knob_pointer_gesture_forwards_native_metadata_by_phase",
        "fn knob_pointer_gesture_uses_empty_metadata_for_synthetic_and_focus_loss",
        "fn knob_pointer_gesture_metadata_is_not_reported_for_keyboard_or_wheel",
        "fn knob_pointer_gesture_omits_clamped_noop_moves",
        "fn knob_pointer_drop_forwards_terminal_metadata",
        "fn knob_reset_forwards_native_metadata_and_cleans_pointer_state",
        "fn knob_reset_emits_once_when_value_already_equals_default",
    ] {
        assert!(
            knob.contains(test),
            "knob provenance coverage should include `{test}`"
        );
    }
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
            && builders.contains("pub fn slider_edits_mapped(")
            && builders.contains("pub fn slider_edits(")
            && builders.contains("impl<Message> WidgetMessageMapper<Message>"),
        "slider runtime builder helpers should live in slider/builders.rs"
    );
    assert!(
        tests.contains("fn slider_pointer_drag_emits_clamped_values")
            && tests.contains("fn focused_slider_responds_to_keyboard_steps")
            && tests
                .contains("fn slider_edit_batch_is_copyable_bounded_and_projects_lifecycle_values"),
        "slider behavior tests should live in slider/tests.rs"
    );
}

#[test]
fn slider_edit_batch_is_fixed_capacity_and_kept_out_of_the_prelude() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let batch = fs::read_to_string(manifest_dir.join("src/widgets/interaction/messages/range.rs"))
        .expect("range interaction messages should be readable");
    let prelude_widgets =
        fs::read_to_string(manifest_dir.join("src/prelude/widgets.rs")).expect("widgets prelude");
    let prelude_controls =
        fs::read_to_string(manifest_dir.join("src/prelude/application/controls.rs"))
            .expect("application controls prelude");

    assert!(
        batch.contains("pub struct SliderEditBatch")
            && batch.contains("events: [EditEvent<f32>; 3]")
            && batch.contains("pub fn events(&self) -> &[EditEvent<f32>]")
            && !batch.contains("Vec<")
            && !batch.contains("SmallVec")
            && !batch.contains("Mutex")
            && !batch.contains("channel"),
        "SliderEditBatch should remain a bounded copy-only typed payload"
    );
    assert!(
        !prelude_widgets.contains("SliderEditBatch")
            && !prelude_controls.contains("slider_edit_mapped"),
        "Slider lifecycle APIs should remain qualified rather than entering the prelude"
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
    let input = fs::read_to_string(manifest_dir.join("src/widgets/primitives/toggle/input.rs"))
        .expect("toggle primitive input should be readable");
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
    assert!(
        input.contains("InteractionProvenance::Pointer")
            && input.contains("sequence_range: None")
            && input.contains("InteractionProvenance::Keyboard")
            && input.contains("timestamp"),
        "toggle input should attach provenance only after accepted pointer and keyboard input"
    );
}

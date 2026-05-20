use super::*;

#[test]
fn text_input_state_keeps_models_selection_navigation_and_editing_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let model = fs::read_to_string(manifest_dir.join("src/widgets/primitives/text_input/model.rs"))
        .expect("text input model root should be readable");
    let selection = fs::read_to_string(
        manifest_dir.join("src/widgets/primitives/text_input/model/selection.rs"),
    )
    .expect("text input selection model should be readable");
    let navigation = fs::read_to_string(
        manifest_dir.join("src/widgets/primitives/text_input/model/navigation.rs"),
    )
    .expect("text input navigation model should be readable");
    let editing =
        fs::read_to_string(manifest_dir.join("src/widgets/primitives/text_input/model/editing.rs"))
            .expect("text input editing model should be readable");
    let editing_command = fs::read_to_string(
        manifest_dir.join("src/widgets/primitives/text_input/model/editing/command.rs"),
    )
    .expect("text input edit command model should be readable");
    let editing_mutation = fs::read_to_string(
        manifest_dir.join("src/widgets/primitives/text_input/model/editing/mutation.rs"),
    )
    .expect("text input edit mutation model should be readable");
    let tests = fs::read_to_string(manifest_dir.join("src/widgets/primitives/text_input/tests.rs"))
        .expect("text input behavior test root should be readable");
    let widget_tests =
        fs::read_to_string(manifest_dir.join("src/widgets/primitives/text_input/tests/widget.rs"))
            .expect("text input widget interaction tests should be readable");
    let state_tests =
        fs::read_to_string(manifest_dir.join("src/widgets/primitives/text_input/tests/state.rs"))
            .expect("text input state behavior tests should be readable");

    for required in ["mod editing;", "mod navigation;", "mod selection;"] {
        assert!(
            model.contains(required),
            "text input model root should delegate `{required}`"
        );
    }
    assert!(
        model.contains("pub struct TextInputProps")
            && model.contains("pub struct TextInputState")
            && model.contains("pub struct TextInputEditResult")
            && model.contains("pub fn from_value")
            && !model.contains("TextEditCommand")
            && !model.contains("WidgetKey"),
        "text input model root should keep public state definitions separate from command handling"
    );
    assert!(
        selection.contains("pub fn selected_text")
            && selection.contains("pub fn selection_range")
            && selection.contains("pub fn has_selection"),
        "text input selection queries should live in model/selection.rs"
    );
    assert!(
        navigation.contains("pub fn set_caret")
            && navigation.contains("fn move_left")
            && navigation.contains("fn move_right"),
        "text input caret movement should live in model/navigation.rs"
    );
    assert!(
        editing.contains("mod command;")
            && editing.contains("mod mutation;")
            && !editing.contains("pub fn apply_edit_command")
            && !editing.contains("pub fn insert_text"),
        "text input editing root should delegate command dispatch and mutation mechanics"
    );
    assert!(
        editing_command.contains("pub fn apply_edit_command")
            && editing_command.contains("pub fn apply_key")
            && editing_command.contains("TextEditCommand")
            && editing_command.contains("WidgetKey")
            && !editing_mutation.contains("TextEditCommand")
            && !editing_mutation.contains("WidgetKey"),
        "text input edit command handling should live in model/editing/command.rs"
    );
    assert!(
        editing_mutation.contains("pub fn insert_text")
            && editing_mutation.contains("pub fn replace_selection")
            && editing_mutation.contains("pub(crate) fn delete_selected_text")
            && editing_mutation.contains("byte_index_for_char")
            && !editing_command.contains("byte_index_for_char"),
        "text input mutation mechanics should live in model/editing/mutation.rs"
    );
    assert!(
        tests.contains("mod widget;")
            && tests.contains("mod state;")
            && !tests.contains("fn text_input_editing_emits_changed_and_submitted_messages")
            && !tests.contains("fn text_input_state_applies_backend_neutral_editing_commands"),
        "text input behavior test root should index focused widget and state groups instead of owning all cases"
    );
    assert!(
        widget_tests.contains("fn text_input_editing_emits_changed_and_submitted_messages")
            && widget_tests
                .contains("fn text_input_pointer_drag_extends_selection_including_caret_character")
            && state_tests.contains("fn text_input_state_applies_backend_neutral_editing_commands")
            && state_tests.contains("fn text_input_state_can_clear_or_delete_active_selection"),
        "text input behavior tests should stay grouped by widget interaction and state editing concerns"
    );
}

#[test]
fn retained_invalidation_primitives_stay_in_focused_modules() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/gui/invalidation.rs"))
        .expect("invalidation root should be readable");
    let tests = fs::read_to_string(manifest_dir.join("src/gui/invalidation/tests.rs"))
        .expect("invalidation behavior tests should be readable");
    let mask = fs::read_to_string(manifest_dir.join("src/gui/invalidation/mask.rs"))
        .expect("invalidation mask module should be readable");
    let retained_mask =
        fs::read_to_string(manifest_dir.join("src/gui/invalidation/retained_mask.rs"))
            .expect("retained mask module should be readable");
    let segment = fs::read_to_string(manifest_dir.join("src/gui/invalidation/segment.rs"))
        .expect("retained segment module should be readable");

    for required in [
        "mod mask;",
        "mod retained_mask;",
        "mod segment;",
        "#[path = \"invalidation/tests.rs\"]",
        "pub use mask::InvalidationMask;",
        "pub use retained_mask::RetainedSegmentMask;",
    ] {
        assert!(
            root.contains(required),
            "invalidation root should delegate `{required}`"
        );
    }
    assert!(
        root.contains("RetainedSegmentPlan")
            && root.contains("RetainedSegmentRevisions")
            && !root.contains("pub struct InvalidationMask")
            && !root.contains("pub struct RetainedSegmentMask")
            && !root.contains("pub struct RetainedSegmentPlan")
            && !root.contains("fn invalidation_mask_clips_to_valid_bits"),
        "invalidation root should re-export public primitives and delegate behavior tests without owning implementations"
    );
    assert!(
        tests.contains("fn invalidation_mask_clips_to_valid_bits")
            && tests.contains("fn retained_segment_plan_names_groups_and_bumps_revisions"),
        "invalidation behavior coverage should live in gui/invalidation/tests.rs"
    );
    assert!(
        mask.contains("pub struct InvalidationMask")
            && mask.contains("pub const fn from_bits")
            && mask.contains("pub fn insert"),
        "raw invalidation bit operations should live in invalidation/mask.rs"
    );
    assert!(
        retained_mask.contains("pub struct RetainedSegmentMask")
            && retained_mask.contains("pub const fn requires_static_rebuild")
            && retained_mask.contains("pub const fn requires_overlay_rebuild"),
        "typed retained segment masks should live in invalidation/retained_mask.rs"
    );
    assert!(
        segment.contains("pub struct RetainedSegmentPlan")
            && segment.contains("pub struct RetainedSegmentRevisions")
            && segment.contains("pub enum RetainedSegmentKind")
            && segment.contains("pub fn bump_revisions"),
        "retained segment metadata, plans, and revisions should live in invalidation/segment.rs"
    );
}

#[test]
fn retained_cache_support_keeps_fingerprint_storage_and_tests_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fingerprint = fs::read_to_string(manifest_dir.join("src/gui/fingerprint.rs"))
        .expect("stable fingerprint source should be readable");
    let fingerprint_tests = fs::read_to_string(manifest_dir.join("src/gui/fingerprint/tests.rs"))
        .expect("stable fingerprint tests should be readable");
    let retained = fs::read_to_string(manifest_dir.join("src/gui/retained.rs"))
        .expect("retained storage source should be readable");
    let retained_tests = fs::read_to_string(manifest_dir.join("src/gui/retained/tests.rs"))
        .expect("retained storage tests should be readable");

    assert!(
        fingerprint.contains("pub struct StableFingerprint")
            && fingerprint.contains("pub fn mix_rgba8")
            && fingerprint.contains("#[path = \"fingerprint/tests.rs\"]")
            && !fingerprint.contains("fn fingerprints_are_stable_for_identical_inputs"),
        "stable fingerprint mixing should live in gui/fingerprint.rs while behavior tests stay delegated"
    );
    assert!(
        fingerprint_tests.contains("fn fingerprints_are_stable_for_identical_inputs")
            && fingerprint_tests.contains("fn color_channels_affect_fingerprint"),
        "fingerprint behavior coverage should live in gui/fingerprint/tests.rs"
    );
    assert!(
        retained.contains("pub struct RetainedVec")
            && retained.contains("pub fn make_mut")
            && retained.contains("#[path = \"retained/tests.rs\"]")
            && !retained.contains("fn retained_vec_clones_share_storage_until_mutation"),
        "retained vector storage should live in gui/retained.rs while behavior tests stay delegated"
    );
    assert!(
        retained_tests.contains("fn retained_vec_clones_share_storage_until_mutation"),
        "retained storage behavior coverage should live in gui/retained/tests.rs"
    );
}

#[test]
fn text_input_primitive_keeps_surface_builders_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/widgets/primitives/text_input.rs"))
        .expect("text-input primitive root should be readable");
    let builders =
        fs::read_to_string(manifest_dir.join("src/widgets/primitives/text_input/builders.rs"))
            .expect("text-input primitive builders should be readable");

    assert!(
        root.contains("mod builders;")
            && root.contains("pub struct TextInputWidget")
            && root.contains("impl Widget for TextInputWidget")
            && !root.contains("impl<Message> SurfaceNode<Message>")
            && !root.contains("impl<Message> WidgetMessageMapper<Message>"),
        "text-input primitive root should own widget behavior and delegate runtime builders"
    );
    assert!(
        builders.contains("impl<Message> SurfaceNode<Message>")
            && builders.contains("pub fn text_input(")
            && builders.contains("pub fn text_input_mapped(")
            && builders.contains("impl<Message> WidgetMessageMapper<Message>"),
        "text-input runtime builder helpers should live in text_input/builders.rs"
    );
}

#[test]
fn input_key_identity_and_keypress_state_stay_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let input = fs::read_to_string(manifest_dir.join("src/gui/input.rs"))
        .expect("input module should be readable");
    let key = fs::read_to_string(manifest_dir.join("src/gui/input/key.rs"))
        .expect("input key module should be readable");
    let press = fs::read_to_string(manifest_dir.join("src/gui/input/key/press.rs"))
        .expect("input keypress module should be readable");
    let press_tests = fs::read_to_string(manifest_dir.join("src/gui/input/key/press/tests.rs"))
        .expect("input keypress behavior tests should be readable");
    let pointer = fs::read_to_string(manifest_dir.join("src/gui/input/pointer.rs"))
        .expect("input pointer module should be readable");
    let pointer_tests = fs::read_to_string(manifest_dir.join("src/gui/input/pointer/tests.rs"))
        .expect("input pointer behavior tests should be readable");

    assert!(
        input.contains("pub use key::{KeyCode, KeyPress};")
            && input.contains("pub use pointer::logical_point_to_u16_coords;")
            && key.contains("mod press;")
            && key.contains("pub use press::KeyPress;"),
        "input facade should preserve key and pointer exports through focused child modules"
    );
    assert!(
        key.contains("pub enum KeyCode")
            && !key.contains("pub struct KeyPress")
            && press.contains("pub struct KeyPress")
            && press.contains("pub const fn with_command")
            && press.contains("#[path = \"press/tests.rs\"]")
            && !press.contains("fn keypress_constructors_preserve_modifier_state"),
        "key identity should stay in key.rs while modifier-bearing keypress state lives in key/press.rs with behavior tests delegated"
    );
    assert!(
        press_tests.contains("fn keypress_constructors_preserve_modifier_state"),
        "keypress behavior coverage should live in input/key/press/tests.rs"
    );
    assert!(
        pointer.contains("pub fn logical_point_to_u16_coords")
            && pointer.contains("#[path = \"pointer/tests.rs\"]")
            && !pointer.contains("fn logical_point_to_u16_coords_clamps_and_rounds"),
        "pointer coordinate conversion should live in input/pointer.rs with behavior tests delegated"
    );
    assert!(
        pointer_tests.contains("fn logical_point_to_u16_coords_clamps_and_rounds"),
        "pointer behavior coverage should live in input/pointer/tests.rs"
    );
}

#[test]
fn shortcut_primitives_stay_in_resolution_gesture_and_layer_modules() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/gui/shortcuts.rs"))
        .expect("shortcut root should be readable");
    let tests = fs::read_to_string(manifest_dir.join("src/gui/shortcuts/tests.rs"))
        .expect("shortcut behavior tests should be readable");
    let resolution = fs::read_to_string(manifest_dir.join("src/gui/shortcuts/resolution.rs"))
        .expect("shortcut resolution module should be readable");
    let gesture = fs::read_to_string(manifest_dir.join("src/gui/shortcuts/gesture.rs"))
        .expect("shortcut gesture module should be readable");
    let layer = fs::read_to_string(manifest_dir.join("src/gui/shortcuts/layer.rs"))
        .expect("shortcut layer module should be readable");

    for required in [
        "mod gesture;",
        "mod layer;",
        "mod resolution;",
        "#[path = \"shortcuts/tests.rs\"]",
        "pub use gesture::{ShortcutGesture, ShortcutModifier};",
        "pub use layer::{ShortcutBinding, ShortcutLayer};",
        "pub use resolution::ShortcutResolution;",
    ] {
        assert!(
            root.contains(required),
            "shortcut root should delegate `{required}`"
        );
    }
    assert!(
        !root.contains("pub struct ShortcutResolution")
            && !root.contains("pub struct ShortcutLayer")
            && !root.contains("pub struct ShortcutGesture")
            && !root.contains("fn shortcut_layer_resolves_actions_and_modal_misses"),
        "shortcut root should re-export public primitives and delegate behavior tests instead of owning implementations"
    );
    assert!(
        tests.contains("fn shortcut_resolution_unhandled_has_no_action_or_chord")
            && tests.contains("fn shortcut_layer_resolves_actions_and_modal_misses"),
        "shortcut behavior coverage should live in gui/shortcuts/tests.rs"
    );
    assert!(
        resolution.contains("pub struct ShortcutResolution")
            && resolution.contains("pub fn unhandled")
            && resolution.contains("pub fn pending_chord"),
        "shortcut result constructors should live in shortcuts/resolution.rs"
    );
    assert!(
        gesture.contains("pub enum ShortcutModifier")
            && gesture.contains("pub struct ShortcutGesture")
            && gesture.contains("impl From<KeyPress> for ShortcutGesture"),
        "shortcut modifier and key matching should live in shortcuts/gesture.rs"
    );
    assert!(
        layer.contains("pub struct ShortcutBinding")
            && layer.contains("pub struct ShortcutLayer")
            && layer.contains("pub fn resolve_or_else"),
        "shortcut binding collections and modal resolution should live in shortcuts/layer.rs"
    );
}

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

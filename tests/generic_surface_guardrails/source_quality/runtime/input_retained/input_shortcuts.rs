use super::*;

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

use super::*;

#[test]
fn native_text_input_rendering_keeps_utf8_clamping_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let module = fs::read_to_string(manifest_dir.join("src/gui_runtime/native_vello/text_edit.rs"))
        .expect("native text edit module should be readable");
    let state =
        fs::read_to_string(manifest_dir.join("src/gui_runtime/native_vello/text_edit/state.rs"))
            .expect("native text edit state should be readable");
    let boundary =
        fs::read_to_string(manifest_dir.join("src/gui_runtime/native_vello/text_edit/boundary.rs"))
            .expect("native text edit boundary helpers should be readable");

    assert!(
        module.contains("mod boundary;") && state.contains("use super::boundary::"),
        "native text-input rendering state should consume UTF-8 boundary policy from a focused module"
    );
    assert!(
        !state.contains("fn clamp_to_char_boundary")
            && boundary.contains("fn clamp_to_char_boundary")
            && !boundary.contains("fn previous_char_boundary")
            && !boundary.contains("fn next_char_boundary"),
        "native text-input rendering should keep only the UTF-8 boundary policy it uses"
    );
}

#[test]
fn native_text_field_layout_keeps_cursor_stop_windowing_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let layout =
        fs::read_to_string(manifest_dir.join("src/gui_runtime/native_vello/text_edit/layout.rs"))
            .expect("native text edit layout module should be readable");
    let cursor_stops = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/text_edit/layout/cursor_stops.rs"),
    )
    .expect("native text edit cursor-stop helpers should be readable");

    assert!(
        layout.contains("mod cursor_stops;")
            && layout.contains("use cursor_stops::{")
            && layout
                .contains("pub(in crate::gui_runtime::native_vello) struct TextFieldLayoutState")
            && layout
                .contains("pub(in crate::gui_runtime::native_vello) fn build_text_field_layout"),
        "native text-field layout root should own the layout state and delegate cursor-stop windowing"
    );
    assert!(
        !layout.contains("fn finite_stop_x")
            && !layout.contains("fn stop_local_x")
            && !layout.contains("fn visible_end_stop_index")
            && cursor_stops.contains("fn cursor_stop_x")
            && cursor_stops.contains("fn visible_end_stop_index")
            && cursor_stops.contains("fn build_visible_cursor_stops")
            && cursor_stops.contains("fn finite_stop_x")
            && cursor_stops.contains("fn stop_local_x"),
        "cursor-stop lookup, sanitization, and visible-window helpers should live in text_edit/layout/cursor_stops.rs"
    );
}

use super::*;

#[test]
fn application_view_lowering_keeps_container_defaults_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let module = fs::read_to_string(manifest_dir.join("src/application/view_node.rs"))
        .expect("application view node module should be readable");
    let lowering = fs::read_to_string(manifest_dir.join("src/application/view_node/lowering.rs"))
        .expect("application view lowering should be readable");
    let defaults =
        fs::read_to_string(manifest_dir.join("src/application/view_node/lowering_defaults.rs"))
            .expect("application view lowering defaults should be readable");
    let defaults_tests = fs::read_to_string(
        manifest_dir.join("src/application/view_node/lowering_defaults/tests.rs"),
    )
    .expect("application view lowering default tests should be readable");

    assert!(
        module.contains("mod lowering_defaults;")
            && lowering.contains("ViewNodeContainerDefaults::new("),
        "view lowering should consume container defaults from a focused helper"
    );
    assert!(
        !lowering.contains("DEFAULT_STYLED_CONTAINER_PADDING")
            && defaults.contains("DEFAULT_STYLED_CONTAINER_PADDING")
            && defaults.contains("fn default_container_padding")
            && defaults.contains("fn base_policy")
            && defaults.contains("#[path = \"lowering_defaults/tests.rs\"]")
            && !defaults.contains("fn styled_container_defaults_to_panel_padding"),
        "declarative container default policy should stay outside the main view lowering match"
    );
    assert!(
        defaults_tests.contains("fn styled_container_defaults_to_panel_padding")
            && defaults_tests
                .contains("fn explicit_container_defaults_override_style_padding_and_alignment"),
        "view lowering default behavior tests should live in lowering_defaults/tests.rs"
    );
}

#[test]
fn application_view_identity_keeps_reserved_id_tests_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let identity = fs::read_to_string(manifest_dir.join("src/application/view_node/identity.rs"))
        .expect("application view node identity module should be readable");
    let tests =
        fs::read_to_string(manifest_dir.join("src/application/view_node/identity/tests.rs"))
            .expect("application view node identity tests should be readable");

    assert!(
        identity.contains("pub(super) fn collect_reserved_ids")
            && identity.contains("fn reserve_child_identity_capacity")
            && identity.contains("fn reserved_identity_capacity_hint")
            && identity.contains("#[path = \"identity/tests.rs\"]")
            && !identity.contains("fn reserved_id_collection_presizes_for_large_child_groups"),
        "view-node identity collection should live in view_node/identity.rs while behavior tests stay delegated"
    );
    assert!(
        tests.contains("fn reserved_id_collection_presizes_for_large_child_groups")
            && tests.contains("fn reserved_id_collection_includes_grid_child_identities")
            && tests.contains("fn reserved_id_collection_presizes_wrapped_runtime_identities"),
        "view-node identity behavior coverage should live in view_node/identity/tests.rs"
    );
}

#[test]
fn application_list_builders_keep_virtualization_tests_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let lists = fs::read_to_string(manifest_dir.join("src/application/layout_builders/lists.rs"))
        .expect("application list builders should be readable");
    let tests =
        fs::read_to_string(manifest_dir.join("src/application/layout_builders/lists/tests.rs"))
            .expect("application list builder tests should be readable");

    assert!(
        lists.contains("pub fn virtual_list<Message, Item>")
            && lists.contains("pub fn virtual_list_window<Message: 'static>")
            && lists.contains("fn apply_list_row_chrome<Message>")
            && lists.contains("#[path = \"lists/tests.rs\"]")
            && !lists.contains("fn virtual_list_window_projects_only_materialized_range"),
        "application list builders should own builder logic while virtualization behavior tests stay delegated"
    );
    assert!(
        tests.contains("fn virtual_list_uses_packed_rows")
            && tests.contains("fn virtual_list_window_projects_only_materialized_range")
            && tests.contains("fn count_layout_nodes"),
        "application list builder behavior coverage should live in layout_builders/lists/tests.rs"
    );
}

#[test]
fn application_dropdown_builder_keeps_menu_overlay_and_tests_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root =
        fs::read_to_string(manifest_dir.join("src/application/control_builders/dropdown.rs"))
            .expect("application dropdown builder root should be readable");
    let model =
        fs::read_to_string(manifest_dir.join("src/application/control_builders/dropdown/model.rs"))
            .expect("application dropdown model should be readable");
    let menu =
        fs::read_to_string(manifest_dir.join("src/application/control_builders/dropdown/menu.rs"))
            .expect("application dropdown menu module should be readable");
    let tests =
        fs::read_to_string(manifest_dir.join("src/application/control_builders/dropdown/tests.rs"))
            .expect("application dropdown tests should be readable");

    assert!(
        root.contains("mod menu;")
            && root.contains("mod model;")
            && root
                .contains("pub use model::{DropdownOption, DropdownOptionParts, DropdownParts};")
            && root.contains("#[path = \"dropdown/tests.rs\"]")
            && root.contains("pub struct DropdownBuilder<Message>")
            && root.contains("pub fn option_from_parts")
            && root.contains("pub fn dropdown_from_parts")
            && !root.contains("pub struct DropdownOptionParts<Message>")
            && !root.contains("fn dropdown_option_button")
            && !root.contains("fn dropdown_builder_accepts_toggle_and_options"),
        "dropdown builder root should own builder entry points while delegating public DTOs, menu overlay, and tests"
    );
    assert!(
        model.contains("pub struct DropdownOption<Message>")
            && model.contains("pub struct DropdownOptionParts<Message>")
            && model.contains("pub struct DropdownParts<Message>")
            && model.contains("pub fn from_parts(parts: DropdownOptionParts<Message>) -> Self"),
        "dropdown public DTOs should live in the focused model module"
    );
    assert!(
        menu.contains("pub fn dropdown_menu<")
            && menu.contains("pub fn dropdown_menu_overlay<")
            && menu.contains("fn dropdown_option_button"),
        "dropdown menu and overlay assembly should live in the focused menu module"
    );
    assert!(
        tests.contains("fn dropdown_height_tracks_expanded_options")
            && tests.contains("fn dropdown_builder_accepts_toggle_and_options")
            && tests
                .contains("fn dropdown_option_compatibility_constructor_delegates_to_named_parts"),
        "dropdown builder and named-option behavior tests should live in dropdown/tests.rs"
    );
}

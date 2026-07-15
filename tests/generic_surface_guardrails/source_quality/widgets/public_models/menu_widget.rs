use super::*;

#[test]
fn application_menu_keeps_model_projection_actions_and_tests_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let menu_dir = manifest_dir.join("src/application/menu");
    let root = fs::read_to_string(manifest_dir.join("src/application/menu.rs"))
        .expect("application menu root should be readable");
    let actions = fs::read_to_string(menu_dir.join("actions.rs"))
        .expect("application menu actions should be readable");
    let projection = fs::read_to_string(menu_dir.join("projection.rs"))
        .expect("application menu projection should be readable");

    assert!(
        root.contains("mod actions;")
            && root.contains("mod model;")
            && root.contains("mod overlays;")
            && root.contains("mod projection;")
            && actions.contains("fn message_menu_from_parts")
            && actions.contains("menu_command_row(index, command, command_text)")
            && projection.contains("pub(super) fn menu_command_row")
            && projection.contains("struct MenuCommandTextColumns")
            && menu_dir.join("tests/actions.rs").is_file()
            && menu_dir.join("tests/overlays.rs").is_file()
            && menu_dir.join("tests/projection.rs").is_file()
            && !menu_dir.join("tests.rs").exists(),
        "application menus should keep model, action mapping, row projection, overlays, and behavior tests in focused owner modules"
    );
}

#[test]
fn application_menus_expose_one_fluent_context_menu_surface() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source_path = manifest_dir.join("src/application/menu.rs");
    let source = fs::read_to_string(&source_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));
    let model = fs::read_to_string(manifest_dir.join("src/application/menu/model.rs"))
        .expect("application menu model module should be readable");
    let actions = fs::read_to_string(manifest_dir.join("src/application/menu/actions.rs"))
        .expect("application menu action module should be readable");
    let overlays = fs::read_to_string(manifest_dir.join("src/application/menu/overlays.rs"))
        .expect("application menu overlay module should be readable");
    let facade = fs::read_to_string(manifest_dir.join("src/application/facade/menus.rs"))
        .expect("application menu facade should be readable");
    let prelude = public_prelude_source(&manifest_dir);

    assert!(model.contains("pub struct MenuCommandParts"));
    assert!(
        model.contains("pub fn from_parts(parts: MenuCommandParts<Message>) -> Self"),
        "menu command named parts remain a useful semantic configuration contract"
    );
    assert!(
        source.contains("mod actions;")
            && source.contains("mod model;")
            && source.contains("mod overlays;")
            && source.contains("mod projection;")
            && source.contains("ContextMenuBuilder")
            && source.contains("AnchoredContextMenuBuilder")
            && !source.contains("MessageContextMenuOverlayParts")
            && !source.contains("DismissibleContextMenuParts")
            && model.contains("Self::from_parts(MenuCommandParts {")
            && actions.contains("pub(super) fn message_menu_from_parts")
            && overlays.contains("pub fn context_menu<Message>")
            && overlays.contains("pub fn anchor(self, anchor: Point)")
            && overlays.contains("pub fn width_policy")
            && overlays.contains("pub fn dismiss_on")
            && overlays.contains("pub fn view(self) -> ViewNode<Message>")
            && facade.contains("ContextMenuBuilder")
            && facade.contains("context_menu")
            && facade.contains("MenuCommandParts")
            && !facade.contains("message_context_menu_overlay")
            && !facade.contains("dismissible_context_menu")
            && !facade.contains("anchored_message_menu_overlay")
            && prelude.contains("ContextMenuBuilder")
            && prelude.contains("context_menu")
            && !prelude.contains("message_context_menu_overlay")
            && !prelude.contains("dismissible_context_menu")
            && !prelude.contains("anchored_message_menu_overlay"),
        "context menus should expose one fluent builder without suffix or weak parts exports"
    );
}

#[test]
fn application_widget_views_use_named_parts_for_custom_widget_mapping() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source = fs::read_to_string(manifest_dir.join("src/application/widget_view.rs"))
        .expect("application widget view module should be readable");
    let view_facade = fs::read_to_string(manifest_dir.join("src/application/facade/view.rs"))
        .expect("application view facade should be readable");
    let surface_facade =
        fs::read_to_string(manifest_dir.join("src/application/facade/surfaces.rs"))
            .expect("application surface facade should be readable");
    let prelude = public_prelude_source(&manifest_dir);

    for (parts, from_parts, wrapper) in [
        (
            "pub struct MappedWidgetParts",
            "pub fn from_parts(parts: MappedWidgetParts<W, Message>) -> Self",
            "Self::from_parts(MappedWidgetParts { widget, messages })",
        ),
        (
            "pub struct DynamicWidgetParts",
            "pub fn from_parts(parts: DynamicWidgetParts<Message>) -> Self",
            "Self::from_parts(DynamicWidgetParts {",
        ),
    ] {
        assert!(
            source.contains(parts) && source.contains(from_parts) && source.contains(wrapper),
            "application widget views should expose named parts and compatibility wrappers for {parts}"
        );
    }
    assert!(
        view_facade.contains("MappedWidgetParts")
            && surface_facade.contains("DynamicWidgetParts")
            && !prelude.contains("MappedWidgetParts")
            && !prelude.contains("DynamicWidgetParts"),
        "custom widget view parts should remain available from application ownership without entering the common prelude"
    );
}

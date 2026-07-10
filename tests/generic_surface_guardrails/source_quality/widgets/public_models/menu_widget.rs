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
            && actions.contains("pub fn message_menu_from_parts")
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
fn application_menus_use_named_parts_for_context_overlay_fields() {
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

    for parts in [
        "pub struct MenuCommandParts",
        "pub struct MessageMenuParts",
        "pub struct MessageContextMenuOverlayParts",
    ] {
        assert!(
            model.contains(parts),
            "application menu APIs should expose named parts for {parts}"
        );
    }
    for constructor in [
        (
            "pub fn from_parts(parts: MenuCommandParts<Message>) -> Self",
            &model,
        ),
        (
            "pub fn message_menu_from_parts<Message>(parts: MessageMenuParts<Message>)",
            &actions,
        ),
        (
            "pub fn message_context_menu_overlay_from_parts<Message>",
            &overlays,
        ),
    ] {
        assert!(
            constructor.1.contains(constructor.0),
            "application menu APIs should expose named-parts constructor {}",
            constructor.0
        );
    }
    assert!(
        source.contains("mod actions;")
            && source.contains("mod model;")
            && source.contains("mod overlays;")
            && source.contains("mod projection;")
            && source.contains("MessageContextMenuOverlayParts,")
            && !source.contains("pub struct MenuCommandParts<Message>")
            && !source.contains("pub struct MessageContextMenuOverlayParts<Message>")
            && model.contains("Self::from_parts(MenuCommandParts {")
            && overlays.contains("anchored_message_menu_overlay(anchor, size, title, commands)")
            && facade.contains("MessageContextMenuOverlayParts")
            && facade.contains("MenuCommandParts")
            && !facade.contains("MenuItemParts")
            && !facade.contains(" ContextMenuOverlayParts")
            && !facade.contains("{ContextMenuOverlayParts")
            && prelude.contains("MessageContextMenuOverlayParts")
            && prelude.contains("message_context_menu_overlay_from_parts")
            && !prelude.contains("MenuItemParts")
            && !prelude.contains(" context_menu_overlay_from_parts")
            && !prelude.contains("{context_menu_overlay_from_parts"),
        "menu model types should live outside the builder root while normal public exports stay message-first"
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
            && prelude.contains("MappedWidgetParts")
            && prelude.contains("DynamicWidgetParts"),
        "custom widget view parts should stay publicly exported through application facades and prelude"
    );
}

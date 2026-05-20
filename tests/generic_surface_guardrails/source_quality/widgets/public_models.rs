use super::*;

#[test]
fn property_panel_rows_use_named_parts_for_public_inspector_fields() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source_path = manifest_dir.join("src/application/property_panel.rs");
    let source = fs::read_to_string(&source_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));
    let application = fs::read_to_string(manifest_dir.join("src/application.rs"))
        .expect("application module should be readable");

    assert!(
        source.contains("pub struct PropertyRowParts")
            && source.contains("pub fn from_parts(parts: PropertyRowParts) -> Self"),
        "property panel rows should expose named parts for id, label, and value construction"
    );
    assert!(
        source.contains("Self::from_parts(PropertyRowParts {")
            && application.contains("PropertyRowParts"),
        "property row compatibility constructor and public exports should keep the named-parts path available"
    );
}

#[test]
fn tree_list_items_use_named_parts_for_public_navigation_fields() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source_path = manifest_dir.join("src/application/tree_list.rs");
    let source = fs::read_to_string(&source_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));
    let row = fs::read_to_string(manifest_dir.join("src/application/tree_list/row.rs"))
        .expect("tree-list row view module should be readable");
    let application = fs::read_to_string(manifest_dir.join("src/application.rs"))
        .expect("application module should be readable");

    assert!(
        source.contains("pub struct TreeListItemParts")
            && source.contains("pub fn from_parts(parts: TreeListItemParts) -> Self"),
        "tree-list items should expose named parts for id, depth, and label construction"
    );
    assert!(
        source.contains("Self::from_parts(TreeListItemParts {")
            && source.contains("mod row;")
            && application.contains("TreeListItemParts"),
        "tree-list compatibility constructor and public exports should keep the named-parts path available"
    );
    assert!(
        !source.contains("fn tree_list_row")
            && row.contains("fn tree_list_row")
            && row.contains("drag_handle()")
            && row.contains("tree-list-toggle-")
            && row.contains("WidgetProminence::Subtle"),
        "tree-list private row assembly should live in application/tree_list/row.rs"
    );
}

#[test]
fn application_menus_use_named_parts_for_context_overlay_fields() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source_path = manifest_dir.join("src/application/menu.rs");
    let source = fs::read_to_string(&source_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));
    let application = fs::read_to_string(manifest_dir.join("src/application.rs"))
        .expect("application module should be readable");
    let lib = fs::read_to_string(manifest_dir.join("src/lib.rs"))
        .expect("library module should be readable");

    for (parts, constructor) in [
        (
            "pub struct MenuItemParts",
            "pub fn from_parts(parts: MenuItemParts<State>) -> Self",
        ),
        (
            "pub struct MenuParts",
            "pub fn menu_from_parts<State: 'static>(parts: MenuParts<State>) -> StateView<State>",
        ),
        (
            "pub struct ContextMenuOverlayParts",
            "pub fn context_menu_overlay_from_parts<State: 'static>",
        ),
    ] {
        assert!(
            source.contains(parts) && source.contains(constructor),
            "application menu APIs should expose named parts for {parts}"
        );
    }
    assert!(
        source.contains("Self::from_parts(MenuItemParts {")
            && source.contains("context_menu_overlay_from_parts(ContextMenuOverlayParts {")
            && application.contains("ContextMenuOverlayParts")
            && application.contains("MenuItemParts")
            && application.contains("MenuParts")
            && lib.contains("ContextMenuOverlayParts")
            && lib.contains("context_menu_overlay_from_parts"),
        "menu compatibility helpers and public exports should keep the named-parts path available"
    );
}

#[test]
fn application_widget_views_use_named_parts_for_custom_widget_mapping() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source = fs::read_to_string(manifest_dir.join("src/application/widget_view.rs"))
        .expect("application widget view module should be readable");
    let application = fs::read_to_string(manifest_dir.join("src/application.rs"))
        .expect("application module should be readable");
    let lib = fs::read_to_string(manifest_dir.join("src/lib.rs"))
        .expect("library module should be readable");

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
        application.contains("MappedWidgetParts")
            && application.contains("DynamicWidgetParts")
            && lib.contains("MappedWidgetParts")
            && lib.contains("DynamicWidgetParts"),
        "custom widget view parts should stay publicly exported through application and prelude"
    );
}

#[test]
fn details_list_rows_use_named_parts_for_public_row_fields() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source_path = manifest_dir.join("src/application/details_list/model.rs");
    let source = fs::read_to_string(&source_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));
    let application = fs::read_to_string(manifest_dir.join("src/application.rs"))
        .expect("application module should be readable");
    let module = fs::read_to_string(manifest_dir.join("src/application/details_list.rs"))
        .expect("details-list module should be readable");

    assert!(
        source.contains("pub struct DetailsRowParts")
            && source.contains("pub fn from_parts(parts: DetailsRowParts) -> Self"),
        "details-list rows should expose named parts for id and cell construction"
    );
    assert!(
        source.contains("Self::from_parts(DetailsRowParts {")
            && module.contains("DetailsRowParts")
            && application.contains("DetailsRowParts"),
        "details-row compatibility constructor and public exports should keep the named-parts path available"
    );
}

#[test]
fn details_list_columns_use_named_parts_for_public_column_fields() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source_path = manifest_dir.join("src/application/details_list/model.rs");
    let source = fs::read_to_string(&source_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));
    let application = fs::read_to_string(manifest_dir.join("src/application.rs"))
        .expect("application module should be readable");
    let module = fs::read_to_string(manifest_dir.join("src/application/details_list.rs"))
        .expect("details-list module should be readable");

    assert!(
        source.contains("pub struct DetailsColumnParts")
            && source.contains("pub fn from_parts(parts: DetailsColumnParts) -> Self"),
        "details-list columns should expose named parts for id, label, and width construction"
    );
    assert!(
        source.contains("Self::from_parts(DetailsColumnParts {")
            && module.contains("DetailsColumnParts")
            && application.contains("DetailsColumnParts"),
        "details-column compatibility constructors and public exports should keep the named-parts path available"
    );
}

#[test]
fn details_list_sort_uses_named_parts_for_public_sort_fields() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source_path = manifest_dir.join("src/application/details_list/model.rs");
    let source = fs::read_to_string(&source_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));
    let application = fs::read_to_string(manifest_dir.join("src/application.rs"))
        .expect("application module should be readable");
    let module = fs::read_to_string(manifest_dir.join("src/application/details_list.rs"))
        .expect("details-list module should be readable");

    assert!(
        source.contains("pub struct DetailsSortParts")
            && source.contains("pub fn from_parts(parts: DetailsSortParts) -> Self"),
        "details-list sort state should expose named parts for column id and direction construction"
    );
    assert!(
        source.contains("Self::from_parts(DetailsSortParts {")
            && module.contains("DetailsSortParts")
            && application.contains("DetailsSortParts"),
        "details-sort compatibility constructor and public exports should keep the named-parts path available"
    );
}

#[test]
fn confirm_dialogs_use_named_parts_for_public_prompt_fields() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source_path = manifest_dir.join("src/runtime/platform.rs");
    let source = fs::read_to_string(&source_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));
    let runtime = fs::read_to_string(manifest_dir.join("src/runtime/mod.rs"))
        .expect("runtime module should be readable");
    let lib = fs::read_to_string(manifest_dir.join("src/lib.rs"))
        .expect("library module should be readable");

    assert!(
        source.contains("pub struct ConfirmDialogParts")
            && source.contains("pub fn from_parts(parts: ConfirmDialogParts) -> Self"),
        "confirmation dialogs should expose named parts for title, message, level, and buttons"
    );
    assert!(
        source.contains("Self::from_parts(ConfirmDialogParts {")
            && runtime.contains("ConfirmDialogParts")
            && lib.contains("ConfirmDialogParts"),
        "confirmation dialog compatibility constructor and public exports should keep the named-parts path available"
    );
}

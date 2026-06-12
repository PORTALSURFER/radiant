use super::*;

#[test]
fn property_panel_rows_use_named_parts_for_public_inspector_fields() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source_path = manifest_dir.join("src/application/property_panel.rs");
    let source = fs::read_to_string(&source_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));
    let facade = fs::read_to_string(manifest_dir.join("src/application/facade/panels.rs"))
        .expect("application panel facade should be readable");

    assert!(
        source.contains("pub struct PropertyRowParts")
            && source.contains("pub fn from_parts(parts: PropertyRowParts) -> Self"),
        "property panel rows should expose named parts for id, label, and value construction"
    );
    assert!(
        source.contains("Self::from_parts(PropertyRowParts {")
            && facade.contains("PropertyRowParts"),
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
    let facade = fs::read_to_string(manifest_dir.join("src/application/facade/details.rs"))
        .expect("application details facade should be readable");

    assert!(
        source.contains("pub struct TreeListItemParts")
            && source.contains("pub fn from_parts(parts: TreeListItemParts) -> Self"),
        "tree-list items should expose named parts for id, depth, and label construction"
    );
    assert!(
        source.contains("Self::from_parts(TreeListItemParts {")
            && source.contains("mod row;")
            && facade.contains("TreeListItemParts"),
        "tree-list compatibility constructor and public exports should keep the named-parts path available"
    );
    assert!(
        !source.contains("fn message_tree_list_row")
            && row.contains("fn message_tree_list_row")
            && row.contains("drag_handle()")
            && row.contains("tree-list-toggle-")
            && row.contains(".message(toggle_message(toggle_id))")
            && row.contains("WidgetProminence::Subtle"),
        "tree-list private message-first row assembly should live in application/tree_list/row.rs"
    );
}

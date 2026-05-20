use super::*;

#[test]
fn editable_tree_rows_use_named_parts_instead_of_boolean_constructor_lists() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source_path = manifest_dir.join("src/gui/list/editable/row.rs");
    let source = fs::read_to_string(&source_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));

    assert!(
        source.contains("pub struct EditableTreeRowParts")
            && source.contains("pub fn from_parts(parts: EditableTreeRowParts) -> Self"),
        "editable tree rows should expose a named parts object for readable public construction"
    );
    assert!(
        !source.contains("too_many_arguments"),
        "editable tree rows should not hide long positional constructors behind clippy suppressions"
    );
}

#[test]
fn column_summaries_use_named_parts_for_title_and_count() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source_path = manifest_dir.join("src/gui/list/editable/column.rs");
    let source = fs::read_to_string(&source_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));
    let editable = fs::read_to_string(manifest_dir.join("src/gui/list/editable.rs"))
        .expect("editable list module should be readable");
    let list = fs::read_to_string(manifest_dir.join("src/gui/list.rs"))
        .expect("list module should be readable");
    let lib = fs::read_to_string(manifest_dir.join("src/lib.rs"))
        .expect("library module should be readable");

    assert!(
        source.contains("pub struct ColumnSummaryParts")
            && source.contains("pub fn from_parts(parts: ColumnSummaryParts) -> Self"),
        "column summaries should expose named parts for title and item count"
    );
    assert!(
        source.contains("Self::from_parts(ColumnSummaryParts {")
            && editable.contains("ColumnSummaryParts")
            && list.contains("ColumnSummaryParts")
            && lib.contains("ColumnSummaryParts"),
        "column summary compatibility constructor and public exports should keep the named-parts path available"
    );
}

#[test]
fn virtual_list_stack_metrics_use_named_parts_for_geometry() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source_path = manifest_dir.join("src/gui/list/virtual_list/geometry.rs");
    let virtual_list_path = manifest_dir.join("src/gui/list/virtual_list.rs");
    let list_path = manifest_dir.join("src/gui/list.rs");
    let source = fs::read_to_string(&source_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));
    let virtual_list = fs::read_to_string(&virtual_list_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", virtual_list_path.display()));
    let list = fs::read_to_string(&list_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", list_path.display()));
    let lib = fs::read_to_string(manifest_dir.join("src/lib.rs"))
        .expect("library module should be readable");

    assert!(
        source.contains("pub struct VirtualListStackMetricsParts")
            && source.contains("pub fn from_parts(parts: VirtualListStackMetricsParts) -> Self"),
        "virtual-list stack metrics should expose named parts for extent, gap, and viewport cap"
    );
    assert!(
        source.contains("Self::from_parts(VirtualListStackMetricsParts {")
            && virtual_list.contains("VirtualListStackMetricsParts")
            && list.contains("VirtualListStackMetricsParts")
            && lib.contains("VirtualListStackMetricsParts"),
        "virtual-list stack metric constructor and public exports should keep the named-parts path available"
    );
}

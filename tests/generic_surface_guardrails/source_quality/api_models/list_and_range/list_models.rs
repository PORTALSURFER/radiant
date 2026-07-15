use super::*;

#[test]
fn editable_tree_rows_use_named_parts_instead_of_boolean_constructor_lists() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source_path = manifest_dir.join("src/gui/list/editable/row.rs");
    let source = fs::read_to_string(&source_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));
    let draft = fs::read_to_string(manifest_dir.join("src/gui/list/editable/row/draft.rs"))
        .expect("editable tree draft input module should be readable");

    assert!(
        source.contains("pub struct EditableTreeRowParts")
            && source.contains("mod draft;")
            && source.contains(
                "pub use draft::{EditableTreeDraftInputParts, EditableTreeInputFocus, EditableTreeRowInput};"
            )
            && source.contains("pub fn from_parts(parts: EditableTreeRowParts) -> Self")
            && source.contains(
                "pub fn create_draft_from_parts(depth: usize, input: EditableTreeDraftInputParts) -> Self"
            )
            && source.contains(
                "pub fn rename_draft_from_parts(depth: usize, input: EditableTreeDraftInputParts) -> Self"
            ),
        "editable tree rows should expose named parts objects for readable public construction"
    );
    assert!(
        draft.contains("pub struct EditableTreeDraftInputParts")
            && draft.contains("pub enum EditableTreeInputFocus")
            && draft.contains("pub struct EditableTreeRowInput"),
        "editable tree draft input policy should stay in the focused draft module"
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
    let prelude = public_prelude_source(&manifest_dir);

    assert!(
        source.contains("pub struct ColumnSummaryParts")
            && source.contains("pub fn from_parts(parts: ColumnSummaryParts) -> Self"),
        "column summaries should expose named parts for title and item count"
    );
    assert!(
        source.contains("Self::from_parts(ColumnSummaryParts {")
            && editable.contains("ColumnSummaryParts")
            && list.contains("ColumnSummaryParts")
            && !prelude.contains("ColumnSummaryParts"),
        "column summary named parts should remain available from gui ownership without entering the common prelude"
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
    let prelude = public_prelude_source(&manifest_dir);

    assert!(
        source.contains("pub struct VirtualListStackMetricsParts")
            && source.contains("pub fn from_parts(parts: VirtualListStackMetricsParts) -> Self"),
        "virtual-list stack metrics should expose named parts for extent, gap, and viewport cap"
    );
    assert!(
        source.contains("Self::from_parts(VirtualListStackMetricsParts {")
            && virtual_list.contains("VirtualListStackMetricsParts")
            && list.contains("VirtualListStackMetricsParts")
            && !prelude.contains("VirtualListStackMetricsParts"),
        "virtual-list stack metric parts should remain available from gui ownership without entering the common prelude"
    );
}

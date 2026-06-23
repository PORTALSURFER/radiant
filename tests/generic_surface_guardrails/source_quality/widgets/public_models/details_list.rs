use super::*;

#[test]
fn details_list_rows_use_named_parts_for_public_row_fields() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source_path = manifest_dir.join("src/application/details_list/model/rows.rs");
    let source = fs::read_to_string(&source_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));
    let facade = fs::read_to_string(manifest_dir.join("src/application/facade/details.rs"))
        .expect("application details facade should be readable");
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
            && facade.contains("DetailsRowParts"),
        "details-row compatibility constructor and public exports should keep the named-parts path available"
    );
}

#[test]
fn details_list_columns_use_named_parts_for_public_column_fields() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source_path = manifest_dir.join("src/application/details_list/model/columns.rs");
    let source = fs::read_to_string(&source_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));
    let facade = fs::read_to_string(manifest_dir.join("src/application/facade/details.rs"))
        .expect("application details facade should be readable");
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
            && facade.contains("DetailsColumnParts"),
        "details-column compatibility constructors and public exports should keep the named-parts path available"
    );
}

#[test]
fn details_list_column_drag_state_stays_in_public_details_model() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source_path = manifest_dir.join("src/application/details_list/model.rs");
    let source = fs::read_to_string(&source_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));
    let drag = fs::read_to_string(manifest_dir.join("src/application/details_list/model/drag.rs"))
        .expect("details-list column drag model should be readable");
    let facade = fs::read_to_string(manifest_dir.join("src/application/facade/details.rs"))
        .expect("application details facade should be readable");
    let module = fs::read_to_string(manifest_dir.join("src/application/details_list.rs"))
        .expect("details-list module should be readable");

    assert!(
        source.contains("mod drag;")
            && source.contains("pub use drag::")
            && !source.contains("#[path =")
            && drag.contains("pub struct DetailsColumnResizeDrag")
            && drag.contains("pub struct DetailsColumnReorderDrag")
            && drag.contains("pub fn details_column_drag_content_left"),
        "details-list column drag state and geometry helpers should live in a focused public details model module"
    );
    assert!(
        module.contains("DetailsColumnResizeDrag")
            && module.contains("DetailsColumnReorderDrag")
            && facade.contains("DetailsColumnResizeDrag")
            && facade.contains("DetailsColumnReorderDrag"),
        "details-column drag helpers should be exported through application facades"
    );
}

#[test]
fn details_list_view_composition_stays_split_by_responsibility() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let view_facade = fs::read_to_string(manifest_dir.join("src/application/details_list/view.rs"))
        .expect("details-list view facade should be readable");
    let list = fs::read_to_string(manifest_dir.join("src/application/details_list/view/list.rs"))
        .expect("details-list composition module should be readable");
    let compact =
        fs::read_to_string(manifest_dir.join("src/application/details_list/view/compact.rs"))
            .expect("details-list compact geometry module should be readable");
    let header =
        fs::read_to_string(manifest_dir.join("src/application/details_list/view/header.rs"))
            .expect("details-list header interaction module should be readable");

    assert!(
        view_facade.contains("mod compact;")
            && view_facade.contains("mod header;")
            && view_facade.contains("mod list;")
            && view_facade.lines().count() <= 24,
        "details-list view facade should stay a small facade over focused child modules"
    );
    assert!(
        list.contains("pub fn message_selectable_sortable_details_list")
            && !list.contains("compact_resizable_details_header_cell"),
        "details-list composition should own full list construction without absorbing header-cell helpers"
    );
    assert!(
        compact.contains("pub struct CompactDetailsAnchoredCellParts")
            && compact.contains("pub fn compact_details_cell")
            && !compact.contains("DragHandleMessage"),
        "compact details-list geometry should stay separate from header interactions"
    );
    assert!(
        header.contains("pub struct CompactDetailsHeaderCellIds")
            && header.contains("pub fn compact_details_header_sort_drag_id")
            && header.contains("pub fn compact_details_header_resize_id")
            && header.contains("pub fn compact_resizable_details_header_cell_with_ids")
            && header.contains("DragHandleMessage"),
        "compact details-list header interactions should stay in a focused header module and derive child ids from parent cell identity"
    );
}

#[test]
fn details_list_sort_uses_named_parts_for_public_sort_fields() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source_path = manifest_dir.join("src/application/details_list/model/sort.rs");
    let source = fs::read_to_string(&source_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));
    let facade = fs::read_to_string(manifest_dir.join("src/application/facade/details.rs"))
        .expect("application details facade should be readable");
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
            && facade.contains("DetailsSortParts"),
        "details-sort compatibility constructor and public exports should keep the named-parts path available"
    );
}

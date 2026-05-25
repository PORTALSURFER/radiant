use super::*;

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

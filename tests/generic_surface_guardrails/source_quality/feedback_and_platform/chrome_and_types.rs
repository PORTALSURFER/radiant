use super::*;

#[test]
fn status_segments_use_named_parts_for_chrome_slots() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source_path = manifest_dir.join("src/gui/chrome.rs");
    let source = fs::read_to_string(&source_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));
    let tests = fs::read_to_string(manifest_dir.join("src/gui/chrome/tests.rs"))
        .expect("chrome behavior tests should be readable");
    let prelude = public_prelude_source(&manifest_dir);

    assert!(
        source.contains("pub struct StatusSegmentsParts")
            && source.contains("pub fn from_parts(parts: StatusSegmentsParts) -> Self")
            && source.contains("#[path = \"chrome/tests.rs\"]")
            && !source.contains("fn status_segments_default_to_empty_text"),
        "status segments should expose named parts for left, center, and right chrome slots while delegating behavior tests"
    );
    assert!(
        tests.contains("fn status_segments_default_to_empty_text")
            && tests.contains("fn content_view_chrome_groups_related_copy")
            && tests.contains("fn content_view_chrome_defaults_to_product_neutral_copy"),
        "chrome behavior coverage should live in gui/chrome/tests.rs"
    );
    assert!(
        source.contains("Self::from_parts(StatusSegmentsParts {")
            && prelude.contains("StatusSegmentsParts"),
        "status segment compatibility constructor and prelude export should keep the named-parts path available"
    );
    assert!(
        source.contains("pub struct ContentViewTabs")
            && source.contains("pub struct ContentViewSearchChrome")
            && source.contains("pub struct ContentViewActivityChrome")
            && source.contains("pub struct ContentViewSortChrome")
            && source.contains("pub struct ContentViewFooterChrome")
            && source.contains("pub struct ContentViewChrome")
            && prelude.contains("ContentViewTabs")
            && prelude.contains("ContentViewFooterChrome"),
        "content-view chrome should stay grouped by view concern and keep subgroup DTOs available from the prelude"
    );
}

#[test]
fn image_rgba_buffer_keeps_diagnostics_and_tests_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source = fs::read_to_string(manifest_dir.join("src/gui/types/image.rs"))
        .expect("image buffer source should be readable");
    let tests = fs::read_to_string(manifest_dir.join("src/gui/types/image/tests.rs"))
        .expect("image buffer behavior tests should be readable");
    let image_definition = source
        .split("pub struct ImageRgbaError")
        .next()
        .expect("image buffer definition should precede its diagnostic");

    assert!(
        source.contains("pub struct ImageRgba")
            && source.contains("pub struct ImageRgbaError")
            && source.contains("pub fn try_new")
            && source.contains("pub const fn width(&self)")
            && source.contains("pub const fn height(&self)")
            && source.contains("pub fn pixels(&self)")
            && !image_definition.contains("pub width: usize")
            && !image_definition.contains("pub height: usize")
            && !image_definition.contains("pub pixels: Arc<[u8]>")
            && source.contains("#[path = \"image/tests.rs\"]")
            && !source.contains("fn image_rgba_try_new_reports_length_mismatch"),
        "RGBA image buffer and diagnostics should live in gui/types/image.rs while behavior tests stay delegated"
    );
    assert!(
        tests.contains("fn image_rgba_try_new_reports_length_mismatch")
            && tests.contains("fn image_rgba_try_new_reports_dimension_overflow"),
        "image buffer behavior coverage should live in gui/types/image/tests.rs"
    );
}

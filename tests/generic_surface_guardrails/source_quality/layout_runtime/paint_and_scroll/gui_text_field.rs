use super::*;

#[test]
fn gui_text_field_paint_input_stays_grouped_by_paint_concern() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let facade = fs::read_to_string(manifest_dir.join("src/gui/paint.rs"))
        .expect("gui paint facade should be readable");
    let text_field = fs::read_to_string(manifest_dir.join("src/gui/paint/text_field.rs"))
        .expect("gui text-field paint helper should be readable");
    let tests = fs::read_to_string(manifest_dir.join("src/gui/paint/tests.rs"))
        .expect("gui paint behavior tests should be readable");

    assert!(
        text_field.contains("pub struct TextFieldPaintGeometry")
            && text_field.contains("pub struct TextFieldPaintContent")
            && text_field.contains("pub struct TextFieldPaintColors")
            && text_field.contains("pub struct TextFieldPaintStroke")
            && text_field.contains("pub struct TextFieldPaint")
            && facade.contains("TextFieldPaintGeometry")
            && facade.contains("TextFieldPaintColors"),
        "text-field paint inputs should stay grouped by geometry, content, colors, and stroke metrics"
    );
    assert!(
        tests.contains("fn text_field_paint_emits_chrome_selection_text_and_caret"),
        "text-field paint behavior coverage should stay in gui/paint/tests.rs"
    );
}

use super::*;

#[test]
fn api_docs_describe_named_parts_for_explicit_widget_construction() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let docs = fs::read_to_string(manifest_dir.join("docs/API.md"))
        .expect("docs/API.md should be readable");
    let widgets = fs::read_to_string(manifest_dir.join("src/widgets/mod.rs"))
        .expect("widgets module docs should be readable");
    let normalized_docs = docs.split_whitespace().collect::<Vec<_>>().join(" ");

    for required in [
        "Explicit widget construction should prefer named parts",
        "Each built-in primitive exports a `*WidgetParts` type",
        "`ButtonWidgetParts`",
        "`TextWidgetParts`",
        "`SliderWidgetParts`",
        "`CanvasWidgetParts`",
        "`ImageWidgetParts`",
        "`WidgetSizingParts`",
        "The positional `new(...)` constructors remain as compact compatibility helpers",
        "`from_parts(...)` is the clearest shape for examples, tests, and host code",
        "stable IDs, content, state, and layout contracts are named at the construction boundary",
    ] {
        assert!(
            normalized_docs.contains(required),
            "docs/API.md should document explicit widget named-parts guidance with `{required}`"
        );
    }
    for required in [
        "ButtonWidgetParts",
        "TextWidgetParts",
        "TextWidget::from_parts(TextWidgetParts {",
        "ButtonWidget::from_parts(ButtonWidgetParts {",
    ] {
        assert!(
            widgets.contains(required),
            "widgets module docs should demonstrate named-parts widget construction with `{required}`"
        );
    }
}

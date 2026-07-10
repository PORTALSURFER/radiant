use super::*;

#[test]
fn widget_sizing_uses_named_parts_for_intrinsic_bounds() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source_path = manifest_dir.join("src/widgets/contract/sizing.rs");
    let source = fs::read_to_string(&source_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));
    let contract = fs::read_to_string(manifest_dir.join("src/widgets/contract.rs"))
        .expect("widget contract module should be readable");
    let widgets = fs::read_to_string(manifest_dir.join("src/widgets/mod.rs"))
        .expect("widgets module should be readable");
    let prelude = public_prelude_source(&manifest_dir);

    assert!(
        source.contains("pub struct WidgetSizingParts")
            && source.contains("pub fn from_parts(parts: WidgetSizingParts) -> Self"),
        "widget sizing should expose named parts for minimum, preferred, and baseline values"
    );
    assert!(
        source.contains("Self::from_parts(WidgetSizingParts {")
            && contract.contains("WidgetSizingParts")
            && widgets.contains("WidgetSizingParts")
            && !prelude.contains("WidgetSizingParts"),
        "widget sizing named parts should remain public through widgets without entering the common prelude"
    );
}

#[test]
fn labeled_primitive_widgets_use_named_parts_for_identity_content_and_sizing() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let primitives_dir = manifest_dir.join("src/widgets/primitives");
    let widgets = fs::read_to_string(manifest_dir.join("src/widgets/mod.rs"))
        .expect("widgets module should be readable");

    for (file, parts, from_parts, wrapper) in [
        (
            "button.rs",
            "pub struct ButtonWidgetParts",
            "pub fn from_parts(parts: ButtonWidgetParts) -> Self",
            "Self::from_parts(ButtonWidgetParts {",
        ),
        (
            "toggle.rs",
            "pub struct ToggleWidgetParts",
            "pub fn from_parts(parts: ToggleWidgetParts) -> Self",
            "Self::from_parts(ToggleWidgetParts {",
        ),
        (
            "text.rs",
            "pub struct TextWidgetParts",
            "pub fn from_parts(parts: TextWidgetParts) -> Self",
            "Self::from_parts(TextWidgetParts {",
        ),
        (
            "badge.rs",
            "pub struct BadgeWidgetParts",
            "pub fn from_parts(parts: BadgeWidgetParts) -> Self",
            "Self::from_parts(BadgeWidgetParts {",
        ),
        (
            "list_item.rs",
            "pub struct ListItemWidgetParts",
            "pub fn from_parts(parts: ListItemWidgetParts) -> Self",
            "Self::from_parts(ListItemWidgetParts {",
        ),
        (
            "selectable.rs",
            "pub struct SelectableWidgetParts",
            "pub fn from_parts(parts: SelectableWidgetParts) -> Self",
            "Self::from_parts(SelectableWidgetParts {",
        ),
        (
            "text_input.rs",
            "pub struct TextInputWidgetParts",
            "pub fn from_parts(parts: TextInputWidgetParts) -> Self",
            "Self::from_parts(TextInputWidgetParts {",
        ),
        (
            "slider.rs",
            "pub struct SliderWidgetParts",
            "pub fn from_parts(parts: SliderWidgetParts) -> Self",
            "Self::from_parts(SliderWidgetParts {",
        ),
        (
            "scrollbar.rs",
            "pub struct ScrollbarWidgetParts",
            "pub fn from_parts(parts: ScrollbarWidgetParts) -> Self",
            "Self::from_parts(ScrollbarWidgetParts {",
        ),
        (
            "icon_button.rs",
            "pub struct IconButtonWidgetParts",
            "pub fn from_parts(parts: IconButtonWidgetParts) -> Self",
            "Self::from_parts(IconButtonWidgetParts {",
        ),
        (
            "image.rs",
            "pub struct ImageWidgetParts",
            "pub fn from_parts(parts: ImageWidgetParts) -> Self",
            "Self::from_parts(ImageWidgetParts {",
        ),
        (
            "canvas.rs",
            "pub struct CanvasWidgetParts",
            "pub fn from_parts(parts: CanvasWidgetParts) -> Self",
            "Self::from_parts(CanvasWidgetParts {",
        ),
        (
            "card.rs",
            "pub struct CardWidgetParts",
            "pub fn from_parts(parts: CardWidgetParts) -> Self",
            "Self::from_parts(CardWidgetParts {",
        ),
        (
            "drag_handle.rs",
            "pub struct DragHandleWidgetParts",
            "pub fn from_parts(parts: DragHandleWidgetParts) -> Self",
            "Self::from_parts(DragHandleWidgetParts {",
        ),
        (
            "interactive_row.rs",
            "pub struct InteractiveRowWidgetParts",
            "pub fn from_parts(parts: InteractiveRowWidgetParts) -> Self",
            "Self::from_parts(InteractiveRowWidgetParts {",
        ),
    ] {
        let source_path = primitives_dir.join(file);
        let source = fs::read_to_string(&source_path)
            .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));
        assert!(
            source.contains(parts) && source.contains(from_parts) && source.contains(wrapper),
            "labeled primitive widget in {file} should expose named parts and compatibility constructor"
        );
        assert!(
            widgets.contains(parts.trim_start_matches("pub struct ")),
            "widgets module should export {parts}"
        );
    }
}

#[test]
fn widget_common_keeps_support_tests_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/widgets/primitives/support/common.rs"))
        .expect("widget common support root should be readable");
    let tests =
        fs::read_to_string(manifest_dir.join("src/widgets/primitives/support/common/tests.rs"))
            .expect("widget common support tests should be readable");

    assert!(
        root.contains("#[path = \"common/tests.rs\"]")
            && root.contains("pub struct WidgetCommon")
            && root.contains("impl WidgetCommon")
            && !root.contains("fn focus_helpers_expose_pointer_hit_testing_intent"),
        "widget common support root should own the shared contract while delegating behavior tests"
    );
    assert!(
        tests.contains("fn focus_helpers_expose_pointer_hit_testing_intent")
            && tests.contains("with_pointer_focus()")
            && tests.contains("with_keyboard_focus()"),
        "widget common behavior tests should live in support/common/tests.rs"
    );
}

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
    let lib = fs::read_to_string(manifest_dir.join("src/lib.rs"))
        .expect("library module should be readable");

    assert!(
        source.contains("pub struct WidgetSizingParts")
            && source.contains("pub fn from_parts(parts: WidgetSizingParts) -> Self"),
        "widget sizing should expose named parts for minimum, preferred, and baseline values"
    );
    assert!(
        source.contains("Self::from_parts(WidgetSizingParts {")
            && contract.contains("WidgetSizingParts")
            && widgets.contains("WidgetSizingParts")
            && lib.contains("WidgetSizingParts"),
        "widget sizing compatibility constructor and public exports should keep the named-parts path available"
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

#[test]
fn scrollbar_primitive_keeps_surface_builders_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/widgets/primitives/scrollbar.rs"))
        .expect("scrollbar primitive root should be readable");
    let builders =
        fs::read_to_string(manifest_dir.join("src/widgets/primitives/scrollbar/builders.rs"))
            .expect("scrollbar primitive builders should be readable");
    let tests = fs::read_to_string(manifest_dir.join("src/widgets/primitives/scrollbar/tests.rs"))
        .expect("scrollbar primitive tests should be readable");

    assert!(
        root.contains("mod builders;")
            && root.contains("pub struct ScrollbarWidget")
            && root.contains("impl Widget for ScrollbarWidget")
            && root.contains("#[path = \"scrollbar/tests.rs\"]")
            && !root.contains("impl<Message> SurfaceNode<Message>")
            && !root.contains("impl<Message> WidgetMessageMapper<Message>")
            && !root.contains("fn scrollbar_drag_emits_clamped_offset_changes"),
        "scrollbar primitive root should own widget behavior while delegating runtime builders and behavior tests"
    );
    assert!(
        builders.contains("impl<Message> SurfaceNode<Message>")
            && builders.contains("pub fn scrollbar(")
            && builders.contains("pub fn scrollbar_mapped(")
            && builders.contains("impl<Message> WidgetMessageMapper<Message>"),
        "scrollbar runtime builder helpers should live in scrollbar/builders.rs"
    );
    assert!(
        tests.contains("fn scrollbar_drag_emits_clamped_offset_changes")
            && tests.contains("fn scrollbar_track_click_centers_thumb"),
        "scrollbar behavior tests should live in scrollbar/tests.rs"
    );
}

#[test]
fn drag_handle_primitive_keeps_surface_builders_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/widgets/primitives/drag_handle.rs"))
        .expect("drag-handle primitive root should be readable");
    let builders =
        fs::read_to_string(manifest_dir.join("src/widgets/primitives/drag_handle/builders.rs"))
            .expect("drag-handle primitive builders should be readable");

    assert!(
        root.contains("mod builders;")
            && root.contains("pub struct DragHandleWidget")
            && root.contains("impl Widget for DragHandleWidget")
            && !root.contains("impl<Message> SurfaceNode<Message>")
            && !root.contains("impl<Message> WidgetMessageMapper<Message>"),
        "drag-handle primitive root should own widget behavior and delegate runtime builders"
    );
    assert!(
        builders.contains("impl<Message> SurfaceNode<Message>")
            && builders.contains("pub fn drag_handle_mapped(")
            && builders.contains("impl<Message> WidgetMessageMapper<Message>")
            && builders.contains("pub fn drag_handle("),
        "drag-handle runtime builder helpers should live in drag_handle/builders.rs"
    );
}

#[test]
fn slider_primitive_keeps_surface_builders_and_tests_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/widgets/primitives/slider.rs"))
        .expect("slider primitive root should be readable");
    let builders =
        fs::read_to_string(manifest_dir.join("src/widgets/primitives/slider/builders.rs"))
            .expect("slider primitive builders should be readable");
    let tests = fs::read_to_string(manifest_dir.join("src/widgets/primitives/slider/tests.rs"))
        .expect("slider primitive tests should be readable");

    assert!(
        root.contains("mod builders;")
            && root.contains("pub struct SliderWidget")
            && root.contains("impl Widget for SliderWidget")
            && root.contains("#[path = \"slider/tests.rs\"]")
            && !root.contains("impl<Message> SurfaceNode<Message>")
            && !root.contains("impl<Message> WidgetMessageMapper<Message>")
            && !root.contains("fn slider_pointer_drag_emits_clamped_values"),
        "slider primitive root should own widget behavior while delegating runtime builders and behavior tests"
    );
    assert!(
        builders.contains("impl<Message> SurfaceNode<Message>")
            && builders.contains("pub fn slider(")
            && builders.contains("pub fn slider_mapped(")
            && builders.contains("impl<Message> WidgetMessageMapper<Message>"),
        "slider runtime builder helpers should live in slider/builders.rs"
    );
    assert!(
        tests.contains("fn slider_pointer_drag_emits_clamped_values")
            && tests.contains("fn focused_slider_responds_to_keyboard_steps"),
        "slider behavior tests should live in slider/tests.rs"
    );
}

#[test]
fn toggle_primitive_keeps_surface_builders_and_tests_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/widgets/primitives/toggle.rs"))
        .expect("toggle primitive root should be readable");
    let builders =
        fs::read_to_string(manifest_dir.join("src/widgets/primitives/toggle/builders.rs"))
            .expect("toggle primitive builders should be readable");
    let tests = fs::read_to_string(manifest_dir.join("src/widgets/primitives/toggle/tests.rs"))
        .expect("toggle primitive tests should be readable");

    assert!(
        root.contains("mod builders;")
            && root.contains("pub struct ToggleWidget")
            && root.contains("impl Widget for ToggleWidget")
            && root.contains("#[path = \"toggle/tests.rs\"]")
            && !root.contains("impl<Message> SurfaceNode<Message>")
            && !root.contains("impl<Message> WidgetMessageMapper<Message>")
            && !root.contains("fn toggle_keyboard_activation_flips_active_state"),
        "toggle primitive root should own widget behavior while delegating runtime builders and behavior tests"
    );
    assert!(
        builders.contains("impl<Message> SurfaceNode<Message>")
            && builders.contains("pub fn toggle(")
            && builders.contains("pub fn toggle_with_checked(")
            && builders.contains("pub fn toggle_mapped(")
            && builders.contains("pub fn toggle_mapped_with_checked(")
            && builders.contains("impl<Message> WidgetMessageMapper<Message>"),
        "toggle runtime builder helpers should live in toggle/builders.rs"
    );
    assert!(
        tests.contains("fn toggle_keyboard_activation_flips_active_state"),
        "toggle behavior tests should live in toggle/tests.rs"
    );
}

#[test]
fn badge_primitive_keeps_surface_builders_and_tests_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/widgets/primitives/badge.rs"))
        .expect("badge primitive root should be readable");
    let builders =
        fs::read_to_string(manifest_dir.join("src/widgets/primitives/badge/builders.rs"))
            .expect("badge primitive builders should be readable");
    let tests = fs::read_to_string(manifest_dir.join("src/widgets/primitives/badge/tests.rs"))
        .expect("badge primitive tests should be readable");

    assert!(
        root.contains("mod builders;")
            && root.contains("pub struct BadgeWidget")
            && root.contains("impl Widget for BadgeWidget")
            && root.contains("#[path = \"badge/tests.rs\"]")
            && !root.contains("impl<Message> SurfaceNode<Message>")
            && !root.contains("impl<Message> WidgetMessageMapper<Message>")
            && !root.contains("fn badge_releases_inside_bounds_emit_activation"),
        "badge primitive root should own widget behavior while delegating runtime builders and behavior tests"
    );
    assert!(
        builders.contains("impl<Message> SurfaceNode<Message>")
            && builders.contains("pub fn badge(")
            && builders.contains("pub fn badge_mapped(")
            && builders.contains("impl<Message> WidgetMessageMapper<Message>"),
        "badge runtime builder helpers should live in badge/builders.rs"
    );
    assert!(
        tests.contains("fn badge_releases_inside_bounds_emit_activation")
            && tests.contains("fn focused_badge_enter_emits_activation"),
        "badge behavior tests should live in badge/tests.rs"
    );
}

#[test]
fn button_primitive_keeps_surface_builders_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/widgets/primitives/button.rs"))
        .expect("button primitive root should be readable");
    let builders =
        fs::read_to_string(manifest_dir.join("src/widgets/primitives/button/builders.rs"))
            .expect("button primitive builders should be readable");
    let tests = fs::read_to_string(manifest_dir.join("src/widgets/primitives/button/tests.rs"))
        .expect("button primitive tests should be readable");

    assert!(
        root.contains("mod builders;")
            && root.contains("pub struct ButtonWidget")
            && root.contains("impl Widget for ButtonWidget")
            && root.contains("mod tests;")
            && !root.contains("impl<Message> SurfaceNode<Message>")
            && !root.contains("impl<Message> WidgetMessageMapper<Message>")
            && !root.contains("fn button_releases_inside_bounds_emit_activation"),
        "button primitive root should own widget behavior while delegating runtime builders and behavior tests"
    );
    assert!(
        builders.contains("impl<Message> SurfaceNode<Message>")
            && builders.contains("pub fn button(")
            && builders.contains("pub fn button_mapped(")
            && builders.contains("impl<Message> WidgetMessageMapper<Message>"),
        "button runtime builder helpers should live in button/builders.rs"
    );
    assert!(
        tests.contains("fn button_releases_inside_bounds_emit_activation")
            && tests.contains("fn focused_button_space_emits_activation"),
        "button behavior tests should stay in button/tests.rs"
    );
}

#[test]
fn icon_button_primitive_keeps_surface_mappers_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/widgets/primitives/icon_button.rs"))
        .expect("icon-button primitive root should be readable");
    let builders =
        fs::read_to_string(manifest_dir.join("src/widgets/primitives/icon_button/builders.rs"))
            .expect("icon-button primitive builders should be readable");

    assert!(
        root.contains("mod builders;")
            && root.contains("pub struct IconButtonWidget")
            && root.contains("impl Widget for IconButtonWidget")
            && !root.contains("impl<Message> WidgetMessageMapper<Message>"),
        "icon-button primitive root should own widget behavior and delegate runtime mappers"
    );
    assert!(
        builders.contains("impl<Message> WidgetMessageMapper<Message>")
            && builders.contains("pub fn icon_button("),
        "icon-button runtime mapper helper should live in icon_button/builders.rs"
    );
}

#[test]
fn interactive_row_primitive_keeps_surface_mappers_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/widgets/primitives/interactive_row.rs"))
        .expect("interactive-row primitive root should be readable");
    let builders =
        fs::read_to_string(manifest_dir.join("src/widgets/primitives/interactive_row/builders.rs"))
            .expect("interactive-row primitive builders should be readable");

    assert!(
        root.contains("mod builders;")
            && root.contains("pub struct InteractiveRowWidget")
            && root.contains("impl Widget for InteractiveRowWidget")
            && !root.contains("impl<Message> WidgetMessageMapper<Message>"),
        "interactive-row primitive root should own widget behavior and delegate runtime mappers"
    );
    assert!(
        builders.contains("impl<Message> WidgetMessageMapper<Message>")
            && builders.contains("pub fn interactive_row("),
        "interactive-row runtime mapper helper should live in interactive_row/builders.rs"
    );
}

#[test]
fn list_item_primitive_keeps_surface_builders_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/widgets/primitives/list_item.rs"))
        .expect("list item primitive root should be readable");
    let builders =
        fs::read_to_string(manifest_dir.join("src/widgets/primitives/list_item/builders.rs"))
            .expect("list item primitive builders should be readable");

    assert!(
        root.contains("mod builders;")
            && root.contains("pub struct ListItemWidget")
            && root.contains("impl Widget for ListItemWidget")
            && !root.contains("impl<Message> SurfaceNode<Message>")
            && !root.contains("impl<Message> WidgetMessageMapper<Message>"),
        "list item primitive root should own widget behavior while delegating runtime builders"
    );
    assert!(
        builders.contains("impl<Message> SurfaceNode<Message>")
            && builders.contains("pub fn list_item(")
            && builders.contains("pub fn list_item_action(")
            && builders.contains("pub fn list_item_mapped(")
            && builders.contains("impl<Message> WidgetMessageMapper<Message>"),
        "list item runtime builder helpers should live in list_item/builders.rs"
    );
}

#[test]
fn selectable_primitive_keeps_surface_builders_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/widgets/primitives/selectable.rs"))
        .expect("selectable primitive root should be readable");
    let builders =
        fs::read_to_string(manifest_dir.join("src/widgets/primitives/selectable/builders.rs"))
            .expect("selectable primitive builders should be readable");

    assert!(
        root.contains("mod builders;")
            && root.contains("pub struct SelectableWidget")
            && root.contains("impl Widget for SelectableWidget")
            && !root.contains("impl<Message> SurfaceNode<Message>")
            && !root.contains("impl<Message> WidgetMessageMapper<Message>"),
        "selectable primitive root should own widget behavior while delegating runtime builders"
    );
    assert!(
        builders.contains("impl<Message> SurfaceNode<Message>")
            && builders.contains("pub fn selectable(")
            && builders.contains("pub fn selectable_mapped(")
            && builders.contains("impl<Message> WidgetMessageMapper<Message>"),
        "selectable runtime builder helpers should live in selectable/builders.rs"
    );
}

#[test]
fn canvas_primitive_keeps_surface_builders_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/widgets/primitives/canvas.rs"))
        .expect("canvas primitive root should be readable");
    let builders =
        fs::read_to_string(manifest_dir.join("src/widgets/primitives/canvas/builders.rs"))
            .expect("canvas primitive builders should be readable");

    assert!(
        root.contains("mod builders;")
            && root.contains("pub struct CanvasWidget")
            && root.contains("pub struct RetainedSurfaceDescriptor")
            && root.contains("impl Widget for CanvasWidget")
            && !root.contains("impl<Message> SurfaceNode<Message>")
            && !root.contains("impl<Message> WidgetMessageMapper<Message>"),
        "canvas primitive root should own widget behavior and retained-surface contract while delegating runtime builders"
    );
    assert!(
        builders.contains("impl<Message> SurfaceNode<Message>")
            && builders.contains("pub fn canvas(")
            && builders.contains("pub fn canvas_mapped(")
            && builders.contains("pub fn retained_canvas_mapped(")
            && builders.contains("impl<Message> WidgetMessageMapper<Message>"),
        "canvas runtime builder helpers should live in canvas/builders.rs"
    );
}

#[test]
fn card_primitive_keeps_surface_builders_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/widgets/primitives/card.rs"))
        .expect("card primitive root should be readable");
    let builders = fs::read_to_string(manifest_dir.join("src/widgets/primitives/card/builders.rs"))
        .expect("card primitive builders should be readable");

    assert!(
        root.contains("mod builders;")
            && root.contains("pub struct CardWidget")
            && root.contains("impl Widget for CardWidget")
            && !root.contains("impl<Message> SurfaceNode<Message>"),
        "card primitive root should own widget behavior while delegating runtime builders"
    );
    assert!(
        builders.contains("impl<Message> SurfaceNode<Message>")
            && builders.contains("pub fn card("),
        "card runtime builder helper should live in card/builders.rs"
    );
}

#[test]
fn image_primitive_keeps_surface_builders_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/widgets/primitives/image.rs"))
        .expect("image primitive root should be readable");
    let builders =
        fs::read_to_string(manifest_dir.join("src/widgets/primitives/image/builders.rs"))
            .expect("image primitive builders should be readable");

    assert!(
        root.contains("mod builders;")
            && root.contains("pub struct ImageWidget")
            && root.contains("impl Widget for ImageWidget")
            && !root.contains("impl<Message> SurfaceNode<Message>"),
        "image primitive root should own widget behavior while delegating runtime builders"
    );
    assert!(
        builders.contains("impl<Message> SurfaceNode<Message>")
            && builders.contains("pub fn image("),
        "image runtime builder helper should live in image/builders.rs"
    );
}

#[test]
fn text_primitive_keeps_surface_builders_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/widgets/primitives/text.rs"))
        .expect("text primitive root should be readable");
    let builders = fs::read_to_string(manifest_dir.join("src/widgets/primitives/text/builders.rs"))
        .expect("text primitive builders should be readable");

    assert!(
        root.contains("mod builders;")
            && root.contains("pub struct TextWidget")
            && root.contains("impl Widget for TextWidget")
            && !root.contains("impl<Message> SurfaceNode<Message>"),
        "text primitive root should own widget behavior while delegating runtime builders"
    );
    assert!(
        builders.contains("impl<Message> SurfaceNode<Message>")
            && builders.contains("pub fn text("),
        "text runtime builder helper should live in text/builders.rs"
    );
}

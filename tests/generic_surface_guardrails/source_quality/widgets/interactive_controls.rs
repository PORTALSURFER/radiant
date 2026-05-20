use super::*;

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
